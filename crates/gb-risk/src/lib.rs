//! Real-time risk metrics and monitoring pipeline for GlowBack.
//!
//! Provides:
//! - Continuous portfolio-level risk assessment (VaR, drawdown, exposure)
//! - Per-position risk metrics (concentration, Greeks placeholder)
//! - Configurable risk limits with breach detection
//! - Event-driven monitoring via channels

pub mod alerts;
pub mod metrics;
pub mod monitor;

pub use alerts::{RiskAlert, RiskAlertKind, RiskSeverity};
pub use metrics::{PortfolioRiskSnapshot, PositionRisk, RiskMetricsCalculator};
pub use monitor::{RiskMonitor, RiskMonitorConfig};
