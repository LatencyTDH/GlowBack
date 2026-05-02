// Core backtesting engine - enhanced implementation
// Provides event-driven backtesting with realistic execution

use chrono::{DateTime, Duration, Utc};
use gb_data::DataManager;
use gb_types::{
    BacktestConfig, BacktestError, BacktestResult, Bar, DataQualityMode, DataValidationSummary,
    EquityCurvePoint, Fill, GbResult, LatencyModel, MarketDataBuffer, MarketEvent, Order,
    Portfolio, ReplayRequestManifest, RunDatasetManifest, RunEngineManifest, RunExecutionManifest,
    RunManifest, RunMetricSnapshot, RunStrategyManifest, Side, SlippageModel, Strategy,
    StrategyContext, StrategyMetrics, Symbol, TradeRecord,
};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::{debug, info, warn};

const STRATEGY_MARKET_DATA_WINDOW: usize = 100;

fn decimal_to_f64(value: Decimal) -> f64 {
    value.to_f64().unwrap_or(0.0)
}

fn execution_commission_bps(settings: &gb_types::ExecutionSettings) -> Option<f64> {
    Some(decimal_to_f64(
        settings.commission_percentage * Decimal::from(10_000),
    ))
}

fn execution_slippage_bps(model: &SlippageModel) -> Option<f64> {
    match model {
        SlippageModel::None => Some(0.0),
        SlippageModel::Fixed { basis_points } | SlippageModel::Linear { basis_points } => {
            Some(*basis_points as f64)
        }
        SlippageModel::VolumeWeighted { min_bps, .. } => Some(*min_bps as f64),
        SlippageModel::SquareRoot { .. } => None,
    }
}

fn execution_latency_ms(model: &LatencyModel) -> Option<u64> {
    match model {
        LatencyModel::None => Some(0),
        LatencyModel::Fixed { milliseconds } => Some(*milliseconds),
        LatencyModel::Random { min_ms, .. } => Some(*min_ms),
        LatencyModel::VenueSpecific { .. } => None,
    }
}

/// Enhanced backtesting engine with event-driven simulation
pub struct Engine {
    config: BacktestConfig,
    portfolio: Portfolio,
    strategy: Box<dyn Strategy>,
    current_time: DateTime<Utc>,
    market_data: HashMap<Symbol, Vec<Bar>>,
    next_bar_indices: HashMap<Symbol, usize>,
    current_market_bars: Vec<(Symbol, Bar)>,
    pending_orders: Vec<Order>,
    strategy_context: StrategyContext,
    strategy_metrics: StrategyMetrics,
    equity_curve: Vec<EquityCurvePoint>,
    trade_log: Vec<TradeRecord>,
    equity_peak: Decimal,
    data_validation_summaries: HashMap<String, DataValidationSummary>,
}

impl Engine {
    /// Create a new engine with strategy and data manager
    pub async fn new(
        config: BacktestConfig,
        data_manager: &mut DataManager,
        strategy: Box<dyn Strategy>,
    ) -> GbResult<Self> {
        info!("Creating enhanced backtesting engine");

        let portfolio = Portfolio::new("backtest_portfolio".to_string(), config.initial_capital);

        let strategy_metrics = StrategyMetrics::new(strategy.get_config().strategy_id.clone());

        // Load market data for all symbols
        let mut market_data = HashMap::new();
        let mut data_validation_summaries = HashMap::new();
        let mut load_failures = Vec::new();
        let mut data_quality_failures = Vec::new();
        for symbol in &config.symbols {
            match data_manager
                .load_data(
                    symbol,
                    config.start_date,
                    config.end_date,
                    config.resolution,
                )
                .await
            {
                Ok(bars) if !bars.is_empty() => {
                    info!("Loaded {} bars for {}", bars.len(), symbol);

                    if let Some(summary) = data_manager
                        .get_validation_summary(symbol, config.resolution)
                        .await?
                    {
                        for warning_message in &summary.warnings {
                            warn!("Data quality warning for {}: {}", symbol, warning_message);
                        }
                        if summary.has_critical_issues {
                            for critical_issue in &summary.critical_issues {
                                warn!(
                                    "Data quality critical issue for {}: {}",
                                    symbol, critical_issue
                                );
                            }
                            if config.data_settings.data_quality_mode == DataQualityMode::Fail {
                                data_quality_failures.push(format!(
                                    "{}: {}",
                                    symbol,
                                    summary.critical_issues.join("; ")
                                ));
                            }
                        }
                        data_validation_summaries.insert(symbol.symbol.clone(), summary);
                    }

                    market_data.insert(symbol.clone(), bars);
                }
                Ok(_) => {
                    let message = format!(
                        "{}: no data available in range {} to {}",
                        symbol,
                        config.start_date.to_rfc3339(),
                        config.end_date.to_rfc3339()
                    );
                    warn!("{}", message);
                    load_failures.push(message);
                }
                Err(e) => {
                    let message = format!("{}: {}", symbol, e);
                    warn!("Failed to load data for {}: {}", symbol, e);
                    load_failures.push(message);
                }
            }
        }

        if !load_failures.is_empty() || !data_quality_failures.is_empty() {
            let mut problems = Vec::new();
            if !load_failures.is_empty() {
                problems.push(format!(
                    "failed to load required market data: {}",
                    load_failures.join("; ")
                ));
            }
            if !data_quality_failures.is_empty() {
                problems.push(format!(
                    "data quality mode '{}' rejected the dataset: {}",
                    match config.data_settings.data_quality_mode {
                        DataQualityMode::Warn => "warn",
                        DataQualityMode::Fail => "fail",
                    },
                    data_quality_failures.join("; ")
                ));
            }

            return Err(BacktestError::EngineInitFailed {
                message: problems.join(" | "),
            }
            .into());
        }

        let mut strategy_context = StrategyContext::new(
            strategy.get_config().strategy_id.clone(),
            config.initial_capital,
        );
        strategy_context.current_time = config.start_date;
        strategy_context.portfolio = portfolio.clone();
        for symbol in market_data.keys() {
            strategy_context.market_data.insert(
                symbol.clone(),
                MarketDataBuffer::new(symbol.clone(), STRATEGY_MARKET_DATA_WINDOW),
            );
        }

        Ok(Self {
            current_time: config.start_date,
            equity_peak: config.initial_capital,
            next_bar_indices: market_data
                .keys()
                .cloned()
                .map(|symbol| (symbol, 0))
                .collect(),
            current_market_bars: Vec::new(),
            config,
            portfolio,
            strategy,
            market_data,
            pending_orders: Vec::new(),
            strategy_context,
            strategy_metrics,
            equity_curve: Vec::new(),
            trade_log: Vec::new(),
            data_validation_summaries,
        })
    }

    /// Run the complete backtesting simulation
    pub async fn run(&mut self) -> GbResult<BacktestResult> {
        info!("Starting enhanced backtesting simulation");

        let mut result = BacktestResult::new(self.config.clone());

        // Initialize strategy
        let mut strategy_config = self.strategy.get_config().clone();

        // Merge backtest-level settings into the strategy config
        if !self.config.symbols.is_empty() {
            strategy_config.symbols = self.config.symbols.clone();
        }
        strategy_config.initial_capital = self.config.initial_capital;
        for (key, value) in &self.config.strategy_config.parameters {
            strategy_config
                .parameters
                .insert(key.clone(), value.clone());
        }

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
        self.strategy_context.current_time = self.current_time;
        self.current_market_bars.clear();

        let current_date = self.current_time.date_naive();
        for symbol in self.config.symbols.clone() {
            let Some(bars) = self.market_data.get(&symbol) else {
                continue;
            };
            let next_index = self.next_bar_indices.entry(symbol.clone()).or_insert(0);

            while let Some(bar) = bars.get(*next_index) {
                let bar_date = bar.timestamp.date_naive();
                if bar_date < current_date {
                    *next_index += 1;
                    continue;
                }
                if bar_date > current_date {
                    break;
                }

                self.current_market_bars.push((symbol.clone(), bar.clone()));
                *next_index += 1;
            }
        }

        for (symbol, bar) in &self.current_market_bars {
            self.strategy_context
                .market_data
                .entry(symbol.clone())
                .or_insert_with(|| {
                    MarketDataBuffer::new(symbol.clone(), STRATEGY_MARKET_DATA_WINDOW)
                })
                .add_event(MarketEvent::Bar(bar.clone()));

            debug!(
                "Market data: {} at {}: {}",
                symbol, bar.timestamp, bar.close
            );
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
                info!(
                    "Executed order: {:?} {} {} at {} (commission {})",
                    order.side, order.quantity, order.symbol, fill.price, fill.commission
                );
                self.trade_log
                    .push(self.trade_record_from_fill(order, &fill));

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

        if !order_events_to_process.is_empty() {
            self.sync_strategy_context_account_state();
        }

        // Notify strategy of order events
        for order_event in order_events_to_process {
            let actions = match self
                .strategy
                .on_order_event(&order_event, &self.strategy_context)
            {
                Ok(actions) => actions,
                Err(e) => {
                    warn!("Strategy on_order_event error: {}", e);
                    continue;
                }
            };

            for action in actions {
                self.process_strategy_action(action)?;
            }
        }

        Ok(())
    }

    fn latency_bar_offset(&self) -> usize {
        let latency_ms = match &self.config.execution_settings.latency_model {
            LatencyModel::None => return 0,
            LatencyModel::Fixed { milliseconds } => *milliseconds,
            LatencyModel::Random { min_ms: _, max_ms } => *max_ms,
            LatencyModel::VenueSpecific { venues } => venues.values().copied().max().unwrap_or(0),
        };

        let Some(seconds_per_bar) = self.config.resolution.to_seconds() else {
            return 0;
        };

        let bar_ms = seconds_per_bar.saturating_mul(1000);
        if bar_ms == 0 || latency_ms == 0 {
            return 0;
        }

        latency_ms.div_ceil(bar_ms) as usize
    }

    fn apply_slippage(&self, base_price: Decimal, side: Side) -> Decimal {
        let price_multiplier = match &self.config.execution_settings.slippage_model {
            SlippageModel::None => return base_price,
            SlippageModel::Fixed { basis_points } | SlippageModel::Linear { basis_points } => {
                Decimal::ONE + Decimal::from(*basis_points) / Decimal::from(10_000)
            }
            SlippageModel::VolumeWeighted { min_bps, max_bps } => {
                let avg_bps = (*min_bps + *max_bps) / 2;
                Decimal::ONE + Decimal::from(avg_bps) / Decimal::from(10_000)
            }
            SlippageModel::SquareRoot { factor } => Decimal::ONE + *factor,
        };

        match side {
            Side::Buy => (base_price * price_multiplier).round_dp(6),
            Side::Sell => (base_price / price_multiplier).round_dp(6),
        }
    }

    fn calculate_commission(&self, quantity: Decimal, execution_price: Decimal) -> Decimal {
        let settings = &self.config.execution_settings;
        let quantity = quantity.abs();
        if quantity == Decimal::ZERO {
            return Decimal::ZERO;
        }

        let per_share = settings.commission_per_share * quantity;
        let gross_notional = quantity * execution_price.abs();
        let percentage = gross_notional * settings.commission_percentage;
        let commission = per_share + percentage;

        if commission > Decimal::ZERO && commission < settings.minimum_commission {
            settings.minimum_commission
        } else {
            commission.round_dp(6)
        }
    }

    fn trade_record_from_fill(&self, order: &Order, fill: &Fill) -> TradeRecord {
        TradeRecord {
            id: order.id,
            symbol: fill.symbol.clone(),
            entry_time: fill.executed_at,
            exit_time: Some(fill.executed_at),
            entry_price: fill.price,
            exit_price: Some(fill.price),
            quantity: fill.quantity,
            side: fill.side,
            pnl: None,
            commission: fill.commission,
            duration_hours: Some(0.0),
            strategy_id: fill.strategy_id.clone(),
            tags: vec!["fill".to_string()],
        }
    }

    /// Try to execute a single order
    async fn try_execute_order(&self, order: &Order) -> GbResult<Option<Fill>> {
        if let Some(bars) = self.market_data.get(&order.symbol) {
            let Some(current_index) = bars
                .iter()
                .position(|bar| bar.timestamp.date_naive() == self.current_time.date_naive())
            else {
                return Ok(None);
            };

            let execution_index = current_index.saturating_add(self.latency_bar_offset());
            let Some(bar) = bars.get(execution_index) else {
                return Ok(None);
            };

            let execution_price = self.apply_slippage(bar.open, order.side);
            let commission = self.calculate_commission(order.quantity, execution_price);

            let fill = Fill::new(
                order.id,
                order.symbol.clone(),
                order.side,
                order.quantity,
                execution_price,
                commission,
                order.strategy_id.clone(),
            );

            return Ok(Some(fill));
        }

        Ok(None)
    }

    /// Update portfolio values with current market prices
    async fn update_portfolio_values(&mut self) -> GbResult<()> {
        let current_prices = self
            .current_market_bars
            .iter()
            .map(|(symbol, bar)| (symbol.clone(), bar.close))
            .collect();

        self.portfolio.update_market_prices(&current_prices);
        self.strategy_context.portfolio = self.portfolio.clone();

        Ok(())
    }

    /// Generate strategy signals by calling the strategy's on_market_event method
    async fn generate_strategy_signals(&mut self) -> GbResult<()> {
        let current_bars_to_process = self.current_market_bars.clone();

        for (symbol, bar) in current_bars_to_process {
            let market_event = MarketEvent::Bar(bar);

            let actions = match self
                .strategy
                .on_market_event(&market_event, &self.strategy_context)
            {
                Ok(actions) => actions,
                Err(e) => {
                    warn!("Strategy error processing {}: {}", symbol, e);
                    continue;
                }
            };

            for action in actions {
                self.process_strategy_action(action)?;
            }
        }

        Ok(())
    }

    fn sync_strategy_context_account_state(&mut self) {
        self.strategy_context.current_time = self.current_time;
        self.strategy_context.portfolio = self.portfolio.clone();
        self.strategy_context.pending_orders = self.pending_orders.clone();
    }

    /// Process a single strategy action
    fn process_strategy_action(&mut self, action: gb_types::StrategyAction) -> GbResult<()> {
        use gb_types::StrategyAction;

        match action {
            StrategyAction::PlaceOrder(order) => {
                debug!(
                    "Strategy placed order: {:?} {} {} at {:?}",
                    order.side, order.quantity, order.symbol, order.order_type
                );
                self.strategy_context.pending_orders.push(order.clone());
                self.pending_orders.push(order);
            }
            StrategyAction::CancelOrder { order_id } => {
                debug!("Strategy cancelled order: {}", order_id);
                self.pending_orders.retain(|o| o.id != order_id);
                self.strategy_context
                    .pending_orders
                    .retain(|o| o.id != order_id);
            }
            StrategyAction::Log { level, message } => match level {
                gb_types::LogLevel::Debug => debug!("[Strategy] {}", message),
                gb_types::LogLevel::Info => info!("[Strategy] {}", message),
                gb_types::LogLevel::Warning => warn!("[Strategy] {}", message),
                gb_types::LogLevel::Error => tracing::error!("[Strategy] {}", message),
            },
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
        let (daily_return, daily_return_opt) =
            if let Some(previous_value) = self.portfolio.daily_returns.last() {
                if previous_value.portfolio_value > Decimal::ZERO {
                    let dr = (total_value - previous_value.portfolio_value)
                        / previous_value.portfolio_value;
                    (dr, Some(dr))
                } else {
                    (Decimal::ZERO, Some(Decimal::ZERO))
                }
            } else {
                (Decimal::ZERO, None)
            };

        self.portfolio
            .add_daily_return(self.current_time, daily_return);

        let positions_value: Decimal = self
            .portfolio
            .positions
            .values()
            .map(|position| position.market_value)
            .sum();

        if total_value > self.equity_peak {
            self.equity_peak = total_value;
        }
        let drawdown = if self.equity_peak > Decimal::ZERO {
            (self.equity_peak - total_value) / self.equity_peak
        } else {
            Decimal::ZERO
        };

        let point = EquityCurvePoint {
            timestamp: self.current_time,
            portfolio_value: total_value,
            cash: self.portfolio.cash,
            positions_value,
            total_pnl: self.portfolio.total_pnl,
            daily_return: daily_return_opt,
            cumulative_return: self.portfolio.get_total_return(),
            drawdown,
        };

        self.equity_curve.push(point);

        Ok(())
    }

    fn build_run_manifest(&self, result: &BacktestResult) -> RunManifest {
        let strategy_config = self.strategy.get_config();
        let symbols = self
            .config
            .symbols
            .iter()
            .map(|symbol| symbol.symbol.clone())
            .collect::<Vec<_>>();
        let bar_counts = self
            .market_data
            .iter()
            .map(|(symbol, bars)| (symbol.symbol.clone(), bars.len()))
            .collect::<HashMap<_, _>>();
        let total_bars = bar_counts.values().sum();
        let execution_settings = &self.config.execution_settings;
        let performance_metrics = result.performance_metrics.as_ref();
        let strategy_metrics = result.strategy_metrics.as_ref();

        RunManifest {
            manifest_version: "1.0".to_string(),
            generated_at: Utc::now(),
            engine: RunEngineManifest {
                crate_name: "gb-engine".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            strategy: RunStrategyManifest {
                strategy_id: strategy_config.strategy_id.clone(),
                name: strategy_config.name.clone(),
                parameters: strategy_config.parameters.clone(),
                code_hash: None,
            },
            dataset: RunDatasetManifest {
                data_source: self.config.data_settings.data_source.clone(),
                resolution: format!("{:?}", self.config.resolution).to_ascii_lowercase(),
                start_date: self.config.start_date,
                end_date: self.config.end_date,
                symbols: symbols.clone(),
                bar_counts,
                total_bars,
                validation_summaries: self.data_validation_summaries.clone(),
            },
            execution: RunExecutionManifest {
                initial_capital: decimal_to_f64(self.config.initial_capital),
                commission_bps: execution_commission_bps(execution_settings),
                slippage_bps: execution_slippage_bps(&execution_settings.slippage_model),
                latency_ms: execution_latency_ms(&execution_settings.latency_model),
                commission_percentage: decimal_to_f64(execution_settings.commission_percentage),
                minimum_commission: decimal_to_f64(execution_settings.minimum_commission),
                slippage_model: execution_settings.slippage_model.clone(),
                latency_model: execution_settings.latency_model.clone(),
                market_impact_model: execution_settings.market_impact_model.clone(),
                data_settings: self.config.data_settings.clone(),
            },
            replay_request: ReplayRequestManifest {
                symbols,
                start_date: self.config.start_date,
                end_date: self.config.end_date,
                resolution: format!("{:?}", self.config.resolution).to_ascii_lowercase(),
                strategy_name: strategy_config.strategy_id.clone(),
                strategy_params: strategy_config.parameters.clone(),
                initial_capital: decimal_to_f64(self.config.initial_capital),
                data_source: self.config.data_settings.data_source.clone(),
                commission_bps: execution_commission_bps(execution_settings),
                slippage_bps: execution_slippage_bps(&execution_settings.slippage_model),
                latency_ms: execution_latency_ms(&execution_settings.latency_model),
                run_name: Some(self.config.name.clone()),
            },
            metric_snapshot: RunMetricSnapshot {
                final_value: decimal_to_f64(self.portfolio.total_equity),
                total_return: performance_metrics
                    .map(|metrics| decimal_to_f64(metrics.total_return) * 100.0)
                    .unwrap_or_else(|| decimal_to_f64(self.portfolio.get_total_return()) * 100.0),
                max_drawdown: performance_metrics
                    .map(|metrics| decimal_to_f64(metrics.max_drawdown) * 100.0)
                    .unwrap_or_else(|| decimal_to_f64(self.strategy_metrics.max_drawdown) * 100.0),
                sharpe_ratio: performance_metrics
                    .and_then(|metrics| metrics.sharpe_ratio)
                    .map(decimal_to_f64)
                    .or_else(|| {
                        strategy_metrics
                            .and_then(|metrics| metrics.sharpe_ratio)
                            .map(decimal_to_f64)
                    })
                    .unwrap_or(0.0),
                total_trades: strategy_metrics
                    .map(|metrics| metrics.total_trades)
                    .unwrap_or(self.trade_log.len() as u64),
            },
        }
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
            let annualized = (1.0 + return_decimal).powf(1.0 / years) - 1.0;
            self.strategy_metrics.annualized_return =
                Decimal::try_from(annualized).unwrap_or_default();
        }

        // Calculate volatility from daily returns
        let daily_returns: Vec<f64> = self
            .portfolio
            .daily_returns
            .iter()
            .map(|dr| dr.daily_return.try_into().unwrap_or(0.0))
            .collect();

        if daily_returns.len() > 1 {
            let mean: f64 = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;
            let variance: f64 = daily_returns
                .iter()
                .map(|r| (r - mean).powi(2))
                .sum::<f64>()
                / (daily_returns.len() - 1) as f64;
            let daily_vol = variance.sqrt();
            let annualized_vol = daily_vol * (252.0_f64).sqrt(); // Annualize assuming 252 trading days
            self.strategy_metrics.volatility =
                Decimal::try_from(annualized_vol).unwrap_or_default();

            // Calculate Sharpe ratio (assuming risk-free rate of 0 for simplicity)
            if annualized_vol > 0.0 {
                let annualized_return: f64 = self
                    .strategy_metrics
                    .annualized_return
                    .try_into()
                    .unwrap_or(0.0);
                let sharpe = annualized_return / annualized_vol;
                self.strategy_metrics.sharpe_ratio =
                    Some(Decimal::try_from(sharpe).unwrap_or_default());
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
        result.equity_curve = self.equity_curve.clone();
        result.trade_log = self.trade_log.clone();
        result.performance_metrics = Some(gb_types::PerformanceMetrics::calculate_with_trades(
            &self.portfolio,
            &self.trade_log,
        ));
        result.metadata.insert(
            "data_validation_summaries".to_string(),
            serde_json::to_value(&self.data_validation_summaries)?,
        );
        result.metadata.insert(
            "data_quality_mode".to_string(),
            serde_json::to_value(&self.config.data_settings.data_quality_mode)?,
        );
        result.metadata.insert(
            "sample_data".to_string(),
            serde_json::json!(self
                .data_validation_summaries
                .values()
                .any(|summary| summary.sample_data)),
        );
        result.manifest = Some(self.build_run_manifest(result));

        info!("Final portfolio value: {}", self.portfolio.total_equity);
        info!("Total return: {:.2}%", total_return * Decimal::from(100));
        info!(
            "Annualized volatility: {:.2}%",
            self.strategy_metrics.volatility * Decimal::from(100)
        );
        info!("Max drawdown: {:.2}%", max_dd * Decimal::from(100));
        info!("Total trades: {}", self.strategy_metrics.total_trades);
        if self.strategy_metrics.total_trades > 0 {
            info!(
                "Win rate: {:.2}%",
                self.strategy_metrics.win_rate * Decimal::from(100)
            );
        }
        if let Some(sharpe) = self.strategy_metrics.sharpe_ratio {
            info!("Sharpe ratio: {:.2}", sharpe);
        }

        Ok(())
    }

    /// Call strategy's on_day_end method for end-of-day processing
    async fn call_strategy_day_end(&mut self) -> GbResult<()> {
        self.sync_strategy_context_account_state();

        let actions = match self.strategy.on_day_end(&self.strategy_context) {
            Ok(actions) => actions,
            Err(e) => {
                warn!("Strategy on_day_end error: {}", e);
                return Ok(());
            }
        };

        for action in actions {
            self.process_strategy_action(action)?;
        }

        Ok(())
    }

    /// Call strategy's on_stop method for cleanup
    async fn call_strategy_stop(&mut self) -> GbResult<()> {
        self.sync_strategy_context_account_state();

        let actions = match self.strategy.on_stop(&self.strategy_context) {
            Ok(actions) => actions,
            Err(e) => {
                warn!("Strategy on_stop error: {}", e);
                info!("Strategy stopped");
                return Ok(());
            }
        };

        for action in actions {
            self.process_strategy_action(action)?;
        }

        info!("Strategy stopped");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use gb_types::{
        DataQualityMode, DataValidationSummary, DatasetKind, OrderEvent, PriceAdjustmentMode,
        Resolution, Side, StrategyAction, StrategyConfig,
    };

    #[derive(Debug, Clone)]
    struct NoopStrategy {
        config: StrategyConfig,
    }

    impl NoopStrategy {
        fn new() -> Self {
            Self {
                config: StrategyConfig::new("noop".to_string(), "Noop Strategy".to_string()),
            }
        }
    }

    impl Strategy for NoopStrategy {
        fn initialize(&mut self, config: &StrategyConfig) -> Result<(), String> {
            self.config = config.clone();
            Ok(())
        }

        fn on_market_event(
            &mut self,
            _event: &MarketEvent,
            _context: &StrategyContext,
        ) -> Result<Vec<StrategyAction>, String> {
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

    fn ts(day: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2024, 1, day, 0, 0, 0).unwrap()
    }

    fn test_bar(symbol: &Symbol, day: u32, price: i64) -> Bar {
        let price = Decimal::from(price);
        Bar::new(
            symbol.clone(),
            ts(day),
            price,
            price,
            price,
            price,
            Decimal::from(1_000),
            Resolution::Day,
        )
    }

    fn test_engine(symbol: Symbol, bars: Vec<Bar>) -> Engine {
        let mut config = BacktestConfig::new(
            "engine-test".to_string(),
            StrategyConfig::new("noop".to_string(), "Noop Strategy".to_string()),
        );
        config.start_date = ts(1);
        config.end_date = ts(5);
        config.initial_capital = Decimal::from(100_000);
        config.resolution = Resolution::Day;
        config.symbols = vec![symbol.clone()];

        let portfolio = Portfolio::new("test-account".to_string(), config.initial_capital);
        let mut strategy_context = StrategyContext::new("noop".to_string(), config.initial_capital);
        strategy_context.current_time = config.start_date;
        strategy_context.portfolio = portfolio.clone();
        strategy_context.market_data.insert(
            symbol.clone(),
            MarketDataBuffer::new(symbol.clone(), STRATEGY_MARKET_DATA_WINDOW),
        );

        Engine {
            config,
            portfolio,
            strategy: Box::new(NoopStrategy::new()),
            current_time: ts(1),
            market_data: HashMap::from([(symbol.clone(), bars)]),
            next_bar_indices: HashMap::from([(symbol.clone(), 0)]),
            current_market_bars: Vec::new(),
            pending_orders: Vec::new(),
            strategy_context,
            strategy_metrics: StrategyMetrics::new("noop".to_string()),
            equity_curve: Vec::new(),
            trade_log: Vec::new(),
            equity_peak: Decimal::from(100_000),
            data_validation_summaries: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn process_market_data_updates_context_incrementally() {
        let symbol = Symbol::equity("AAPL");
        let mut engine = test_engine(
            symbol.clone(),
            vec![
                test_bar(&symbol, 1, 101),
                test_bar(&symbol, 2, 102),
                test_bar(&symbol, 4, 104),
            ],
        );

        engine.process_market_data().await.unwrap();
        assert_eq!(engine.current_market_bars.len(), 1);
        assert_eq!(engine.next_bar_indices[&symbol], 1);
        let buffer = engine.strategy_context.market_data.get(&symbol).unwrap();
        assert_eq!(buffer.data.len(), 1);
        assert_eq!(buffer.get_current_price(), Some(Decimal::from(101)));

        engine.current_time = ts(2);
        engine.process_market_data().await.unwrap();
        assert_eq!(engine.current_market_bars.len(), 1);
        assert_eq!(engine.next_bar_indices[&symbol], 2);
        let buffer = engine.strategy_context.market_data.get(&symbol).unwrap();
        assert_eq!(buffer.data.len(), 2);
        assert_eq!(buffer.get_current_price(), Some(Decimal::from(102)));

        engine.current_time = ts(3);
        engine.process_market_data().await.unwrap();
        assert!(engine.current_market_bars.is_empty());
        assert_eq!(engine.next_bar_indices[&symbol], 2);
        let buffer = engine.strategy_context.market_data.get(&symbol).unwrap();
        assert_eq!(buffer.data.len(), 2);
        assert_eq!(buffer.get_current_price(), Some(Decimal::from(102)));

        engine.current_time = ts(4);
        engine.process_market_data().await.unwrap();
        assert_eq!(engine.current_market_bars.len(), 1);
        assert_eq!(engine.next_bar_indices[&symbol], 3);
        let buffer = engine.strategy_context.market_data.get(&symbol).unwrap();
        assert_eq!(buffer.data.len(), 3);
        assert_eq!(buffer.get_current_price(), Some(Decimal::from(104)));
    }

    #[test]
    fn process_strategy_action_keeps_pending_order_snapshots_in_sync() {
        let symbol = Symbol::equity("AAPL");
        let mut engine = test_engine(symbol.clone(), Vec::new());
        let order = Order::market_order(symbol, Side::Buy, Decimal::from(5), "noop".to_string());
        let order_id = order.id;

        engine
            .process_strategy_action(StrategyAction::PlaceOrder(order.clone()))
            .unwrap();
        assert_eq!(engine.pending_orders.len(), 1);
        assert_eq!(engine.strategy_context.pending_orders.len(), 1);
        assert_eq!(engine.strategy_context.pending_orders[0].id, order_id);

        engine
            .process_strategy_action(StrategyAction::CancelOrder { order_id })
            .unwrap();
        assert!(engine.pending_orders.is_empty());
        assert!(engine.strategy_context.pending_orders.is_empty());
    }

    #[tokio::test]
    async fn engine_rejects_critical_data_quality_failures_in_fail_mode() {
        let symbol = Symbol::equity("AAPL");
        let mut config = BacktestConfig::new(
            "strict-data-quality".to_string(),
            StrategyConfig::new("noop".to_string(), "Noop Strategy".to_string()),
        );
        config.start_date = ts(1);
        config.end_date = ts(2);
        config.symbols = vec![symbol.clone()];
        config.resolution = Resolution::Day;
        config.data_settings.data_quality_mode = DataQualityMode::Fail;

        let mut data_manager = DataManager::new_ephemeral("gb-engine-strict-data-quality")
            .await
            .unwrap();
        data_manager
            .storage
            .save_bars(&symbol, &[test_bar(&symbol, 1, 101)], Resolution::Day)
            .await
            .unwrap();

        let summary = DataValidationSummary {
            total_rows_seen: 2,
            total_bars: 1,
            duplicate_timestamps: 1,
            missing_intervals: 0,
            invalid_ohlcv_rows: 0,
            negative_price_rows: 0,
            negative_volume_rows: 0,
            has_critical_issues: true,
            critical_issue_count: 1,
            warning_issue_count: 0,
            timezone: "UTC".to_string(),
            resolution: "1d".to_string(),
            dataset_kind: DatasetKind::UserProvided,
            price_adjustment: PriceAdjustmentMode::Raw,
            sample_data: false,
            critical_issues: vec![
                "Detected 1 duplicate timestamp rows for NASDAQ:AAPL.".to_string()
            ],
            warnings: Vec::new(),
        };
        data_manager
            .catalog
            .register_symbol_data(
                &symbol,
                ts(1),
                ts(1),
                Resolution::Day,
                1,
                DatasetKind::UserProvided,
                PriceAdjustmentMode::Raw,
                Some(&summary),
            )
            .await
            .unwrap();

        let result = Engine::new(config, &mut data_manager, Box::new(NoopStrategy::new())).await;
        assert!(result.is_err());
        let error = result.err().unwrap().to_string();
        assert!(error.contains("data quality mode 'fail' rejected the dataset"));
        assert!(error.contains("duplicate timestamp"));
    }
}
