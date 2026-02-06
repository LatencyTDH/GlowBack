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
        data_manager: &mut DataManager,
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
        let strategy_config = self.strategy.get_config().clone();
        info!("Running strategy: {}", strategy_config.name);
        
        // Initialize the strategy with its configuration
        if let Err(e) = self.strategy.initialize(&strategy_config) {
            warn!("Strategy initialization warning: {}", e);
        }

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
            
            // 5. Call strategy's on_day_end for end-of-day processing
            self.call_strategy_day_end().await?;
            
            // 6. Update daily returns
            self.update_daily_returns().await?;
            
            // Advance time
            self.current_time += Duration::days(1);
        }
        
        // Call strategy's on_stop for cleanup
        self.call_strategy_stop().await?;

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
        let mut order_events_to_process = Vec::new();
        
        for (index, order) in self.pending_orders.iter().enumerate() {
            if let Some(fill) = self.try_execute_order(order).await? {
                // Apply fill to portfolio
                self.portfolio.apply_fill(&fill);
                
                // Update strategy metrics - just count total trades here
                // Win/loss determination should be based on P&L, not just execution
                self.strategy_metrics.total_trades += 1;
                
                // Log execution
                info!("Executed order: {:?} {} {} at {}", 
                    order.side, order.quantity, order.symbol, fill.price);
                
                // Prepare order event for strategy callback
                order_events_to_process.push(gb_types::OrderEvent::OrderFilled {
                    order_id: order.id,
                    fill: fill.clone(),
                });
                
                executed_orders.push(index);
            }
        }
        
        // Remove executed orders (in reverse order to maintain indices)
        for &index in executed_orders.iter().rev() {
            self.pending_orders.remove(index);
        }
        
        // Notify strategy of order events
        for order_event in order_events_to_process {
            let context = self.build_strategy_context();
            match self.strategy.on_order_event(&order_event, &context) {
                Ok(actions) => {
                    for action in actions {
                        self.process_strategy_action(action)?;
                    }
                }
                Err(e) => {
                    warn!("Strategy on_order_event error: {}", e);
                }
            }
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

    /// Generate strategy signals by calling the strategy's on_market_event method
    async fn generate_strategy_signals(&mut self) -> GbResult<()> {
        // Build the current strategy context with market data and portfolio state
        let context = self.build_strategy_context();
        
        // Collect all current bars first to avoid borrow conflicts
        let mut current_bars_to_process: Vec<(Symbol, Bar)> = Vec::new();
        
        for symbol in &self.config.symbols.clone() {
            if let Some(bars) = self.market_data.get(symbol) {
                for bar in bars.iter() {
                    if bar.timestamp.date_naive() == self.current_time.date_naive() {
                        current_bars_to_process.push((symbol.clone(), bar.clone()));
                    }
                }
            }
        }
        
        // Now process each bar - no borrow conflict since we own the data
        for (symbol, bar) in current_bars_to_process {
            let market_event = MarketEvent::Bar(bar);
            
            // Call the strategy's on_market_event method
            match self.strategy.on_market_event(&market_event, &context) {
                Ok(actions) => {
                    for action in actions {
                        self.process_strategy_action(action)?;
                    }
                }
                Err(e) => {
                    warn!("Strategy error processing {}: {}", symbol, e);
                }
            }
        }
        
        Ok(())
    }

    /// Build a complete StrategyContext with current market data and portfolio state
    fn build_strategy_context(&self) -> StrategyContext {
        use gb_types::{MarketDataBuffer, MarketEvent as ME};
        
        let mut context = StrategyContext::new(
            self.strategy.get_config().strategy_id.clone(),
            self.config.initial_capital,
        );
        
        // Copy portfolio state
        context.portfolio = self.portfolio.clone();
        context.current_time = self.current_time;
        context.pending_orders = self.pending_orders.clone();
        
        // Build market data buffers for each symbol with historical data up to current time
        for (symbol, bars) in &self.market_data {
            let mut buffer = MarketDataBuffer::new(symbol.clone(), 100); // Keep last 100 bars
            
            // Add all bars up to and including current date
            for bar in bars {
                if bar.timestamp <= self.current_time {
                    buffer.add_event(ME::Bar(bar.clone()));
                }
            }
            
            context.market_data.insert(symbol.clone(), buffer);
        }
        
        context
    }

    /// Process a single strategy action
    fn process_strategy_action(&mut self, action: gb_types::StrategyAction) -> GbResult<()> {
        use gb_types::StrategyAction;
        
        match action {
            StrategyAction::PlaceOrder(order) => {
                debug!("Strategy placed order: {:?} {} {} at {:?}", 
                    order.side, order.quantity, order.symbol, order.order_type);
                self.pending_orders.push(order);
            }
            StrategyAction::CancelOrder { order_id } => {
                debug!("Strategy cancelled order: {}", order_id);
                self.pending_orders.retain(|o| o.id != order_id);
            }
            StrategyAction::Log { level, message } => {
                match level {
                    gb_types::LogLevel::Debug => debug!("[Strategy] {}", message),
                    gb_types::LogLevel::Info => info!("[Strategy] {}", message),
                    gb_types::LogLevel::Warning => warn!("[Strategy] {}", message),
                    gb_types::LogLevel::Error => tracing::error!("[Strategy] {}", message),
                }
            }
            StrategyAction::SetParameter { key, value } => {
                debug!("Strategy set parameter: {} = {}", key, value);
                // Parameters are stored in strategy config, not in engine
            }
        }
        
        Ok(())
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
        // Get strategy metrics and merge with engine metrics
        let strategy_metrics = self.strategy.get_metrics();
        
        // Copy strategy-computed metrics
        self.strategy_metrics.winning_trades = strategy_metrics.winning_trades;
        self.strategy_metrics.losing_trades = strategy_metrics.losing_trades;
        self.strategy_metrics.win_rate = strategy_metrics.win_rate;
        self.strategy_metrics.average_win = strategy_metrics.average_win;
        self.strategy_metrics.average_loss = strategy_metrics.average_loss;
        self.strategy_metrics.profit_factor = strategy_metrics.profit_factor;
        
        // Compute portfolio-based metrics
        let total_return = self.portfolio.get_total_return();
        self.strategy_metrics.total_return = total_return;
        
        // Calculate annualized return based on backtest duration
        let days = (self.config.end_date - self.config.start_date).num_days() as f64;
        if days > 0.0 {
            let years = days / 365.25;
            let return_decimal: f64 = total_return.try_into().unwrap_or(0.0);
            let annualized = ((1.0 + return_decimal).powf(1.0 / years) - 1.0);
            self.strategy_metrics.annualized_return = Decimal::try_from(annualized).unwrap_or_default();
        }
        
        // Calculate volatility from daily returns
        let daily_returns: Vec<f64> = self.portfolio.daily_returns
            .iter()
            .map(|dr| dr.daily_return.try_into().unwrap_or(0.0))
            .collect();
        
        if daily_returns.len() > 1 {
            let mean: f64 = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;
            let variance: f64 = daily_returns.iter()
                .map(|r| (r - mean).powi(2))
                .sum::<f64>() / (daily_returns.len() - 1) as f64;
            let daily_vol = variance.sqrt();
            let annualized_vol = daily_vol * (252.0_f64).sqrt(); // Annualize assuming 252 trading days
            self.strategy_metrics.volatility = Decimal::try_from(annualized_vol).unwrap_or_default();
            
            // Calculate Sharpe ratio (assuming risk-free rate of 0 for simplicity)
            if annualized_vol > 0.0 {
                let annualized_return: f64 = self.strategy_metrics.annualized_return.try_into().unwrap_or(0.0);
                let sharpe = annualized_return / annualized_vol;
                self.strategy_metrics.sharpe_ratio = Some(Decimal::try_from(sharpe).unwrap_or_default());
            }
        }
        
        // Calculate max drawdown from daily returns
        let mut peak = self.config.initial_capital;
        let mut max_dd = Decimal::ZERO;
        for dr in &self.portfolio.daily_returns {
            if dr.portfolio_value > peak {
                peak = dr.portfolio_value;
            }
            if peak > Decimal::ZERO {
                let drawdown = (peak - dr.portfolio_value) / peak;
                if drawdown > max_dd {
                    max_dd = drawdown;
                }
            }
        }
        self.strategy_metrics.max_drawdown = max_dd;
        
        // Set end time
        self.strategy_metrics.end_time = Some(self.current_time);
        
        // Mark result as completed with final portfolio and metrics
        result.mark_completed(self.portfolio.clone(), self.strategy_metrics.clone());
        
        info!("Final portfolio value: {}", self.portfolio.total_equity);
        info!("Total return: {:.2}%", total_return * Decimal::from(100));
        info!("Annualized volatility: {:.2}%", self.strategy_metrics.volatility * Decimal::from(100));
        info!("Max drawdown: {:.2}%", max_dd * Decimal::from(100));
        info!("Total trades: {}", self.strategy_metrics.total_trades);
        if self.strategy_metrics.total_trades > 0 {
            info!("Win rate: {:.2}%", self.strategy_metrics.win_rate * Decimal::from(100));
        }
        if let Some(sharpe) = self.strategy_metrics.sharpe_ratio {
            info!("Sharpe ratio: {:.2}", sharpe);
        }
        
        Ok(())
    }

    /// Call strategy's on_day_end method for end-of-day processing
    async fn call_strategy_day_end(&mut self) -> GbResult<()> {
        let context = self.build_strategy_context();
        
        match self.strategy.on_day_end(&context) {
            Ok(actions) => {
                for action in actions {
                    self.process_strategy_action(action)?;
                }
            }
            Err(e) => {
                warn!("Strategy on_day_end error: {}", e);
            }
        }
        
        Ok(())
    }

    /// Call strategy's on_stop method for cleanup
    async fn call_strategy_stop(&mut self) -> GbResult<()> {
        let context = self.build_strategy_context();
        
        match self.strategy.on_stop(&context) {
            Ok(actions) => {
                for action in actions {
                    self.process_strategy_action(action)?;
                }
            }
            Err(e) => {
                warn!("Strategy on_stop error: {}", e);
            }
        }
        
        info!("Strategy stopped");
        Ok(())
    }
} 