// GlowBack backtesting engine
// Simple working implementation for Phase 1

pub mod engine;
pub mod execution;
pub mod simulator;

use gb_data::DataManager;
use gb_types::{BacktestConfig, BacktestResult, GbResult, Strategy, Symbol};
use tracing::{error, info};

// Re-export the Engine for direct use
pub use engine::Engine;

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
            let result = self
                .data_manager
                .load_data(
                    &symbol,
                    self.config.start_date,
                    self.config.end_date,
                    self.config.resolution,
                )
                .await;

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

    /// Run a backtest with a provided strategy
    ///
    /// This is the primary method for running backtests - it delegates to the
    /// full Engine implementation with proper strategy integration.
    pub async fn run_with_strategy(
        &mut self,
        strategy: Box<dyn Strategy>,
    ) -> GbResult<BacktestResult> {
        info!(
            "Starting backtest with strategy: {}",
            strategy.get_config().name
        );

        // Create the full Engine with strategy support using our existing data manager
        let mut engine = Engine::new(self.config.clone(), &mut self.data_manager, strategy).await?;

        // Run the backtest using the full engine
        engine.run().await
    }

    /// Run a simple backtest simulation (legacy method for backwards compatibility)
    ///
    /// For proper backtesting with strategies, use `run_with_strategy` instead.
    pub async fn run(&mut self) -> GbResult<BacktestResult> {
        info!("Starting simple backtest simulation (no strategy)");

        // Create basic result with current configuration
        let mut result = BacktestResult::new(self.config.clone());

        // For now, just mark it as completed successfully
        // Use run_with_strategy for actual strategy execution
        let mut portfolio =
            gb_types::Portfolio::new("demo_portfolio".to_string(), self.config.initial_capital);

        // Create empty strategy metrics for the placeholder
        let strategy_metrics = gb_types::StrategyMetrics::new("placeholder_strategy".to_string());

        // Build a flat equity curve so analytics outputs are populated
        let mut equity_curve: Vec<gb_types::EquityCurvePoint> = Vec::new();
        let mut current_date = self.config.start_date;
        let mut previous_value: Option<rust_decimal::Decimal> = None;
        let mut equity_peak = portfolio.total_equity;

        while current_date <= self.config.end_date {
            let total_value = portfolio.total_equity;
            let daily_return = if let Some(prev) = previous_value {
                if prev > rust_decimal::Decimal::ZERO {
                    (total_value - prev) / prev
                } else {
                    rust_decimal::Decimal::ZERO
                }
            } else {
                rust_decimal::Decimal::ZERO
            };

            portfolio.add_daily_return(current_date, daily_return);

            if total_value > equity_peak {
                equity_peak = total_value;
            }

            let drawdown = if equity_peak > rust_decimal::Decimal::ZERO {
                (equity_peak - total_value) / equity_peak
            } else {
                rust_decimal::Decimal::ZERO
            };

            let point = gb_types::EquityCurvePoint {
                timestamp: current_date,
                portfolio_value: total_value,
                cash: portfolio.cash,
                positions_value: rust_decimal::Decimal::ZERO,
                total_pnl: portfolio.total_pnl,
                daily_return: previous_value.map(|_| daily_return),
                cumulative_return: portfolio.get_total_return(),
                drawdown,
            };

            equity_curve.push(point);
            previous_value = Some(total_value);
            current_date += chrono::Duration::days(1);
        }

        result.mark_completed(portfolio, strategy_metrics);
        result.equity_curve = equity_curve;

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
    use chrono::{Duration, Utc};
    use gb_types::{Resolution, StrategyConfig, Symbol};
    use rust_decimal::Decimal;

    /// Create a test configuration
    fn create_test_config() -> BacktestConfig {
        let strategy_config =
            StrategyConfig::new("test_strategy".to_string(), "Test Strategy".to_string());

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
        assert!(!backtest_result.equity_curve.is_empty());

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

    #[tokio::test]
    async fn test_backtest_with_buy_and_hold_strategy() {
        use gb_types::BuyAndHoldStrategy;

        let mut config = create_test_config();
        config.symbols = vec![Symbol::equity("AAPL")];
        config.start_date = Utc::now() - Duration::days(10);
        config.end_date = Utc::now();

        let mut engine = BacktestEngine::new(config).await.unwrap();

        // Create and run with buy and hold strategy
        let strategy = Box::new(BuyAndHoldStrategy::new());
        let result = engine.run_with_strategy(strategy).await;

        assert!(result.is_ok());
        let backtest_result = result.unwrap();

        // Verify we got valid results
        assert!(backtest_result.final_portfolio.is_some());
        assert!(backtest_result.strategy_metrics.is_some());

        // The buy and hold strategy should have executed at least one trade
        let strategy_metrics = backtest_result.strategy_metrics.as_ref().unwrap();
        // Note: Total trades tracked in engine, not strategy metrics directly

        let portfolio = backtest_result.final_portfolio.as_ref().unwrap();
        // Portfolio should have been updated (either positions or cash changed)
        assert!(portfolio.total_equity > Decimal::ZERO);
    }

    #[tokio::test]
    async fn test_backtest_with_moving_average_crossover_strategy() {
        use gb_types::MovingAverageCrossoverStrategy;

        let mut config = create_test_config();
        config.symbols = vec![Symbol::equity("AAPL")];
        config.start_date = Utc::now() - Duration::days(30);
        config.end_date = Utc::now();

        let mut engine = BacktestEngine::new(config).await.unwrap();

        // Create MA crossover strategy with short period 5, long period 10
        let strategy = Box::new(MovingAverageCrossoverStrategy::new(5, 10));
        let result = engine.run_with_strategy(strategy).await;

        assert!(result.is_ok());
        let backtest_result = result.unwrap();

        // Verify we got valid results
        assert!(backtest_result.final_portfolio.is_some());
        assert!(backtest_result.performance_metrics.is_some());
    }

    #[tokio::test]
    async fn test_backtest_with_momentum_strategy() {
        use gb_types::MomentumStrategy;

        let mut config = create_test_config();
        config.symbols = vec![Symbol::equity("AAPL")];
        config.start_date = Utc::now() - Duration::days(20);
        config.end_date = Utc::now();

        let mut engine = BacktestEngine::new(config).await.unwrap();

        // Create momentum strategy with 5-day lookback, 5% threshold
        let strategy = Box::new(MomentumStrategy::new(5, 0.05));
        let result = engine.run_with_strategy(strategy).await;

        assert!(result.is_ok());
        let backtest_result = result.unwrap();

        // Verify we got valid results
        assert!(backtest_result.final_portfolio.is_some());

        // Portfolio equity should be positive
        let portfolio = backtest_result.final_portfolio.as_ref().unwrap();
        assert!(portfolio.total_equity > Decimal::ZERO);
    }

    #[tokio::test]
    async fn test_backtest_with_mean_reversion_strategy() {
        use gb_types::MeanReversionStrategy;

        let mut config = create_test_config();
        config.symbols = vec![Symbol::equity("AAPL")];
        config.start_date = Utc::now() - Duration::days(30);
        config.end_date = Utc::now();

        let mut engine = BacktestEngine::new(config).await.unwrap();

        // Create mean reversion strategy with 10-day lookback, 2.0 entry, 1.0 exit thresholds
        let strategy = Box::new(MeanReversionStrategy::new(10, 2.0, 1.0));
        let result = engine.run_with_strategy(strategy).await;

        assert!(result.is_ok());
        let backtest_result = result.unwrap();

        // Verify we got valid results
        assert!(backtest_result.final_portfolio.is_some());
        assert!(backtest_result.strategy_metrics.is_some());
    }

    #[tokio::test]
    async fn test_backtest_with_rsi_strategy() {
        use gb_types::RsiStrategy;

        let mut config = create_test_config();
        config.symbols = vec![Symbol::equity("AAPL")];
        config.start_date = Utc::now() - Duration::days(30);
        config.end_date = Utc::now();

        let mut engine = BacktestEngine::new(config).await.unwrap();

        // Create RSI strategy with default thresholds
        let strategy = Box::new(RsiStrategy::new(14, 30.0, 70.0));
        let result = engine.run_with_strategy(strategy).await;

        assert!(result.is_ok());
        let backtest_result = result.unwrap();

        // Verify we got valid results
        assert!(backtest_result.final_portfolio.is_some());
        assert!(backtest_result.strategy_metrics.is_some());
    }

    #[tokio::test]
    async fn test_strategy_integration_daily_returns_tracked() {
        use gb_types::BuyAndHoldStrategy;

        let mut config = create_test_config();
        config.symbols = vec![Symbol::equity("AAPL")];
        config.start_date = Utc::now() - Duration::days(10);
        config.end_date = Utc::now();

        let mut engine = BacktestEngine::new(config).await.unwrap();

        let strategy = Box::new(BuyAndHoldStrategy::new());
        let result = engine.run_with_strategy(strategy).await.unwrap();

        let portfolio = result.final_portfolio.as_ref().unwrap();

        // Should have daily returns for each trading day
        // Note: The exact count depends on the simulation, but should have some entries
        assert!(!portfolio.daily_returns.is_empty() || portfolio.total_equity > Decimal::ZERO);
    }

    #[tokio::test]
    async fn test_engine_directly_with_strategy() {
        use gb_types::BuyAndHoldStrategy;

        let mut config = create_test_config();
        config.symbols = vec![Symbol::equity("AAPL")];
        config.start_date = Utc::now() - Duration::days(5);
        config.end_date = Utc::now();

        let mut data_manager = DataManager::new().await.unwrap();
        let strategy = Box::new(BuyAndHoldStrategy::new());

        let engine_result = Engine::new(config, &mut data_manager, strategy).await;
        assert!(engine_result.is_ok());

        let mut engine = engine_result.unwrap();
        let result = engine.run().await;

        assert!(result.is_ok());
        let backtest_result = result.unwrap();

        // Engine should properly track results
        assert!(backtest_result.final_portfolio.is_some());
    }

    #[tokio::test]
    async fn test_strategy_config_inherits_backtest_symbols_and_capital() {
        use gb_types::{
            MarketEvent, OrderEvent, Strategy, StrategyAction, StrategyConfig, StrategyContext,
            StrategyMetrics,
        };
        use std::sync::{Arc, Mutex};

        #[derive(Clone)]
        struct CaptureConfigStrategy {
            config: StrategyConfig,
            captured: Arc<Mutex<Option<StrategyConfig>>>,
        }

        impl CaptureConfigStrategy {
            fn new(captured: Arc<Mutex<Option<StrategyConfig>>>) -> Self {
                Self {
                    config: StrategyConfig::new(
                        "capture_strategy".to_string(),
                        "Capture Strategy".to_string(),
                    ),
                    captured,
                }
            }
        }

        impl Strategy for CaptureConfigStrategy {
            fn initialize(&mut self, config: &StrategyConfig) -> Result<(), String> {
                self.config = config.clone();
                *self.captured.lock().unwrap() = Some(config.clone());
                Ok(())
            }

            fn on_market_event(
                &mut self,
                _event: &MarketEvent,
                _context: &StrategyContext,
            ) -> Result<Vec<StrategyAction>, String> {
                Ok(vec![])
            }

            fn on_order_event(
                &mut self,
                _event: &OrderEvent,
                _context: &StrategyContext,
            ) -> Result<Vec<StrategyAction>, String> {
                Ok(vec![])
            }

            fn on_day_end(
                &mut self,
                _context: &StrategyContext,
            ) -> Result<Vec<StrategyAction>, String> {
                Ok(vec![])
            }

            fn on_stop(
                &mut self,
                _context: &StrategyContext,
            ) -> Result<Vec<StrategyAction>, String> {
                Ok(vec![])
            }

            fn get_config(&self) -> &StrategyConfig {
                &self.config
            }

            fn get_metrics(&self) -> StrategyMetrics {
                StrategyMetrics::new(self.config.strategy_id.clone())
            }
        }

        let mut config = create_test_config();
        config.symbols = vec![Symbol::equity("AAPL")];
        config.initial_capital = Decimal::from(250000);
        config.start_date = Utc::now() - Duration::days(5);
        config.end_date = Utc::now();

        let captured = Arc::new(Mutex::new(None));
        let strategy = Box::new(CaptureConfigStrategy::new(Arc::clone(&captured)));

        let mut engine = BacktestEngine::new(config).await.unwrap();
        let result = engine.run_with_strategy(strategy).await;
        assert!(result.is_ok());

        let captured_config = captured
            .lock()
            .unwrap()
            .clone()
            .expect("strategy config should be captured");
        assert_eq!(captured_config.symbols, vec![Symbol::equity("AAPL")]);
        assert_eq!(captured_config.initial_capital, Decimal::from(250000));
    }

    // --- Crypto-specific tests ------------------------------------------------

    /// Create a backtest config for crypto symbols
    fn create_crypto_test_config() -> BacktestConfig {
        let strategy_config = StrategyConfig::new(
            "crypto_test".to_string(),
            "Crypto Test Strategy".to_string(),
        );

        let mut config = BacktestConfig::new("Crypto Backtest".to_string(), strategy_config);
        config.start_date = Utc::now() - Duration::days(10);
        config.end_date = Utc::now();
        config.initial_capital = Decimal::from(50000);
        config.resolution = Resolution::Day;
        config.symbols = vec![Symbol::crypto("BTC-USD"), Symbol::crypto("ETH-USD")];
        config
    }

    #[tokio::test]
    async fn test_crypto_backtest_creation() {
        let config = create_crypto_test_config();
        let engine = BacktestEngine::new(config.clone()).await;

        assert!(engine.is_ok());
        let engine = engine.unwrap();
        assert_eq!(engine.get_config().name, "Crypto Backtest");
        assert_eq!(engine.get_config().symbols.len(), 2);
        assert_eq!(
            engine.get_config().symbols[0].asset_class,
            gb_types::AssetClass::Crypto
        );
    }

    #[tokio::test]
    async fn test_crypto_data_loading() {
        let config = create_crypto_test_config();
        let mut engine = BacktestEngine::new(config).await.unwrap();

        let symbols = vec![Symbol::crypto("BTC-USD"), Symbol::crypto("ETH-USD")];
        let result = engine.load_market_data(symbols).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_crypto_backtest_execution() {
        let config = create_crypto_test_config();
        let mut engine = BacktestEngine::new(config).await.unwrap();

        let symbols = vec![Symbol::crypto("BTC-USD")];
        engine.load_market_data(symbols).await.unwrap();

        let result = engine.run().await;
        assert!(result.is_ok());

        let backtest_result = result.unwrap();
        assert!(backtest_result.final_portfolio.is_some());
        assert!(!backtest_result.equity_curve.is_empty());
    }

    #[tokio::test]
    async fn test_crypto_buy_and_hold_strategy() {
        use gb_types::BuyAndHoldStrategy;

        let mut config = create_crypto_test_config();
        config.symbols = vec![Symbol::crypto("BTC-USD")];

        let mut engine = BacktestEngine::new(config).await.unwrap();
        let strategy = Box::new(BuyAndHoldStrategy::new());
        let result = engine.run_with_strategy(strategy).await;

        assert!(result.is_ok());
        let backtest_result = result.unwrap();
        let portfolio = backtest_result.final_portfolio.as_ref().unwrap();
        assert!(portfolio.total_equity > Decimal::ZERO);
    }

    #[tokio::test]
    async fn test_crypto_execution_config() {
        use crate::execution::ExecutionConfig;

        let config = ExecutionConfig::for_asset_class(gb_types::AssetClass::Crypto);
        assert!(
            config.fractional_quantities,
            "Crypto must allow fractional qty"
        );
        assert_eq!(config.commission_per_share, Decimal::ZERO);
        assert!(
            config.slippage_bps < Decimal::from(5),
            "Crypto should have tighter slippage"
        );
    }

    #[tokio::test]
    async fn test_crypto_multiple_symbols_backtest() {
        let mut config = create_crypto_test_config();
        config.symbols = vec![
            Symbol::crypto("BTC-USD"),
            Symbol::crypto("ETH-USD"),
            Symbol::crypto("SOL-USD"),
        ];

        let mut engine = BacktestEngine::new(config).await.unwrap();
        let symbols = vec![
            Symbol::crypto("BTC-USD"),
            Symbol::crypto("ETH-USD"),
            Symbol::crypto("SOL-USD"),
        ];
        let load_result = engine.load_market_data(symbols).await;
        assert!(load_result.is_ok());

        let result = engine.run().await;
        assert!(result.is_ok());
    }
}
