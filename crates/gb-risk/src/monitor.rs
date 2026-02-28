//! Risk monitor — continuous evaluation loop with alert emission.
//!
//! The [`RiskMonitor`] accepts portfolio updates and market events, recomputes
//! risk metrics, checks configurable limits, and emits [`RiskAlert`]s via a
//! channel.

use crossbeam_channel::Sender;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use gb_types::portfolio::{DailyReturn, Portfolio, RiskLimits};

use crate::alerts::{RiskAlert, RiskAlertKind, RiskSeverity};
use crate::metrics::{PortfolioRiskSnapshot, RiskMetricsCalculator};

/// Configuration for the risk monitor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskMonitorConfig {
    pub risk_limits: RiskLimits,
    /// Fraction of the limit at which a *warning* fires (e.g. 0.80 = 80%).
    pub warning_threshold_pct: Decimal,
    /// Maximum gross exposure (as fraction of equity). `None` = unlimited.
    pub max_gross_exposure: Option<Decimal>,
    /// Maximum portfolio-level VaR (95%, 1-day) as fraction.
    pub max_var_95: Option<Decimal>,
}

impl Default for RiskMonitorConfig {
    fn default() -> Self {
        Self {
            risk_limits: RiskLimits::default(),
            warning_threshold_pct: Decimal::new(80, 2), // 80%
            max_gross_exposure: Some(Decimal::from(3)), // 300% gross
            max_var_95: Some(Decimal::new(5, 2)),       // 5%
        }
    }
}

/// Real-time risk monitor.
///
/// Call [`RiskMonitor::update`] after every portfolio change or market tick.
/// Alerts are emitted on the channel supplied at construction time.
pub struct RiskMonitor {
    config: RiskMonitorConfig,
    alert_tx: Sender<RiskAlert>,
    equity_peak: Decimal,
    daily_returns: Vec<DailyReturn>,
    last_snapshot: Option<PortfolioRiskSnapshot>,
}

impl RiskMonitor {
    /// Create a new risk monitor.
    pub fn new(config: RiskMonitorConfig, alert_tx: Sender<RiskAlert>) -> Self {
        Self {
            config,
            alert_tx,
            equity_peak: Decimal::ZERO,
            daily_returns: Vec::new(),
            last_snapshot: None,
        }
    }

    /// Replace the current daily-return history (e.g. after warm-up / backtest
    /// reset).
    pub fn set_daily_returns(&mut self, returns: Vec<DailyReturn>) {
        self.daily_returns = returns;
    }

    /// Append a single daily return observation.
    pub fn push_daily_return(&mut self, dr: DailyReturn) {
        self.daily_returns.push(dr);
    }

    /// Update the equity high-water mark.
    pub fn set_equity_peak(&mut self, peak: Decimal) {
        self.equity_peak = peak;
    }

    /// Get the most recently computed risk snapshot, if any.
    pub fn last_snapshot(&self) -> Option<&PortfolioRiskSnapshot> {
        self.last_snapshot.as_ref()
    }

    /// Main entry point: recompute risk metrics, check limits, emit alerts.
    ///
    /// Returns the freshly computed snapshot.
    pub fn update(&mut self, portfolio: &Portfolio) -> PortfolioRiskSnapshot {
        // Track peak
        if portfolio.total_equity > self.equity_peak {
            self.equity_peak = portfolio.total_equity;
        }

        let snapshot =
            RiskMetricsCalculator::compute(portfolio, &self.daily_returns, self.equity_peak);

        self.check_limits(&snapshot, portfolio);

        self.last_snapshot = Some(snapshot.clone());
        snapshot
    }

    // ---- internal limit checks ----

    fn check_limits(&self, snap: &PortfolioRiskSnapshot, portfolio: &Portfolio) {
        let limits = &self.config.risk_limits;

        // --- daily loss ---
        self.check_daily_loss(snap, limits);

        // --- drawdown ---
        self.check_drawdown(snap, limits);

        // --- concentration ---
        self.check_concentration(snap, limits, portfolio);

        // --- leverage ---
        self.check_leverage(snap, limits);

        // --- gross exposure ---
        self.check_gross_exposure(snap);

        // --- VaR ---
        self.check_var(snap);
    }

    fn check_daily_loss(&self, snap: &PortfolioRiskSnapshot, limits: &RiskLimits) {
        let loss_pct = -snap.daily_pnl_pct; // positive when losing
        if loss_pct <= Decimal::ZERO {
            return;
        }

        let limit = limits.max_daily_loss;
        if loss_pct >= limit {
            self.emit(RiskAlert::new(
                RiskSeverity::Critical,
                RiskAlertKind::DailyLossExceeded {
                    current_loss_pct: loss_pct,
                    limit_pct: limit,
                },
                format!(
                    "Daily loss {:.2}% exceeds {:.2}% limit",
                    loss_pct * Decimal::from(100),
                    limit * Decimal::from(100),
                ),
            ));
        } else if loss_pct >= limit * self.config.warning_threshold_pct {
            self.emit(RiskAlert::new(
                RiskSeverity::Warning,
                RiskAlertKind::DailyLossExceeded {
                    current_loss_pct: loss_pct,
                    limit_pct: limit,
                },
                format!(
                    "Daily loss {:.2}% approaching {:.2}% limit",
                    loss_pct * Decimal::from(100),
                    limit * Decimal::from(100),
                ),
            ));
        }
    }

    fn check_drawdown(&self, snap: &PortfolioRiskSnapshot, limits: &RiskLimits) {
        let dd = snap.current_drawdown;
        let limit = limits.max_drawdown;

        if dd >= limit {
            self.emit(RiskAlert::new(
                RiskSeverity::Critical,
                RiskAlertKind::DrawdownExceeded {
                    current_drawdown_pct: dd,
                    limit_pct: limit,
                },
                format!(
                    "Drawdown {:.2}% exceeds {:.2}% limit",
                    dd * Decimal::from(100),
                    limit * Decimal::from(100),
                ),
            ));
        } else if dd >= limit * self.config.warning_threshold_pct {
            self.emit(RiskAlert::new(
                RiskSeverity::Warning,
                RiskAlertKind::DrawdownExceeded {
                    current_drawdown_pct: dd,
                    limit_pct: limit,
                },
                format!(
                    "Drawdown {:.2}% approaching {:.2}% limit",
                    dd * Decimal::from(100),
                    limit * Decimal::from(100),
                ),
            ));
        }
    }

    fn check_concentration(
        &self,
        snap: &PortfolioRiskSnapshot,
        limits: &RiskLimits,
        _portfolio: &Portfolio,
    ) {
        let limit = limits.position_concentration_limit;
        for pr in &snap.position_risks {
            if pr.weight_abs >= limit {
                self.emit(RiskAlert::new(
                    RiskSeverity::Critical,
                    RiskAlertKind::ConcentrationExceeded {
                        symbol: format!("{}", pr.symbol.symbol),
                        weight_pct: pr.weight_abs,
                        limit_pct: limit,
                    },
                    format!(
                        "Position {} at {:.2}% exceeds {:.2}% concentration limit",
                        pr.symbol.symbol,
                        pr.weight_abs * Decimal::from(100),
                        limit * Decimal::from(100),
                    ),
                ));
            } else if pr.weight_abs >= limit * self.config.warning_threshold_pct {
                self.emit(RiskAlert::new(
                    RiskSeverity::Warning,
                    RiskAlertKind::ConcentrationExceeded {
                        symbol: format!("{}", pr.symbol.symbol),
                        weight_pct: pr.weight_abs,
                        limit_pct: limit,
                    },
                    format!(
                        "Position {} at {:.2}% approaching {:.2}% concentration limit",
                        pr.symbol.symbol,
                        pr.weight_abs * Decimal::from(100),
                        limit * Decimal::from(100),
                    ),
                ));
            }
        }
    }

    fn check_leverage(&self, snap: &PortfolioRiskSnapshot, limits: &RiskLimits) {
        let lev = snap.leverage;
        let limit = limits.max_portfolio_leverage;

        if lev >= limit {
            self.emit(RiskAlert::new(
                RiskSeverity::Critical,
                RiskAlertKind::LeverageExceeded {
                    current_leverage: lev,
                    limit,
                },
                format!("Leverage {:.2}x exceeds {:.2}x limit", lev, limit),
            ));
        } else if lev >= limit * self.config.warning_threshold_pct {
            self.emit(RiskAlert::new(
                RiskSeverity::Warning,
                RiskAlertKind::LeverageExceeded {
                    current_leverage: lev,
                    limit,
                },
                format!("Leverage {:.2}x approaching {:.2}x limit", lev, limit),
            ));
        }
    }

    fn check_gross_exposure(&self, snap: &PortfolioRiskSnapshot) {
        if let Some(limit) = self.config.max_gross_exposure {
            let ge = snap.gross_exposure;
            if ge >= limit {
                self.emit(RiskAlert::new(
                    RiskSeverity::Critical,
                    RiskAlertKind::GrossExposureExceeded {
                        gross_exposure: ge,
                        limit,
                    },
                    format!("Gross exposure {:.2} exceeds {:.2} limit", ge, limit),
                ));
            } else if ge >= limit * self.config.warning_threshold_pct {
                self.emit(RiskAlert::new(
                    RiskSeverity::Warning,
                    RiskAlertKind::GrossExposureExceeded {
                        gross_exposure: ge,
                        limit,
                    },
                    format!("Gross exposure {:.2} approaching {:.2} limit", ge, limit),
                ));
            }
        }
    }

    fn check_var(&self, snap: &PortfolioRiskSnapshot) {
        if let (Some(limit), Some(var)) = (self.config.max_var_95, snap.var_95) {
            if var >= limit {
                self.emit(RiskAlert::new(
                    RiskSeverity::Critical,
                    RiskAlertKind::VarExceeded {
                        var_pct: var,
                        limit_pct: limit,
                    },
                    format!(
                        "VaR(95%) {:.2}% exceeds {:.2}% limit",
                        var * Decimal::from(100),
                        limit * Decimal::from(100),
                    ),
                ));
            } else if var >= limit * self.config.warning_threshold_pct {
                self.emit(RiskAlert::new(
                    RiskSeverity::Warning,
                    RiskAlertKind::VarExceeded {
                        var_pct: var,
                        limit_pct: limit,
                    },
                    format!(
                        "VaR(95%) {:.2}% approaching {:.2}% limit",
                        var * Decimal::from(100),
                        limit * Decimal::from(100),
                    ),
                ));
            }
        }
    }

    fn emit(&self, alert: RiskAlert) {
        match alert.severity {
            RiskSeverity::Critical => warn!(%alert.message, "RISK CRITICAL"),
            RiskSeverity::Warning => warn!(%alert.message, "RISK WARNING"),
            RiskSeverity::Info => info!(%alert.message, "RISK INFO"),
        }
        // Best-effort send; if receiver is dropped we just log.
        let _ = self.alert_tx.try_send(alert);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crossbeam_channel::unbounded;
    use gb_types::market::{AssetClass, Symbol};
    use gb_types::portfolio::{Portfolio, Position};
    use rust_decimal_macros::dec;

    fn sym(ticker: &str) -> Symbol {
        Symbol::new(ticker, "TEST", AssetClass::Equity)
    }

    fn make_portfolio_with_positions(
        positions: Vec<(Symbol, Decimal, Decimal, Decimal)>,
    ) -> Portfolio {
        let mut p = Portfolio::new("test".into(), dec!(100_000));
        for (symbol, qty, avg_price, market_price) in positions {
            let mut pos = Position::new(symbol.clone());
            pos.quantity = qty;
            pos.average_price = avg_price;
            pos.update_market_price(market_price);
            p.positions.insert(symbol, pos);
        }
        let mv: Decimal = p.positions.values().map(|p| p.market_value).sum();
        p.total_equity = p.cash + mv;
        p
    }

    #[test]
    fn no_alerts_on_empty_portfolio() {
        let (tx, rx) = unbounded();
        let mut monitor = RiskMonitor::new(RiskMonitorConfig::default(), tx);
        let portfolio = Portfolio::new("test".into(), dec!(100_000));
        let snap = monitor.update(&portfolio);

        assert_eq!(snap.num_positions, 0);
        assert!(rx.try_recv().is_err()); // No alerts
    }

    #[test]
    fn drawdown_alert_fires() {
        let (tx, rx) = unbounded();
        let mut config = RiskMonitorConfig::default();
        config.risk_limits.max_drawdown = dec!(0.10); // 10% limit
        let mut monitor = RiskMonitor::new(config, tx);
        monitor.set_equity_peak(dec!(120_000)); // Peak was 120k

        // Current equity is 100k ⇒ 16.7% drawdown → should fire critical
        let portfolio = Portfolio::new("test".into(), dec!(100_000));
        monitor.update(&portfolio);

        let alert = rx.try_recv().expect("expected drawdown alert");
        assert_eq!(alert.severity, RiskSeverity::Critical);
        assert!(matches!(alert.kind, RiskAlertKind::DrawdownExceeded { .. }));
    }

    #[test]
    fn drawdown_warning_fires() {
        let (tx, rx) = unbounded();
        let mut config = RiskMonitorConfig::default();
        config.risk_limits.max_drawdown = dec!(0.20); // 20% limit
        config.warning_threshold_pct = dec!(0.80); // warn at 80%
        let mut monitor = RiskMonitor::new(config, tx);
        monitor.set_equity_peak(dec!(110_000)); // Peak was 110k

        // Equity 92k → drawdown = 18/110 ≈ 16.4% → 16.4/20 = 82% of limit → warning
        let mut portfolio = Portfolio::new("test".into(), dec!(92_000));
        portfolio.total_equity = dec!(92_000);
        portfolio.cash = dec!(92_000);
        monitor.update(&portfolio);

        let alert = rx.try_recv().expect("expected drawdown warning");
        assert_eq!(alert.severity, RiskSeverity::Warning);
    }

    #[test]
    fn concentration_alert_fires() {
        let (tx, rx) = unbounded();
        let mut config = RiskMonitorConfig::default();
        config.risk_limits.position_concentration_limit = dec!(0.25); // 25%
        let mut monitor = RiskMonitor::new(config, tx);

        // Position is 50k / 150k equity ≈ 33% → exceeds 25%
        let portfolio = make_portfolio_with_positions(vec![
            (sym("AAPL"), dec!(500), dec!(100), dec!(100)), // 50k market value
        ]);
        monitor.update(&portfolio);

        let alert = rx.try_recv().expect("expected concentration alert");
        assert!(matches!(
            alert.kind,
            RiskAlertKind::ConcentrationExceeded { .. }
        ));
    }

    #[test]
    fn daily_loss_alert_with_returns() {
        let (tx, rx) = unbounded();
        let mut config = RiskMonitorConfig::default();
        config.risk_limits.max_daily_loss = dec!(0.03); // 3%
        let mut monitor = RiskMonitor::new(config, tx);

        // Simulate a -4% daily loss
        let dr = DailyReturn {
            date: Utc::now(),
            portfolio_value: dec!(96_000),
            daily_return: dec!(-0.04),
            cumulative_return: dec!(-0.04),
        };
        monitor.push_daily_return(dr);

        let portfolio = Portfolio::new("test".into(), dec!(96_000));
        monitor.update(&portfolio);

        let alert = rx.try_recv().expect("expected daily loss alert");
        assert_eq!(alert.severity, RiskSeverity::Critical);
        assert!(matches!(
            alert.kind,
            RiskAlertKind::DailyLossExceeded { .. }
        ));
    }

    #[test]
    fn equity_peak_auto_updated() {
        let (tx, _rx) = unbounded();
        let mut monitor = RiskMonitor::new(RiskMonitorConfig::default(), tx);
        assert_eq!(monitor.equity_peak, dec!(0));

        let portfolio = Portfolio::new("test".into(), dec!(100_000));
        monitor.update(&portfolio);
        assert_eq!(monitor.equity_peak, dec!(100_000));

        // Simulate equity increase
        let mut p2 = Portfolio::new("test".into(), dec!(110_000));
        p2.total_equity = dec!(110_000);
        p2.cash = dec!(110_000);
        monitor.update(&p2);
        assert_eq!(monitor.equity_peak, dec!(110_000));
    }

    #[test]
    fn last_snapshot_updates() {
        let (tx, _rx) = unbounded();
        let mut monitor = RiskMonitor::new(RiskMonitorConfig::default(), tx);
        assert!(monitor.last_snapshot().is_none());

        let portfolio = Portfolio::new("test".into(), dec!(100_000));
        monitor.update(&portfolio);
        assert!(monitor.last_snapshot().is_some());
    }
}
