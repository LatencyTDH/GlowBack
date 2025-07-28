use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
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