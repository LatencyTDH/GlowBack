//! Options chain — a collection of contracts for a single underlying and expiration.

use chrono::{DateTime, Utc};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use gb_types::market::Symbol;

use crate::contract::{ExerciseStyle, OptionContract, OptionKind};
use crate::pricing::{black_scholes_price, PricingInput, PricingResult};

/// A single row in an option chain (call + put at the same strike).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChainRow {
    pub strike: Decimal,
    pub call: PricingResult,
    pub put: PricingResult,
}

/// An option chain for a single underlying/expiration pair.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionChain {
    pub underlying: Symbol,
    pub expiration: DateTime<Utc>,
    pub rows: Vec<ChainRow>,
    pub spot: Decimal,
    pub generated_at: DateTime<Utc>,
}

/// Build an option chain with strikes spaced evenly around the spot price.
///
/// * `num_strikes` — total number of strikes (centered around ATM).
/// * `strike_step` — spacing between consecutive strikes.
pub fn build_chain(
    underlying: Symbol,
    expiration: DateTime<Utc>,
    spot: f64,
    risk_free_rate: f64,
    volatility: f64,
    dividend_yield: f64,
    time_to_expiry: f64,
    num_strikes: usize,
    strike_step: f64,
    exercise_style: ExerciseStyle,
    multiplier: Decimal,
) -> OptionChain {
    let half = num_strikes / 2;
    let atm_strike = (spot / strike_step).round() * strike_step;

    let mut rows = Vec::with_capacity(num_strikes);

    for i in 0..num_strikes {
        let offset = i as f64 - half as f64;
        let strike_f = atm_strike + offset * strike_step;
        if strike_f <= 0.0 {
            continue;
        }
        let strike = Decimal::from_f64_retain(strike_f).unwrap_or_default();

        let call_contract = OptionContract::new(
            underlying.clone(),
            OptionKind::Call,
            strike,
            expiration,
            exercise_style,
            multiplier,
        );
        let put_contract = OptionContract::new(
            underlying.clone(),
            OptionKind::Put,
            strike,
            expiration,
            exercise_style,
            multiplier,
        );

        let input = PricingInput {
            spot,
            risk_free_rate,
            volatility,
            dividend_yield,
            time_to_expiry,
        };

        let call_result = black_scholes_price(&call_contract, &input);
        let put_result = black_scholes_price(&put_contract, &input);

        rows.push(ChainRow {
            strike,
            call: call_result,
            put: put_result,
        });
    }

    OptionChain {
        underlying,
        expiration,
        rows,
        spot: Decimal::from_f64_retain(spot).unwrap_or_default(),
        generated_at: Utc::now(),
    }
}

impl OptionChain {
    /// Find the ATM strike (closest to spot).
    pub fn atm_strike(&self) -> Option<Decimal> {
        self.rows
            .iter()
            .min_by_key(|r| {
                let diff = (r.strike - self.spot).abs();
                // Convert to a sortable integer (cents precision)
                (diff * Decimal::from(10000)).to_i64().unwrap_or(i64::MAX)
            })
            .map(|r| r.strike)
    }

    /// Get a specific row by strike.
    pub fn get_strike(&self, strike: Decimal) -> Option<&ChainRow> {
        self.rows.iter().find(|r| r.strike == strike)
    }

    /// Number of strikes in the chain.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// True if the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::ExerciseStyle;
    use chrono::{TimeZone, Utc};
    use gb_types::market::Symbol;
    use rust_decimal_macros::dec;

    fn build_test_chain() -> OptionChain {
        let underlying = Symbol::equity("AAPL");
        let expiration = Utc.with_ymd_and_hms(2026, 6, 20, 20, 0, 0).unwrap();
        build_chain(
            underlying,
            expiration,
            150.0,
            0.05,
            0.25,
            0.0,
            0.25,
            11,  // 11 strikes
            5.0, // $5 apart
            ExerciseStyle::European,
            dec!(100),
        )
    }

    #[test]
    fn test_chain_has_strikes() {
        let chain = build_test_chain();
        assert_eq!(chain.len(), 11);
        assert!(!chain.is_empty());
    }

    #[test]
    fn test_chain_atm_strike() {
        let chain = build_test_chain();
        let atm = chain.atm_strike().unwrap();
        // ATM should be 150
        assert_eq!(atm, dec!(150));
    }

    #[test]
    fn test_chain_strike_ordering() {
        let chain = build_test_chain();
        for i in 1..chain.rows.len() {
            assert!(chain.rows[i].strike > chain.rows[i - 1].strike);
        }
    }

    #[test]
    fn test_chain_call_put_prices_positive() {
        let chain = build_test_chain();
        for row in &chain.rows {
            let call_price = row.call.price.to_f64().unwrap();
            let put_price = row.put.price.to_f64().unwrap();
            assert!(
                call_price >= 0.0,
                "call price negative at strike {}",
                row.strike
            );
            assert!(
                put_price >= 0.0,
                "put price negative at strike {}",
                row.strike
            );
        }
    }

    #[test]
    fn test_chain_get_strike() {
        let chain = build_test_chain();
        let row = chain.get_strike(dec!(150));
        assert!(row.is_some());
        assert_eq!(row.unwrap().strike, dec!(150));
    }

    #[test]
    fn test_chain_get_strike_missing() {
        let chain = build_test_chain();
        assert!(chain.get_strike(dec!(999)).is_none());
    }

    #[test]
    fn test_put_call_parity_across_chain() {
        let chain = build_test_chain();
        let s: f64 = 150.0;
        let r: f64 = 0.05;
        let t: f64 = 0.25;
        for row in &chain.rows {
            let c = row.call.price.to_f64().unwrap();
            let p = row.put.price.to_f64().unwrap();
            let k = row.strike.to_f64().unwrap();
            let lhs = c - p;
            let rhs = s - k * (-r * t).exp();
            assert!(
                (lhs - rhs).abs() < 0.02,
                "put-call parity violated at strike {}: lhs={lhs}, rhs={rhs}",
                row.strike,
            );
        }
    }
}
