use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::market::{Symbol, Resolution};
use crate::portfolio::Portfolio;
use crate::strategy::{StrategyConfig, StrategyMetrics};

/// Unique backtest identifier
pub type BacktestId = Uuid;

/// Backtest configuration and parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BacktestConfig {
    pub id: BacktestId,
    pub name: String,
    pub description: String,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub initial_capital: Decimal,
    pub symbols: Vec<Symbol>,
    pub resolution: Resolution,
    pub strategy_config: StrategyConfig,
    pub execution_settings: ExecutionSettings,
    pub data_settings: DataSettings,
    pub created_at: DateTime<Utc>,
}

impl BacktestConfig {
    pub fn new(name: String, strategy_config: StrategyConfig) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            description: String::new(),
            start_date: Utc::now() - chrono::Duration::days(365), // Default to 1 year ago
            end_date: Utc::now(),
            initial_capital: Decimal::from(100000),
            symbols: Vec::new(),
            resolution: Resolution::Day,
            strategy_config,
            execution_settings: ExecutionSettings::default(),
            data_settings: DataSettings::default(),
            created_at: Utc::now(),
        }
    }
    
    pub fn with_symbols(mut self, symbols: Vec<Symbol>) -> Self {
        self.symbols = symbols;
        self
    }
    
    pub fn with_date_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.start_date = start;
        self.end_date = end;
        self
    }
    
    pub fn with_capital(mut self, capital: Decimal) -> Self {
        self.initial_capital = capital;
        self
    }
    
    pub fn with_resolution(mut self, resolution: Resolution) -> Self {
        self.resolution = resolution;
        self
    }
}

/// Execution settings for realistic trading simulation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionSettings {
    pub commission_per_share: Decimal,
    pub commission_percentage: Decimal,
    pub minimum_commission: Decimal,
    pub slippage_model: SlippageModel,
    pub latency_model: LatencyModel,
    pub market_impact_model: MarketImpactModel,
}

impl Default for ExecutionSettings {
    fn default() -> Self {
        Self {
            commission_per_share: Decimal::new(1, 3), // $0.001 per share
            commission_percentage: Decimal::new(5, 4), // 0.05%
            minimum_commission: Decimal::new(1, 0), // $1.00 minimum
            slippage_model: SlippageModel::Linear { basis_points: 5 },
            latency_model: LatencyModel::Fixed { milliseconds: 100 },
            market_impact_model: MarketImpactModel::SquareRoot { factor: Decimal::new(1, 4) },
        }
    }
}

/// Slippage model for order execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SlippageModel {
    None,
    Fixed { basis_points: u32 },
    Linear { basis_points: u32 },
    SquareRoot { factor: Decimal },
    VolumeWeighted { min_bps: u32, max_bps: u32 },
}

/// Latency model for order execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LatencyModel {
    None,
    Fixed { milliseconds: u64 },
    Random { min_ms: u64, max_ms: u64 },
    VenueSpecific { venues: HashMap<String, u64> },
}

/// Market impact model
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MarketImpactModel {
    None,
    Linear { factor: Decimal },
    SquareRoot { factor: Decimal },
    Logarithmic { factor: Decimal },
}

/// Data settings for backtest
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataSettings {
    pub data_source: String,
    pub adjust_for_splits: bool,
    pub adjust_for_dividends: bool,
    pub fill_gaps: bool,
    pub survivor_bias_free: bool,
    pub max_bars_in_memory: usize,
}

impl Default for DataSettings {
    fn default() -> Self {
        Self {
            data_source: "default".to_string(),
            adjust_for_splits: true,
            adjust_for_dividends: true,
            fill_gaps: false,
            survivor_bias_free: true,
            max_bars_in_memory: 10000,
        }
    }
}

/// Backtest execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BacktestStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Backtest result with comprehensive metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BacktestResult {
    pub id: BacktestId,
    pub config: BacktestConfig,
    pub status: BacktestStatus,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_seconds: Option<u64>,
    pub final_portfolio: Option<Portfolio>,
    pub strategy_metrics: Option<StrategyMetrics>,
    pub performance_metrics: Option<PerformanceMetrics>,
    pub equity_curve: Vec<EquityCurvePoint>,
    pub trade_log: Vec<TradeRecord>,
    pub error_message: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl BacktestResult {
    pub fn new(config: BacktestConfig) -> Self {
        Self {
            id: config.id,
            config,
            status: BacktestStatus::Pending,
            start_time: Utc::now(),
            end_time: None,
            duration_seconds: None,
            final_portfolio: None,
            strategy_metrics: None,
            performance_metrics: None,
            equity_curve: Vec::new(),
            trade_log: Vec::new(),
            error_message: None,
            metadata: HashMap::new(),
        }
    }
    
    pub fn mark_started(&mut self) {
        self.status = BacktestStatus::Running;
        self.start_time = Utc::now();
    }
    
    pub fn mark_completed(&mut self, portfolio: Portfolio, metrics: StrategyMetrics) {
        let end_time = Utc::now();
        self.status = BacktestStatus::Completed;
        self.end_time = Some(end_time);
        self.duration_seconds = Some((end_time - self.start_time).num_seconds() as u64);
        
        // Calculate performance metrics
        self.performance_metrics = Some(PerformanceMetrics::calculate(&portfolio));
        self.final_portfolio = Some(portfolio);
        self.strategy_metrics = Some(metrics);
    }
    
    pub fn mark_failed(&mut self, error: String) {
        self.status = BacktestStatus::Failed;
        self.end_time = Some(Utc::now());
        self.error_message = Some(error);
    }
}

/// Performance metrics for backtest evaluation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_return: Decimal,
    pub annualized_return: Decimal,
    pub volatility: Decimal,
    pub sharpe_ratio: Option<Decimal>,
    pub sortino_ratio: Option<Decimal>,
    pub calmar_ratio: Option<Decimal>,
    pub max_drawdown: Decimal,
    pub max_drawdown_duration_days: Option<u32>,
    pub var_95: Option<Decimal>,
    pub cvar_95: Option<Decimal>,
    pub beta: Option<Decimal>,
    pub alpha: Option<Decimal>,
    pub information_ratio: Option<Decimal>,
    pub skewness: Option<Decimal>,
    pub kurtosis: Option<Decimal>,
    pub win_rate: Decimal,
    pub profit_factor: Option<Decimal>,
    pub average_win: Decimal,
    pub average_loss: Decimal,
    pub largest_win: Decimal,
    pub largest_loss: Decimal,
    pub total_trades: u64,
    pub total_commissions: Decimal,
}

impl PerformanceMetrics {
    pub fn calculate(portfolio: &Portfolio) -> Self {
        let daily_returns = &portfolio.daily_returns;
        
        Self {
            total_return: portfolio.get_total_return(),
            annualized_return: Self::calculate_annualized_return(daily_returns),
            volatility: Self::calculate_volatility(daily_returns),
            sharpe_ratio: portfolio.get_sharpe_ratio(Decimal::new(2, 2)), // 2% risk-free rate
            sortino_ratio: Self::calculate_sortino_ratio(daily_returns, Decimal::new(2, 2)),
            calmar_ratio: Self::calculate_calmar_ratio(daily_returns, portfolio.get_max_drawdown()),
            max_drawdown: portfolio.get_max_drawdown(),
            max_drawdown_duration_days: Self::calculate_max_drawdown_duration(daily_returns),
            var_95: Self::calculate_var_95(daily_returns),
            cvar_95: Self::calculate_cvar_95(daily_returns),
            beta: None,          // Requires benchmark data
            alpha: None,         // Requires benchmark data
            information_ratio: None, // Requires benchmark data
            skewness: Self::calculate_skewness(daily_returns),
            kurtosis: Self::calculate_kurtosis(daily_returns),
            win_rate: Decimal::ZERO,     // Requires trade data
            profit_factor: None,         // Requires trade data
            average_win: Decimal::ZERO,  // Requires trade data
            average_loss: Decimal::ZERO, // Requires trade data
            largest_win: Decimal::ZERO,  // Requires trade data
            largest_loss: Decimal::ZERO, // Requires trade data
            total_trades: 0,             // Requires trade data
            total_commissions: portfolio.total_commissions,
        }
    }

    /// Calculate performance metrics with trade data
    pub fn calculate_with_trades(portfolio: &Portfolio, trades: &[TradeRecord]) -> Self {
        let mut metrics = Self::calculate(portfolio);
        
        // Add trade-based metrics
        if !trades.is_empty() {
            metrics.total_trades = trades.len() as u64;
            metrics.win_rate = Self::calculate_win_rate(trades);
            metrics.profit_factor = Self::calculate_profit_factor(trades);
            metrics.average_win = Self::calculate_average_win(trades);
            metrics.average_loss = Self::calculate_average_loss(trades);
            metrics.largest_win = Self::calculate_largest_win(trades);
            metrics.largest_loss = Self::calculate_largest_loss(trades);
        }
        
        metrics
    }
    
    fn calculate_annualized_return(daily_returns: &[crate::portfolio::DailyReturn]) -> Decimal {
        if daily_returns.is_empty() {
            return Decimal::ZERO;
        }
        
        let total_return = daily_returns.last().unwrap().cumulative_return;
        let years = Decimal::from(daily_returns.len()) / Decimal::from(252); // Trading days per year
        
        if years > Decimal::ZERO {
            // (1 + total_return)^(1/years) - 1
            // Simplified calculation
            total_return / years
        } else {
            Decimal::ZERO
        }
    }
    
    fn calculate_volatility(daily_returns: &[crate::portfolio::DailyReturn]) -> Decimal {
        if daily_returns.len() < 2 {
            return Decimal::ZERO;
        }
        
        let returns: Vec<Decimal> = daily_returns.iter()
            .map(|r| r.daily_return)
            .collect();
            
        let mean = returns.iter().sum::<Decimal>() / Decimal::from(returns.len());
        let variance = returns.iter()
            .map(|r| {
                let diff = *r - mean;
                let diff_f64 = diff.to_f64().unwrap_or(0.0);
                Decimal::from_f64_retain(diff_f64 * diff_f64).unwrap_or_default()
            })
            .sum::<Decimal>() / Decimal::from(returns.len() - 1);
            
        let variance_f64 = variance.to_f64().unwrap_or(0.0);
        let std_dev = Decimal::from_f64_retain(variance_f64.sqrt()).unwrap_or_default();
        let annualization_factor = Decimal::from_f64_retain((252.0_f64).sqrt()).unwrap_or_default();
        std_dev * annualization_factor
    }

    /// Calculate Sortino ratio (like Sharpe but only considers downside volatility)
    fn calculate_sortino_ratio(daily_returns: &[crate::portfolio::DailyReturn], risk_free_rate: Decimal) -> Option<Decimal> {
        if daily_returns.is_empty() {
            return None;
        }

        let annual_return = Self::calculate_annualized_return(daily_returns);
        let daily_risk_free = risk_free_rate / Decimal::from(252);
        
        // Calculate downside deviation (only negative returns)
        let downside_returns: Vec<Decimal> = daily_returns.iter()
            .map(|r| r.daily_return - daily_risk_free)
            .filter(|&r| r < Decimal::ZERO)
            .collect();

        if downside_returns.is_empty() {
            return Some(Decimal::from(9999)); // No downside volatility
        }

        let downside_variance = downside_returns.iter()
            .map(|&r| {
                let r_f64 = r.to_f64().unwrap_or(0.0);
                Decimal::from_f64_retain(r_f64 * r_f64).unwrap_or_default()
            })
            .sum::<Decimal>() / Decimal::from(downside_returns.len());

        let downside_std = Decimal::from_f64_retain(
            downside_variance.to_f64().unwrap_or(0.0).sqrt()
        ).unwrap_or_default();
        
        let annualized_downside_std = downside_std * Decimal::from_f64_retain((252.0_f64).sqrt()).unwrap_or_default();

        if annualized_downside_std > Decimal::ZERO {
            Some((annual_return - risk_free_rate) / annualized_downside_std)
        } else {
            None
        }
    }

    /// Calculate Calmar ratio (annualized return / max drawdown)
    fn calculate_calmar_ratio(daily_returns: &[crate::portfolio::DailyReturn], max_drawdown: Decimal) -> Option<Decimal> {
        if max_drawdown <= Decimal::ZERO {
            return None;
        }

        let annual_return = Self::calculate_annualized_return(daily_returns);
        Some(annual_return / max_drawdown.abs())
    }

    /// Calculate maximum drawdown duration in days
    fn calculate_max_drawdown_duration(daily_returns: &[crate::portfolio::DailyReturn]) -> Option<u32> {
        if daily_returns.is_empty() {
            return None;
        }

        let mut peak = daily_returns[0].portfolio_value;
        let mut max_duration = 0u32;
        let mut current_duration = 0u32;

        for daily_return in daily_returns {
            if daily_return.portfolio_value > peak {
                peak = daily_return.portfolio_value;
                current_duration = 0;
            } else {
                current_duration += 1;
                max_duration = max_duration.max(current_duration);
            }
        }

        if max_duration > 0 { Some(max_duration) } else { None }
    }

    /// Calculate Value at Risk (95% confidence)
    fn calculate_var_95(daily_returns: &[crate::portfolio::DailyReturn]) -> Option<Decimal> {
        if daily_returns.len() < 20 {
            return None; // Need sufficient data
        }

        let mut returns: Vec<Decimal> = daily_returns.iter()
            .map(|r| r.daily_return)
            .collect();
        
        returns.sort();
        
        let index = (returns.len() as f64 * 0.05) as usize; // 5th percentile
        let var = -returns[index]; // VaR is positive loss
        
        Some(var)
    }

    /// Calculate Conditional Value at Risk (95% confidence)
    fn calculate_cvar_95(daily_returns: &[crate::portfolio::DailyReturn]) -> Option<Decimal> {
        if daily_returns.len() < 20 {
            return None;
        }

        let mut returns: Vec<Decimal> = daily_returns.iter()
            .map(|r| r.daily_return)
            .collect();
        
        returns.sort();
        
        let index = (returns.len() as f64 * 0.05) as usize;
        let tail_returns = &returns[..=index];
        
        if tail_returns.is_empty() {
            return None;
        }

        let cvar = -(tail_returns.iter().sum::<Decimal>() / Decimal::from(tail_returns.len()));
        Some(cvar)
    }

    /// Calculate skewness of returns
    fn calculate_skewness(daily_returns: &[crate::portfolio::DailyReturn]) -> Option<Decimal> {
        if daily_returns.len() < 3 {
            return None;
        }

        let mean = daily_returns.iter()
            .map(|r| r.daily_return)
            .sum::<Decimal>() / Decimal::from(daily_returns.len());

        let variance = daily_returns.iter()
            .map(|r| {
                let diff = r.daily_return - mean;
                let diff_f64 = diff.to_f64().unwrap_or(0.0);
                Decimal::from_f64_retain(diff_f64 * diff_f64).unwrap_or_default()
            })
            .sum::<Decimal>() / Decimal::from(daily_returns.len());

        let std_dev = Decimal::from_f64_retain(variance.to_f64().unwrap_or(0.0).sqrt()).unwrap_or_default();
        
        if std_dev <= Decimal::ZERO {
            return None;
        }

        let skewness = daily_returns.iter()
            .map(|r| {
                let diff = r.daily_return - mean;
                let standardized = diff / std_dev;
                let standardized_f64 = standardized.to_f64().unwrap_or(0.0);
                Decimal::from_f64_retain(standardized_f64.powi(3)).unwrap_or_default()
            })
            .sum::<Decimal>() / Decimal::from(daily_returns.len());

        Some(skewness)
    }

    /// Calculate kurtosis of returns
    fn calculate_kurtosis(daily_returns: &[crate::portfolio::DailyReturn]) -> Option<Decimal> {
        if daily_returns.len() < 4 {
            return None;
        }

        let mean = daily_returns.iter()
            .map(|r| r.daily_return)
            .sum::<Decimal>() / Decimal::from(daily_returns.len());

        let variance = daily_returns.iter()
            .map(|r| {
                let diff = r.daily_return - mean;
                let diff_f64 = diff.to_f64().unwrap_or(0.0);
                Decimal::from_f64_retain(diff_f64 * diff_f64).unwrap_or_default()
            })
            .sum::<Decimal>() / Decimal::from(daily_returns.len());

        let std_dev = Decimal::from_f64_retain(variance.to_f64().unwrap_or(0.0).sqrt()).unwrap_or_default();
        
        if std_dev <= Decimal::ZERO {
            return None;
        }

        let kurtosis = daily_returns.iter()
            .map(|r| {
                let diff = r.daily_return - mean;
                let standardized = diff / std_dev;
                let standardized_f64 = standardized.to_f64().unwrap_or(0.0);
                Decimal::from_f64_retain(standardized_f64.powi(4)).unwrap_or_default()
            })
            .sum::<Decimal>() / Decimal::from(daily_returns.len());

        Some(kurtosis - Decimal::from(3)) // Excess kurtosis
    }

    /// Calculate win rate from trades
    fn calculate_win_rate(trades: &[TradeRecord]) -> Decimal {
        if trades.is_empty() {
            return Decimal::ZERO;
        }

        let winning_trades = trades.iter()
            .filter(|trade| trade.pnl.unwrap_or(Decimal::ZERO) > Decimal::ZERO)
            .count();

        Decimal::from(winning_trades) / Decimal::from(trades.len())
    }

    /// Calculate profit factor (gross profit / gross loss)
    fn calculate_profit_factor(trades: &[TradeRecord]) -> Option<Decimal> {
        let gross_profit: Decimal = trades.iter()
            .filter_map(|trade| trade.pnl)
            .filter(|&pnl| pnl > Decimal::ZERO)
            .sum();

        let gross_loss: Decimal = trades.iter()
            .filter_map(|trade| trade.pnl)
            .filter(|&pnl| pnl < Decimal::ZERO)
            .map(|pnl| pnl.abs())
            .sum();

        if gross_loss > Decimal::ZERO {
            Some(gross_profit / gross_loss)
        } else if gross_profit > Decimal::ZERO {
            Some(Decimal::from(9999)) // Only winning trades
        } else {
            None
        }
    }

    /// Calculate average winning trade
    fn calculate_average_win(trades: &[TradeRecord]) -> Decimal {
        let winning_trades: Vec<Decimal> = trades.iter()
            .filter_map(|trade| trade.pnl)
            .filter(|&pnl| pnl > Decimal::ZERO)
            .collect();

        if winning_trades.is_empty() {
            Decimal::ZERO
        } else {
            winning_trades.iter().sum::<Decimal>() / Decimal::from(winning_trades.len())
        }
    }

    /// Calculate average losing trade
    fn calculate_average_loss(trades: &[TradeRecord]) -> Decimal {
        let losing_trades: Vec<Decimal> = trades.iter()
            .filter_map(|trade| trade.pnl)
            .filter(|&pnl| pnl < Decimal::ZERO)
            .collect();

        if losing_trades.is_empty() {
            Decimal::ZERO
        } else {
            losing_trades.iter().sum::<Decimal>() / Decimal::from(losing_trades.len())
        }
    }

    /// Calculate largest winning trade
    fn calculate_largest_win(trades: &[TradeRecord]) -> Decimal {
        trades.iter()
            .filter_map(|trade| trade.pnl)
            .filter(|&pnl| pnl > Decimal::ZERO)
            .max()
            .unwrap_or(Decimal::ZERO)
    }

    /// Calculate largest losing trade
    fn calculate_largest_loss(trades: &[TradeRecord]) -> Decimal {
        trades.iter()
            .filter_map(|trade| trade.pnl)
            .filter(|&pnl| pnl < Decimal::ZERO)
            .min()
            .unwrap_or(Decimal::ZERO)
    }
}

/// Point on the equity curve
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquityCurvePoint {
    pub timestamp: DateTime<Utc>,
    pub portfolio_value: Decimal,
    pub cash: Decimal,
    pub positions_value: Decimal,
    pub total_pnl: Decimal,
    pub daily_return: Option<Decimal>,
    pub cumulative_return: Decimal,
    pub drawdown: Decimal,
}

/// Trade record for analysis
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeRecord {
    pub id: Uuid,
    pub symbol: Symbol,
    pub entry_time: DateTime<Utc>,
    pub exit_time: Option<DateTime<Utc>>,
    pub entry_price: Decimal,
    pub exit_price: Option<Decimal>,
    pub quantity: Decimal,
    pub side: crate::orders::Side,
    pub pnl: Option<Decimal>,
    pub commission: Decimal,
    pub duration_hours: Option<f64>,
    pub strategy_id: String,
    pub tags: Vec<String>,
}

/// Backtest event for real-time monitoring
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BacktestEvent {
    Started { backtest_id: BacktestId, config: BacktestConfig },
    Progress { backtest_id: BacktestId, progress_pct: f64, current_date: DateTime<Utc> },
    EquityUpdate { backtest_id: BacktestId, point: EquityCurvePoint },
    TradeExecuted { backtest_id: BacktestId, trade: TradeRecord },
    Completed { backtest_id: BacktestId, result: BacktestResult },
    Failed { backtest_id: BacktestId, error: String },
} 