// Order execution engine - realistic implementation
// Provides realistic execution with slippage and commission models

use gb_types::{Order, Fill, Bar, Symbol, Side, GbResult};
use chrono::{DateTime, Utc, Duration};
use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::{info, debug, warn};

/// Execution configuration
#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    pub commission_per_share: Decimal,
    pub commission_percentage: Decimal,
    pub minimum_commission: Decimal,
    pub slippage_bps: Decimal,
    pub latency_ms: u64,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            commission_per_share: Decimal::new(1, 3), // $0.001 per share
            commission_percentage: Decimal::new(5, 3), // 0.005%
            minimum_commission: Decimal::new(1, 0), // $1.00 minimum
            slippage_bps: Decimal::from(5), // 5 basis points
            latency_ms: 50, // 50ms latency
        }
    }
}

/// Realistic execution engine with market simulation
#[derive(Debug)]
pub struct ExecutionEngine {
    config: ExecutionConfig,
    current_market_data: HashMap<Symbol, Bar>,
    last_execution_time: Option<DateTime<Utc>>,
}

impl ExecutionEngine {
    /// Create a new execution engine with configuration
    pub fn new(config: ExecutionConfig) -> Self {
        Self {
            config,
            current_market_data: HashMap::new(),
            last_execution_time: None,
        }
    }

    /// Update current market data for execution calculations
    pub fn update_market_data(&mut self, symbol: Symbol, bar: Bar) {
        self.current_market_data.insert(symbol, bar);
    }

    /// Execute an order with realistic market conditions
    pub async fn execute_order(&mut self, order: &Order, current_time: DateTime<Utc>) -> GbResult<Option<Fill>> {
        debug!("Attempting to execute order: {:?} {} {} shares", order.side, order.symbol, order.quantity);

        // Check if we have market data for this symbol
        let market_bar = match self.current_market_data.get(&order.symbol) {
            Some(bar) => bar,
            None => {
                warn!("No market data available for symbol: {}", order.symbol);
                return Ok(None);
            }
        };

        // Apply latency model - check if enough time has passed since last execution
        if let Some(last_exec) = self.last_execution_time {
            let time_since_last = current_time.signed_duration_since(last_exec);
            let required_latency = Duration::milliseconds(self.config.latency_ms as i64);
            
            if time_since_last < required_latency {
                debug!("Order delayed due to latency model");
                return Ok(None);
            }
        }

        // Determine execution price based on order type and market conditions
        let base_price = self.get_execution_price(order, market_bar)?;

        if base_price == Decimal::ZERO {
            debug!("Order cannot be executed at current market conditions");
            return Ok(None);
        }

        // Apply slippage
        let slipped_price = self.apply_slippage(order, base_price)?;

        // Calculate commission
        let commission = self.calculate_commission(order, slipped_price)?;

        // Create fill
        let fill = Fill::new(
            order.id,
            order.symbol.clone(),
            order.side,
            order.quantity,
            slipped_price,
            commission,
            order.strategy_id.clone(),
        );

        // Update execution time
        self.last_execution_time = Some(current_time);

        info!("Executed order: {:?} {} {} shares at {} (commission: {})", 
            order.side, order.symbol, order.quantity, slipped_price, commission);

        Ok(Some(fill))
    }

    /// Determine base execution price based on order type
    fn get_execution_price(&self, order: &Order, market_bar: &Bar) -> GbResult<Decimal> {
        let price = match order.order_type {
            gb_types::OrderType::Market => {
                // Market orders execute at bid/ask based on side
                match order.side {
                    Side::Buy => {
                        // Estimate ask price (slightly above market)
                        let spread = (market_bar.high - market_bar.low) * Decimal::new(5, 3); // 0.5% of range
                        market_bar.close + (spread / Decimal::from(2))
                    }
                    Side::Sell => {
                        // Estimate bid price (slightly below market)
                        let spread = (market_bar.high - market_bar.low) * Decimal::new(5, 3);
                        market_bar.close - (spread / Decimal::from(2))
                    }
                }
            }
            gb_types::OrderType::Limit { price } => {
                // Limit orders execute at limit price if market allows
                // For simplicity, we'll execute if price is within the bar's range
                if price >= market_bar.low && price <= market_bar.high {
                    price
                } else {
                    return Ok(Decimal::ZERO); // No execution
                }
            }
            gb_types::OrderType::Stop { stop_price } => {
                // Stop orders become market orders when triggered
                if (order.side == Side::Buy && market_bar.high >= stop_price) ||
                   (order.side == Side::Sell && market_bar.low <= stop_price) {
                    market_bar.close
                } else {
                    return Ok(Decimal::ZERO); // Not triggered
                }
            }
            gb_types::OrderType::StopLimit { stop_price, limit_price } => {
                // Stop-limit orders become limit orders when triggered
                if (order.side == Side::Buy && market_bar.high >= stop_price) ||
                   (order.side == Side::Sell && market_bar.low <= stop_price) {
                    // Now check if limit price can be filled
                    if limit_price >= market_bar.low && limit_price <= market_bar.high {
                        limit_price
                    } else {
                        return Ok(Decimal::ZERO); // Triggered but can't fill at limit
                    }
                } else {
                    return Ok(Decimal::ZERO); // Not triggered
                }
            }
        };

        Ok(price)
    }

    /// Apply slippage model to execution price
    fn apply_slippage(&self, order: &Order, base_price: Decimal) -> GbResult<Decimal> {
        // Apply slippage
        let slippage_factor = self.config.slippage_bps / Decimal::from(10000); // Convert bps to decimal
        let slippage_amount = base_price * slippage_factor;

        let slipped_price = match order.side {
            Side::Buy => base_price + slippage_amount,  // Pay more when buying
            Side::Sell => base_price - slippage_amount, // Receive less when selling
        };

        debug!("Applied slippage: {} bps, {} -> {}", self.config.slippage_bps, base_price, slipped_price);
        Ok(slipped_price)
    }

    /// Calculate commission for order execution
    fn calculate_commission(&self, order: &Order, execution_price: Decimal) -> GbResult<Decimal> {
        let notional_value = order.quantity * execution_price;
        
        let commission = self.config.commission_per_share * order.quantity +
                        (notional_value * self.config.commission_percentage / Decimal::from(100));

        Ok(commission.max(self.config.minimum_commission))
    }

    /// Set execution latency
    pub fn set_latency(&mut self, latency_ms: u64) {
        self.config.latency_ms = latency_ms;
    }

    /// Set slippage in basis points
    pub fn set_slippage(&mut self, slippage_bps: Decimal) {
        self.config.slippage_bps = slippage_bps;
    }

    /// Get current execution configuration
    pub fn get_config(&self) -> &ExecutionConfig {
        &self.config
    }

    /// Update execution configuration
    pub fn update_config(&mut self, config: ExecutionConfig) {
        self.config = config;
    }
}

impl Default for ExecutionEngine {
    fn default() -> Self {
        Self::new(ExecutionConfig::default())
    }
} 