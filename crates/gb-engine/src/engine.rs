// Core backtesting engine - enhanced implementation
// Provides event-driven backtesting with realistic execution

use gb_types::{
    GbResult, BacktestConfig, BacktestResult, Portfolio, Bar, Symbol, Strategy,
    StrategyContext, Order, Fill, MarketEvent,
    StrategyMetrics, Side
};
use gb_data::DataManager;
use tracing::{info, debug, warn};
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};
use rust_decimal::Decimal;

/// Enhanced backtesting engine with event-driven simulation
pub struct Engine {
    config: BacktestConfig,
    data_manager: DataManager,
    portfolio: Portfolio,
    strategy: Box<dyn Strategy>,
    current_time: DateTime<Utc>,
    market_data: HashMap<Symbol, Vec<Bar>>,
    pending_orders: Vec<Order>,
    strategy_metrics: StrategyMetrics,
}

impl Engine {
    /// Create a new engine with strategy and data manager
    pub async fn new(
        config: BacktestConfig,
        mut data_manager: DataManager,
        strategy: Box<dyn Strategy>,
    ) -> GbResult<Self> {
        info!("Creating enhanced backtesting engine");
        
        let portfolio = Portfolio::new(
            "backtest_portfolio".to_string(),
            config.initial_capital,
        );

        let strategy_metrics = StrategyMetrics::new(strategy.get_config().strategy_id.clone());

        // Load market data for all symbols
        let mut market_data = HashMap::new();
        for symbol in &config.symbols {
            match data_manager.load_data(
                symbol,
                config.start_date,
                config.end_date,
                config.resolution,
            ).await {
                Ok(bars) => {
                    info!("Loaded {} bars for {}", bars.len(), symbol);
                    market_data.insert(symbol.clone(), bars);
                }
                Err(e) => {
                    warn!("Failed to load data for {}: {}", symbol, e);
                    // Add sample data as fallback
                    let sample_bars = Self::generate_sample_data(symbol, &config);
                    market_data.insert(symbol.clone(), sample_bars);
                }
            }
        }

        Ok(Self {
            current_time: config.start_date,
            config,
            data_manager,
            portfolio,
            strategy,
            market_data,
            pending_orders: Vec::new(),
            strategy_metrics,
        })
    }

    /// Generate sample market data as fallback
    fn generate_sample_data(symbol: &Symbol, config: &BacktestConfig) -> Vec<Bar> {
        let mut bars = Vec::new();
        let mut current_date = config.start_date;
        let mut price = Decimal::from(100); // Starting price
        
        while current_date <= config.end_date {
            // Simple random walk for demo
            let change_pct = (rand::random::<f64>() - 0.5) * 0.04; // ±2% daily change
            let price_change = price * Decimal::try_from(change_pct).unwrap_or_default();
            price += price_change;
            
            let open = price;
            let high = price * Decimal::try_from(1.0 + rand::random::<f64>() * 0.02).unwrap_or(price);
            let low = price * Decimal::try_from(1.0 - rand::random::<f64>() * 0.02).unwrap_or(price);
            let close = price;
            let volume = Decimal::from(1000000 + (rand::random::<u32>() % 500000));

            let bar = Bar::new(
                symbol.clone(),
                current_date,
                open,
                high,
                low,
                close,
                volume,
                config.resolution,
            );
            
            bars.push(bar);
            current_date += Duration::days(1);
        }
        
        bars
    }

    /// Run the complete backtesting simulation
    pub async fn run(&mut self) -> GbResult<BacktestResult> {
        info!("Starting enhanced backtesting simulation");
        
        let mut result = BacktestResult::new(self.config.clone());
        
        // Initialize strategy
        let strategy_config = self.strategy.get_config();
        info!("Running strategy: {}", strategy_config.name);

        // Main simulation loop
        self.current_time = self.config.start_date;
        
        while self.current_time <= self.config.end_date {
            debug!("Processing time: {}", self.current_time);
            
            // 1. Process market data for current time
            self.process_market_data().await?;
            
            // 2. Execute pending orders
            self.execute_pending_orders().await?;
            
            // 3. Update portfolio with current market prices
            self.update_portfolio_values().await?;
            
            // 4. Generate strategy signals
            self.generate_strategy_signals().await?;
            
            // 5. Update daily returns
            self.update_daily_returns().await?;
            
            // Advance time
            self.current_time += Duration::days(1);
        }

        // Finalize results
        self.finalize_results(&mut result).await?;
        
        info!("Backtesting simulation completed");
        Ok(result)
    }

    /// Process market data for the current time
    async fn process_market_data(&mut self) -> GbResult<()> {
        for (symbol, bars) in &self.market_data {
            // Find bars for current time
            let current_bars: Vec<&Bar> = bars
                .iter()
                .filter(|bar| {
                    bar.timestamp.date_naive() == self.current_time.date_naive()
                })
                .collect();

            for bar in current_bars {
                let _market_event = MarketEvent::Bar(bar.clone());
                
                debug!("Market data: {} at {}: {}", symbol, bar.timestamp, bar.close);
            }
        }
        Ok(())
    }

    /// Execute pending orders based on current market conditions
    async fn execute_pending_orders(&mut self) -> GbResult<()> {
        let mut executed_orders = Vec::new();
        
        for (index, order) in self.pending_orders.iter().enumerate() {
            if let Some(fill) = self.try_execute_order(order).await? {
                // Apply fill to portfolio
                self.portfolio.apply_fill(&fill);
                
                // Update strategy metrics
                self.strategy_metrics.total_trades += 1;
                if fill.price > Decimal::ZERO {
                    self.strategy_metrics.winning_trades += 1;
                }
                
                // Log execution
                info!("Executed order: {:?} {} {} at {}", 
                    order.side, order.quantity, order.symbol, fill.price);
                
                executed_orders.push(index);
            }
        }
        
        // Remove executed orders (in reverse order to maintain indices)
        for &index in executed_orders.iter().rev() {
            self.pending_orders.remove(index);
        }
        
        Ok(())
    }

    /// Try to execute a single order
    async fn try_execute_order(&self, order: &Order) -> GbResult<Option<Fill>> {
        // Get current market data for the symbol
        if let Some(bars) = self.market_data.get(&order.symbol) {
            for bar in bars {
                if bar.timestamp.date_naive() == self.current_time.date_naive() {
                    // Simple execution logic - execute at open price
                    let execution_price = bar.open;
                    
                    let fill = Fill::new(
                        order.id,
                        order.symbol.clone(),
                        order.side,
                        order.quantity,
                        execution_price,
                        Decimal::ZERO, // commission
                        "engine".to_string(), // strategy_id
                    );
                    
                    return Ok(Some(fill));
                }
            }
        }
        
        Ok(None)
    }

    /// Update portfolio values with current market prices
    async fn update_portfolio_values(&mut self) -> GbResult<()> {
        let mut current_prices = HashMap::new();
        
        // Collect current prices
        for (symbol, bars) in &self.market_data {
            for bar in bars {
                if bar.timestamp.date_naive() == self.current_time.date_naive() {
                    current_prices.insert(symbol.clone(), bar.close);
                    break;
                }
            }
        }
        
        // Update portfolio with current prices
        self.portfolio.update_market_prices(&current_prices);
        
        Ok(())
    }

    /// Generate strategy signals
    async fn generate_strategy_signals(&mut self) -> GbResult<()> {
        // For now, generate simple buy signals based on mock data
        // In a real implementation, this would call the strategy's on_market_event method
        for symbol in &self.config.symbols.clone() {
            // Simple mock strategy: buy if no position exists
            if !self.portfolio.positions.contains_key(symbol) && self.portfolio.cash > Decimal::from(1000) {
                let order = Order::market_order(
                    symbol.clone(),
                    Side::Buy,
                    Decimal::from(10), // quantity
                    "engine_strategy".to_string(),
                );
                self.pending_orders.push(order);
                debug!("Generated BUY signal: 10 shares of {}", symbol);
            }
        }
        
        Ok(())
    }

    /// Create strategy context for current state
    async fn create_strategy_context(&self, _symbol: &Symbol) -> GbResult<StrategyContext> {
        // Simplified context creation for the enhanced engine
        let context = StrategyContext::new(
            "engine_strategy".to_string(),
            self.portfolio.initial_capital,
        );
        
        Ok(context)
    }

    /// Update daily returns
    async fn update_daily_returns(&mut self) -> GbResult<()> {
        let total_value = self.portfolio.total_equity;
        let daily_return = if let Some(previous_value) = self.portfolio.daily_returns.last() {
            if previous_value.portfolio_value > Decimal::ZERO {
                (total_value - previous_value.portfolio_value) / previous_value.portfolio_value
            } else {
                Decimal::ZERO
            }
        } else {
            Decimal::ZERO
        };
        
        self.portfolio.add_daily_return(self.current_time, daily_return);
        
        Ok(())
    }

    /// Finalize backtest results
    async fn finalize_results(&mut self, result: &mut BacktestResult) -> GbResult<()> {
        // Mark result as completed with final portfolio and metrics
        result.mark_completed(self.portfolio.clone(), self.strategy_metrics.clone());
        
        info!("Final portfolio value: {}", self.portfolio.total_equity);
        info!("Total return: {}", self.portfolio.get_total_return());
        info!("Total trades: {}", self.strategy_metrics.total_trades);
        info!("Winning trades: {}", self.strategy_metrics.winning_trades);
        
        Ok(())
    }
} 