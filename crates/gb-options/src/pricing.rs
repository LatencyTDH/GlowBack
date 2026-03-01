//! Black-Scholes pricing and greeks for European options.

use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;

use crate::contract::{OptionContract, OptionKind};
use crate::greeks::Greeks;

/// Inputs shared by all pricing calls.
#[derive(Debug, Clone)]
pub struct PricingInput {
    /// Current underlying spot price.
    pub spot: f64,
    /// Annualised risk-free rate (e.g. 0.05 = 5 %).
    pub risk_free_rate: f64,
    /// Annualised implied volatility (e.g. 0.20 = 20 %).
    pub volatility: f64,
    /// Continuous dividend yield (e.g. 0.02 = 2 %).
    pub dividend_yield: f64,
    /// Time to expiry in years.
    pub time_to_expiry: f64,
}

/// Result of a pricing calculation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PricingResult {
    /// Theoretical option price.
    pub price: Decimal,
    /// Greeks.
    pub greeks: Greeks,
}

use serde::{Deserialize, Serialize};

// ---------- normal distribution helpers (no external dep) ----------

/// Standard normal cumulative distribution function (Abramowitz & Stegun 26.2.17).
fn norm_cdf(x: f64) -> f64 {
    if x >= 8.0 {
        return 1.0;
    }
    if x <= -8.0 {
        return 0.0;
    }

    let a1 = 0.254829592_f64;
    let a2 = -0.284496736_f64;
    let a3 = 1.421413741_f64;
    let a4 = -1.453152027_f64;
    let a5 = 1.061405429_f64;
    let p = 0.3275911_f64;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x_abs = x.abs();
    let t = 1.0 / (1.0 + p * x_abs);
    let y =
        1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x_abs * x_abs / 2.0).exp();

    0.5 * (1.0 + sign * y)
}

/// Standard normal probability density function.
fn norm_pdf(x: f64) -> f64 {
    const INV_SQRT_2PI: f64 = 0.398_942_280_401_432_7;
    INV_SQRT_2PI * (-0.5 * x * x).exp()
}

// ---------- Black-Scholes core ----------

/// Compute d1 and d2.
fn d1_d2(s: f64, k: f64, r: f64, q: f64, sigma: f64, t: f64) -> (f64, f64) {
    let d1 = ((s / k).ln() + (r - q + 0.5 * sigma * sigma) * t) / (sigma * t.sqrt());
    let d2 = d1 - sigma * t.sqrt();
    (d1, d2)
}

/// Price a European option using the Black-Scholes model.
pub fn black_scholes_price(contract: &OptionContract, input: &PricingInput) -> PricingResult {
    let s = input.spot;
    let k = contract.strike.to_f64().unwrap_or(0.0);
    let r = input.risk_free_rate;
    let q = input.dividend_yield;
    let sigma = input.volatility;
    let t = input.time_to_expiry;

    // Degenerate: expired option
    if t <= 0.0 {
        let iv = contract.intrinsic_value(Decimal::from_f64(s).unwrap_or_default());
        return PricingResult {
            price: iv,
            greeks: Greeks::zero(),
        };
    }

    let (d1, d2) = d1_d2(s, k, r, q, sigma, t);
    let disc = (-r * t).exp();
    let div_disc = (-q * t).exp();

    let price = match contract.kind {
        OptionKind::Call => s * div_disc * norm_cdf(d1) - k * disc * norm_cdf(d2),
        OptionKind::Put => k * disc * norm_cdf(-d2) - s * div_disc * norm_cdf(-d1),
    };

    // --- Greeks ---
    let delta = match contract.kind {
        OptionKind::Call => div_disc * norm_cdf(d1),
        OptionKind::Put => -div_disc * norm_cdf(-d1),
    };

    let gamma = div_disc * norm_pdf(d1) / (s * sigma * t.sqrt());

    let theta_common = -(s * div_disc * norm_pdf(d1) * sigma) / (2.0 * t.sqrt());
    let theta = match contract.kind {
        OptionKind::Call => {
            theta_common - r * k * disc * norm_cdf(d2) + q * s * div_disc * norm_cdf(d1)
        }
        OptionKind::Put => {
            theta_common + r * k * disc * norm_cdf(-d2) - q * s * div_disc * norm_cdf(-d1)
        }
    };
    // Convert theta to per-calendar-day
    let theta_daily = theta / 365.0;

    let vega = s * div_disc * norm_pdf(d1) * t.sqrt();
    // Vega per 1 % vol move
    let vega_pct = vega / 100.0;

    let rho = match contract.kind {
        OptionKind::Call => k * t * disc * norm_cdf(d2),
        OptionKind::Put => -k * t * disc * norm_cdf(-d2),
    };
    let rho_pct = rho / 100.0;

    let to_dec = |v: f64| Decimal::from_f64(v).unwrap_or(Decimal::ZERO);

    PricingResult {
        price: to_dec(price),
        greeks: Greeks {
            delta: to_dec(delta),
            gamma: to_dec(gamma),
            theta: to_dec(theta_daily),
            vega: to_dec(vega_pct),
            rho: to_dec(rho_pct),
        },
    }
}

/// Implied volatility via Newton-Raphson on Black-Scholes vega.
/// Returns `None` if it fails to converge.
pub fn implied_volatility(
    contract: &OptionContract,
    market_price: f64,
    spot: f64,
    risk_free_rate: f64,
    dividend_yield: f64,
    time_to_expiry: f64,
) -> Option<f64> {
    let k = contract.strike.to_f64().unwrap_or(0.0);
    if time_to_expiry <= 0.0 || market_price <= 0.0 || spot <= 0.0 || k <= 0.0 {
        return None;
    }

    let mut sigma = 0.30; // initial guess
    let max_iter = 100;
    let tol = 1e-8;

    for _ in 0..max_iter {
        let input = PricingInput {
            spot,
            risk_free_rate,
            volatility: sigma,
            dividend_yield,
            time_to_expiry,
        };
        let result = black_scholes_price(contract, &input);
        let model_price = result.price.to_f64().unwrap_or(0.0);
        let diff = model_price - market_price;

        if diff.abs() < tol {
            return Some(sigma);
        }

        // Vega in absolute terms (undo the /100 scaling)
        let vega_abs = result.greeks.vega.to_f64().unwrap_or(0.0) * 100.0;
        if vega_abs.abs() < 1e-12 {
            return None; // vega too small to converge
        }

        sigma -= diff / vega_abs;
        if sigma <= 0.0 {
            sigma = 0.001; // clamp positive
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{ExerciseStyle, OptionKind};
    use chrono::{TimeZone, Utc};
    use gb_types::market::Symbol;
    use rust_decimal_macros::dec;

    fn make_contract(kind: OptionKind, strike: Decimal) -> OptionContract {
        let exp = Utc.with_ymd_and_hms(2026, 6, 20, 20, 0, 0).unwrap();
        OptionContract::new(
            Symbol::equity("AAPL"),
            kind,
            strike,
            exp,
            ExerciseStyle::European,
            dec!(100),
        )
    }

    #[test]
    fn test_call_price_sanity() {
        let c = make_contract(OptionKind::Call, dec!(150));
        let input = PricingInput {
            spot: 155.0,
            risk_free_rate: 0.05,
            volatility: 0.25,
            dividend_yield: 0.0,
            time_to_expiry: 0.25,
        };
        let res = black_scholes_price(&c, &input);
        let price = res.price.to_f64().unwrap();
        // ITM call should be worth at least intrinsic ($5)
        assert!(price > 5.0, "call price = {price}");
        assert!(price < 20.0, "call price unreasonably high = {price}");
    }

    #[test]
    fn test_put_price_sanity() {
        let c = make_contract(OptionKind::Put, dec!(150));
        let input = PricingInput {
            spot: 145.0,
            risk_free_rate: 0.05,
            volatility: 0.25,
            dividend_yield: 0.0,
            time_to_expiry: 0.25,
        };
        let res = black_scholes_price(&c, &input);
        let price = res.price.to_f64().unwrap();
        assert!(price > 5.0, "put price = {price}");
        assert!(price < 20.0, "put price unreasonably high = {price}");
    }

    #[test]
    fn test_put_call_parity() {
        let strike = dec!(150);
        let call = make_contract(OptionKind::Call, strike);
        let put = make_contract(OptionKind::Put, strike);
        let input = PricingInput {
            spot: 150.0,
            risk_free_rate: 0.05,
            volatility: 0.30,
            dividend_yield: 0.0,
            time_to_expiry: 0.5,
        };
        let c_price = black_scholes_price(&call, &input).price.to_f64().unwrap();
        let p_price = black_scholes_price(&put, &input).price.to_f64().unwrap();
        let k = strike.to_f64().unwrap();
        // C - P = S - K*exp(-rT)
        let lhs = c_price - p_price;
        let rhs = input.spot - k * (-input.risk_free_rate * input.time_to_expiry).exp();
        assert!(
            (lhs - rhs).abs() < 0.01,
            "put-call parity violated: lhs={lhs}, rhs={rhs}"
        );
    }

    #[test]
    fn test_expired_option_returns_intrinsic() {
        let c = make_contract(OptionKind::Call, dec!(150));
        let input = PricingInput {
            spot: 160.0,
            risk_free_rate: 0.05,
            volatility: 0.25,
            dividend_yield: 0.0,
            time_to_expiry: 0.0,
        };
        let res = black_scholes_price(&c, &input);
        assert_eq!(res.price, dec!(10));
    }

    #[test]
    fn test_greeks_sign_call() {
        let c = make_contract(OptionKind::Call, dec!(150));
        let input = PricingInput {
            spot: 150.0,
            risk_free_rate: 0.05,
            volatility: 0.25,
            dividend_yield: 0.0,
            time_to_expiry: 0.25,
        };
        let res = black_scholes_price(&c, &input);
        let g = &res.greeks;
        assert!(g.delta > Decimal::ZERO, "call delta should be positive");
        assert!(g.gamma > Decimal::ZERO, "gamma should be positive");
        assert!(
            g.theta < Decimal::ZERO,
            "theta should be negative (time decay)"
        );
        assert!(g.vega > Decimal::ZERO, "vega should be positive");
        assert!(g.rho > Decimal::ZERO, "call rho should be positive");
    }

    #[test]
    fn test_greeks_sign_put() {
        let c = make_contract(OptionKind::Put, dec!(150));
        let input = PricingInput {
            spot: 150.0,
            risk_free_rate: 0.05,
            volatility: 0.25,
            dividend_yield: 0.0,
            time_to_expiry: 0.25,
        };
        let res = black_scholes_price(&c, &input);
        let g = &res.greeks;
        assert!(g.delta < Decimal::ZERO, "put delta should be negative");
        assert!(g.gamma > Decimal::ZERO, "gamma should be positive");
        assert!(g.vega > Decimal::ZERO, "vega should be positive");
        assert!(g.rho < Decimal::ZERO, "put rho should be negative");
    }

    #[test]
    fn test_implied_volatility_roundtrip() {
        let c = make_contract(OptionKind::Call, dec!(150));
        let true_vol = 0.25;
        let input = PricingInput {
            spot: 155.0,
            risk_free_rate: 0.05,
            volatility: true_vol,
            dividend_yield: 0.0,
            time_to_expiry: 0.25,
        };
        let price = black_scholes_price(&c, &input).price.to_f64().unwrap();

        let iv = implied_volatility(&c, price, 155.0, 0.05, 0.0, 0.25);
        assert!(iv.is_some(), "IV should converge");
        let iv = iv.unwrap();
        assert!(
            (iv - true_vol).abs() < 0.001,
            "IV={iv} should match true vol={true_vol}"
        );
    }

    #[test]
    fn test_implied_volatility_put() {
        let c = make_contract(OptionKind::Put, dec!(150));
        let true_vol = 0.30;
        let input = PricingInput {
            spot: 148.0,
            risk_free_rate: 0.04,
            volatility: true_vol,
            dividend_yield: 0.01,
            time_to_expiry: 0.5,
        };
        let price = black_scholes_price(&c, &input).price.to_f64().unwrap();

        let iv = implied_volatility(&c, price, 148.0, 0.04, 0.01, 0.5);
        assert!(iv.is_some());
        assert!((iv.unwrap() - true_vol).abs() < 0.001);
    }

    #[test]
    fn test_norm_cdf_boundaries() {
        assert!((norm_cdf(0.0) - 0.5).abs() < 1e-6);
        assert!(norm_cdf(8.0) == 1.0);
        assert!(norm_cdf(-8.0) == 0.0);
    }
}
