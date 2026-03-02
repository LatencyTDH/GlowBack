//! Broker abstraction for live and paper trading.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use gb_types::market::{MarketEvent, Symbol};
use gb_types::orders::{Fill, Order, OrderId, OrderStatus};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Snapshot of an account balance returned by a broker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountBalance {
    pub cash: Decimal,
    pub buying_power: Decimal,
    pub equity: Decimal,
    pub timestamp: DateTime<Utc>,
}

/// Snapshot of a single position held at the broker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrokerPosition {
    pub symbol: Symbol,
    pub quantity: Decimal,
    pub market_value: Decimal,
    pub average_cost: Decimal,
    pub unrealized_pnl: Decimal,
}

/// Connection status of a broker adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Reconnecting,
}

/// Errors surfaced by broker operations.
#[derive(Debug, thiserror::Error)]
pub enum BrokerError {
    #[error("not connected to broker")]
    NotConnected,
    #[error("order rejected by broker: {reason}")]
    OrderRejected { reason: String },
    #[error("order not found: {order_id}")]
    OrderNotFound { order_id: String },
    #[error("authentication failed: {message}")]
    AuthenticationFailed { message: String },
    #[error("rate limited — retry after {retry_after_ms} ms")]
    RateLimited { retry_after_ms: u64 },
    #[error("broker internal error: {message}")]
    Internal { message: String },
}

/// Result alias for broker operations.
pub type BrokerResult<T> = Result<T, BrokerError>;

/// Callback receiver for asynchronous broker events (fills, status changes, etc.).
#[async_trait]
pub trait BrokerCallback: Send + Sync {
    /// Called when a fill is received from the broker.
    async fn on_fill(&self, fill: Fill);
    /// Called when an order's status changes.
    async fn on_order_status(&self, order_id: OrderId, status: OrderStatus);
    /// Called when a market data event arrives.
    async fn on_market_data(&self, event: MarketEvent);
    /// Called when the connection status changes.
    async fn on_connection_status(&self, status: ConnectionStatus);
}

/// Core broker interface.
///
/// Implementations may talk to a real brokerage REST / WebSocket API or
/// simulate execution locally (see [`super::paper::PaperBroker`]).
#[async_trait]
pub trait Broker: Send + Sync {
    /// Connect to the broker and authenticate.
    async fn connect(&mut self) -> BrokerResult<()>;

    /// Disconnect gracefully.
    async fn disconnect(&mut self) -> BrokerResult<()>;

    /// Current connection status.
    fn connection_status(&self) -> ConnectionStatus;

    // -- Order management ---------------------------------------------------

    /// Submit a new order. Returns the broker-assigned order id.
    async fn submit_order(&mut self, order: Order) -> BrokerResult<OrderId>;

    /// Cancel an open order.
    async fn cancel_order(&mut self, order_id: OrderId) -> BrokerResult<()>;

    /// Query the current status of an order.
    async fn get_order_status(&self, order_id: OrderId) -> BrokerResult<OrderStatus>;

    /// List all open (active) orders.
    async fn get_open_orders(&self) -> BrokerResult<Vec<Order>>;

    // -- Account queries ----------------------------------------------------

    /// Retrieve the current account balance.
    async fn get_account_balance(&self) -> BrokerResult<AccountBalance>;

    /// Retrieve all positions currently held.
    async fn get_positions(&self) -> BrokerResult<Vec<BrokerPosition>>;

    /// Retrieve the position for a specific symbol (if any).
    async fn get_position(&self, symbol: &Symbol) -> BrokerResult<Option<BrokerPosition>>;

    // -- Market data --------------------------------------------------------

    /// Subscribe to real-time market data for the given symbols.
    async fn subscribe_market_data(&mut self, symbols: &[Symbol]) -> BrokerResult<()>;

    /// Unsubscribe from market data.
    async fn unsubscribe_market_data(&mut self, symbols: &[Symbol]) -> BrokerResult<()>;

    /// Get the latest known price for a symbol. Returns `None` if no data has
    /// been received yet.
    fn get_latest_price(&self, symbol: &Symbol) -> Option<Decimal>;

    /// Get all latest prices.
    fn get_all_prices(&self) -> HashMap<Symbol, Decimal>;
}
