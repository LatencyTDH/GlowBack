//! Live trading engine that ties a [`Strategy`], [`Broker`], and [`RiskManager`]
//! together in an event-driven loop.

use gb_types::market::MarketEvent;
use gb_types::orders::{Fill, Order, OrderEvent, OrderId};
use gb_types::strategy::{Strategy, StrategyAction, StrategyConfig, StrategyContext};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, info, warn};

use crate::broker::Broker;
use crate::risk::{RiskCheckResult, RiskConfig, RiskManager};

/// Operating mode of the live engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradingMode {
    /// Orders are sent to the real broker (after passing risk checks).
    Live,
    /// Orders are executed on a paper broker — no real money at risk.
    Sandbox,
}

/// Events emitted by the live engine for external consumption (logging, UI,
/// alerting).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LiveEngineEvent {
    Started {
        mode: TradingMode,
        strategy_id: String,
    },
    Stopped {
        strategy_id: String,
        reason: String,
    },
    OrderSubmitted {
        order_id: OrderId,
        symbol: String,
        side: String,
        quantity: Decimal,
    },
    OrderFilled {
        order_id: OrderId,
        price: Decimal,
        quantity: Decimal,
    },
    OrderRejectedByRisk {
        order_id: OrderId,
        reason: String,
    },
    OrderRejectedByBroker {
        order_id: OrderId,
        error: String,
    },
    CircuitBreakerTripped {
        equity: Decimal,
    },
    MarketDataReceived {
        symbol: String,
    },
    Error {
        message: String,
    },
}

/// Configuration for the live trading engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveEngineConfig {
    pub mode: TradingMode,
    pub strategy_config: StrategyConfig,
    pub risk_config: RiskConfig,
    pub initial_capital: Decimal,
}

/// The live trading engine.  Generic over the broker and strategy
/// implementations so callers can plug in a paper broker for sandbox mode or a
/// real brokerage adapter for live trading.
pub struct LiveEngine<B: Broker, S: Strategy> {
    broker: B,
    strategy: S,
    risk_manager: RiskManager,
    config: LiveEngineConfig,
    context: StrategyContext,
    events: Vec<LiveEngineEvent>,
    running: bool,
    /// Maps order IDs to the orders tracked locally.
    pending_orders: HashMap<OrderId, Order>,
}

impl<B: Broker, S: Strategy> LiveEngine<B, S> {
    /// Create a new live engine.
    pub fn new(broker: B, strategy: S, config: LiveEngineConfig) -> Self {
        let context = StrategyContext::new(
            config.strategy_config.strategy_id.clone(),
            config.initial_capital,
        );
        let risk_manager = RiskManager::new(config.risk_config.clone(), config.initial_capital);

        Self {
            broker,
            strategy,
            risk_manager,
            config,
            context,
            events: Vec::new(),
            running: false,
            pending_orders: HashMap::new(),
        }
    }

    /// Start the engine: connect the broker, initialize the strategy, and
    /// subscribe to market data.
    pub async fn start(&mut self) -> Result<(), String> {
        self.broker
            .connect()
            .await
            .map_err(|e| format!("broker connect failed: {e}"))?;

        self.strategy
            .initialize(&self.config.strategy_config)
            .map_err(|e| format!("strategy init failed: {e}"))?;

        // Subscribe to market data for configured symbols.
        self.broker
            .subscribe_market_data(&self.config.strategy_config.symbols)
            .await
            .map_err(|e| format!("market data subscription failed: {e}"))?;

        self.running = true;

        let event = LiveEngineEvent::Started {
            mode: self.config.mode,
            strategy_id: self.config.strategy_config.strategy_id.clone(),
        };
        self.emit(event);

        info!(
            mode = ?self.config.mode,
            strategy = %self.config.strategy_config.strategy_id,
            "live engine started"
        );

        Ok(())
    }

    /// Stop the engine gracefully.
    pub async fn stop(&mut self, reason: &str) -> Result<(), String> {
        self.running = false;

        let _ = self.strategy.on_stop(&self.context);

        self.broker
            .unsubscribe_market_data(&self.config.strategy_config.symbols)
            .await
            .map_err(|e| format!("unsubscribe failed: {e}"))?;

        self.broker
            .disconnect()
            .await
            .map_err(|e| format!("broker disconnect failed: {e}"))?;

        let event = LiveEngineEvent::Stopped {
            strategy_id: self.config.strategy_config.strategy_id.clone(),
            reason: reason.to_string(),
        };
        self.emit(event);

        info!(
            strategy = %self.config.strategy_config.strategy_id,
            reason = %reason,
            "live engine stopped"
        );

        Ok(())
    }

    /// Process an incoming market data event.  Feeds it to the strategy and
    /// routes any resulting actions through the risk manager and broker.
    pub async fn on_market_event(&mut self, event: MarketEvent) -> Result<(), String> {
        if !self.running {
            return Err("engine not running".into());
        }

        let symbol = event.symbol().clone();

        // Update strategy context's market data buffer.
        {
            let buffer = self
                .context
                .market_data
                .entry(symbol.clone())
                .or_insert_with(|| gb_types::strategy::MarketDataBuffer::new(symbol.clone(), 500));
            buffer.add_event(event.clone());
        }

        self.context.current_time = event.timestamp();

        // Let the strategy react.
        let actions = self
            .strategy
            .on_market_event(&event, &self.context)
            .map_err(|e| format!("strategy error: {e}"))?;

        for action in actions {
            self.handle_action(action).await?;
        }

        Ok(())
    }

    /// Process an order fill received from the broker.
    pub async fn on_fill(&mut self, fill: Fill) -> Result<(), String> {
        // Update portfolio
        self.context.portfolio.apply_fill(&fill);

        // Update risk manager position tracking
        self.risk_manager
            .update_position(&fill.symbol, fill.side, fill.quantity);

        // Remove from pending if fully filled
        if let Some(order) = self.pending_orders.get(&fill.order_id) {
            if order.remaining_quantity <= fill.quantity {
                self.pending_orders.remove(&fill.order_id);
            }
        }

        self.emit(LiveEngineEvent::OrderFilled {
            order_id: fill.order_id,
            price: fill.price,
            quantity: fill.quantity,
        });

        // Notify strategy
        let order_event = OrderEvent::OrderFilled {
            order_id: fill.order_id,
            fill,
        };
        let actions = self
            .strategy
            .on_order_event(&order_event, &self.context)
            .map_err(|e| format!("strategy error on fill: {e}"))?;

        for action in actions {
            self.handle_action(action).await?;
        }

        Ok(())
    }

    /// Signal end of trading day to the strategy.
    pub async fn on_day_end(&mut self) -> Result<(), String> {
        if !self.running {
            return Ok(());
        }

        let actions = self
            .strategy
            .on_day_end(&self.context)
            .map_err(|e| format!("strategy day-end error: {e}"))?;

        for action in actions {
            self.handle_action(action).await?;
        }

        // Refresh risk manager daily state using the current equity.
        let equity = self.context.portfolio.total_equity;
        self.risk_manager.reset_daily(equity);

        Ok(())
    }

    /// Route a single [`StrategyAction`] through risk checks and the broker.
    async fn handle_action(&mut self, action: StrategyAction) -> Result<(), String> {
        match action {
            StrategyAction::PlaceOrder(order) => {
                self.submit_order(order).await?;
            }
            StrategyAction::CancelOrder { order_id } => {
                if let Err(e) = self.broker.cancel_order(order_id).await {
                    warn!(order_id = %order_id, error = %e, "cancel failed");
                }
            }
            StrategyAction::Log { level, message } => match level {
                gb_types::strategy::LogLevel::Debug => {
                    tracing::debug!(strategy = %self.config.strategy_config.strategy_id, "{message}")
                }
                gb_types::strategy::LogLevel::Info => {
                    tracing::info!(strategy = %self.config.strategy_config.strategy_id, "{message}")
                }
                gb_types::strategy::LogLevel::Warning => {
                    tracing::warn!(strategy = %self.config.strategy_config.strategy_id, "{message}")
                }
                gb_types::strategy::LogLevel::Error => {
                    tracing::error!(strategy = %self.config.strategy_config.strategy_id, "{message}")
                }
            },
            StrategyAction::SetParameter { .. } => {
                // Parameter updates are strategy-internal; nothing to route.
            }
        }
        Ok(())
    }

    /// Submit an order through the risk manager and, if approved, to the
    /// broker.
    async fn submit_order(&mut self, order: Order) -> Result<(), String> {
        let symbol = &order.symbol;
        let price = self
            .broker
            .get_latest_price(symbol)
            .unwrap_or(Decimal::ZERO);
        let equity = self.context.portfolio.total_equity;

        // Pre-trade risk check
        let result = self.risk_manager.check_order(&order, price, equity);

        match result {
            RiskCheckResult::Approved => match self.broker.submit_order(order.clone()).await {
                Ok(oid) => {
                    self.emit(LiveEngineEvent::OrderSubmitted {
                        order_id: oid,
                        symbol: order.symbol.to_string(),
                        side: format!("{:?}", order.side),
                        quantity: order.quantity,
                    });
                    self.pending_orders.insert(oid, order);
                }
                Err(e) => {
                    self.emit(LiveEngineEvent::OrderRejectedByBroker {
                        order_id: order.id,
                        error: e.to_string(),
                    });
                    error!(order_id = %order.id, error = %e, "broker rejected order");
                }
            },
            RiskCheckResult::Rejected { reason } => {
                self.emit(LiveEngineEvent::OrderRejectedByRisk {
                    order_id: order.id,
                    reason: reason.clone(),
                });
                warn!(order_id = %order.id, reason = %reason, "risk manager rejected order");

                if self.risk_manager.is_circuit_breaker_tripped() {
                    self.emit(LiveEngineEvent::CircuitBreakerTripped { equity });
                }
            }
        }

        Ok(())
    }

    // -- accessors ----------------------------------------------------------

    /// Whether the engine is currently running.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Current trading mode.
    pub fn mode(&self) -> TradingMode {
        self.config.mode
    }

    /// Drain all emitted events (consuming them).
    pub fn drain_events(&mut self) -> Vec<LiveEngineEvent> {
        std::mem::take(&mut self.events)
    }

    /// Borrow the current strategy context (portfolio, market data, etc.).
    pub fn context(&self) -> &StrategyContext {
        &self.context
    }

    /// Mutable access to the broker (e.g. for feeding market events on a paper
    /// broker).
    pub fn broker_mut(&mut self) -> &mut B {
        &mut self.broker
    }

    /// Immutable access to the broker.
    pub fn broker(&self) -> &B {
        &self.broker
    }

    /// Access to the risk manager.
    pub fn risk_manager(&self) -> &RiskManager {
        &self.risk_manager
    }

    fn emit(&mut self, event: LiveEngineEvent) {
        self.events.push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paper::{PaperBroker, PaperBrokerConfig};
    use chrono::Utc;
    use gb_types::market::{AssetClass, Bar, Resolution, Symbol};
    use gb_types::strategy::{BuyAndHoldStrategy, StrategyConfig};
    use rust_decimal_macros::dec;

    fn test_symbol() -> Symbol {
        Symbol::new("AAPL", "NASDAQ", AssetClass::Equity)
    }

    fn make_bar(close: Decimal) -> MarketEvent {
        MarketEvent::Bar(Bar {
            symbol: test_symbol(),
            timestamp: Utc::now(),
            open: close,
            high: close,
            low: close,
            close,
            volume: dec!(1000),
            resolution: Resolution::Day,
        })
    }

    fn default_engine() -> LiveEngine<PaperBroker, BuyAndHoldStrategy> {
        let broker = PaperBroker::new(PaperBrokerConfig {
            initial_cash: dec!(100_000),
            ..Default::default()
        });
        let strategy = BuyAndHoldStrategy::new();

        let mut strategy_config =
            StrategyConfig::new("test_live".into(), "Test Live Strategy".into());
        strategy_config.add_symbol(test_symbol());

        let config = LiveEngineConfig {
            mode: TradingMode::Sandbox,
            strategy_config,
            risk_config: RiskConfig {
                // Relax limits for the buy-and-hold strategy which uses 95% of cash.
                max_order_notional: Decimal::from(1_000_000),
                max_total_exposure: Decimal::from(1_000_000),
                limits: gb_types::portfolio::RiskLimits {
                    position_concentration_limit: dec!(1.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            initial_capital: dec!(100_000),
        };

        LiveEngine::new(broker, strategy, config)
    }

    #[tokio::test]
    async fn test_engine_start_stop() {
        let mut engine = default_engine();
        assert!(!engine.is_running());

        engine.start().await.unwrap();
        assert!(engine.is_running());
        assert_eq!(engine.mode(), TradingMode::Sandbox);

        let events = engine.drain_events();
        assert!(events
            .iter()
            .any(|e| matches!(e, LiveEngineEvent::Started { .. })));

        engine.stop("test").await.unwrap();
        assert!(!engine.is_running());

        let events = engine.drain_events();
        assert!(events
            .iter()
            .any(|e| matches!(e, LiveEngineEvent::Stopped { .. })));
    }

    #[tokio::test]
    async fn test_engine_rejects_event_when_stopped() {
        let mut engine = default_engine();
        let result = engine.on_market_event(make_bar(dec!(150))).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_engine_processes_market_event() {
        let mut engine = default_engine();
        engine.start().await.unwrap();
        engine.drain_events(); // clear started event

        // Seed price on the paper broker
        engine
            .broker_mut()
            .process_market_event(&make_bar(dec!(150)));

        // Feed event to engine
        engine.on_market_event(make_bar(dec!(150))).await.unwrap();

        // Buy-and-hold should have placed an order
        let events = engine.drain_events();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, LiveEngineEvent::OrderSubmitted { .. })),
            "expected OrderSubmitted event, got: {events:?}"
        );
    }

    #[tokio::test]
    async fn test_engine_day_end() {
        let mut engine = default_engine();
        engine.start().await.unwrap();
        engine.drain_events();

        // day_end should not fail even without market data
        engine.on_day_end().await.unwrap();
    }

    #[tokio::test]
    async fn test_engine_circuit_breaker_propagates() {
        let risk_config = RiskConfig {
            daily_loss_circuit_breaker: dec!(0.01), // 1% — very tight
            ..Default::default()
        };

        let broker = PaperBroker::with_defaults();
        let strategy = BuyAndHoldStrategy::new();

        let mut strategy_config = StrategyConfig::new("cb_test".into(), "CB Test".into());
        strategy_config.add_symbol(test_symbol());

        let config = LiveEngineConfig {
            mode: TradingMode::Sandbox,
            strategy_config,
            risk_config,
            initial_capital: dec!(100_000),
        };

        let mut engine = LiveEngine::new(broker, strategy, config);
        engine.start().await.unwrap();
        engine.drain_events();

        // Simulate a portfolio loss by mutating equity directly
        engine.context.portfolio.cash = dec!(98_000);
        engine.context.portfolio.total_equity = dec!(98_000);

        // Seed broker price
        engine
            .broker_mut()
            .process_market_event(&make_bar(dec!(150)));

        // Feed event — strategy will try to place an order
        engine.on_market_event(make_bar(dec!(150))).await.unwrap();

        let events = engine.drain_events();
        let has_cb = events
            .iter()
            .any(|e| matches!(e, LiveEngineEvent::CircuitBreakerTripped { .. }));
        let has_risk_reject = events
            .iter()
            .any(|e| matches!(e, LiveEngineEvent::OrderRejectedByRisk { .. }));

        assert!(
            has_cb || has_risk_reject,
            "expected circuit breaker or risk rejection, got: {events:?}"
        );
    }
}
