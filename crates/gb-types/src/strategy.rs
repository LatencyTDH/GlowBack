use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::{ToPrimitive, FromPrimitive};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::market::{MarketEvent, Symbol};
use crate::orders::{Order, OrderEvent};
use crate::portfolio::{Portfolio, Position};

/// Strategy context provides access to market data, portfolio, and order management
#[derive(Debug, Clone)]
pub struct StrategyContext {
    pub current_time: DateTime<Utc>,
    pub portfolio: Portfolio,
    pub market_data: HashMap<Symbol, MarketDataBuffer>,
    pub pending_orders: Vec<Order>,
    pub strategy_id: String,
}

impl StrategyContext {
    pub fn new(strategy_id: String, initial_capital: Decimal) -> Self {
        Self {
            current_time: Utc::now(),
            portfolio: Portfolio::new("default".to_string(), initial_capital),
            market_data: HashMap::new(),
            pending_orders: Vec::new(),
            strategy_id,
        }
    }
    
    pub fn get_position(&self, symbol: &Symbol) -> Option<&Position> {
        self.portfolio.get_position(symbol)
    }
    
    pub fn get_current_price(&self, symbol: &Symbol) -> Option<Decimal> {
        self.market_data.get(symbol)?.get_current_price()
    }
    
    pub fn get_market_data(&self, symbol: &Symbol) -> Option<&MarketDataBuffer> {
        self.market_data.get(symbol)
    }
    
    pub fn get_available_cash(&self) -> Decimal {
        self.portfolio.get_available_cash()
    }
    
    pub fn get_portfolio_value(&self) -> Decimal {
        self.portfolio.total_equity
    }
}

/// Buffer for market data with rolling window
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketDataBuffer {
    pub symbol: Symbol,
    pub data: Vec<MarketEvent>,
    pub max_size: usize,
}

impl MarketDataBuffer {
    pub fn new(symbol: Symbol, max_size: usize) -> Self {
        Self {
            symbol,
            data: Vec::new(),
            max_size,
        }
    }
    
    pub fn add_event(&mut self, event: MarketEvent) {
        self.data.push(event);
        if self.data.len() > self.max_size {
            self.data.remove(0);
        }
    }
    
    pub fn get_current_price(&self) -> Option<Decimal> {
        self.data.last().and_then(|event| match event {
            MarketEvent::Bar(bar) => Some(bar.close),
            MarketEvent::Tick(tick) => Some(tick.price),
            MarketEvent::Quote { bid, ask, .. } => Some((*bid + *ask) / Decimal::from(2)),
        })
    }
    
    pub fn get_latest_bar(&self) -> Option<&crate::market::Bar> {
        self.data.iter().rev().find_map(|event| match event {
            MarketEvent::Bar(bar) => Some(bar),
            _ => None,
        })
    }
    
    pub fn get_bars(&self, count: usize) -> Vec<&crate::market::Bar> {
        self.data.iter()
            .filter_map(|event| match event {
                MarketEvent::Bar(bar) => Some(bar),
                _ => None,
            })
            .rev()
            .take(count)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }
}

/// Strategy action that can be taken
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StrategyAction {
    PlaceOrder(Order),
    CancelOrder { order_id: crate::orders::OrderId },
    Log { level: LogLevel, message: String },
    SetParameter { key: String, value: serde_json::Value },
}

/// Log levels for strategy output
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

/// Strategy performance metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyMetrics {
    pub strategy_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub total_return: Decimal,
    pub annualized_return: Decimal,
    pub volatility: Decimal,
    pub sharpe_ratio: Option<Decimal>,
    pub max_drawdown: Decimal,
    pub total_trades: u64,
    pub winning_trades: u64,
    pub losing_trades: u64,
    pub win_rate: Decimal,
    pub average_win: Decimal,
    pub average_loss: Decimal,
    pub profit_factor: Decimal,
    pub total_commissions: Decimal,
}

impl StrategyMetrics {
    pub fn new(strategy_id: String) -> Self {
        Self {
            strategy_id,
            start_time: Utc::now(),
            end_time: None,
            total_return: Decimal::ZERO,
            annualized_return: Decimal::ZERO,
            volatility: Decimal::ZERO,
            sharpe_ratio: None,
            max_drawdown: Decimal::ZERO,
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            win_rate: Decimal::ZERO,
            average_win: Decimal::ZERO,
            average_loss: Decimal::ZERO,
            profit_factor: Decimal::ZERO,
            total_commissions: Decimal::ZERO,
        }
    }
}

/// Strategy configuration parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyConfig {
    pub strategy_id: String,
    pub name: String,
    pub description: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub symbols: Vec<Symbol>,
    pub initial_capital: Decimal,
    pub risk_limits: crate::portfolio::RiskLimits,
    pub enabled: bool,
}

impl StrategyConfig {
    pub fn new(strategy_id: String, name: String) -> Self {
        Self {
            strategy_id,
            name,
            description: String::new(),
            parameters: HashMap::new(),
            symbols: Vec::new(),
            initial_capital: Decimal::from(100000),
            risk_limits: Default::default(),
            enabled: true,
        }
    }
    
    pub fn add_symbol(&mut self, symbol: Symbol) -> &mut Self {
        self.symbols.push(symbol);
        self
    }
    
    pub fn set_parameter<T: Serialize>(&mut self, key: &str, value: T) -> &mut Self {
        self.parameters.insert(
            key.to_string(),
            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
        );
        self
    }
    
    pub fn get_parameter<T>(&self, key: &str) -> Option<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let value = self.parameters.get(key)?;
        serde_json::from_value(value.clone()).ok()
    }
}

/// Main strategy trait that all strategies must implement
pub trait Strategy: Send + Sync {
    /// Initialize the strategy with configuration
    fn initialize(&mut self, config: &StrategyConfig) -> Result<(), String>;
    
    /// Process market data event and return actions
    fn on_market_event(
        &mut self,
        event: &MarketEvent,
        context: &StrategyContext,
    ) -> Result<Vec<StrategyAction>, String>;
    
    /// Process order event (fills, cancellations, etc.)
    fn on_order_event(
        &mut self,
        event: &OrderEvent,
        context: &StrategyContext,
    ) -> Result<Vec<StrategyAction>, String>;
    
    /// Called at the end of each trading day
    fn on_day_end(
        &mut self,
        context: &StrategyContext,
    ) -> Result<Vec<StrategyAction>, String>;
    
    /// Called when strategy is stopped
    fn on_stop(&mut self, context: &StrategyContext) -> Result<Vec<StrategyAction>, String>;
    
    /// Get strategy configuration
    fn get_config(&self) -> &StrategyConfig;
    
    /// Get strategy metrics
    fn get_metrics(&self) -> StrategyMetrics;
}

/// Event emitted by strategies
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StrategyEvent {
    Initialized { strategy_id: String, config: StrategyConfig },
    ActionTaken { strategy_id: String, action: StrategyAction },
    Error { strategy_id: String, error: String },
    Stopped { strategy_id: String, metrics: StrategyMetrics },
}

/// Simple buy and hold strategy for testing
#[derive(Debug, Clone)]
pub struct BuyAndHoldStrategy {
    config: StrategyConfig,
    initialized: bool,
    position_opened: bool,
}

impl BuyAndHoldStrategy {
    pub fn new() -> Self {
        Self {
            config: StrategyConfig::new("buy_and_hold".to_string(), "Buy and Hold".to_string()),
            initialized: false,
            position_opened: false,
        }
    }
}

impl Strategy for BuyAndHoldStrategy {
    fn initialize(&mut self, config: &StrategyConfig) -> Result<(), String> {
        self.config = config.clone();
        self.initialized = true;
        Ok(())
    }
    
    fn on_market_event(
        &mut self,
        event: &MarketEvent,
        context: &StrategyContext,
    ) -> Result<Vec<StrategyAction>, String> {
        if !self.initialized || self.position_opened {
            return Ok(vec![]);
        }
        
        // Buy on first market event
        if let Some(symbol) = self.config.symbols.first() {
            if event.symbol() == symbol {
                let available_cash = context.get_available_cash();
                if let Some(price) = context.get_current_price(symbol) {
                    let quantity = available_cash * Decimal::new(95, 2) / price; // Use 95% of cash
                    
                    let order = Order::market_order(
                        symbol.clone(),
                        crate::orders::Side::Buy,
                        quantity,
                        self.config.strategy_id.clone()
                    );
                    
                    self.position_opened = true;
                    return Ok(vec![StrategyAction::PlaceOrder(order)]);
                }
            }
        }
        
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
    
    fn on_stop(&mut self, _context: &StrategyContext) -> Result<Vec<StrategyAction>, String> {
        Ok(vec![])
    }
    
    fn get_config(&self) -> &StrategyConfig {
        &self.config
    }
    
    fn get_metrics(&self) -> StrategyMetrics {
        StrategyMetrics::new(self.config.strategy_id.clone())
    }
}

/// Moving Average Crossover Strategy
/// Buys when short MA crosses above long MA, sells when short MA crosses below long MA
#[derive(Debug, Clone)]
pub struct MovingAverageCrossoverStrategy {
    config: StrategyConfig,
    initialized: bool,
    short_period: usize,
    long_period: usize,
    position_size: Decimal,
    last_signal: Option<Signal>,
}

#[derive(Debug, Clone, PartialEq)]
enum Signal {
    Buy,
    Sell,
}

impl MovingAverageCrossoverStrategy {
    pub fn new(short_period: usize, long_period: usize) -> Self {
        let mut config = StrategyConfig::new(
            "ma_crossover".to_string(), 
            "Moving Average Crossover".to_string()
        );
        config.set_parameter("short_period", short_period);
        config.set_parameter("long_period", long_period);
        config.set_parameter("position_size", 0.95f64); // Use 95% of available capital
        
        Self {
            config,
            initialized: false,
            short_period,
            long_period,
            position_size: Decimal::new(95, 2),
            last_signal: None,
        }
    }
    
    fn calculate_sma(&self, prices: &[Decimal], period: usize) -> Option<Decimal> {
        if prices.len() < period {
            return None;
        }
        
        let sum: Decimal = prices.iter().rev().take(period).sum();
        Some(sum / Decimal::from(period))
    }
    
    fn get_recent_prices(&self, context: &StrategyContext, symbol: &Symbol) -> Vec<Decimal> {
        if let Some(buffer) = context.get_market_data(symbol) {
            buffer.get_bars(self.long_period + 1)
                .iter()
                .map(|bar| bar.close)
                .collect()
        } else {
            Vec::new()
        }
    }
}

impl Strategy for MovingAverageCrossoverStrategy {
    fn initialize(&mut self, config: &StrategyConfig) -> Result<(), String> {
        self.config = config.clone();
        self.short_period = self.config.get_parameter("short_period").unwrap_or(10);
        self.long_period = self.config.get_parameter("long_period").unwrap_or(20);
        self.position_size = self.config.get_parameter::<f64>("position_size").map(Decimal::from_f64_retain).flatten().unwrap_or(Decimal::new(95, 2));
        self.initialized = true;
        Ok(())
    }
    
    fn on_market_event(
        &mut self,
        event: &MarketEvent,
        context: &StrategyContext,
    ) -> Result<Vec<StrategyAction>, String> {
        if !self.initialized {
            return Ok(vec![]);
        }
        
        let symbol = event.symbol();
        let prices = self.get_recent_prices(context, symbol);
        
        if prices.len() < self.long_period {
            return Ok(vec![]);
        }
        
        let short_ma = self.calculate_sma(&prices, self.short_period);
        let long_ma = self.calculate_sma(&prices, self.long_period);
        
        if let (Some(short), Some(long)) = (short_ma, long_ma) {
            let current_signal = if short > long {
                Some(Signal::Buy)
            } else if short < long {
                Some(Signal::Sell)
            } else {
                None
            };
            
            // Check for signal change
            if current_signal != self.last_signal {
                let mut actions = Vec::new();
                
                match current_signal {
                    Some(Signal::Buy) => {
                        // Close short position if any, then go long
                        if let Some(position) = context.get_position(symbol) {
                            if position.quantity < Decimal::ZERO {
                                let close_order = Order::market_order(
                                    symbol.clone(),
                                    crate::orders::Side::Buy,
                                    position.quantity.abs(),
                                    self.config.strategy_id.clone()
                                );
                                actions.push(StrategyAction::PlaceOrder(close_order));
                            }
                        }
                        
                        // Open long position
                        let available_cash = context.get_available_cash();
                        if let Some(price) = context.get_current_price(symbol) {
                            let quantity = (available_cash * self.position_size) / price;
                            if quantity > Decimal::ZERO {
                                let order = Order::market_order(
                                    symbol.clone(),
                                    crate::orders::Side::Buy,
                                    quantity,
                                    self.config.strategy_id.clone()
                                );
                                actions.push(StrategyAction::PlaceOrder(order));
                            }
                        }
                    },
                    Some(Signal::Sell) => {
                        // Close long position if any, then go short
                        if let Some(position) = context.get_position(symbol) {
                            if position.quantity > Decimal::ZERO {
                                let close_order = Order::market_order(
                                    symbol.clone(),
                                    crate::orders::Side::Sell,
                                    position.quantity,
                                    self.config.strategy_id.clone()
                                );
                                actions.push(StrategyAction::PlaceOrder(close_order));
                            }
                        }
                        
                        // Open short position (if shorting is enabled)
                        let portfolio_value = context.get_portfolio_value();
                        if let Some(price) = context.get_current_price(symbol) {
                            let quantity = (portfolio_value * self.position_size) / price;
                            if quantity > Decimal::ZERO {
                                let order = Order::market_order(
                                    symbol.clone(),
                                    crate::orders::Side::Sell,
                                    quantity,
                                    self.config.strategy_id.clone()
                                );
                                actions.push(StrategyAction::PlaceOrder(order));
                            }
                        }
                    },
                    None => {
                        // No clear signal, could close positions
                    }
                }
                
                self.last_signal = current_signal;
                return Ok(actions);
            }
        }
        
        Ok(vec![])
    }
    
    fn on_order_event(&mut self, _event: &OrderEvent, _context: &StrategyContext) -> Result<Vec<StrategyAction>, String> {
        Ok(vec![])
    }
    
    fn on_day_end(&mut self, _context: &StrategyContext) -> Result<Vec<StrategyAction>, String> {
        Ok(vec![])
    }
    
    fn on_stop(&mut self, _context: &StrategyContext) -> Result<Vec<StrategyAction>, String> {
        Ok(vec![])
    }
    
    fn get_config(&self) -> &StrategyConfig {
        &self.config
    }
    
    fn get_metrics(&self) -> StrategyMetrics {
        StrategyMetrics::new(self.config.strategy_id.clone())
    }
}

/// Momentum Strategy
/// Buys when price momentum is positive, sells when momentum turns negative
#[derive(Debug, Clone)]
pub struct MomentumStrategy {
    config: StrategyConfig,
    initialized: bool,
    lookback_period: usize,
    momentum_threshold: Decimal,
    position_size: Decimal,
    rebalance_frequency: usize,
    days_since_rebalance: usize,
}

impl MomentumStrategy {
    pub fn new(lookback_period: usize, momentum_threshold: f64) -> Self {
        let mut config = StrategyConfig::new(
            "momentum".to_string(), 
            "Momentum Strategy".to_string()
        );
        config.set_parameter("lookback_period", lookback_period);
        config.set_parameter("momentum_threshold", momentum_threshold);
        config.set_parameter("position_size", 0.95f64);
        config.set_parameter("rebalance_frequency", 5); // Rebalance every 5 days
        
        Self {
            config,
            initialized: false,
            lookback_period,
            momentum_threshold: Decimal::from_f64_retain(momentum_threshold).unwrap_or(Decimal::new(5, 2)),
            position_size: Decimal::new(95, 2),
            rebalance_frequency: 5,
            days_since_rebalance: 0,
        }
    }
    
    fn calculate_momentum(&self, prices: &[Decimal]) -> Option<Decimal> {
        if prices.len() < self.lookback_period {
            return None;
        }
        
        let current_price = prices.last()?;
        let past_price = prices.get(prices.len() - self.lookback_period)?;
        
        // Calculate percentage change
        if *past_price != Decimal::ZERO {
            Some((*current_price - *past_price) / *past_price * Decimal::from(100))
        } else {
            None
        }
    }
}

impl Strategy for MomentumStrategy {
    fn initialize(&mut self, config: &StrategyConfig) -> Result<(), String> {
        self.config = config.clone();
        self.lookback_period = self.config.get_parameter("lookback_period").unwrap_or(10);
        self.momentum_threshold = self.config.get_parameter::<f64>("momentum_threshold").map(Decimal::from_f64_retain).flatten().unwrap_or(Decimal::new(5, 2));
        self.position_size = self.config.get_parameter::<f64>("position_size").map(Decimal::from_f64_retain).flatten().unwrap_or(Decimal::new(95, 2));
        self.rebalance_frequency = self.config.get_parameter("rebalance_frequency").unwrap_or(5);
        self.initialized = true;
        Ok(())
    }
    
    fn on_market_event(
        &mut self,
        event: &MarketEvent,
        context: &StrategyContext,
    ) -> Result<Vec<StrategyAction>, String> {
        if !self.initialized {
            return Ok(vec![]);
        }
        
        let symbol = event.symbol();
        
        // Only rebalance on specified frequency
        if self.days_since_rebalance < self.rebalance_frequency {
            return Ok(vec![]);
        }
        
        if let Some(buffer) = context.get_market_data(symbol) {
            let bars = buffer.get_bars(self.lookback_period + 1);
            let prices: Vec<Decimal> = bars.iter().map(|bar| bar.close).collect();
            
            if let Some(momentum) = self.calculate_momentum(&prices) {
                let mut actions = Vec::new();
                let current_position = context.get_position(symbol);
                
                if momentum > self.momentum_threshold {
                    // Strong positive momentum - go long
                    let target_quantity = if let Some(price) = context.get_current_price(symbol) {
                        (context.get_portfolio_value() * self.position_size) / price
                    } else {
                        Decimal::ZERO
                    };
                    
                    let current_quantity = current_position.map(|p| p.quantity).unwrap_or(Decimal::ZERO);
                    let quantity_diff = target_quantity - current_quantity;
                    
                    if quantity_diff.abs() > Decimal::new(1, 4) { // Minimum trade size
                        let side = if quantity_diff > Decimal::ZERO {
                            crate::orders::Side::Buy
                        } else {
                            crate::orders::Side::Sell
                        };
                        
                        let order = Order::market_order(
                            symbol.clone(),
                            side,
                            quantity_diff.abs(),
                            self.config.strategy_id.clone()
                        );
                        actions.push(StrategyAction::PlaceOrder(order));
                    }
                } else if momentum < -self.momentum_threshold {
                    // Strong negative momentum - close positions or go short
                    if let Some(position) = current_position {
                        if position.quantity > Decimal::ZERO {
                            let order = Order::market_order(
                                symbol.clone(),
                                crate::orders::Side::Sell,
                                position.quantity,
                                self.config.strategy_id.clone()
                            );
                            actions.push(StrategyAction::PlaceOrder(order));
                        }
                    }
                }
                
                self.days_since_rebalance = 0;
                return Ok(actions);
            }
        }
        
        Ok(vec![])
    }
    
    fn on_order_event(&mut self, _event: &OrderEvent, _context: &StrategyContext) -> Result<Vec<StrategyAction>, String> {
        Ok(vec![])
    }
    
    fn on_day_end(&mut self, _context: &StrategyContext) -> Result<Vec<StrategyAction>, String> {
        self.days_since_rebalance += 1;
        Ok(vec![])
    }
    
    fn on_stop(&mut self, _context: &StrategyContext) -> Result<Vec<StrategyAction>, String> {
        Ok(vec![])
    }
    
    fn get_config(&self) -> &StrategyConfig {
        &self.config
    }
    
    fn get_metrics(&self) -> StrategyMetrics {
        StrategyMetrics::new(self.config.strategy_id.clone())
    }
}

/// Mean Reversion Strategy
/// Buys when price is significantly below average, sells when price is significantly above average
#[derive(Debug, Clone)]
pub struct MeanReversionStrategy {
    config: StrategyConfig,
    initialized: bool,
    lookback_period: usize,
    entry_threshold: Decimal, // Standard deviations
    exit_threshold: Decimal,
    position_size: Decimal,
    max_position_size: Decimal,
}

/// RSI Strategy
/// Buys when RSI is oversold, sells when RSI is overbought
#[derive(Debug, Clone)]
pub struct RsiStrategy {
    config: StrategyConfig,
    initialized: bool,
    lookback_period: usize,
    oversold_threshold: Decimal,
    overbought_threshold: Decimal,
    position_size: Decimal,
}

impl MeanReversionStrategy {
    pub fn new(lookback_period: usize, entry_threshold: f64, exit_threshold: f64) -> Self {
        let mut config = StrategyConfig::new(
            "mean_reversion".to_string(), 
            "Mean Reversion Strategy".to_string()
        );
        config.set_parameter("lookback_period", lookback_period);
        config.set_parameter("entry_threshold", entry_threshold);
        config.set_parameter("exit_threshold", exit_threshold);
        config.set_parameter("position_size", 0.25f64); // Smaller positions for mean reversion
        config.set_parameter("max_position_size", 0.95f64);
        
        Self {
            config,
            initialized: false,
            lookback_period,
            entry_threshold: Decimal::from_f64_retain(entry_threshold).unwrap_or(Decimal::from(2)),
            exit_threshold: Decimal::from_f64_retain(exit_threshold).unwrap_or(Decimal::from(1)),
            position_size: Decimal::new(25, 2),
            max_position_size: Decimal::new(95, 2),
        }
    }
    
    fn calculate_z_score(&self, prices: &[Decimal]) -> Option<Decimal> {
        if prices.len() < self.lookback_period {
            return None;
        }
        
        let recent_prices: Vec<Decimal> = prices.iter().rev().take(self.lookback_period).cloned().collect();
        let current_price = *prices.last()?;
        
        // Calculate mean
        let mean: Decimal = recent_prices.iter().sum::<Decimal>() / Decimal::from(recent_prices.len());
        
        // Calculate standard deviation
        let variance: Decimal = recent_prices.iter()
            .map(|price| (*price - mean) * (*price - mean))
            .sum::<Decimal>() / Decimal::from(recent_prices.len());
        
        let std_dev = variance.to_f64()
            .map(|v| v.sqrt())
            .and_then(Decimal::from_f64)
            .unwrap_or(Decimal::ZERO);
        
        if std_dev != Decimal::ZERO {
            Some((current_price - mean) / std_dev)
        } else {
            None
        }
    }
}

impl Strategy for MeanReversionStrategy {
    fn initialize(&mut self, config: &StrategyConfig) -> Result<(), String> {
        self.config = config.clone();
        self.lookback_period = self.config.get_parameter("lookback_period").unwrap_or(20);
        self.entry_threshold = self.config.get_parameter::<f64>("entry_threshold").map(Decimal::from_f64_retain).flatten().unwrap_or(Decimal::from(2));
        self.exit_threshold = self.config.get_parameter::<f64>("exit_threshold").map(Decimal::from_f64_retain).flatten().unwrap_or(Decimal::from(1));
        self.position_size = self.config.get_parameter::<f64>("position_size").map(Decimal::from_f64_retain).flatten().unwrap_or(Decimal::new(25, 2));
        self.max_position_size = self.config.get_parameter::<f64>("max_position_size").map(Decimal::from_f64_retain).flatten().unwrap_or(Decimal::new(95, 2));
        self.initialized = true;
        Ok(())
    }
    
    fn on_market_event(
        &mut self,
        event: &MarketEvent,
        context: &StrategyContext,
    ) -> Result<Vec<StrategyAction>, String> {
        if !self.initialized {
            return Ok(vec![]);
        }
        
        let symbol = event.symbol();
        
        if let Some(buffer) = context.get_market_data(symbol) {
            let bars = buffer.get_bars(self.lookback_period + 5); // Extra buffer for calculation
            let prices: Vec<Decimal> = bars.iter().map(|bar| bar.close).collect();
            
            if let Some(z_score) = self.calculate_z_score(&prices) {
                let mut actions = Vec::new();
                let current_position = context.get_position(symbol);
                let current_quantity = current_position.map(|p| p.quantity).unwrap_or(Decimal::ZERO);
                
                // Entry signals
                if z_score < -self.entry_threshold {
                    // Price is significantly below mean - buy opportunity
                    let available_cash = context.get_available_cash();
                    if let Some(price) = context.get_current_price(symbol) {
                        let max_quantity = (context.get_portfolio_value() * self.max_position_size) / price;
                        let position_increment = (context.get_portfolio_value() * self.position_size) / price;
                        
                        if current_quantity < max_quantity {
                            let quantity = (max_quantity - current_quantity).min(position_increment);
                            if quantity > Decimal::new(1, 4) && available_cash > quantity * price {
                                let order = Order::market_order(
                                    symbol.clone(),
                                    crate::orders::Side::Buy,
                                    quantity,
                                    self.config.strategy_id.clone()
                                );
                                actions.push(StrategyAction::PlaceOrder(order));
                            }
                        }
                    }
                } else if z_score > self.entry_threshold {
                    // Price is significantly above mean - sell opportunity
                    let portfolio_value = context.get_portfolio_value();
                    if let Some(price) = context.get_current_price(symbol) {
                        let max_short_quantity = -(portfolio_value * self.max_position_size) / price;
                        let position_decrement = (portfolio_value * self.position_size) / price;
                        
                        if current_quantity > max_short_quantity {
                            let quantity = (current_quantity - max_short_quantity).min(position_decrement);
                            if quantity > Decimal::new(1, 4) {
                                let order = Order::market_order(
                                    symbol.clone(),
                                    crate::orders::Side::Sell,
                                    quantity,
                                    self.config.strategy_id.clone()
                                );
                                actions.push(StrategyAction::PlaceOrder(order));
                            }
                        }
                    }
                }
                
                // Exit signals - take profits when price moves back toward mean
                if current_quantity > Decimal::ZERO && z_score > -self.exit_threshold {
                    // Long position, price moving back up toward mean
                    let exit_quantity = current_quantity.min(
                        (context.get_portfolio_value() * self.position_size) / 
                        context.get_current_price(symbol).unwrap_or(Decimal::ONE)
                    );
                    
                    if exit_quantity > Decimal::new(1, 4) {
                        let order = Order::market_order(
                            symbol.clone(),
                            crate::orders::Side::Sell,
                            exit_quantity,
                            self.config.strategy_id.clone()
                        );
                        actions.push(StrategyAction::PlaceOrder(order));
                    }
                } else if current_quantity < Decimal::ZERO && z_score < self.exit_threshold {
                    // Short position, price moving back down toward mean
                    let exit_quantity = current_quantity.abs().min(
                        (context.get_portfolio_value() * self.position_size) / 
                        context.get_current_price(symbol).unwrap_or(Decimal::ONE)
                    );
                    
                    if exit_quantity > Decimal::new(1, 4) {
                        let order = Order::market_order(
                            symbol.clone(),
                            crate::orders::Side::Buy,
                            exit_quantity,
                            self.config.strategy_id.clone()
                        );
                        actions.push(StrategyAction::PlaceOrder(order));
                    }
                }
                
                return Ok(actions);
            }
        }
        
        Ok(vec![])
    }
    
    fn on_order_event(&mut self, _event: &OrderEvent, _context: &StrategyContext) -> Result<Vec<StrategyAction>, String> {
        Ok(vec![])
    }
    
    fn on_day_end(&mut self, _context: &StrategyContext) -> Result<Vec<StrategyAction>, String> {
        Ok(vec![])
    }
    
    fn on_stop(&mut self, _context: &StrategyContext) -> Result<Vec<StrategyAction>, String> {
        Ok(vec![])
    }
    
    fn get_config(&self) -> &StrategyConfig {
        &self.config
    }
    
    fn get_metrics(&self) -> StrategyMetrics {
        StrategyMetrics::new(self.config.strategy_id.clone())
    }
} 

impl RsiStrategy {
    pub fn new(lookback_period: usize, oversold_threshold: f64, overbought_threshold: f64) -> Self {
        let mut config = StrategyConfig::new(
            "rsi".to_string(),
            "RSI Strategy".to_string()
        );
        config.set_parameter("lookback_period", lookback_period);
        config.set_parameter("oversold_threshold", oversold_threshold);
        config.set_parameter("overbought_threshold", overbought_threshold);
        config.set_parameter("position_size", 0.95f64);

        Self {
            config,
            initialized: false,
            lookback_period,
            oversold_threshold: Decimal::from_f64_retain(oversold_threshold).unwrap_or(Decimal::from(30)),
            overbought_threshold: Decimal::from_f64_retain(overbought_threshold).unwrap_or(Decimal::from(70)),
            position_size: Decimal::new(95, 2),
        }
    }

    fn calculate_rsi(&self, prices: &[Decimal]) -> Option<Decimal> {
        if prices.len() < self.lookback_period + 1 {
            return None;
        }

        let mut gains = Decimal::ZERO;
        let mut losses = Decimal::ZERO;

        let recent_prices = prices.iter().rev().take(self.lookback_period + 1).cloned().collect::<Vec<_>>();
        let mut recent_prices = recent_prices.into_iter().rev();
        let mut previous = recent_prices.next()?;

        for price in recent_prices {
            let change = price - previous;
            if change > Decimal::ZERO {
                gains += change;
            } else if change < Decimal::ZERO {
                losses += change.abs();
            }
            previous = price;
        }

        let period = Decimal::from(self.lookback_period);
        let avg_gain = gains / period;
        let avg_loss = losses / period;

        if avg_loss == Decimal::ZERO {
            return Some(Decimal::from(100));
        }
        if avg_gain == Decimal::ZERO {
            return Some(Decimal::ZERO);
        }

        let rs = avg_gain / avg_loss;
        let rsi = Decimal::from(100) - (Decimal::from(100) / (Decimal::ONE + rs));
        Some(rsi)
    }
}

impl Strategy for RsiStrategy {
    fn initialize(&mut self, config: &StrategyConfig) -> Result<(), String> {
        self.config = config.clone();
        self.lookback_period = self.config.get_parameter("lookback_period").unwrap_or(14);
        self.oversold_threshold = self.config.get_parameter::<f64>("oversold_threshold").map(Decimal::from_f64_retain).flatten().unwrap_or(Decimal::from(30));
        self.overbought_threshold = self.config.get_parameter::<f64>("overbought_threshold").map(Decimal::from_f64_retain).flatten().unwrap_or(Decimal::from(70));
        self.position_size = self.config.get_parameter::<f64>("position_size").map(Decimal::from_f64_retain).flatten().unwrap_or(Decimal::new(95, 2));
        self.initialized = true;
        Ok(())
    }

    fn on_market_event(
        &mut self,
        event: &MarketEvent,
        context: &StrategyContext,
    ) -> Result<Vec<StrategyAction>, String> {
        if !self.initialized {
            return Ok(vec![]);
        }

        let symbol = event.symbol();
        if let Some(buffer) = context.get_market_data(symbol) {
            let bars = buffer.get_bars(self.lookback_period + 1);
            let prices: Vec<Decimal> = bars.iter().map(|bar| bar.close).collect();

            if let Some(rsi) = self.calculate_rsi(&prices) {
                let mut actions = Vec::new();
                let current_position = context.get_position(symbol);
                let current_quantity = current_position.map(|p| p.quantity).unwrap_or(Decimal::ZERO);

                if rsi < self.oversold_threshold {
                    let target_quantity = if let Some(price) = context.get_current_price(symbol) {
                        (context.get_portfolio_value() * self.position_size) / price
                    } else {
                        Decimal::ZERO
                    };

                    let quantity_diff = target_quantity - current_quantity;
                    if quantity_diff > Decimal::new(1, 4) {
                        let order = Order::market_order(
                            symbol.clone(),
                            crate::orders::Side::Buy,
                            quantity_diff,
                            self.config.strategy_id.clone()
                        );
                        actions.push(StrategyAction::PlaceOrder(order));
                    }
                } else if rsi > self.overbought_threshold {
                    if current_quantity > Decimal::ZERO {
                        let order = Order::market_order(
                            symbol.clone(),
                            crate::orders::Side::Sell,
                            current_quantity,
                            self.config.strategy_id.clone()
                        );
                        actions.push(StrategyAction::PlaceOrder(order));
                    }
                }

                return Ok(actions);
            }
        }

        Ok(vec![])
    }

    fn on_order_event(&mut self, _event: &OrderEvent, _context: &StrategyContext) -> Result<Vec<StrategyAction>, String> {
        Ok(vec![])
    }

    fn on_day_end(&mut self, _context: &StrategyContext) -> Result<Vec<StrategyAction>, String> {
        Ok(vec![])
    }

    fn on_stop(&mut self, _context: &StrategyContext) -> Result<Vec<StrategyAction>, String> {
        Ok(vec![])
    }

    fn get_config(&self) -> &StrategyConfig {
        &self.config
    }

    fn get_metrics(&self) -> StrategyMetrics {
        StrategyMetrics::new(self.config.strategy_id.clone())
    }
}

#[cfg(test)]
mod strategy_tests {
    use super::*;
    use crate::market::{Bar, AssetClass, Resolution};
    use rust_decimal_macros::dec;

    fn create_test_symbol() -> Symbol {
        Symbol::new("AAPL", "NASDAQ", AssetClass::Equity)
    }

    fn create_test_bar(close_price: Decimal, timestamp: DateTime<Utc>) -> Bar {
        Bar {
            symbol: create_test_symbol(),
            timestamp,
            open: close_price,
            high: close_price * dec!(1.01),
            low: close_price * dec!(0.99),
            close: close_price,
            volume: dec!(1000),
            resolution: Resolution::Day,
        }
    }

    fn create_test_context_with_data(bars: Vec<Bar>) -> StrategyContext {
        let mut context = StrategyContext::new("test_strategy".to_string(), dec!(100000));
        
        let symbol = create_test_symbol();
        let mut buffer = MarketDataBuffer::new(symbol.clone(), 50);
        
        for bar in bars {
            buffer.add_event(MarketEvent::Bar(bar));
        }
        
        context.market_data.insert(symbol, buffer);
        context
    }

    #[test]
    fn test_buy_and_hold_strategy() {
        let mut strategy = BuyAndHoldStrategy::new();
        let config = StrategyConfig::new("test_bah".to_string(), "Test Buy and Hold".to_string());
        
        // Initialize strategy
        assert!(strategy.initialize(&config).is_ok());
        assert_eq!(strategy.get_config().strategy_id, "test_bah");
        
        // Test metrics
        let metrics = strategy.get_metrics();
        assert_eq!(metrics.strategy_id, "test_bah");
    }

    #[test]
    fn test_moving_average_crossover_strategy() {
        let mut strategy = MovingAverageCrossoverStrategy::new(5, 10);
        let mut config = StrategyConfig::new("test_ma".to_string(), "Test MA Crossover".to_string());
        config.add_symbol(create_test_symbol());
        
        // Initialize strategy
        assert!(strategy.initialize(&config).is_ok());
        assert_eq!(strategy.get_config().strategy_id, "test_ma");
        
        // Create test data with upward trend
        let mut bars = Vec::new();
        let base_time = Utc::now();
        for i in 0..15 {
            let price = dec!(100) + Decimal::from(i); // Increasing prices
            let bar = create_test_bar(price, base_time + chrono::Duration::days(i));
            bars.push(bar);
        }
        
        let context = create_test_context_with_data(bars);
        
        // Test market event processing
        let last_bar = create_test_bar(dec!(115), base_time + chrono::Duration::days(15));
        let event = MarketEvent::Bar(last_bar);
        
        let actions = strategy.on_market_event(&event, &context);
        assert!(actions.is_ok());
        // Should generate buy signal as short MA crosses above long MA
    }

    #[test]
    fn test_momentum_strategy() {
        let mut strategy = MomentumStrategy::new(5, 0.05); // 5-day lookback, 5% threshold
        let mut config = StrategyConfig::new("test_momentum".to_string(), "Test Momentum".to_string());
        config.add_symbol(create_test_symbol());
        
        // Initialize strategy
        assert!(strategy.initialize(&config).is_ok());
        assert_eq!(strategy.get_config().strategy_id, "test_momentum");
        
        // Create test data with strong upward momentum
        let mut bars = Vec::new();
        let base_time = Utc::now();
        for i in 0..10 {
            let price = dec!(100) * (dec!(1) + Decimal::from(i) / dec!(50)); // 2% increase per day
            let bar = create_test_bar(price, base_time + chrono::Duration::days(i));
            bars.push(bar);
        }
        
        let context = create_test_context_with_data(bars);
        
        // Test market event processing after rebalance period
        strategy.days_since_rebalance = 5; // Set to rebalance frequency
        
        let last_bar = create_test_bar(dec!(120), base_time + chrono::Duration::days(10));
        let event = MarketEvent::Bar(last_bar);
        
        let actions = strategy.on_market_event(&event, &context);
        assert!(actions.is_ok());
        
        // Test day end increment
        let day_end_actions = strategy.on_day_end(&context);
        assert!(day_end_actions.is_ok());
        assert_eq!(strategy.days_since_rebalance, 1); // Should reset after rebalancing
    }

    #[test]
    fn test_mean_reversion_strategy() {
        let mut strategy = MeanReversionStrategy::new(10, 2.0, 1.0); // 10-day lookback, 2.0 entry, 1.0 exit
        let mut config = StrategyConfig::new("test_mean_rev".to_string(), "Test Mean Reversion".to_string());
        config.add_symbol(create_test_symbol());
        
        // Initialize strategy
        assert!(strategy.initialize(&config).is_ok());
        assert_eq!(strategy.get_config().strategy_id, "test_mean_rev");
        
        // Create test data with mean-reverting pattern
        let mut bars = Vec::new();
        let base_time = Utc::now();
        let base_price = dec!(100);
        
        // First create stable prices around 100
        for i in 0..10 {
            let price = base_price + Decimal::from(i % 3 - 1); // 99, 100, 101 pattern
            let bar = create_test_bar(price, base_time + chrono::Duration::days(i));
            bars.push(bar);
        }
        
        let context = create_test_context_with_data(bars);
        
        // Test with price significantly below mean (should trigger buy)
        let low_bar = create_test_bar(dec!(90), base_time + chrono::Duration::days(11)); // -10% from mean
        let event = MarketEvent::Bar(low_bar);
        
        let actions = strategy.on_market_event(&event, &context);
        assert!(actions.is_ok());
        // Should generate buy signal as price is far below mean
    }

    #[test]
    fn test_rsi_calculation() {
        let strategy = RsiStrategy::new(5, 30.0, 70.0);
        let prices = vec![dec!(100), dec!(102), dec!(104), dec!(103), dec!(105), dec!(107)];

        let rsi = strategy.calculate_rsi(&prices);
        assert!(rsi.is_some());
        let value = rsi.unwrap();
        assert!(value > dec!(50)); // Mostly rising prices should produce RSI above 50
    }

    #[test]
    fn test_rsi_strategy_buy_signal() {
        let mut strategy = RsiStrategy::new(5, 30.0, 70.0);
        let mut config = StrategyConfig::new("test_rsi".to_string(), "Test RSI".to_string());
        config.add_symbol(create_test_symbol());

        assert!(strategy.initialize(&config).is_ok());

        let mut bars = Vec::new();
        let base_time = Utc::now();
        let prices = vec![dec!(100), dec!(98), dec!(96), dec!(95), dec!(94), dec!(93)];
        for (i, price) in prices.into_iter().enumerate() {
            let bar = create_test_bar(price, base_time + chrono::Duration::days(i as i64));
            bars.push(bar);
        }

        let context = create_test_context_with_data(bars);
        let event = MarketEvent::Bar(create_test_bar(dec!(92), base_time + chrono::Duration::days(7)));
        let actions = strategy.on_market_event(&event, &context);
        assert!(actions.is_ok());
    }

    #[test]
    fn test_momentum_calculation() {
        let strategy = MomentumStrategy::new(3, 0.05);
        let prices = vec![dec!(100), dec!(102), dec!(101), dec!(105)]; // +5% over 3 periods from start to end
        
        let momentum = strategy.calculate_momentum(&prices);
        assert!(momentum.is_some());
        
        let result = momentum.unwrap();
        // (105 - 102) / 102 * 100 = 2.94% (comparing current to 3 periods ago)
        let expected = (dec!(105) - dec!(102)) / dec!(102) * dec!(100);
        assert!((result - expected).abs() < dec!(0.01)); // Allow small floating point differences
    }

    #[test]
    fn test_z_score_calculation() {
        let strategy = MeanReversionStrategy::new(4, 2.0, 1.0);
        let prices = vec![dec!(98), dec!(100), dec!(102), dec!(100), dec!(110)]; // Last price is outlier
        
        let z_score = strategy.calculate_z_score(&prices);
        assert!(z_score.is_some());
        
        let result = z_score.unwrap();
        // With mean around 100 and std dev around 4.47, z-score should be positive
        assert!(result > dec!(1)); // 110 is above the mean
    }
    
    #[test]
    fn test_strategy_parameter_handling() {
        // Test that strategies properly handle custom parameters
        let mut config = StrategyConfig::new("test_params".to_string(), "Test Parameters".to_string());
        config.set_parameter("short_period", 8);
        config.set_parameter("long_period", 21);
        config.set_parameter("position_size", 0.80f64);
        
        let mut ma_strategy = MovingAverageCrossoverStrategy::new(5, 10);
        assert!(ma_strategy.initialize(&config).is_ok());
        
        // Verify parameters were applied
        assert_eq!(ma_strategy.short_period, 8);
        assert_eq!(ma_strategy.long_period, 21);
        // Use approximate equality for floating point conversions
        assert!((ma_strategy.position_size - dec!(0.80)).abs() < dec!(0.01));
    }

    #[test]
    fn test_sma_calculation() {
        let strategy = MovingAverageCrossoverStrategy::new(3, 5);
        let prices = vec![dec!(100), dec!(101), dec!(102), dec!(103), dec!(104)];
        
        // Test 3-period SMA
        let sma_3 = strategy.calculate_sma(&prices, 3);
        assert!(sma_3.is_some());
        assert_eq!(sma_3.unwrap(), dec!(103)); // (102 + 103 + 104) / 3
        
        // Test 5-period SMA
        let sma_5 = strategy.calculate_sma(&prices, 5);
        assert!(sma_5.is_some());
        assert_eq!(sma_5.unwrap(), dec!(102)); // (100 + 101 + 102 + 103 + 104) / 5
        
        // Test insufficient data
        let sma_6 = strategy.calculate_sma(&prices, 6);
        assert!(sma_6.is_none());
    }
} 