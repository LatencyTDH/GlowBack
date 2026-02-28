//! Risk metrics computation.
//!
//! [`RiskMetricsCalculator`] takes a portfolio snapshot and historical returns to
//! produce a [`PortfolioRiskSnapshot`] that captures the current risk posture.

use chrono::{DateTime, Utc};
use rust_decimal::prelude::Signed;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use gb_types::market::Symbol;
use gb_types::portfolio::{DailyReturn, Portfolio};

/// Per-position risk breakdown.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionRisk {
    pub symbol: Symbol,
    /// Position weight as fraction of total equity (signed: negative = short).
    pub weight: Decimal,
    /// Absolute weight (always positive).
    pub weight_abs: Decimal,
    /// Unrealized P&L for this position.
    pub unrealized_pnl: Decimal,
    /// Contribution to portfolio VaR (approximate, assumes independent).
    pub var_contribution: Decimal,
}

/// A point-in-time snapshot of portfolio-level risk metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortfolioRiskSnapshot {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,

    // --- exposure ---
    /// Sum of absolute position weights.
    pub gross_exposure: Decimal,
    /// Sum of signed position weights (long – short).
    pub net_exposure: Decimal,
    /// Gross exposure / equity.
    pub leverage: Decimal,
    /// Number of distinct positions.
    pub num_positions: usize,

    // --- drawdown ---
    /// Current drawdown from equity peak (0–1 fraction).
    pub current_drawdown: Decimal,
    /// Maximum drawdown observed over the supplied history.
    pub max_drawdown: Decimal,

    // --- VaR / tail ---
    /// 1-day 95% parametric VaR as a positive fraction of equity.
    pub var_95: Option<Decimal>,
    /// 1-day 95% Conditional VaR (expected shortfall).
    pub cvar_95: Option<Decimal>,

    // --- daily P&L ---
    /// Today's P&L as a fraction of starting equity.
    pub daily_pnl_pct: Decimal,

    // --- per-position ---
    pub position_risks: Vec<PositionRisk>,
}

/// Stateless calculator for risk metrics.
pub struct RiskMetricsCalculator;

impl RiskMetricsCalculator {
    /// Compute a full risk snapshot from the current portfolio and historical
    /// daily returns.
    pub fn compute(
        portfolio: &Portfolio,
        daily_returns: &[DailyReturn],
        equity_peak: Decimal,
    ) -> PortfolioRiskSnapshot {
        let equity = portfolio.total_equity;
        let safe_equity = if equity > Decimal::ZERO {
            equity
        } else {
            Decimal::ONE
        };

        // --- per-position metrics ---
        let mut position_risks = Vec::new();
        let mut gross_exposure = Decimal::ZERO;
        let mut net_exposure = Decimal::ZERO;

        for (symbol, pos) in &portfolio.positions {
            let weight = if safe_equity > Decimal::ZERO {
                pos.market_value * pos.quantity.signum() / safe_equity
            } else {
                Decimal::ZERO
            };
            let weight_abs = weight.abs();
            gross_exposure += weight_abs;
            net_exposure += weight;

            position_risks.push(PositionRisk {
                symbol: symbol.clone(),
                weight,
                weight_abs,
                unrealized_pnl: pos.unrealized_pnl,
                var_contribution: Decimal::ZERO, // filled below
            });
        }

        // --- drawdown ---
        let current_drawdown = if equity_peak > Decimal::ZERO {
            ((equity_peak - equity) / equity_peak).max(Decimal::ZERO)
        } else {
            Decimal::ZERO
        };
        let max_drawdown = Self::max_drawdown(daily_returns, portfolio.initial_capital);

        // --- VaR / CVaR ---
        let (var_95, cvar_95) = Self::compute_var_cvar(daily_returns);

        // Approximate per-position VaR contribution (weight * portfolio VaR).
        if let Some(total_var) = var_95 {
            for pr in &mut position_risks {
                pr.var_contribution = pr.weight_abs * total_var;
            }
        }

        // --- daily P&L ---
        let daily_pnl_pct = daily_returns
            .last()
            .map(|dr| dr.daily_return)
            .unwrap_or(Decimal::ZERO);

        let leverage = gross_exposure; // gross_exposure is already relative to equity

        PortfolioRiskSnapshot {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            gross_exposure,
            net_exposure,
            leverage,
            num_positions: portfolio.positions.len(),
            current_drawdown,
            max_drawdown,
            var_95,
            cvar_95,
            daily_pnl_pct,
            position_risks,
        }
    }

    /// Compute VaR (95%) and CVaR from daily return history.
    fn compute_var_cvar(daily_returns: &[DailyReturn]) -> (Option<Decimal>, Option<Decimal>) {
        if daily_returns.len() < 20 {
            return (None, None);
        }

        let mut returns: Vec<Decimal> = daily_returns.iter().map(|r| r.daily_return).collect();
        returns.sort();

        let idx = (returns.len() as f64 * 0.05) as usize;
        let var = -returns[idx]; // VaR as positive loss

        let tail = &returns[..=idx];
        let cvar = if !tail.is_empty() {
            let sum: Decimal = tail.iter().copied().sum();
            Some(-(sum / Decimal::from(tail.len())))
        } else {
            None
        };

        (Some(var), cvar)
    }

    /// Compute max drawdown from daily returns.
    fn max_drawdown(daily_returns: &[DailyReturn], initial_capital: Decimal) -> Decimal {
        let mut peak = initial_capital;
        let mut max_dd = Decimal::ZERO;

        for dr in daily_returns {
            if dr.portfolio_value > peak {
                peak = dr.portfolio_value;
            }
            if peak > Decimal::ZERO {
                let dd = (peak - dr.portfolio_value) / peak;
                if dd > max_dd {
                    max_dd = dd;
                }
            }
        }

        max_dd
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gb_types::market::{AssetClass, Symbol};
    use gb_types::portfolio::{DailyReturn, Portfolio, Position};
    use rust_decimal_macros::dec;

    fn sym(ticker: &str) -> Symbol {
        Symbol::new(ticker, "TEST", AssetClass::Equity)
    }

    fn make_portfolio(positions: Vec<(Symbol, Decimal, Decimal, Decimal)>) -> Portfolio {
        let mut p = Portfolio::new("test".into(), dec!(100_000));
        for (symbol, qty, avg_price, market_price) in positions {
            let mut pos = Position::new(symbol.clone());
            pos.quantity = qty;
            pos.average_price = avg_price;
            pos.update_market_price(market_price);
            p.positions.insert(symbol, pos);
        }
        // Recompute equity
        let market_value: Decimal = p.positions.values().map(|pos| pos.market_value).sum();
        p.total_equity = p.cash + market_value;
        p
    }

    fn make_returns(values: &[f64]) -> Vec<DailyReturn> {
        let base = Utc::now();
        values
            .iter()
            .enumerate()
            .map(|(i, &v)| DailyReturn {
                date: base + chrono::Duration::days(i as i64),
                portfolio_value: Decimal::from(100_000),
                daily_return: Decimal::from_f64_retain(v).unwrap_or_default(),
                cumulative_return: Decimal::ZERO,
            })
            .collect()
    }

    #[test]
    fn empty_portfolio_produces_zero_exposure() {
        let portfolio = Portfolio::new("test".into(), dec!(100_000));
        let snap = RiskMetricsCalculator::compute(&portfolio, &[], dec!(100_000));

        assert_eq!(snap.gross_exposure, dec!(0));
        assert_eq!(snap.net_exposure, dec!(0));
        assert_eq!(snap.leverage, dec!(0));
        assert_eq!(snap.num_positions, 0);
        assert_eq!(snap.current_drawdown, dec!(0));
    }

    #[test]
    fn single_long_position_metrics() {
        // 100 shares at $100 = $10,000 market value, $100k equity ⇒ 10% weight
        let portfolio = make_portfolio(vec![(sym("AAPL"), dec!(100), dec!(95), dec!(100))]);
        let snap = RiskMetricsCalculator::compute(&portfolio, &[], dec!(110_000));

        // Weight ≈ 10,000 / 110,000 (equity is cash + positions)
        assert_eq!(snap.num_positions, 1);
        assert!(snap.gross_exposure > dec!(0));
        assert_eq!(snap.gross_exposure, snap.net_exposure); // single long
    }

    #[test]
    fn drawdown_computation() {
        let portfolio = Portfolio::new("test".into(), dec!(100_000));
        let peak = dec!(120_000);
        // Equity dropped to 100k from 120k peak ⇒ 16.67% drawdown
        let snap = RiskMetricsCalculator::compute(&portfolio, &[], peak);
        let expected_dd = (dec!(120_000) - dec!(100_000)) / dec!(120_000);
        assert_eq!(snap.current_drawdown, expected_dd);
    }

    #[test]
    fn var_needs_enough_data() {
        let portfolio = Portfolio::new("test".into(), dec!(100_000));
        let few_returns = make_returns(&[0.01, -0.005, 0.002]);
        let snap = RiskMetricsCalculator::compute(&portfolio, &few_returns, dec!(100_000));
        assert!(snap.var_95.is_none());
    }

    #[test]
    fn var_computed_with_enough_data() {
        let portfolio = Portfolio::new("test".into(), dec!(100_000));
        let returns = make_returns(&vec![0.01; 30]); // 30 days of +1%
        let snap = RiskMetricsCalculator::compute(&portfolio, &returns, dec!(100_000));
        // All returns identical and positive ⇒ VaR is negative of 5th percentile = -0.01
        // which means VaR = -(-0.01) = actually that's 0.01 inverted... let's verify
        // returns sorted ascending = all 0.01, idx=1, var = -returns[1] = -0.01
        // Hmm, all positive returns → VaR should be negative (no loss).
        // That's fine — it means the 95% daily loss floor is actually a gain.
        assert!(snap.var_95.is_some());
    }

    #[test]
    fn max_drawdown_from_history() {
        let base = Utc::now();
        let returns = vec![
            DailyReturn {
                date: base,
                portfolio_value: dec!(100_000),
                daily_return: dec!(0),
                cumulative_return: dec!(0),
            },
            DailyReturn {
                date: base + chrono::Duration::days(1),
                portfolio_value: dec!(110_000),
                daily_return: dec!(0.10),
                cumulative_return: dec!(0.10),
            },
            DailyReturn {
                date: base + chrono::Duration::days(2),
                portfolio_value: dec!(99_000),
                daily_return: dec!(-0.10),
                cumulative_return: dec!(-0.01),
            },
            DailyReturn {
                date: base + chrono::Duration::days(3),
                portfolio_value: dec!(105_000),
                daily_return: dec!(0.06),
                cumulative_return: dec!(0.05),
            },
        ];
        let portfolio = Portfolio::new("test".into(), dec!(100_000));
        let snap = RiskMetricsCalculator::compute(&portfolio, &returns, dec!(110_000));
        // Peak was 110k, trough was 99k ⇒ dd = 11/110 = 10%
        assert_eq!(snap.max_drawdown, dec!(11_000) / dec!(110_000));
    }

    #[test]
    fn position_risks_populated() {
        let portfolio = make_portfolio(vec![
            (sym("AAPL"), dec!(50), dec!(100), dec!(105)),
            (sym("GOOG"), dec!(20), dec!(200), dec!(190)),
        ]);
        let snap = RiskMetricsCalculator::compute(&portfolio, &[], dec!(100_000));
        assert_eq!(snap.position_risks.len(), 2);
        // All weights should be positive for long positions
        for pr in &snap.position_risks {
            assert!(pr.weight > dec!(0));
        }
    }

    #[test]
    fn snapshot_serialization_roundtrip() {
        let portfolio = Portfolio::new("test".into(), dec!(100_000));
        let snap = RiskMetricsCalculator::compute(&portfolio, &[], dec!(100_000));
        let json = serde_json::to_string(&snap).unwrap();
        let deserialized: PortfolioRiskSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(snap.gross_exposure, deserialized.gross_exposure);
        assert_eq!(snap.current_drawdown, deserialized.current_drawdown);
    }
}
