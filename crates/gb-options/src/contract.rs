use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;

use gb_types::market::Symbol;

/// Option type — call or put.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OptionKind {
    Call,
    Put,
}

impl fmt::Display for OptionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OptionKind::Call => write!(f, "Call"),
            OptionKind::Put => write!(f, "Put"),
        }
    }
}

/// Exercise style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExerciseStyle {
    /// Can only be exercised at expiration.
    European,
    /// Can be exercised any time before expiration.
    American,
}

impl fmt::Display for ExerciseStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExerciseStyle::European => write!(f, "European"),
            ExerciseStyle::American => write!(f, "American"),
        }
    }
}

/// A single options contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionContract {
    /// Underlying symbol.
    pub underlying: Symbol,
    /// Call or put.
    pub kind: OptionKind,
    /// Strike price.
    pub strike: Decimal,
    /// Expiration date (end of day UTC).
    pub expiration: DateTime<Utc>,
    /// Exercise style.
    pub exercise_style: ExerciseStyle,
    /// Contract multiplier (typically 100 for equity options).
    pub multiplier: Decimal,
}

impl OptionContract {
    pub fn new(
        underlying: Symbol,
        kind: OptionKind,
        strike: Decimal,
        expiration: DateTime<Utc>,
        exercise_style: ExerciseStyle,
        multiplier: Decimal,
    ) -> Self {
        Self {
            underlying,
            kind,
            strike,
            expiration,
            exercise_style,
            multiplier,
        }
    }

    /// Convenience constructor for a standard equity option (multiplier = 100, European).
    pub fn equity(
        underlying: Symbol,
        kind: OptionKind,
        strike: Decimal,
        expiration: DateTime<Utc>,
    ) -> Self {
        Self::new(
            underlying,
            kind,
            strike,
            expiration,
            ExerciseStyle::European,
            Decimal::from(100),
        )
    }

    /// Years remaining until expiration from `now`.
    /// Returns 0 if already expired.
    pub fn time_to_expiry(&self, now: DateTime<Utc>) -> f64 {
        let secs = (self.expiration - now).num_seconds();
        if secs <= 0 {
            0.0
        } else {
            secs as f64 / (365.25 * 86400.0)
        }
    }

    /// True if the option has expired relative to `now`.
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expiration
    }

    /// Intrinsic value given the current underlying price.
    pub fn intrinsic_value(&self, spot: Decimal) -> Decimal {
        let iv = match self.kind {
            OptionKind::Call => spot - self.strike,
            OptionKind::Put => self.strike - spot,
        };
        if iv > Decimal::ZERO {
            iv
        } else {
            Decimal::ZERO
        }
    }

    /// True when the option is in-the-money.
    pub fn is_itm(&self, spot: Decimal) -> bool {
        self.intrinsic_value(spot) > Decimal::ZERO
    }

    /// True when at-the-money (strike == spot, within tolerance).
    pub fn is_atm(&self, spot: Decimal, tolerance: Decimal) -> bool {
        (self.strike - spot).abs() <= tolerance
    }
}

impl fmt::Display for OptionContract {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {} {} ({})",
            self.underlying.symbol,
            self.expiration.format("%Y-%m-%d"),
            self.strike,
            self.kind,
            self.exercise_style,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use rust_decimal_macros::dec;

    fn sample_contract(kind: OptionKind, strike: Decimal) -> OptionContract {
        let underlying = Symbol::equity("AAPL");
        let expiration = Utc.with_ymd_and_hms(2026, 6, 20, 20, 0, 0).unwrap();
        OptionContract::equity(underlying, kind, strike, expiration)
    }

    #[test]
    fn test_intrinsic_value_call_itm() {
        let c = sample_contract(OptionKind::Call, dec!(150));
        assert_eq!(c.intrinsic_value(dec!(160)), dec!(10));
    }

    #[test]
    fn test_intrinsic_value_call_otm() {
        let c = sample_contract(OptionKind::Call, dec!(150));
        assert_eq!(c.intrinsic_value(dec!(140)), dec!(0));
    }

    #[test]
    fn test_intrinsic_value_put_itm() {
        let c = sample_contract(OptionKind::Put, dec!(150));
        assert_eq!(c.intrinsic_value(dec!(140)), dec!(10));
    }

    #[test]
    fn test_intrinsic_value_put_otm() {
        let c = sample_contract(OptionKind::Put, dec!(150));
        assert_eq!(c.intrinsic_value(dec!(160)), dec!(0));
    }

    #[test]
    fn test_is_itm() {
        let call = sample_contract(OptionKind::Call, dec!(150));
        assert!(call.is_itm(dec!(160)));
        assert!(!call.is_itm(dec!(140)));
    }

    #[test]
    fn test_is_atm() {
        let c = sample_contract(OptionKind::Call, dec!(150));
        assert!(c.is_atm(dec!(150), dec!(1)));
        assert!(c.is_atm(dec!(150.5), dec!(1)));
        assert!(!c.is_atm(dec!(155), dec!(1)));
    }

    #[test]
    fn test_time_to_expiry() {
        let c = sample_contract(OptionKind::Call, dec!(150));
        let now = Utc.with_ymd_and_hms(2026, 3, 20, 20, 0, 0).unwrap();
        let tte = c.time_to_expiry(now);
        // ~92 days ≈ 0.252 years
        assert!(tte > 0.24 && tte < 0.26, "tte = {tte}");
    }

    #[test]
    fn test_expired() {
        let c = sample_contract(OptionKind::Call, dec!(150));
        let before = Utc.with_ymd_and_hms(2026, 6, 19, 0, 0, 0).unwrap();
        let after = Utc.with_ymd_and_hms(2026, 6, 21, 0, 0, 0).unwrap();
        assert!(!c.is_expired(before));
        assert!(c.is_expired(after));
    }

    #[test]
    fn test_display() {
        let c = sample_contract(OptionKind::Call, dec!(150));
        let s = format!("{c}");
        assert!(s.contains("AAPL"));
        assert!(s.contains("150"));
        assert!(s.contains("Call"));
    }
}
