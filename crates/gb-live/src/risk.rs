//! Pre-trade risk controls and circuit breakers for live trading.

use chrono::{DateTime, Duration, Utc};
use gb_types::market::Symbol;
use gb_types::orders::{Order, Side};
use gb_types::portfolio::RiskLimits;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::warn;

/// Result of a risk check — either the order passes or it is rejected with a
/// human-readable reason.
#[derive(Debug, Clone, PartialEq)]
pub enum RiskCheckResult {
    Approved,
    Rejected { reason: String },
}

impl RiskCheckResult {
    pub fn is_approved(&self) -> bool {
        matches!(self, RiskCheckResult::Approved)
    }
}

/// Configuration for the live risk manager.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskConfig {
    /// Base risk limits (max position size, leverage, etc.)
    pub limits: RiskLimits,

    /// Maximum number of orders per rolling window.
    pub max_orders_per_window: u32,
    /// Rolling window size in seconds.
    pub order_window_seconds: u64,

    /// Maximum notional value of a single order.
    pub max_order_notional: Decimal,
    /// Maximum total notional exposure across all positions.
    pub max_total_exposure: Decimal,

    /// If the portfolio loses more than this fraction of starting equity in a
    /// single day, halt all trading (circuit breaker).
    pub daily_loss_circuit_breaker: Decimal,

    /// When true, log rejections as warnings but still allow the order through.
    /// Useful during initial deployment to observe the risk engine.
    pub dry_run: bool,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            limits: RiskLimits::default(),
            max_orders_per_window: 100,
            order_window_seconds: 60,
            max_order_notional: Decimal::from(100_000),
            max_total_exposure: Decimal::from(500_000),
            daily_loss_circuit_breaker: Decimal::new(5, 2), // 5%
            dry_run: false,
        }
    }
}

/// Tracks live-session state needed for risk decisions.
#[derive(Debug)]
struct SessionState {
    /// Timestamps of recent order submissions (for rate limiting).
    recent_orders: Vec<DateTime<Utc>>,
    /// Position quantities keyed by symbol.
    positions: HashMap<Symbol, Decimal>,
    /// Starting equity for the current trading day.
    start_of_day_equity: Decimal,
    /// Whether the circuit breaker has been tripped.
    circuit_breaker_tripped: bool,
    /// The time at which the circuit breaker was tripped, if at all.
    circuit_breaker_tripped_at: Option<DateTime<Utc>>,
}

/// Live risk manager that validates every order before it reaches the broker.
#[derive(Debug)]
pub struct RiskManager {
    config: RiskConfig,
    state: SessionState,
}

impl RiskManager {
    /// Create a new risk manager with the given configuration and starting
    /// equity (used for the daily-loss circuit breaker).
    pub fn new(config: RiskConfig, starting_equity: Decimal) -> Self {
        Self {
            config,
            state: SessionState {
                recent_orders: Vec::new(),
                positions: HashMap::new(),
                start_of_day_equity: starting_equity,
                circuit_breaker_tripped: false,
                circuit_breaker_tripped_at: None,
            },
        }
    }

    /// Validate an order against all risk rules.  Returns [`RiskCheckResult::Approved`]
    /// or [`RiskCheckResult::Rejected`].
    ///
    /// When `dry_run` is enabled in the config, rejected orders are logged but
    /// returned as approved so the caller can observe without blocking.
    pub fn check_order(
        &mut self,
        order: &Order,
        current_price: Decimal,
        current_equity: Decimal,
    ) -> RiskCheckResult {
        let result = self.run_checks(order, current_price, current_equity);

        if let RiskCheckResult::Rejected { ref reason } = result {
            if self.config.dry_run {
                warn!(
                    order_id = %order.id,
                    symbol = %order.symbol,
                    reason = %reason,
                    "risk check WOULD reject (dry-run mode)"
                );
                return RiskCheckResult::Approved;
            }
        }

        result
    }

    /// Run all individual checks in sequence, short-circuiting on the first
    /// rejection.
    fn run_checks(
        &mut self,
        order: &Order,
        current_price: Decimal,
        current_equity: Decimal,
    ) -> RiskCheckResult {
        // 1) Circuit breaker
        if let result @ RiskCheckResult::Rejected { .. } =
            self.check_circuit_breaker(current_equity)
        {
            return result;
        }

        // 2) Order rate limit
        if let result @ RiskCheckResult::Rejected { .. } = self.check_order_rate() {
            return result;
        }

        // 3) Single-order notional limit
        let notional = order.quantity * current_price;
        if notional > self.config.max_order_notional {
            return RiskCheckResult::Rejected {
                reason: format!(
                    "order notional {notional} exceeds limit {}",
                    self.config.max_order_notional
                ),
            };
        }

        // 4) Position concentration
        if let result @ RiskCheckResult::Rejected { .. } =
            self.check_position_concentration(order, current_price, current_equity)
        {
            return result;
        }

        // 5) Total exposure
        if let result @ RiskCheckResult::Rejected { .. } =
            self.check_total_exposure(order, current_price)
        {
            return result;
        }

        // All checks passed — record the order timestamp for rate limiting.
        self.state.recent_orders.push(Utc::now());

        RiskCheckResult::Approved
    }

    // -- individual checks --------------------------------------------------

    fn check_circuit_breaker(&mut self, current_equity: Decimal) -> RiskCheckResult {
        if self.state.circuit_breaker_tripped {
            return RiskCheckResult::Rejected {
                reason: "circuit breaker tripped — trading halted for the day".into(),
            };
        }

        if self.state.start_of_day_equity > Decimal::ZERO {
            let loss_pct =
                (self.state.start_of_day_equity - current_equity) / self.state.start_of_day_equity;
            if loss_pct >= self.config.daily_loss_circuit_breaker {
                self.state.circuit_breaker_tripped = true;
                self.state.circuit_breaker_tripped_at = Some(Utc::now());
                warn!(
                    loss_pct = %loss_pct,
                    threshold = %self.config.daily_loss_circuit_breaker,
                    "daily loss circuit breaker tripped"
                );
                return RiskCheckResult::Rejected {
                    reason: format!(
                        "daily loss {loss_pct} exceeds circuit breaker threshold {}",
                        self.config.daily_loss_circuit_breaker
                    ),
                };
            }
        }

        RiskCheckResult::Approved
    }

    fn check_order_rate(&mut self) -> RiskCheckResult {
        let window = Duration::seconds(self.config.order_window_seconds as i64);
        let cutoff = Utc::now() - window;

        // Prune old entries
        self.state.recent_orders.retain(|t| *t >= cutoff);

        if self.state.recent_orders.len() >= self.config.max_orders_per_window as usize {
            return RiskCheckResult::Rejected {
                reason: format!(
                    "order rate limit: {} orders in {} s window",
                    self.config.max_orders_per_window, self.config.order_window_seconds
                ),
            };
        }

        RiskCheckResult::Approved
    }

    fn check_position_concentration(
        &self,
        order: &Order,
        current_price: Decimal,
        current_equity: Decimal,
    ) -> RiskCheckResult {
        if current_equity == Decimal::ZERO {
            return RiskCheckResult::Approved;
        }

        let current_qty = self
            .state
            .positions
            .get(&order.symbol)
            .copied()
            .unwrap_or(Decimal::ZERO);

        let delta = match order.side {
            Side::Buy => order.quantity,
            Side::Sell => -order.quantity,
        };

        let new_qty = current_qty + delta;
        let position_value = new_qty.abs() * current_price;
        let concentration = position_value / current_equity;

        if concentration > self.config.limits.position_concentration_limit {
            return RiskCheckResult::Rejected {
                reason: format!(
                    "position concentration {concentration:.2} exceeds limit {}",
                    self.config.limits.position_concentration_limit
                ),
            };
        }

        RiskCheckResult::Approved
    }

    fn check_total_exposure(&self, order: &Order, current_price: Decimal) -> RiskCheckResult {
        let order_notional = order.quantity * current_price;

        let existing_exposure: Decimal = self
            .state
            .positions
            .values()
            .map(|q| q.abs() * current_price) // simplified: uses same price
            .sum();

        let new_exposure = existing_exposure + order_notional;

        if new_exposure > self.config.max_total_exposure {
            return RiskCheckResult::Rejected {
                reason: format!(
                    "total exposure {new_exposure} would exceed limit {}",
                    self.config.max_total_exposure
                ),
            };
        }

        RiskCheckResult::Approved
    }

    // -- state updates called by the engine ---------------------------------

    /// Update internal position tracking after a fill.
    pub fn update_position(&mut self, symbol: &Symbol, side: Side, quantity: Decimal) {
        let entry = self
            .state
            .positions
            .entry(symbol.clone())
            .or_insert(Decimal::ZERO);
        match side {
            Side::Buy => *entry += quantity,
            Side::Sell => *entry -= quantity,
        }
    }

    /// Reset the start-of-day equity (call at market open / start of session).
    pub fn reset_daily(&mut self, equity: Decimal) {
        self.state.start_of_day_equity = equity;
        self.state.circuit_breaker_tripped = false;
        self.state.circuit_breaker_tripped_at = None;
        self.state.recent_orders.clear();
    }

    /// Returns `true` if the circuit breaker is currently tripped.
    pub fn is_circuit_breaker_tripped(&self) -> bool {
        self.state.circuit_breaker_tripped
    }

    /// Returns the time at which the circuit breaker was tripped, if at all.
    pub fn circuit_breaker_tripped_at(&self) -> Option<DateTime<Utc>> {
        self.state.circuit_breaker_tripped_at
    }

    /// Returns a reference to the current risk configuration.
    pub fn config(&self) -> &RiskConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gb_types::market::{AssetClass, Symbol};
    use gb_types::orders::{Order, Side};
    use rust_decimal_macros::dec;

    fn test_symbol() -> Symbol {
        Symbol::new("AAPL", "NASDAQ", AssetClass::Equity)
    }

    fn default_risk_manager() -> RiskManager {
        RiskManager::new(RiskConfig::default(), dec!(100_000))
    }

    #[test]
    fn test_order_passes_basic_checks() {
        let mut rm = default_risk_manager();
        let order = Order::market_order(test_symbol(), Side::Buy, dec!(10), "test".into());
        let result = rm.check_order(&order, dec!(150), dec!(100_000));
        assert!(result.is_approved());
    }

    #[test]
    fn test_max_order_notional_rejection() {
        let mut rm = default_risk_manager();
        // 1000 shares * $150 = $150k > default $100k limit
        let order = Order::market_order(test_symbol(), Side::Buy, dec!(1000), "test".into());
        let result = rm.check_order(&order, dec!(150), dec!(100_000));
        assert!(!result.is_approved());
    }

    #[test]
    fn test_circuit_breaker_trips_on_daily_loss() {
        let config = RiskConfig {
            daily_loss_circuit_breaker: dec!(0.05),
            ..Default::default()
        };
        let mut rm = RiskManager::new(config, dec!(100_000));

        let order = Order::market_order(test_symbol(), Side::Buy, dec!(1), "test".into());
        // Current equity = 94k → 6% loss → trips the 5% breaker
        let result = rm.check_order(&order, dec!(150), dec!(94_000));
        assert!(!result.is_approved());
        assert!(rm.is_circuit_breaker_tripped());
    }

    #[test]
    fn test_circuit_breaker_reset() {
        let config = RiskConfig {
            daily_loss_circuit_breaker: dec!(0.05),
            ..Default::default()
        };
        let mut rm = RiskManager::new(config, dec!(100_000));

        let order = Order::market_order(test_symbol(), Side::Buy, dec!(1), "test".into());
        let _ = rm.check_order(&order, dec!(150), dec!(94_000));
        assert!(rm.is_circuit_breaker_tripped());

        rm.reset_daily(dec!(95_000));
        assert!(!rm.is_circuit_breaker_tripped());

        // Now it should pass
        let result = rm.check_order(&order, dec!(150), dec!(95_000));
        assert!(result.is_approved());
    }

    #[test]
    fn test_order_rate_limit() {
        let config = RiskConfig {
            max_orders_per_window: 3,
            order_window_seconds: 60,
            ..Default::default()
        };
        let mut rm = RiskManager::new(config, dec!(100_000));

        let order = Order::market_order(test_symbol(), Side::Buy, dec!(1), "test".into());

        // First 3 should pass
        assert!(rm
            .check_order(&order, dec!(150), dec!(100_000))
            .is_approved());
        assert!(rm
            .check_order(&order, dec!(150), dec!(100_000))
            .is_approved());
        assert!(rm
            .check_order(&order, dec!(150), dec!(100_000))
            .is_approved());

        // 4th should be rejected
        assert!(!rm
            .check_order(&order, dec!(150), dec!(100_000))
            .is_approved());
    }

    #[test]
    fn test_position_concentration_limit() {
        let config = RiskConfig {
            limits: RiskLimits {
                position_concentration_limit: dec!(0.25), // max 25%
                ..Default::default()
            },
            max_order_notional: Decimal::from(1_000_000), // high enough to not interfere
            max_total_exposure: Decimal::from(1_000_000),
            ..Default::default()
        };
        let mut rm = RiskManager::new(config, dec!(100_000));

        // 200 shares * $150 = $30k → 30% of $100k → exceeds 25%
        let order = Order::market_order(test_symbol(), Side::Buy, dec!(200), "test".into());
        let result = rm.check_order(&order, dec!(150), dec!(100_000));
        assert!(!result.is_approved());
    }

    #[test]
    fn test_dry_run_mode_allows_rejected_orders() {
        let config = RiskConfig {
            max_order_notional: dec!(1_000), // very low
            dry_run: true,
            ..Default::default()
        };
        let mut rm = RiskManager::new(config, dec!(100_000));

        // Would normally be rejected ($15k notional > $1k limit)
        let order = Order::market_order(test_symbol(), Side::Buy, dec!(100), "test".into());
        let result = rm.check_order(&order, dec!(150), dec!(100_000));
        // dry-run → approved anyway
        assert!(result.is_approved());
    }

    #[test]
    fn test_total_exposure_limit() {
        let config = RiskConfig {
            max_total_exposure: dec!(50_000),
            max_order_notional: Decimal::from(1_000_000),
            limits: RiskLimits {
                position_concentration_limit: dec!(1.0), // disable concentration check
                ..Default::default()
            },
            ..Default::default()
        };
        let mut rm = RiskManager::new(config, dec!(100_000));

        // 400 shares * $150 = $60k > $50k exposure limit
        let order = Order::market_order(test_symbol(), Side::Buy, dec!(400), "test".into());
        let result = rm.check_order(&order, dec!(150), dec!(100_000));
        assert!(!result.is_approved());
    }

    #[test]
    fn test_update_position_tracking() {
        let mut rm = default_risk_manager();
        let sym = test_symbol();

        rm.update_position(&sym, Side::Buy, dec!(100));
        assert_eq!(
            rm.state.positions.get(&sym).copied().unwrap_or_default(),
            dec!(100)
        );

        rm.update_position(&sym, Side::Sell, dec!(40));
        assert_eq!(
            rm.state.positions.get(&sym).copied().unwrap_or_default(),
            dec!(60)
        );
    }
}
