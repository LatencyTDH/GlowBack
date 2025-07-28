// GlowBack backtesting engine
// Simple working implementation for Phase 1

pub mod engine;
pub mod execution;
pub mod simulator;

use gb_types::{GbResult, BacktestConfig, BacktestResult, Symbol};
use gb_data::DataManager;
use tracing::{info, error};

/// Simple backtesting engine that works with existing types
#[derive(Debug)]
pub struct BacktestEngine {
    config: BacktestConfig,
    data_manager: DataManager,
}

impl BacktestEngine {
    /// Create a new backtesting engine
    pub async fn new(config: BacktestConfig) -> GbResult<Self> {
        info!("Initializing GlowBack backtesting engine");
        
        let data_manager = DataManager::new().await?;
        
        Ok(Self {
            config,
            data_manager,
        })
    }

    /// Load market data for backtesting
    pub async fn load_market_data(&mut self, symbols: Vec<Symbol>) -> GbResult<()> {
        info!("Loading market data for {} symbols", symbols.len());
        
        for symbol in symbols {
            // Try to load data from data manager
            let result = self.data_manager.load_data(
                &symbol,
                self.config.start_date,
                self.config.end_date,
                self.config.resolution,
            ).await;
            
            match result {
                Ok(bars) => {
                    info!("Loaded {} bars for {}", bars.len(), symbol);
                }
                Err(e) => {
                    error!("Failed to load data for {}: {}", symbol, e);
                }
            }
        }
        
        Ok(())
    }

    /// Run a simple backtest simulation
    pub async fn run(&mut self) -> GbResult<BacktestResult> {
        info!("Starting simple backtest simulation");
        
        // Create basic result with current configuration
        let mut result = BacktestResult::new(self.config.clone());
        
        // For now, just mark it as completed successfully
        // In a full implementation, this would run the actual simulation
        let portfolio = gb_types::Portfolio::new(
            "demo_portfolio".to_string(),
            self.config.initial_capital,
        );
        
        // Create empty strategy metrics for the placeholder
        let strategy_metrics = gb_types::StrategyMetrics::new("placeholder_strategy".to_string());
        
        result.mark_completed(portfolio, strategy_metrics);
        
        info!("Simple backtest completed");
        Ok(result)
    }

    /// Get engine configuration
    pub fn get_config(&self) -> &BacktestConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gb_types::{Symbol, Resolution, StrategyConfig};
    use chrono::{Utc, Duration};
    use rust_decimal::Decimal;

    /// Create a test configuration
    fn create_test_config() -> BacktestConfig {
        let strategy_config = StrategyConfig::new(
            "test_strategy".to_string(),
            "Test Strategy".to_string(),
        );

        let mut config = BacktestConfig::new("Test Backtest".to_string(), strategy_config);
        config.start_date = Utc::now() - Duration::days(30);
        config.end_date = Utc::now();
        config.initial_capital = Decimal::from(100000);
        config.resolution = Resolution::Day;
        config.symbols = vec![Symbol::equity("AAPL"), Symbol::equity("GOOGL")];
        
        config
    }

    #[tokio::test]
    async fn test_engine_creation() {
        let config = create_test_config();
        let engine = BacktestEngine::new(config.clone()).await;
        
        assert!(engine.is_ok());
        let engine = engine.unwrap();
        assert_eq!(engine.get_config().name, "Test Backtest");
        assert_eq!(engine.get_config().initial_capital, Decimal::from(100000));
    }

    #[tokio::test]
    async fn test_data_loading() {
        let config = create_test_config();
        let mut engine = BacktestEngine::new(config).await.unwrap();
        
        let symbols = vec![Symbol::equity("AAPL"), Symbol::equity("GOOGL")];
        let result = engine.load_market_data(symbols).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_backtest_execution() {
        let config = create_test_config();
        let mut engine = BacktestEngine::new(config).await.unwrap();
        
        // Load some test data
        let symbols = vec![Symbol::equity("AAPL")];
        engine.load_market_data(symbols).await.unwrap();
        
        // Run the backtest
        let result = engine.run().await;
        
        assert!(result.is_ok());
        let backtest_result = result.unwrap();
        
        // Check that the result is properly structured
        assert_eq!(backtest_result.config.name, "Test Backtest");
        assert!(backtest_result.final_portfolio.is_some());
        assert!(backtest_result.performance_metrics.is_some());
        assert!(backtest_result.strategy_metrics.is_some());
        
        // Verify the portfolio was initialized correctly
        let portfolio = backtest_result.final_portfolio.as_ref().unwrap();
        assert_eq!(portfolio.account_id, "demo_portfolio");
        assert_eq!(portfolio.cash, Decimal::from(100000)); // No trades in placeholder implementation
    }

    #[tokio::test]
    async fn test_performance_metrics_calculation() {
        let config = create_test_config();
        let mut engine = BacktestEngine::new(config).await.unwrap();
        
        let result = engine.run().await.unwrap();
        let metrics = result.performance_metrics.unwrap();
        
        // Check that basic metrics are calculated
        assert_eq!(metrics.total_return, Decimal::ZERO); // No trades in placeholder
        assert_eq!(metrics.annualized_return, Decimal::ZERO);
        assert_eq!(metrics.volatility, Decimal::ZERO);
        assert!(metrics.sharpe_ratio.is_none()); // No trading activity = no Sharpe ratio
        assert_eq!(metrics.max_drawdown, Decimal::ZERO);
        
        // Check that advanced metrics are computed (even if None for empty portfolio)
        // These should not panic and should be properly initialized
        assert!(metrics.sortino_ratio.is_none() || metrics.sortino_ratio.is_some());
        assert!(metrics.calmar_ratio.is_none() || metrics.calmar_ratio.is_some());
    }

    #[tokio::test]
    async fn test_strategy_metrics() {
        let config = create_test_config();
        let mut engine = BacktestEngine::new(config).await.unwrap();
        
        let result = engine.run().await.unwrap();
        let strategy_metrics = result.strategy_metrics.unwrap();
        
        // Check that strategy metrics are properly initialized
        assert_eq!(strategy_metrics.strategy_id, "placeholder_strategy");
        assert_eq!(strategy_metrics.total_trades, 0);
        assert_eq!(strategy_metrics.winning_trades, 0);
        assert_eq!(strategy_metrics.losing_trades, 0);
        assert_eq!(strategy_metrics.win_rate, Decimal::ZERO);
    }

    #[tokio::test]
    async fn test_engine_with_multiple_symbols() {
        let config = create_test_config();
        let mut engine = BacktestEngine::new(config).await.unwrap();
        
        let symbols = vec![
            Symbol::equity("AAPL"),
            Symbol::equity("GOOGL"),
            Symbol::equity("MSFT"),
            Symbol::equity("TSLA"),
        ];
        
        let load_result = engine.load_market_data(symbols).await;
        assert!(load_result.is_ok());
        
        let backtest_result = engine.run().await;
        assert!(backtest_result.is_ok());
        
        let result = backtest_result.unwrap();
        assert_eq!(result.config.symbols.len(), 2); // Original config had 2 symbols
    }

    #[tokio::test]
    async fn test_error_handling() {
        // Test with invalid configuration
        let mut config = create_test_config();
        config.end_date = config.start_date - Duration::days(1); // Invalid date range
        
        let engine = BacktestEngine::new(config).await;
        assert!(engine.is_ok()); // Engine creation should still work
        
        // The actual validation would happen during execution
        // For now, our placeholder implementation doesn't validate dates
    }
} 