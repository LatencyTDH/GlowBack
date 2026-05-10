use chrono::{Duration, Utc};
use gb_engine::BacktestEngine;
use gb_types::{
    BacktestConfig, MarketEvent, Order, OrderEvent, Resolution, Side, Strategy, StrategyAction,
    StrategyConfig, StrategyContext, StrategyMetrics, Symbol,
};
use rust_decimal::Decimal;
use std::error::Error;
use std::sync::{Arc, Mutex};

#[derive(Debug, Default)]
struct LifecycleCounts {
    initialize_calls: usize,
    market_event_calls: usize,
    order_event_calls: usize,
    day_end_calls: usize,
    stop_calls: usize,
    sequence: Vec<String>,
}

#[derive(Clone)]
struct LifecycleTemplateStrategy {
    config: StrategyConfig,
    counts: Arc<Mutex<LifecycleCounts>>,
    submitted_order: bool,
}

impl LifecycleTemplateStrategy {
    fn new(counts: Arc<Mutex<LifecycleCounts>>) -> Self {
        let mut config = StrategyConfig::new(
            "lifecycle_template".to_string(),
            "Lifecycle Template".to_string(),
        );
        config.description =
            "Minimal runnable template showing the engine strategy lifecycle".to_string();

        Self {
            config,
            counts,
            submitted_order: false,
        }
    }

    fn record(&self, label: &str) {
        let mut counts = self.counts.lock().expect("lifecycle counts lock poisoned");
        counts.sequence.push(label.to_string());
        match label {
            "initialize" => counts.initialize_calls += 1,
            "on_market_event" => counts.market_event_calls += 1,
            "on_order_event" => counts.order_event_calls += 1,
            "on_day_end" => counts.day_end_calls += 1,
            "on_stop" => counts.stop_calls += 1,
            _ => {}
        }
    }
}

impl Strategy for LifecycleTemplateStrategy {
    fn initialize(&mut self, config: &StrategyConfig) -> Result<(), String> {
        self.config = config.clone();
        self.record("initialize");
        Ok(())
    }

    fn on_market_event(
        &mut self,
        event: &MarketEvent,
        context: &StrategyContext,
    ) -> Result<Vec<StrategyAction>, String> {
        self.record("on_market_event");
        if self.submitted_order {
            return Ok(vec![]);
        }

        let symbol = event.symbol().clone();
        let Some(price) = context.get_current_price(&symbol) else {
            return Ok(vec![]);
        };
        let quantity = (context.get_available_cash() * Decimal::new(50, 2)) / price;
        if quantity <= Decimal::ZERO {
            return Ok(vec![]);
        }

        self.submitted_order = true;
        Ok(vec![StrategyAction::PlaceOrder(Order::market_order(
            symbol,
            Side::Buy,
            quantity,
            self.config.strategy_id.clone(),
        ))])
    }

    fn on_order_event(
        &mut self,
        _event: &OrderEvent,
        _context: &StrategyContext,
    ) -> Result<Vec<StrategyAction>, String> {
        self.record("on_order_event");
        Ok(vec![])
    }

    fn on_day_end(&mut self, _context: &StrategyContext) -> Result<Vec<StrategyAction>, String> {
        self.record("on_day_end");
        Ok(vec![])
    }

    fn on_stop(&mut self, _context: &StrategyContext) -> Result<Vec<StrategyAction>, String> {
        self.record("on_stop");
        Ok(vec![])
    }

    fn get_config(&self) -> &StrategyConfig {
        &self.config
    }

    fn get_metrics(&self) -> StrategyMetrics {
        StrategyMetrics::new(self.config.strategy_id.clone())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let symbol = Symbol::equity("AAPL");
    let mut strategy_config = StrategyConfig::new(
        "lifecycle_template".to_string(),
        "Lifecycle Template".to_string(),
    );
    strategy_config.add_symbol(symbol.clone());

    let mut config = BacktestConfig::new(
        "Strategy Lifecycle Template".to_string(),
        strategy_config.clone(),
    );
    config.start_date = Utc::now() - Duration::days(5);
    config.end_date = Utc::now();
    config.initial_capital = Decimal::from(100_000);
    config.resolution = Resolution::Day;
    config.symbols = vec![symbol];
    config.strategy_config = strategy_config;
    config.data_settings.data_source = "sample".to_string();

    let counts = Arc::new(Mutex::new(LifecycleCounts::default()));
    let strategy = Box::new(LifecycleTemplateStrategy::new(Arc::clone(&counts)));
    let mut engine = BacktestEngine::new(config).await?;
    let result = engine.run_with_strategy(strategy).await?;

    let final_portfolio = result
        .final_portfolio
        .as_ref()
        .ok_or("expected a final portfolio from the lifecycle template run")?;
    let counts = counts.lock().expect("lifecycle counts lock poisoned");

    println!("GlowBack strategy lifecycle template completed successfully");
    println!("Hook counts: {:?}", *counts);
    println!("Final equity: ${:.2}", final_portfolio.total_equity);
    println!("Trades recorded: {}", result.trade_log.len());

    if counts.initialize_calls != 1
        || counts.stop_calls != 1
        || counts.market_event_calls == 0
        || counts.order_event_calls == 0
        || counts.day_end_calls == 0
        || result.trade_log.is_empty()
    {
        return Err("strategy lifecycle template did not exercise the expected hook flow".into());
    }

    Ok(())
}
