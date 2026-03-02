//! Paper (simulated) broker for sandbox mode.
//!
//! Executes orders locally with no external dependencies.  Useful for strategy
//! development, integration testing, and validating risk controls before going
//! live.

use async_trait::async_trait;
use chrono::Utc;
use gb_types::market::{MarketEvent, Symbol};
use gb_types::orders::{Fill, Order, OrderId, OrderStatus, OrderType, Side};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

use crate::broker::{
    AccountBalance, Broker, BrokerError, BrokerPosition, BrokerResult, ConnectionStatus,
};

/// Configuration for the paper broker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaperBrokerConfig {
    /// Starting cash balance.
    pub initial_cash: Decimal,
    /// Commission per share (applied to both buys and sells).
    pub commission_per_share: Decimal,
    /// Simulated slippage as a fraction of price (e.g. 0.001 = 0.1%).
    pub slippage_bps: Decimal,
    /// Whether to fill market orders immediately at the current price or wait
    /// for the next market event.
    pub fill_market_orders_immediately: bool,
}

impl Default for PaperBrokerConfig {
    fn default() -> Self {
        Self {
            initial_cash: Decimal::from(100_000),
            commission_per_share: Decimal::new(1, 2), // $0.01
            slippage_bps: Decimal::new(5, 4),         // 0.05%
            fill_market_orders_immediately: true,
        }
    }
}

/// Internal position tracking.
#[derive(Debug, Clone)]
struct PaperPosition {
    symbol: Symbol,
    quantity: Decimal,
    average_cost: Decimal,
}

/// A fully in-process broker that simulates order execution.
#[derive(Debug)]
pub struct PaperBroker {
    config: PaperBrokerConfig,
    connected: bool,
    cash: Decimal,
    positions: HashMap<Symbol, PaperPosition>,
    orders: HashMap<OrderId, Order>,
    fills: Vec<Fill>,
    latest_prices: HashMap<Symbol, Decimal>,
    subscribed_symbols: Vec<Symbol>,
}

impl PaperBroker {
    pub fn new(config: PaperBrokerConfig) -> Self {
        let cash = config.initial_cash;
        Self {
            config,
            connected: false,
            cash,
            positions: HashMap::new(),
            orders: HashMap::new(),
            fills: Vec::new(),
            latest_prices: HashMap::new(),
            subscribed_symbols: Vec::new(),
        }
    }

    /// Create a paper broker with default settings.
    pub fn with_defaults() -> Self {
        Self::new(PaperBrokerConfig::default())
    }

    /// Feed a market event to update the latest price and attempt to fill
    /// pending limit / stop orders.
    pub fn process_market_event(&mut self, event: &MarketEvent) {
        let symbol = event.symbol().clone();
        let price = match event {
            MarketEvent::Bar(bar) => bar.close,
            MarketEvent::Tick(tick) => tick.price,
            MarketEvent::Quote { bid, ask, .. } => (*bid + *ask) / Decimal::from(2),
        };
        self.latest_prices.insert(symbol.clone(), price);

        // Try to fill pending orders for this symbol.
        let pending: Vec<OrderId> = self
            .orders
            .iter()
            .filter(|(_, o)| o.symbol == symbol && o.is_active())
            .map(|(id, _)| *id)
            .collect();

        for order_id in pending {
            let _ = self.try_fill_order(order_id, price);
        }
    }

    /// Attempt to fill an order at `market_price`.  Returns `true` if filled.
    fn try_fill_order(&mut self, order_id: OrderId, market_price: Decimal) -> bool {
        let order = match self.orders.get(&order_id) {
            Some(o) if o.is_active() => o.clone(),
            _ => return false,
        };

        let fill_price = match &order.order_type {
            OrderType::Market => {
                // Apply slippage
                let slip = market_price * self.config.slippage_bps;
                match order.side {
                    Side::Buy => market_price + slip,
                    Side::Sell => market_price - slip,
                }
            }
            OrderType::Limit { price } => {
                match order.side {
                    Side::Buy if market_price <= *price => *price,
                    Side::Sell if market_price >= *price => *price,
                    _ => return false, // Not yet fillable
                }
            }
            OrderType::Stop { stop_price } => match order.side {
                Side::Buy if market_price >= *stop_price => market_price,
                Side::Sell if market_price <= *stop_price => market_price,
                _ => return false,
            },
            OrderType::StopLimit {
                stop_price,
                limit_price,
            } => match order.side {
                Side::Buy if market_price >= *stop_price && market_price <= *limit_price => {
                    *limit_price
                }
                Side::Sell if market_price <= *stop_price && market_price >= *limit_price => {
                    *limit_price
                }
                _ => return false,
            },
        };

        let quantity = order.remaining_quantity;
        let commission = quantity * self.config.commission_per_share;

        // Update cash
        match order.side {
            Side::Buy => {
                let cost = quantity * fill_price + commission;
                if cost > self.cash {
                    // Insufficient funds — reject
                    if let Some(o) = self.orders.get_mut(&order_id) {
                        o.status = OrderStatus::Rejected;
                    }
                    return false;
                }
                self.cash -= cost;
            }
            Side::Sell => {
                self.cash += quantity * fill_price - commission;
            }
        }

        // Update position
        let pos = self
            .positions
            .entry(order.symbol.clone())
            .or_insert_with(|| PaperPosition {
                symbol: order.symbol.clone(),
                quantity: Decimal::ZERO,
                average_cost: Decimal::ZERO,
            });

        match order.side {
            Side::Buy => {
                let total_cost = pos.quantity * pos.average_cost + quantity * fill_price;
                pos.quantity += quantity;
                if pos.quantity > Decimal::ZERO {
                    pos.average_cost = total_cost / pos.quantity;
                }
            }
            Side::Sell => {
                pos.quantity -= quantity;
                if pos.quantity <= Decimal::ZERO {
                    pos.quantity = Decimal::ZERO;
                    pos.average_cost = Decimal::ZERO;
                }
            }
        }

        // Record fill
        let fill = Fill::new(
            order_id,
            order.symbol.clone(),
            order.side,
            quantity,
            fill_price,
            commission,
            order.strategy_id.clone(),
        );
        self.fills.push(fill);

        // Update order status
        if let Some(o) = self.orders.get_mut(&order_id) {
            o.fill(quantity, fill_price);
        }

        info!(
            order_id = %order_id,
            symbol = %order.symbol,
            side = ?order.side,
            quantity = %quantity,
            price = %fill_price,
            "paper broker: order filled"
        );

        true
    }

    /// Get all recorded fills.
    pub fn get_fills(&self) -> &[Fill] {
        &self.fills
    }

    /// Current cash balance.
    pub fn cash(&self) -> Decimal {
        self.cash
    }
}

#[async_trait]
impl Broker for PaperBroker {
    async fn connect(&mut self) -> BrokerResult<()> {
        self.connected = true;
        info!("paper broker connected (sandbox mode)");
        Ok(())
    }

    async fn disconnect(&mut self) -> BrokerResult<()> {
        self.connected = false;
        info!("paper broker disconnected");
        Ok(())
    }

    fn connection_status(&self) -> ConnectionStatus {
        if self.connected {
            ConnectionStatus::Connected
        } else {
            ConnectionStatus::Disconnected
        }
    }

    async fn submit_order(&mut self, mut order: Order) -> BrokerResult<OrderId> {
        if !self.connected {
            return Err(BrokerError::NotConnected);
        }

        let order_id = order.id;
        order.status = OrderStatus::Submitted;

        // For market orders with immediate fill, try to fill now.
        if self.config.fill_market_orders_immediately
            && matches!(order.order_type, OrderType::Market)
        {
            if let Some(&price) = self.latest_prices.get(&order.symbol) {
                self.orders.insert(order_id, order);
                self.try_fill_order(order_id, price);
                return Ok(order_id);
            }
        }

        self.orders.insert(order_id, order);
        Ok(order_id)
    }

    async fn cancel_order(&mut self, order_id: OrderId) -> BrokerResult<()> {
        if !self.connected {
            return Err(BrokerError::NotConnected);
        }

        match self.orders.get_mut(&order_id) {
            Some(order) if order.is_active() => {
                order.cancel();
                Ok(())
            }
            Some(_) => Err(BrokerError::OrderRejected {
                reason: "order is not active".into(),
            }),
            None => Err(BrokerError::OrderNotFound {
                order_id: order_id.to_string(),
            }),
        }
    }

    async fn get_order_status(&self, order_id: OrderId) -> BrokerResult<OrderStatus> {
        self.orders
            .get(&order_id)
            .map(|o| o.status)
            .ok_or(BrokerError::OrderNotFound {
                order_id: order_id.to_string(),
            })
    }

    async fn get_open_orders(&self) -> BrokerResult<Vec<Order>> {
        Ok(self
            .orders
            .values()
            .filter(|o| o.is_active())
            .cloned()
            .collect())
    }

    async fn get_account_balance(&self) -> BrokerResult<AccountBalance> {
        let position_value: Decimal = self
            .positions
            .values()
            .map(|p| {
                let price = self
                    .latest_prices
                    .get(&p.symbol)
                    .copied()
                    .unwrap_or(p.average_cost);
                p.quantity * price
            })
            .sum();

        let equity = self.cash + position_value;

        Ok(AccountBalance {
            cash: self.cash,
            buying_power: self.cash,
            equity,
            timestamp: Utc::now(),
        })
    }

    async fn get_positions(&self) -> BrokerResult<Vec<BrokerPosition>> {
        Ok(self
            .positions
            .values()
            .filter(|p| p.quantity > Decimal::ZERO)
            .map(|p| {
                let market_price = self
                    .latest_prices
                    .get(&p.symbol)
                    .copied()
                    .unwrap_or(p.average_cost);
                BrokerPosition {
                    symbol: p.symbol.clone(),
                    quantity: p.quantity,
                    market_value: p.quantity * market_price,
                    average_cost: p.average_cost,
                    unrealized_pnl: p.quantity * (market_price - p.average_cost),
                }
            })
            .collect())
    }

    async fn get_position(&self, symbol: &Symbol) -> BrokerResult<Option<BrokerPosition>> {
        Ok(self.positions.get(symbol).and_then(|p| {
            if p.quantity <= Decimal::ZERO {
                return None;
            }
            let market_price = self
                .latest_prices
                .get(&p.symbol)
                .copied()
                .unwrap_or(p.average_cost);
            Some(BrokerPosition {
                symbol: p.symbol.clone(),
                quantity: p.quantity,
                market_value: p.quantity * market_price,
                average_cost: p.average_cost,
                unrealized_pnl: p.quantity * (market_price - p.average_cost),
            })
        }))
    }

    async fn subscribe_market_data(&mut self, symbols: &[Symbol]) -> BrokerResult<()> {
        for s in symbols {
            if !self.subscribed_symbols.contains(s) {
                self.subscribed_symbols.push(s.clone());
            }
        }
        Ok(())
    }

    async fn unsubscribe_market_data(&mut self, symbols: &[Symbol]) -> BrokerResult<()> {
        self.subscribed_symbols.retain(|s| !symbols.contains(s));
        Ok(())
    }

    fn get_latest_price(&self, symbol: &Symbol) -> Option<Decimal> {
        self.latest_prices.get(symbol).copied()
    }

    fn get_all_prices(&self) -> HashMap<Symbol, Decimal> {
        self.latest_prices.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gb_types::market::{AssetClass, Bar, Resolution};
    use rust_decimal_macros::dec;

    fn test_symbol() -> Symbol {
        Symbol::new("AAPL", "NASDAQ", AssetClass::Equity)
    }

    fn make_bar(symbol: Symbol, close: Decimal) -> MarketEvent {
        MarketEvent::Bar(Bar {
            symbol,
            timestamp: Utc::now(),
            open: close,
            high: close,
            low: close,
            close,
            volume: dec!(1000),
            resolution: Resolution::Day,
        })
    }

    #[tokio::test]
    async fn test_paper_broker_connect_disconnect() {
        let mut broker = PaperBroker::with_defaults();
        assert_eq!(broker.connection_status(), ConnectionStatus::Disconnected);

        broker.connect().await.unwrap();
        assert_eq!(broker.connection_status(), ConnectionStatus::Connected);

        broker.disconnect().await.unwrap();
        assert_eq!(broker.connection_status(), ConnectionStatus::Disconnected);
    }

    #[tokio::test]
    async fn test_paper_broker_submit_requires_connection() {
        let mut broker = PaperBroker::with_defaults();
        let order = Order::market_order(test_symbol(), Side::Buy, dec!(10), "s".into());
        let result = broker.submit_order(order).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_paper_broker_market_order_fill() {
        let mut broker = PaperBroker::with_defaults();
        broker.connect().await.unwrap();

        // Seed a price
        let bar = make_bar(test_symbol(), dec!(150));
        broker.process_market_event(&bar);

        // Submit a market buy
        let order = Order::market_order(test_symbol(), Side::Buy, dec!(10), "s".into());
        let oid = broker.submit_order(order).await.unwrap();

        let status = broker.get_order_status(oid).await.unwrap();
        assert_eq!(status, OrderStatus::Filled);

        // Check position
        let pos = broker.get_position(&test_symbol()).await.unwrap();
        assert!(pos.is_some());
        assert_eq!(pos.unwrap().quantity, dec!(10));

        // Check balance decreased
        let bal = broker.get_account_balance().await.unwrap();
        assert!(bal.cash < dec!(100_000));
    }

    #[tokio::test]
    async fn test_paper_broker_limit_order_pending_then_filled() {
        let mut broker = PaperBroker::with_defaults();
        broker.connect().await.unwrap();

        // Seed price at 150
        broker.process_market_event(&make_bar(test_symbol(), dec!(150)));

        // Limit buy at 145 — should NOT fill immediately
        let order = Order::limit_order(test_symbol(), Side::Buy, dec!(10), dec!(145), "s".into());
        let oid = broker.submit_order(order).await.unwrap();

        let status = broker.get_order_status(oid).await.unwrap();
        assert_eq!(status, OrderStatus::Submitted);

        // Price drops to 144 — should fill
        broker.process_market_event(&make_bar(test_symbol(), dec!(144)));
        let status = broker.get_order_status(oid).await.unwrap();
        assert_eq!(status, OrderStatus::Filled);
    }

    #[tokio::test]
    async fn test_paper_broker_cancel_order() {
        let mut broker = PaperBroker::with_defaults();
        broker.connect().await.unwrap();
        broker.process_market_event(&make_bar(test_symbol(), dec!(150)));

        let order = Order::limit_order(test_symbol(), Side::Buy, dec!(10), dec!(100), "s".into());
        let oid = broker.submit_order(order).await.unwrap();

        broker.cancel_order(oid).await.unwrap();
        let status = broker.get_order_status(oid).await.unwrap();
        assert_eq!(status, OrderStatus::Canceled);
    }

    #[tokio::test]
    async fn test_paper_broker_get_positions_filters_flat() {
        let mut broker = PaperBroker::with_defaults();
        broker.connect().await.unwrap();
        broker.process_market_event(&make_bar(test_symbol(), dec!(150)));

        // Buy then sell same quantity
        let buy = Order::market_order(test_symbol(), Side::Buy, dec!(10), "s".into());
        broker.submit_order(buy).await.unwrap();

        let sell = Order::market_order(test_symbol(), Side::Sell, dec!(10), "s".into());
        broker.submit_order(sell).await.unwrap();

        let positions = broker.get_positions().await.unwrap();
        assert!(positions.is_empty());
    }

    #[tokio::test]
    async fn test_paper_broker_insufficient_funds() {
        let config = PaperBrokerConfig {
            initial_cash: dec!(100),
            ..Default::default()
        };
        let mut broker = PaperBroker::new(config);
        broker.connect().await.unwrap();
        broker.process_market_event(&make_bar(test_symbol(), dec!(150)));

        // Try to buy 10 shares at ~$150 with only $100 cash
        let order = Order::market_order(test_symbol(), Side::Buy, dec!(10), "s".into());
        let oid = broker.submit_order(order).await.unwrap();

        let status = broker.get_order_status(oid).await.unwrap();
        assert_eq!(status, OrderStatus::Rejected);
    }

    #[tokio::test]
    async fn test_paper_broker_fills_recorded() {
        let mut broker = PaperBroker::with_defaults();
        broker.connect().await.unwrap();
        broker.process_market_event(&make_bar(test_symbol(), dec!(150)));

        let order = Order::market_order(test_symbol(), Side::Buy, dec!(5), "s".into());
        broker.submit_order(order).await.unwrap();

        assert_eq!(broker.get_fills().len(), 1);
        let fill = &broker.get_fills()[0];
        assert_eq!(fill.quantity, dec!(5));
        assert_eq!(fill.side, Side::Buy);
    }

    #[tokio::test]
    async fn test_paper_broker_subscribe_unsubscribe() {
        let mut broker = PaperBroker::with_defaults();
        broker.connect().await.unwrap();

        let sym = test_symbol();
        broker.subscribe_market_data(&[sym.clone()]).await.unwrap();
        assert!(broker.subscribed_symbols.contains(&sym));

        broker
            .unsubscribe_market_data(&[sym.clone()])
            .await
            .unwrap();
        assert!(!broker.subscribed_symbols.contains(&sym));
    }
}
