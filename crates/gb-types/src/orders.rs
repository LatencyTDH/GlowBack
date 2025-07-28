use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::market::Symbol;

/// Unique order identifier
pub type OrderId = Uuid;

/// Direction of an order (buy or sell)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

impl Side {
    pub fn opposite(&self) -> Side {
        match self {
            Side::Buy => Side::Sell,
            Side::Sell => Side::Buy,
        }
    }
    
    pub fn sign(&self) -> i32 {
        match self {
            Side::Buy => 1,
            Side::Sell => -1,
        }
    }
}

/// Order types supported by the engine
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit { price: Decimal },
    Stop { stop_price: Decimal },
    StopLimit { stop_price: Decimal, limit_price: Decimal },
}

/// Time in force specifications
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeInForce {
    Day,
    GTC, // Good Till Canceled
    IOC, // Immediate or Cancel
    FOK, // Fill or Kill
}

/// Order status during lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,
    Submitted,
    PartiallyFilled,
    Filled,
    Canceled,
    Rejected,
    Expired,
}

/// Order request from strategy
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Order {
    pub id: OrderId,
    pub symbol: Symbol,
    pub side: Side,
    pub quantity: Decimal,
    pub order_type: OrderType,
    pub time_in_force: TimeInForce,
    pub submitted_at: DateTime<Utc>,
    pub status: OrderStatus,
    pub filled_quantity: Decimal,
    pub remaining_quantity: Decimal,
    pub average_fill_price: Option<Decimal>,
    pub strategy_id: String,
    pub metadata: serde_json::Value,
}

impl Order {
    pub fn new(
        symbol: Symbol,
        side: Side,
        quantity: Decimal,
        order_type: OrderType,
        strategy_id: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            symbol,
            side,
            quantity,
            order_type,
            time_in_force: TimeInForce::GTC,
            submitted_at: Utc::now(),
            status: OrderStatus::Pending,
            filled_quantity: Decimal::ZERO,
            remaining_quantity: quantity,
            average_fill_price: None,
            strategy_id,
            metadata: serde_json::Value::Null,
        }
    }
    
    pub fn market_order(symbol: Symbol, side: Side, quantity: Decimal, strategy_id: String) -> Self {
        Self::new(symbol, side, quantity, OrderType::Market, strategy_id)
    }
    
    pub fn limit_order(
        symbol: Symbol,
        side: Side,
        quantity: Decimal,
        price: Decimal,
        strategy_id: String,
    ) -> Self {
        Self::new(symbol, side, quantity, OrderType::Limit { price }, strategy_id)
    }
    
    pub fn stop_order(
        symbol: Symbol,
        side: Side,
        quantity: Decimal,
        stop_price: Decimal,
        strategy_id: String,
    ) -> Self {
        Self::new(symbol, side, quantity, OrderType::Stop { stop_price }, strategy_id)
    }
    
    pub fn is_buy(&self) -> bool {
        matches!(self.side, Side::Buy)
    }
    
    pub fn is_sell(&self) -> bool {
        matches!(self.side, Side::Sell)
    }
    
    pub fn is_filled(&self) -> bool {
        self.status == OrderStatus::Filled
    }
    
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            OrderStatus::Pending | OrderStatus::Submitted | OrderStatus::PartiallyFilled
        )
    }
    
    pub fn fill(&mut self, quantity: Decimal, price: Decimal) {
        let fill_quantity = quantity.min(self.remaining_quantity);
        
        // Update filled quantity and average price
        let total_filled = self.filled_quantity + fill_quantity;
        if let Some(avg_price) = self.average_fill_price {
            self.average_fill_price = Some(
                (avg_price * self.filled_quantity + price * fill_quantity) / total_filled
            );
        } else {
            self.average_fill_price = Some(price);
        }
        
        self.filled_quantity = total_filled;
        self.remaining_quantity = self.quantity - total_filled;
        
        // Update status
        if self.remaining_quantity == Decimal::ZERO {
            self.status = OrderStatus::Filled;
        } else {
            self.status = OrderStatus::PartiallyFilled;
        }
    }
    
    pub fn cancel(&mut self) {
        if self.is_active() {
            self.status = OrderStatus::Canceled;
        }
    }
}

/// Order execution record
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fill {
    pub id: Uuid,
    pub order_id: OrderId,
    pub symbol: Symbol,
    pub side: Side,
    pub quantity: Decimal,
    pub price: Decimal,
    pub commission: Decimal,
    pub executed_at: DateTime<Utc>,
    pub strategy_id: String,
}

impl Fill {
    pub fn new(
        order_id: OrderId,
        symbol: Symbol,
        side: Side,
        quantity: Decimal,
        price: Decimal,
        commission: Decimal,
        strategy_id: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            order_id,
            symbol,
            side,
            quantity,
            price,
            commission,
            executed_at: Utc::now(),
            strategy_id,
        }
    }
    
    pub fn gross_amount(&self) -> Decimal {
        self.quantity * self.price
    }
    
    pub fn net_amount(&self) -> Decimal {
        match self.side {
            Side::Buy => -(self.gross_amount() + self.commission),
            Side::Sell => self.gross_amount() - self.commission,
        }
    }
}

/// Order event for the event-driven engine
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrderEvent {
    OrderSubmitted(Order),
    OrderFilled { order_id: OrderId, fill: Fill },
    OrderCanceled { order_id: OrderId, reason: String },
    OrderRejected { order_id: OrderId, reason: String },
}

impl OrderEvent {
    pub fn order_id(&self) -> OrderId {
        match self {
            OrderEvent::OrderSubmitted(order) => order.id,
            OrderEvent::OrderFilled { order_id, .. } => *order_id,
            OrderEvent::OrderCanceled { order_id, .. } => *order_id,
            OrderEvent::OrderRejected { order_id, .. } => *order_id,
        }
    }
}

/// Order management system interface
pub trait OrderManager {
    fn submit_order(&mut self, order: Order) -> Result<OrderId, String>;
    fn cancel_order(&mut self, order_id: OrderId) -> Result<(), String>;
    fn get_order(&self, order_id: OrderId) -> Option<&Order>;
    fn get_active_orders(&self) -> Vec<&Order>;
    fn get_fills(&self) -> Vec<&Fill>;
} 