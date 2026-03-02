use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Option greeks computed from a pricing model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Greeks {
    /// Rate of change of option price w.r.t. underlying price.
    pub delta: Decimal,
    /// Rate of change of delta w.r.t. underlying price.
    pub gamma: Decimal,
    /// Rate of change of option price w.r.t. time (per calendar day).
    pub theta: Decimal,
    /// Rate of change of option price w.r.t. volatility (per 1% move).
    pub vega: Decimal,
    /// Rate of change of option price w.r.t. risk-free rate (per 1% move).
    pub rho: Decimal,
}

impl Greeks {
    pub fn zero() -> Self {
        Self {
            delta: Decimal::ZERO,
            gamma: Decimal::ZERO,
            theta: Decimal::ZERO,
            vega: Decimal::ZERO,
            rho: Decimal::ZERO,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_greeks() {
        let g = Greeks::zero();
        assert_eq!(g.delta, Decimal::ZERO);
        assert_eq!(g.gamma, Decimal::ZERO);
        assert_eq!(g.theta, Decimal::ZERO);
        assert_eq!(g.vega, Decimal::ZERO);
        assert_eq!(g.rho, Decimal::ZERO);
    }
}
