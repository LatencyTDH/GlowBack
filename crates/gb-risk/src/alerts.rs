//! Risk alert types and severity levels.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Severity of a risk alert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RiskSeverity {
    /// Informational — within normal operating range.
    Info,
    /// Warning — approaching a limit.
    Warning,
    /// Critical — limit breached; action required.
    Critical,
}

/// Discriminant for the kind of risk alert.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RiskAlertKind {
    /// Daily loss exceeds threshold.
    DailyLossExceeded {
        current_loss_pct: Decimal,
        limit_pct: Decimal,
    },
    /// Portfolio drawdown exceeds threshold.
    DrawdownExceeded {
        current_drawdown_pct: Decimal,
        limit_pct: Decimal,
    },
    /// A single position is too concentrated.
    ConcentrationExceeded {
        symbol: String,
        weight_pct: Decimal,
        limit_pct: Decimal,
    },
    /// Portfolio leverage exceeds limit.
    LeverageExceeded {
        current_leverage: Decimal,
        limit: Decimal,
    },
    /// Value-at-Risk exceeds the risk budget.
    VarExceeded {
        var_pct: Decimal,
        limit_pct: Decimal,
    },
    /// Portfolio gross exposure exceeds limit.
    GrossExposureExceeded {
        gross_exposure: Decimal,
        limit: Decimal,
    },
    /// Custom/user-defined alert.
    Custom { name: String, message: String },
}

/// A single risk alert emitted by the monitor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskAlert {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub severity: RiskSeverity,
    pub kind: RiskAlertKind,
    pub message: String,
    /// Whether the alert has been acknowledged by a human operator.
    pub acknowledged: bool,
}

impl RiskAlert {
    /// Create a new alert.
    pub fn new(severity: RiskSeverity, kind: RiskAlertKind, message: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            severity,
            kind,
            message,
            acknowledged: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn severity_ordering() {
        assert!(RiskSeverity::Info < RiskSeverity::Warning);
        assert!(RiskSeverity::Warning < RiskSeverity::Critical);
    }

    #[test]
    fn alert_creation() {
        let alert = RiskAlert::new(
            RiskSeverity::Warning,
            RiskAlertKind::DrawdownExceeded {
                current_drawdown_pct: dec!(15),
                limit_pct: dec!(10),
            },
            "Drawdown of 15% exceeds 10% limit".into(),
        );
        assert_eq!(alert.severity, RiskSeverity::Warning);
        assert!(!alert.acknowledged);
    }

    #[test]
    fn alert_serialization_roundtrip() {
        let alert = RiskAlert::new(
            RiskSeverity::Critical,
            RiskAlertKind::DailyLossExceeded {
                current_loss_pct: dec!(6),
                limit_pct: dec!(5),
            },
            "Daily loss 6% > 5% limit".into(),
        );
        let json = serde_json::to_string(&alert).unwrap();
        let deserialized: RiskAlert = serde_json::from_str(&json).unwrap();
        assert_eq!(alert.severity, deserialized.severity);
        assert_eq!(alert.kind, deserialized.kind);
    }
}
