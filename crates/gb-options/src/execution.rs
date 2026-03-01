//! Options execution — fill simulation and exercise/assignment handling.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use gb_types::orders::Side;

use crate::contract::{OptionContract, OptionKind};
use crate::pricing::{black_scholes_price, PricingInput};

/// Errors specific to options execution.
#[derive(Debug, Error)]
pub enum OptionsExecError {
    #[error("option has expired")]
    Expired,
    #[error("option is out of the money at expiration")]
    OutOfTheMoney,
    #[error("insufficient premium: required {required}, available {available}")]
    InsufficientPremium {
        required: Decimal,
        available: Decimal,
    },
    #[error("invalid quantity: {0}")]
    InvalidQuantity(String),
}

/// An options trade (open or close).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionsTrade {
    pub id: Uuid,
    pub contract: OptionContract,
    pub side: Side,
    pub quantity: Decimal,
    pub premium: Decimal,
    pub commission: Decimal,
    pub executed_at: DateTime<Utc>,
    pub strategy_id: String,
}

impl OptionsTrade {
    /// Total cash impact of the trade (premium × multiplier × quantity ± commission).
    pub fn cash_flow(&self) -> Decimal {
        let notional = self.premium * self.contract.multiplier * self.quantity;
        match self.side {
            Side::Buy => -(notional + self.commission),
            Side::Sell => notional - self.commission,
        }
    }
}

/// Result of exercising or being assigned on an option.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExerciseResult {
    pub contract: OptionContract,
    /// Shares delivered (positive = bought, negative = sold).
    pub shares_delivered: Decimal,
    /// Cash exchanged at the strike price (negative = paid, positive = received).
    pub cash_exchanged: Decimal,
    pub exercised_at: DateTime<Utc>,
}

/// Simulate opening an options position.
pub fn simulate_open(
    contract: &OptionContract,
    side: Side,
    quantity: Decimal,
    input: &PricingInput,
    commission_per_contract: Decimal,
    strategy_id: &str,
) -> Result<OptionsTrade, OptionsExecError> {
    if quantity <= Decimal::ZERO {
        return Err(OptionsExecError::InvalidQuantity(
            "quantity must be positive".into(),
        ));
    }
    if input.time_to_expiry <= 0.0 {
        return Err(OptionsExecError::Expired);
    }

    let result = black_scholes_price(contract, input);
    let premium = result.price;
    let commission = commission_per_contract * quantity;

    Ok(OptionsTrade {
        id: Uuid::new_v4(),
        contract: contract.clone(),
        side,
        quantity,
        premium,
        commission,
        executed_at: Utc::now(),
        strategy_id: strategy_id.to_string(),
    })
}

/// Simulate exercise at expiration (auto-exercise if ITM).
pub fn simulate_exercise(
    contract: &OptionContract,
    spot: Decimal,
    quantity: Decimal,
    now: DateTime<Utc>,
) -> Result<ExerciseResult, OptionsExecError> {
    if quantity <= Decimal::ZERO {
        return Err(OptionsExecError::InvalidQuantity(
            "quantity must be positive".into(),
        ));
    }

    if !contract.is_itm(spot) {
        return Err(OptionsExecError::OutOfTheMoney);
    }

    let total_shares = quantity * contract.multiplier;

    let (shares_delivered, cash_exchanged) = match contract.kind {
        OptionKind::Call => {
            // Exercise call: buy shares at strike
            let cash = -(contract.strike * total_shares);
            (total_shares, cash)
        }
        OptionKind::Put => {
            // Exercise put: sell shares at strike
            let cash = contract.strike * total_shares;
            (-total_shares, cash)
        }
    };

    Ok(ExerciseResult {
        contract: contract.clone(),
        shares_delivered,
        cash_exchanged,
        exercised_at: now,
    })
}

/// Simple P&L for a closed options round-trip.
pub fn options_pnl(entry: &OptionsTrade, exit: &OptionsTrade) -> Decimal {
    entry.cash_flow() + exit.cash_flow()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{ExerciseStyle, OptionKind};
    use chrono::{TimeZone, Utc};
    use gb_types::market::Symbol;
    use rust_decimal_macros::dec;

    fn make_contract(kind: OptionKind) -> OptionContract {
        OptionContract::new(
            Symbol::equity("AAPL"),
            kind,
            dec!(150),
            Utc.with_ymd_and_hms(2026, 6, 20, 20, 0, 0).unwrap(),
            ExerciseStyle::European,
            dec!(100),
        )
    }

    fn default_input() -> PricingInput {
        PricingInput {
            spot: 155.0,
            risk_free_rate: 0.05,
            volatility: 0.25,
            dividend_yield: 0.0,
            time_to_expiry: 0.25,
        }
    }

    #[test]
    fn test_simulate_open_buy_call() {
        let c = make_contract(OptionKind::Call);
        let input = default_input();
        let trade = simulate_open(&c, Side::Buy, dec!(1), &input, dec!(0.65), "test").unwrap();
        assert_eq!(trade.side, Side::Buy);
        assert!(trade.premium > Decimal::ZERO);
        assert!(trade.cash_flow() < Decimal::ZERO); // buyer pays
    }

    #[test]
    fn test_simulate_open_sell_put() {
        let c = make_contract(OptionKind::Put);
        let input = default_input();
        let trade = simulate_open(&c, Side::Sell, dec!(2), &input, dec!(0.65), "test").unwrap();
        assert_eq!(trade.side, Side::Sell);
        assert!(trade.cash_flow() > Decimal::ZERO); // seller receives
    }

    #[test]
    fn test_simulate_open_expired() {
        let c = make_contract(OptionKind::Call);
        let mut input = default_input();
        input.time_to_expiry = 0.0;
        let err = simulate_open(&c, Side::Buy, dec!(1), &input, dec!(0.65), "test");
        assert!(matches!(err, Err(OptionsExecError::Expired)));
    }

    #[test]
    fn test_simulate_open_zero_quantity() {
        let c = make_contract(OptionKind::Call);
        let input = default_input();
        let err = simulate_open(&c, Side::Buy, dec!(0), &input, dec!(0.65), "test");
        assert!(matches!(err, Err(OptionsExecError::InvalidQuantity(_))));
    }

    #[test]
    fn test_exercise_call_itm() {
        let c = make_contract(OptionKind::Call); // strike = 150
        let now = Utc.with_ymd_and_hms(2026, 6, 20, 20, 0, 0).unwrap();
        let result = simulate_exercise(&c, dec!(160), dec!(1), now).unwrap();
        assert_eq!(result.shares_delivered, dec!(100)); // 1 contract * 100 multiplier
        assert_eq!(result.cash_exchanged, dec!(-15000)); // -(150 * 100)
    }

    #[test]
    fn test_exercise_put_itm() {
        let c = make_contract(OptionKind::Put); // strike = 150
        let now = Utc.with_ymd_and_hms(2026, 6, 20, 20, 0, 0).unwrap();
        let result = simulate_exercise(&c, dec!(140), dec!(1), now).unwrap();
        assert_eq!(result.shares_delivered, dec!(-100)); // sold 100 shares
        assert_eq!(result.cash_exchanged, dec!(15000)); // 150 * 100
    }

    #[test]
    fn test_exercise_otm_fails() {
        let c = make_contract(OptionKind::Call); // strike = 150
        let now = Utc.with_ymd_and_hms(2026, 6, 20, 20, 0, 0).unwrap();
        let err = simulate_exercise(&c, dec!(140), dec!(1), now);
        assert!(matches!(err, Err(OptionsExecError::OutOfTheMoney)));
    }

    #[test]
    fn test_pnl_round_trip() {
        let c = make_contract(OptionKind::Call);
        let input = default_input();
        let buy = simulate_open(&c, Side::Buy, dec!(1), &input, dec!(0.65), "test").unwrap();

        // Simulate exit at a higher price
        let exit_input = PricingInput {
            spot: 165.0,
            ..input
        };
        let sell = simulate_open(&c, Side::Sell, dec!(1), &exit_input, dec!(0.65), "test").unwrap();

        let pnl = options_pnl(&buy, &sell);
        // Should be positive (underlying moved in our favour)
        assert!(pnl > Decimal::ZERO, "pnl = {pnl}");
    }
}
