use num_traits::cast::ToPrimitive;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

use gb_engine::BacktestEngine as RustBacktestEngine;
use gb_types::{
    BacktestConfig, BacktestResult as RustBacktestResult, BuyAndHoldStrategy, LatencyModel,
    MeanReversionStrategy, MomentumStrategy, MovingAverageCrossoverStrategy, Resolution,
    RsiStrategy, SlippageModel, Strategy, StrategyConfig, Symbol,
};

/// GlowBack Python module
#[pymodule]
fn glowback(_py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    m.add("__version__", "0.1.0")?;
    m.add_class::<PySymbol>()?;
    m.add_class::<PyDataManager>()?;
    m.add_class::<PyBar>()?;
    m.add_class::<PyCatalogStats>()?;
    m.add_class::<PyBacktestEngine>()?;
    m.add_class::<PyBacktestResult>()?;
    m.add_function(wrap_pyfunction!(run_buy_and_hold, m)?)?;
    m.add_function(wrap_pyfunction!(run_builtin_strategy, m)?)?;

    // Backwards-compatible aliases (Py* names)
    m.add("PySymbol", m.getattr("Symbol")?)?;
    m.add("PyDataManager", m.getattr("DataManager")?)?;
    m.add("PyBar", m.getattr("Bar")?)?;
    m.add("PyCatalogStats", m.getattr("CatalogStats")?)?;
    m.add("PyBacktestEngine", m.getattr("BacktestEngine")?)?;
    m.add("PyBacktestResult", m.getattr("BacktestResult")?)?;
    Ok(())
}

fn decimal_to_f64(value: Decimal) -> f64 {
    value.to_f64().unwrap_or(0.0)
}

fn parse_resolution(resolution: &str) -> PyResult<Resolution> {
    match resolution.to_lowercase().as_str() {
        "minute" | "1m" => Ok(Resolution::Minute),
        "hour" | "1h" => Ok(Resolution::Hour),
        "day" | "1d" => Ok(Resolution::Day),
        _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Invalid resolution: {}",
            resolution
        ))),
    }
}

fn dict_get_usize(params: Option<&Bound<PyDict>>, key: &str, default: usize) -> PyResult<usize> {
    let Some(params) = params else {
        return Ok(default);
    };
    let Some(value) = params.get_item(key)? else {
        return Ok(default);
    };
    if let Ok(extracted) = value.extract::<usize>() {
        return Ok(extracted);
    }
    Ok(value.extract::<f64>()?.round() as usize)
}

fn dict_get_f64(params: Option<&Bound<PyDict>>, key: &str, default: f64) -> PyResult<f64> {
    let Some(params) = params else {
        return Ok(default);
    };
    let Some(value) = params.get_item(key)? else {
        return Ok(default);
    };
    value.extract::<f64>()
}

fn build_strategy(name: &str, params: Option<&Bound<PyDict>>) -> PyResult<Box<dyn Strategy>> {
    match name.trim().to_ascii_lowercase().as_str() {
        "buy_and_hold" => Ok(Box::new(BuyAndHoldStrategy::new())),
        "ma_crossover" | "moving_average_crossover" => {
            let short_period = dict_get_usize(params, "short_period", 10)?;
            let long_period = dict_get_usize(params, "long_period", 20)?;
            Ok(Box::new(MovingAverageCrossoverStrategy::new(
                short_period,
                long_period,
            )))
        }
        "momentum" => {
            let lookback_period = dict_get_usize(params, "lookback_period", 10)?;
            let momentum_threshold = dict_get_f64(params, "momentum_threshold", 0.05)?;
            Ok(Box::new(MomentumStrategy::new(
                lookback_period,
                momentum_threshold,
            )))
        }
        "mean_reversion" => {
            let lookback_period = dict_get_usize(params, "lookback_period", 20)?;
            let entry_threshold = dict_get_f64(params, "entry_threshold", 2.0)?;
            let exit_threshold = dict_get_f64(params, "exit_threshold", 1.0)?;
            Ok(Box::new(MeanReversionStrategy::new(
                lookback_period,
                entry_threshold,
                exit_threshold,
            )))
        }
        "rsi" => {
            let lookback_period = dict_get_usize(params, "lookback_period", 14)?;
            let oversold_threshold = dict_get_f64(params, "oversold_threshold", 30.0)?;
            let overbought_threshold = dict_get_f64(params, "overbought_threshold", 70.0)?;
            Ok(Box::new(RsiStrategy::new(
                lookback_period,
                oversold_threshold,
                overbought_threshold,
            )))
        }
        other => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Unsupported strategy: {}",
            other
        ))),
    }
}

#[pyfunction]
#[pyo3(signature = (symbols, start_date, end_date, resolution=None, initial_capital=None, name=None))]
fn run_buy_and_hold(
    symbols: Vec<String>,
    start_date: &str,
    end_date: &str,
    resolution: Option<&str>,
    initial_capital: Option<f64>,
    name: Option<&str>,
) -> PyResult<PyBacktestResult> {
    let mut engine = PyBacktestEngine::new(
        symbols,
        start_date,
        end_date,
        resolution,
        initial_capital,
        name,
        None,
        None,
        None,
        Some("sample"),
        None,
    )?;
    engine.run_buy_and_hold()
}

fn build_backtest_config(
    symbols: Vec<String>,
    start_date: &str,
    end_date: &str,
    resolution: Option<&str>,
    initial_capital: Option<f64>,
    name: Option<&str>,
    data_source: Option<&str>,
    commission_bps: Option<f64>,
    slippage_bps: Option<f64>,
    latency_ms: Option<u64>,
    strategy_config: StrategyConfig,
) -> PyResult<BacktestConfig> {
    if symbols.is_empty() {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "symbols list cannot be empty",
        ));
    }

    let start_date = chrono::DateTime::parse_from_rfc3339(start_date)
        .map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid start_date format: {}", e))
        })?
        .with_timezone(&chrono::Utc);

    let end_date = chrono::DateTime::parse_from_rfc3339(end_date)
        .map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid end_date format: {}", e))
        })?
        .with_timezone(&chrono::Utc);

    let resolution = parse_resolution(resolution.unwrap_or("day"))?;
    let initial_capital = Decimal::from_f64(initial_capital.unwrap_or(100_000.0))
        .unwrap_or_else(|| Decimal::from(100_000));

    let rust_symbols: Vec<Symbol> = symbols
        .iter()
        .map(|symbol| Symbol::equity(symbol))
        .collect();

    let mut strategy_config = strategy_config;
    strategy_config.symbols = rust_symbols.clone();
    strategy_config.initial_capital = initial_capital;

    let mut config = BacktestConfig::new(
        name.unwrap_or("Python Backtest").to_string(),
        strategy_config.clone(),
    );
    config.start_date = start_date;
    config.end_date = end_date;
    config.initial_capital = initial_capital;
    config.resolution = resolution;
    config.symbols = rust_symbols;
    config.strategy_config = strategy_config;

    if let Some(source) = data_source {
        if !source.trim().is_empty() {
            config.data_settings.data_source = source.trim().to_string();
        }
    }

    if let Some(commission_bps) = commission_bps {
        let commission_fraction = (commission_bps.max(0.0)) / 10_000.0;
        if let Some(decimal) = Decimal::from_f64(commission_fraction) {
            config.execution_settings.commission_percentage = decimal;
        }
    }

    if let Some(slippage_bps) = slippage_bps {
        config.execution_settings.slippage_model = SlippageModel::Linear {
            basis_points: slippage_bps.max(0.0).round() as u32,
        };
    }

    if let Some(latency_ms) = latency_ms {
        config.execution_settings.latency_model = LatencyModel::Fixed {
            milliseconds: latency_ms,
        };
    }

    Ok(config)
}

fn apply_strategy_params(
    strategy_config: &mut StrategyConfig,
    strategy_params: Option<&Bound<'_, PyDict>>,
) -> PyResult<()> {
    let Some(strategy_params) = strategy_params else {
        return Ok(());
    };

    for (key, value) in strategy_params.iter() {
        let key: String = key.extract()?;

        if let Ok(extracted) = value.extract::<bool>() {
            strategy_config.set_parameter(&key, extracted);
            continue;
        }
        if let Ok(extracted) = value.extract::<i64>() {
            strategy_config.set_parameter(&key, extracted);
            continue;
        }
        if let Ok(extracted) = value.extract::<f64>() {
            strategy_config.set_parameter(&key, extracted);
            continue;
        }
        if let Ok(extracted) = value.extract::<String>() {
            strategy_config.set_parameter(&key, extracted);
            continue;
        }
        if let Ok(extracted) = value.extract::<Vec<i64>>() {
            strategy_config.set_parameter(&key, extracted);
            continue;
        }
        if let Ok(extracted) = value.extract::<Vec<f64>>() {
            strategy_config.set_parameter(&key, extracted);
            continue;
        }
        if let Ok(extracted) = value.extract::<Vec<String>>() {
            strategy_config.set_parameter(&key, extracted);
            continue;
        }
        if let Ok(extracted) = value.extract::<Vec<bool>>() {
            strategy_config.set_parameter(&key, extracted);
            continue;
        }

        return Err(pyo3::exceptions::PyTypeError::new_err(format!(
            "Unsupported parameter type for strategy parameter '{}'",
            key
        )));
    }

    Ok(())
}

fn build_builtin_strategy(
    strategy_name: &str,
    strategy_config: &StrategyConfig,
) -> PyResult<Box<dyn Strategy>> {
    match strategy_name.trim().to_lowercase().as_str() {
        "buy_and_hold" => Ok(Box::new(BuyAndHoldStrategy::new())),
        "ma_crossover" | "moving_average_crossover" => {
            let short_period = strategy_config.get_parameter("short_period").unwrap_or(10);
            let long_period = strategy_config.get_parameter("long_period").unwrap_or(20);
            Ok(Box::new(MovingAverageCrossoverStrategy::new(
                short_period,
                long_period,
            )))
        }
        "momentum" => {
            let lookback_period = strategy_config
                .get_parameter("lookback_period")
                .unwrap_or(10);
            let momentum_threshold = strategy_config
                .get_parameter("momentum_threshold")
                .unwrap_or(5.0f64);
            Ok(Box::new(MomentumStrategy::new(
                lookback_period,
                momentum_threshold,
            )))
        }
        "mean_reversion" => {
            let lookback_period = strategy_config
                .get_parameter("lookback_period")
                .unwrap_or(20);
            let entry_threshold = strategy_config
                .get_parameter("entry_threshold")
                .unwrap_or(2.0f64);
            let exit_threshold = strategy_config
                .get_parameter("exit_threshold")
                .unwrap_or(1.0f64);
            Ok(Box::new(MeanReversionStrategy::new(
                lookback_period,
                entry_threshold,
                exit_threshold,
            )))
        }
        "rsi" => {
            let lookback_period = strategy_config
                .get_parameter("lookback_period")
                .unwrap_or(14);
            let oversold_threshold = strategy_config
                .get_parameter("oversold_threshold")
                .unwrap_or(30.0f64);
            let overbought_threshold = strategy_config
                .get_parameter("overbought_threshold")
                .unwrap_or(70.0f64);
            Ok(Box::new(RsiStrategy::new(
                lookback_period,
                oversold_threshold,
                overbought_threshold,
            )))
        }
        other => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Unsupported built-in strategy: {}",
            other
        ))),
    }
}

#[pyfunction]
#[pyo3(signature = (symbols, start_date, end_date, strategy_name, strategy_params=None, resolution=None, initial_capital=None, name=None, data_source=None, commission_bps=None, slippage_bps=None, latency_ms=None))]
fn run_builtin_strategy(
    symbols: Vec<String>,
    start_date: &str,
    end_date: &str,
    strategy_name: &str,
    strategy_params: Option<&Bound<'_, PyDict>>,
    resolution: Option<&str>,
    initial_capital: Option<f64>,
    name: Option<&str>,
    data_source: Option<&str>,
    commission_bps: Option<f64>,
    slippage_bps: Option<f64>,
    latency_ms: Option<u64>,
) -> PyResult<PyBacktestResult> {
    let strategy_name = strategy_name.trim().to_lowercase();
    let mut strategy_config = StrategyConfig::new(strategy_name.clone(), strategy_name.clone());
    apply_strategy_params(&mut strategy_config, strategy_params)?;

    let config = build_backtest_config(
        symbols,
        start_date,
        end_date,
        resolution,
        initial_capital,
        name,
        data_source,
        commission_bps,
        slippage_bps,
        latency_ms,
        strategy_config.clone(),
    )?;

    let strategy = build_builtin_strategy(&strategy_name, &strategy_config)?;

    let runtime = tokio::runtime::Runtime::new().map_err(|e| {
        pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create async runtime: {}", e))
    })?;

    let mut engine = runtime
        .block_on(async { RustBacktestEngine::new(config).await })
        .map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to create backtest engine: {}",
                e
            ))
        })?;

    let result = runtime
        .block_on(async { engine.run_with_strategy(strategy).await })
        .map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Backtest execution failed: {}", e))
        })?;

    Ok(PyBacktestResult::from_backtest_result(result))
}

/// Python wrapper for Symbol
#[pyclass(name = "Symbol")]
struct PySymbol {
    inner: gb_types::Symbol,
}

#[pymethods]
impl PySymbol {
    #[new]
    fn new(symbol: &str, exchange: &str, asset_class: &str) -> Self {
        let asset_class = match asset_class.to_lowercase().as_str() {
            "crypto" => gb_types::AssetClass::Crypto,
            "forex" => gb_types::AssetClass::Forex,
            "commodity" => gb_types::AssetClass::Commodity,
            "bond" => gb_types::AssetClass::Bond,
            _ => gb_types::AssetClass::Equity,
        };

        Self {
            inner: gb_types::Symbol::new(symbol, exchange, asset_class),
        }
    }

    #[getter]
    fn symbol(&self) -> String {
        self.inner.symbol.clone()
    }

    #[getter]
    fn exchange(&self) -> String {
        self.inner.exchange.clone()
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!(
            "Symbol(symbol='{}', exchange='{}', asset_class='{:?}')",
            self.inner.symbol, self.inner.exchange, self.inner.asset_class
        )
    }
}

/// Python wrapper for DataManager
#[pyclass(name = "DataManager")]
struct PyDataManager {
    inner: std::sync::Mutex<gb_data::DataManager>,
    runtime: tokio::runtime::Runtime,
}

#[pymethods]
impl PyDataManager {
    #[new]
    fn new() -> PyResult<Self> {
        // Create tokio runtime for async operations
        let runtime = tokio::runtime::Runtime::new().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to create async runtime: {}",
                e
            ))
        })?;

        // Create data manager
        let inner = runtime
            .block_on(async { gb_data::DataManager::new().await })
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "Failed to create data manager: {}",
                    e
                ))
            })?;

        Ok(Self {
            inner: std::sync::Mutex::new(inner),
            runtime,
        })
    }

    /// Load market data for a symbol
    fn load_data(
        &mut self,
        symbol: &PySymbol,
        start_date: &str,
        end_date: &str,
        resolution: &str,
    ) -> PyResult<Vec<PyBar>> {
        // Parse dates
        let start_date = chrono::DateTime::parse_from_rfc3339(start_date)
            .map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Invalid start_date format: {}", e))
            })?
            .with_timezone(&chrono::Utc);

        let end_date = chrono::DateTime::parse_from_rfc3339(end_date)
            .map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Invalid end_date format: {}", e))
            })?
            .with_timezone(&chrono::Utc);

        // Parse resolution
        let resolution = match resolution.to_lowercase().as_str() {
            "minute" | "1m" => gb_types::Resolution::Minute,
            "hour" | "1h" => gb_types::Resolution::Hour,
            "day" | "1d" => gb_types::Resolution::Day,
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Invalid resolution: {}",
                    resolution
                )))
            }
        };

        // Load data asynchronously
        let bars = self.runtime.block_on(async {
            let mut inner = self.inner.lock().map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire lock: {}", e))
            })?;
            inner
                .load_data(&symbol.inner, start_date, end_date, resolution)
                .await
                .map_err(|e| {
                    pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to load data: {}", e))
                })
        })?;

        // Convert to Python bars
        let py_bars = bars.into_iter().map(|bar| PyBar { inner: bar }).collect();
        Ok(py_bars)
    }

    /// Add a sample data provider
    fn add_sample_provider(&mut self) -> PyResult<()> {
        let provider = Box::new(gb_data::SampleDataProvider::new());
        let mut inner = self.inner.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire lock: {}", e))
        })?;
        inner.add_provider(provider);
        Ok(())
    }

    /// Add a CSV data provider
    fn add_csv_provider(&mut self, base_path: &str) -> PyResult<()> {
        let provider = Box::new(gb_data::CsvDataProvider::new(base_path));
        let mut inner = self.inner.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire lock: {}", e))
        })?;
        inner.add_provider(provider);
        Ok(())
    }

    /// Add an Alpha Vantage provider
    fn add_alpha_vantage_provider(&mut self, api_key: &str) -> PyResult<()> {
        let provider = Box::new(gb_data::AlphaVantageProvider::new(api_key.to_string()));
        let mut inner = self.inner.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire lock: {}", e))
        })?;
        inner.add_provider(provider);
        Ok(())
    }

    /// Get catalog statistics
    fn get_catalog_stats(&self) -> PyResult<PyCatalogStats> {
        let stats = self.runtime.block_on(async {
            let inner = self.inner.lock().map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire lock: {}", e))
            })?;
            inner.catalog.get_catalog_stats().await.map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "Failed to get catalog stats: {}",
                    e
                ))
            })
        })?;

        Ok(PyCatalogStats {
            total_symbols: stats.total_symbols as usize,
            total_records: stats.total_records,
            date_range_start: stats.earliest_date.map(|d| d.to_rfc3339()),
            date_range_end: stats.latest_date.map(|d| d.to_rfc3339()),
        })
    }

    /// Get number of configured data providers
    fn get_provider_count(&self) -> PyResult<usize> {
        let inner = self.inner.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire lock: {}", e))
        })?;
        Ok(inner.providers.len())
    }
}

/// Python wrapper for Bar (OHLCV data)
#[pyclass(name = "Bar")]
struct PyBar {
    inner: gb_types::Bar,
}

#[pymethods]
impl PyBar {
    #[getter]
    fn symbol(&self) -> PySymbol {
        PySymbol {
            inner: self.inner.symbol.clone(),
        }
    }

    #[getter]
    fn timestamp(&self) -> String {
        self.inner.timestamp.to_rfc3339()
    }

    #[getter]
    fn open(&self) -> f64 {
        self.inner.open.to_f64().unwrap_or(0.0)
    }

    #[getter]
    fn high(&self) -> f64 {
        self.inner.high.to_f64().unwrap_or(0.0)
    }

    #[getter]
    fn low(&self) -> f64 {
        self.inner.low.to_f64().unwrap_or(0.0)
    }

    #[getter]
    fn close(&self) -> f64 {
        self.inner.close.to_f64().unwrap_or(0.0)
    }

    #[getter]
    fn volume(&self) -> f64 {
        self.inner.volume.to_f64().unwrap_or(0.0)
    }

    #[getter]
    fn resolution(&self) -> String {
        format!("{:?}", self.inner.resolution)
    }

    fn __str__(&self) -> String {
        format!(
            "Bar({} {} O:{} H:{} L:{} C:{} V:{})",
            self.inner.symbol,
            self.inner.timestamp.format("%Y-%m-%d %H:%M:%S"),
            self.inner.open,
            self.inner.high,
            self.inner.low,
            self.inner.close,
            self.inner.volume
        )
    }

    fn __repr__(&self) -> String {
        format!(
            "PyBar(symbol='{}', timestamp='{}', open={}, high={}, low={}, close={}, volume={})",
            self.inner.symbol,
            self.inner.timestamp.to_rfc3339(),
            self.inner.open,
            self.inner.high,
            self.inner.low,
            self.inner.close,
            self.inner.volume
        )
    }
}

/// Python wrapper for catalog statistics
#[pyclass(name = "CatalogStats")]
struct PyCatalogStats {
    #[pyo3(get)]
    total_symbols: usize,
    #[pyo3(get)]
    total_records: u64,
    date_range_start: Option<String>,
    date_range_end: Option<String>,
}

#[pymethods]
impl PyCatalogStats {
    #[getter]
    fn date_range_start(&self) -> Option<String> {
        self.date_range_start.clone()
    }

    #[getter]
    fn date_range_end(&self) -> Option<String> {
        self.date_range_end.clone()
    }

    fn __str__(&self) -> String {
        format!(
            "CatalogStats(symbols: {}, records: {}, date_range: {} to {})",
            self.total_symbols,
            self.total_records,
            self.date_range_start
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("None"),
            self.date_range_end
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("None")
        )
    }

    fn __repr__(&self) -> String {
        format!("PyCatalogStats(total_symbols={}, total_records={}, date_range_start={:?}, date_range_end={:?})",
            self.total_symbols,
            self.total_records,
            self.date_range_start,
            self.date_range_end
        )
    }
}

struct EquityPoint {
    timestamp: String,
    value: f64,
    cash: f64,
    positions: f64,
    total_pnl: f64,
    daily_return: Option<f64>,
    returns: f64,
    drawdown: f64,
}

struct TradePoint {
    timestamp: String,
    symbol: String,
    action: String,
    shares: f64,
    price: f64,
    commission: f64,
    pnl: Option<f64>,
    strategy_id: String,
}

struct ExposurePoint {
    timestamp: String,
    cash_pct: f64,
    positions_pct: f64,
    gross_exposure_pct: f64,
    net_exposure_pct: f64,
}

#[pyclass(name = "BacktestResult")]
struct PyBacktestResult {
    metrics_summary: std::collections::HashMap<String, f64>,
    equity_curve: Vec<EquityPoint>,
    trades: Vec<TradePoint>,
    exposures: Vec<ExposurePoint>,
    logs: Vec<String>,
    final_cash: f64,
    final_positions: std::collections::HashMap<String, f64>,
    manifest: Option<serde_json::Value>,
}

impl PyBacktestResult {
    fn from_backtest_result(result: RustBacktestResult) -> Self {
        let mut metrics_summary = std::collections::HashMap::new();
        metrics_summary.insert(
            "initial_capital".to_string(),
            decimal_to_f64(result.config.initial_capital),
        );

        let (final_cash, final_positions) = if let Some(portfolio) = result.final_portfolio.as_ref()
        {
            metrics_summary.insert(
                "final_value".to_string(),
                decimal_to_f64(portfolio.total_equity),
            );
            (
                decimal_to_f64(portfolio.cash),
                portfolio
                    .positions
                    .iter()
                    .map(|(symbol, position)| {
                        (symbol.symbol.clone(), decimal_to_f64(position.quantity))
                    })
                    .collect::<std::collections::HashMap<_, _>>(),
            )
        } else {
            (0.0, std::collections::HashMap::new())
        };

        if let Some(performance) = result.performance_metrics.as_ref() {
            metrics_summary.insert(
                "total_return".to_string(),
                decimal_to_f64(performance.total_return) * 100.0,
            );
            metrics_summary.insert(
                "annualized_return".to_string(),
                decimal_to_f64(performance.annualized_return) * 100.0,
            );
            metrics_summary.insert(
                "volatility".to_string(),
                decimal_to_f64(performance.volatility) * 100.0,
            );
            metrics_summary.insert(
                "sharpe_ratio".to_string(),
                performance.sharpe_ratio.map(decimal_to_f64).unwrap_or(0.0),
            );
            metrics_summary.insert(
                "max_drawdown".to_string(),
                decimal_to_f64(performance.max_drawdown) * 100.0,
            );
            metrics_summary.insert(
                "max_drawdown_duration_days".to_string(),
                performance
                    .max_drawdown_duration_days
                    .map(|days| days as f64)
                    .unwrap_or(0.0),
            );
            metrics_summary.insert(
                "sortino_ratio".to_string(),
                performance.sortino_ratio.map(decimal_to_f64).unwrap_or(0.0),
            );
            metrics_summary.insert(
                "calmar_ratio".to_string(),
                performance.calmar_ratio.map(decimal_to_f64).unwrap_or(0.0),
            );
            metrics_summary.insert(
                "var_95".to_string(),
                performance.var_95.map(decimal_to_f64).unwrap_or(0.0),
            );
            metrics_summary.insert(
                "cvar_95".to_string(),
                performance.cvar_95.map(decimal_to_f64).unwrap_or(0.0),
            );
            metrics_summary.insert(
                "skewness".to_string(),
                performance.skewness.map(decimal_to_f64).unwrap_or(0.0),
            );
            metrics_summary.insert(
                "kurtosis".to_string(),
                performance.kurtosis.map(decimal_to_f64).unwrap_or(0.0),
            );
            metrics_summary.insert(
                "total_commissions".to_string(),
                decimal_to_f64(performance.total_commissions),
            );
            metrics_summary.insert("total_trades".to_string(), performance.total_trades as f64);
            metrics_summary.insert(
                "win_rate".to_string(),
                decimal_to_f64(performance.win_rate) * 100.0,
            );
            metrics_summary.insert(
                "profit_factor".to_string(),
                performance.profit_factor.map(decimal_to_f64).unwrap_or(0.0),
            );
            metrics_summary.insert(
                "average_win".to_string(),
                decimal_to_f64(performance.average_win),
            );
            metrics_summary.insert(
                "average_loss".to_string(),
                decimal_to_f64(performance.average_loss),
            );
            metrics_summary.insert(
                "largest_win".to_string(),
                decimal_to_f64(performance.largest_win),
            );
            metrics_summary.insert(
                "largest_loss".to_string(),
                decimal_to_f64(performance.largest_loss),
            );
        }

        if let Some(strategy_metrics) = result.strategy_metrics.as_ref() {
            metrics_summary.insert(
                "total_trades".to_string(),
                strategy_metrics.total_trades as f64,
            );
            metrics_summary.insert(
                "win_rate".to_string(),
                decimal_to_f64(strategy_metrics.win_rate) * 100.0,
            );
            metrics_summary.insert(
                "profit_factor".to_string(),
                decimal_to_f64(strategy_metrics.profit_factor),
            );
            metrics_summary.insert(
                "average_win".to_string(),
                decimal_to_f64(strategy_metrics.average_win),
            );
            metrics_summary.insert(
                "average_loss".to_string(),
                decimal_to_f64(strategy_metrics.average_loss),
            );
            metrics_summary.insert(
                "total_commissions".to_string(),
                decimal_to_f64(strategy_metrics.total_commissions),
            );
        }

        let equity_curve = result
            .equity_curve
            .iter()
            .map(|point| EquityPoint {
                timestamp: point.timestamp.to_rfc3339(),
                value: decimal_to_f64(point.portfolio_value),
                cash: decimal_to_f64(point.cash),
                positions: decimal_to_f64(point.positions_value),
                total_pnl: decimal_to_f64(point.total_pnl),
                daily_return: point.daily_return.map(|dr| decimal_to_f64(dr) * 100.0),
                returns: decimal_to_f64(point.cumulative_return) * 100.0,
                drawdown: decimal_to_f64(point.drawdown) * 100.0,
            })
            .collect::<Vec<_>>();

        let exposures = result
            .equity_curve
            .iter()
            .map(|point| {
                let portfolio_value = decimal_to_f64(point.portfolio_value);
                let cash = decimal_to_f64(point.cash);
                let positions = decimal_to_f64(point.positions_value);
                let denominator = if portfolio_value.abs() > f64::EPSILON {
                    portfolio_value
                } else {
                    1.0
                };
                ExposurePoint {
                    timestamp: point.timestamp.to_rfc3339(),
                    cash_pct: cash / denominator * 100.0,
                    positions_pct: positions / denominator * 100.0,
                    gross_exposure_pct: positions.abs() / denominator * 100.0,
                    net_exposure_pct: positions / denominator * 100.0,
                }
            })
            .collect::<Vec<_>>();

        let trades = result
            .trade_log
            .into_iter()
            .map(|trade| TradePoint {
                timestamp: trade.exit_time.unwrap_or(trade.entry_time).to_rfc3339(),
                symbol: trade.symbol.symbol,
                action: match trade.side {
                    gb_types::Side::Buy => "BUY".to_string(),
                    gb_types::Side::Sell => "SELL".to_string(),
                },
                shares: decimal_to_f64(trade.quantity),
                price: decimal_to_f64(trade.exit_price.unwrap_or(trade.entry_price)),
                commission: decimal_to_f64(trade.commission),
                pnl: trade.pnl.map(decimal_to_f64),
                strategy_id: trade.strategy_id,
            })
            .collect::<Vec<_>>();

        let manifest = result
            .manifest
            .as_ref()
            .and_then(|manifest| serde_json::to_value(manifest).ok());

        let total_trades = metrics_summary
            .get("total_trades")
            .copied()
            .unwrap_or(trades.len() as f64);
        let final_value = metrics_summary
            .get("final_value")
            .copied()
            .unwrap_or(final_cash);
        let logs = vec![
            "Engine-backed backtest completed".to_string(),
            format!("Processed {} equity points", equity_curve.len()),
            format!("Executed {} trades", total_trades as usize),
            format!("Final portfolio value ${:.2}", final_value),
        ];

        Self {
            metrics_summary,
            equity_curve,
            trades,
            exposures,
            logs,
            final_cash,
            final_positions,
            manifest,
        }
    }
}

#[pymethods]
impl PyBacktestResult {
    #[getter]
    fn metrics_summary(&self, py: Python) -> PyResult<PyObject> {
        Ok(self.metrics_summary.clone().into_pyobject(py)?.into())
    }

    #[getter]
    fn equity_curve(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::empty(py);
        for point in &self.equity_curve {
            let dict = PyDict::new(py);
            let _ = dict.set_item("timestamp", &point.timestamp);
            let _ = dict.set_item("value", point.value);
            let _ = dict.set_item("cash", point.cash);
            let _ = dict.set_item("positions", point.positions);
            let _ = dict.set_item("total_pnl", point.total_pnl);
            let _ = dict.set_item("returns", point.returns);
            match point.daily_return {
                Some(value) => {
                    let _ = dict.set_item("daily_return", value);
                }
                None => {
                    let _ = dict.set_item("daily_return", py.None());
                }
            }
            let _ = dict.set_item("drawdown", point.drawdown);
            let _ = list.append(dict);
        }
        Ok(list.unbind().into())
    }

    #[getter]
    fn trades(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::empty(py);
        for trade in &self.trades {
            let dict = PyDict::new(py);
            let _ = dict.set_item("timestamp", &trade.timestamp);
            let _ = dict.set_item("symbol", &trade.symbol);
            let _ = dict.set_item("action", &trade.action);
            let _ = dict.set_item("shares", trade.shares);
            let _ = dict.set_item("price", trade.price);
            let _ = dict.set_item("commission", trade.commission);
            match trade.pnl {
                Some(value) => {
                    let _ = dict.set_item("pnl", value);
                }
                None => {
                    let _ = dict.set_item("pnl", py.None());
                }
            }
            let _ = dict.set_item("strategy_id", &trade.strategy_id);
            let _ = list.append(dict);
        }
        Ok(list.unbind().into())
    }

    #[getter]
    fn exposures(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::empty(py);
        for exposure in &self.exposures {
            let dict = PyDict::new(py);
            let _ = dict.set_item("timestamp", &exposure.timestamp);
            let _ = dict.set_item("cash_pct", exposure.cash_pct);
            let _ = dict.set_item("positions_pct", exposure.positions_pct);
            let _ = dict.set_item("gross_exposure_pct", exposure.gross_exposure_pct);
            let _ = dict.set_item("net_exposure_pct", exposure.net_exposure_pct);
            let _ = list.append(dict);
        }
        Ok(list.unbind().into())
    }

    #[getter]
    fn logs(&self) -> Vec<String> {
        self.logs.clone()
    }

    #[getter]
    fn final_cash(&self) -> f64 {
        self.final_cash
    }

    #[getter]
    fn final_positions(&self, py: Python) -> PyResult<PyObject> {
        Ok(self.final_positions.clone().into_pyobject(py)?.into())
    }

    #[getter]
    fn manifest(&self, py: Python) -> PyResult<PyObject> {
        match &self.manifest {
            Some(manifest) => {
                let json = py.import("json")?;
                let payload = serde_json::to_string(manifest).map_err(|error| {
                    pyo3::exceptions::PyRuntimeError::new_err(format!(
                        "Failed to serialize run manifest: {}",
                        error
                    ))
                })?;
                Ok(json.call_method1("loads", (payload,))?.into())
            }
            None => Ok(py.None()),
        }
    }

    /// Convert the equity curve to a pandas DataFrame (Jupyter-friendly)
    #[pyo3(signature = (index=None))]
    fn to_dataframe(&self, py: Python, index: Option<&str>) -> PyResult<PyObject> {
        let pandas = py.import("pandas").map_err(|_| {
            pyo3::exceptions::PyImportError::new_err(
                "pandas is required for to_dataframe(). Install with `pip install pandas`.",
            )
        })?;

        let data = self.equity_curve(py)?;
        let mut df = pandas.call_method1("DataFrame", (data,))?;

        let columns = df.getattr("columns")?;
        let has_timestamp = columns
            .call_method1("__contains__", ("timestamp",))?
            .is_truthy()?;
        if has_timestamp {
            let to_datetime = pandas.getattr("to_datetime")?;
            let ts = df.call_method1("__getitem__", ("timestamp",))?;
            let ts_dt = to_datetime.call1((ts,))?;
            df.call_method1("__setitem__", ("timestamp", ts_dt))?;
        }

        if let Some(index_col) = index {
            let columns = df.getattr("columns")?;
            let has_index = columns
                .call_method1("__contains__", (index_col,))?
                .is_truthy()?;
            if has_index {
                df = df.call_method1("set_index", (index_col,))?;
            }
        }

        Ok(df.into())
    }

    /// Convert metrics summary to a pandas DataFrame (Jupyter-friendly)
    fn metrics_dataframe(&self, py: Python) -> PyResult<PyObject> {
        let pandas = py.import("pandas").map_err(|_| {
            pyo3::exceptions::PyImportError::new_err(
                "pandas is required for metrics_dataframe(). Install with `pip install pandas`.",
            )
        })?;

        let metrics = self.metrics_summary(py)?;
        let series = pandas.getattr("Series")?.call1((metrics,))?;
        let df = series.call_method1("to_frame", ("value",))?;
        let df = df.call_method0("sort_index")?;
        Ok(df.into())
    }

    /// Plot the equity curve using matplotlib (returns Axes)
    fn plot_equity(&self, py: Python, show: Option<bool>) -> PyResult<PyObject> {
        let df = self.to_dataframe(py, None)?;
        let plt = py.import("matplotlib.pyplot").map_err(|_| {
            pyo3::exceptions::PyImportError::new_err(
                "matplotlib is required for plot_equity(). Install with `pip install matplotlib`.",
            )
        })?;

        let kwargs = PyDict::new(py);
        kwargs.set_item("x", "timestamp")?;
        kwargs.set_item("y", "value")?;
        kwargs.set_item("title", "GlowBack Equity Curve")?;
        let plot = df.getattr(py, "plot")?;
        let ax = plot.call(py, (), Some(&kwargs))?;

        if show.unwrap_or(false) {
            let _ = plt.call_method0("show")?;
        }

        Ok(ax.into())
    }

    /// Notebook-friendly summary with optional plot
    #[pyo3(signature = (plot=false, index=None))]
    fn summary(&self, py: Python, plot: Option<bool>, index: Option<&str>) -> PyResult<PyObject> {
        let metrics = self.metrics_dataframe(py)?;
        let curve = self.to_dataframe(py, index)?;

        if plot.unwrap_or(false) {
            let _ = self.plot_equity(py, Some(false))?;
        }

        let summary = PyDict::new(py);
        summary.set_item("metrics", metrics)?;
        summary.set_item("equity_curve", curve)?;
        Ok(summary.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "PyBacktestResult(metrics_summary_keys={:?}, equity_points={}, trades={})",
            self.metrics_summary.keys().collect::<Vec<_>>(),
            self.equity_curve.len(),
            self.trades.len()
        )
    }
}

/// Python wrapper for running backtests
#[pyclass(name = "BacktestEngine")]
struct PyBacktestEngine {
    inner: std::sync::Mutex<RustBacktestEngine>,
    runtime: tokio::runtime::Runtime,
}

#[pymethods]
impl PyBacktestEngine {
    #[new]
    #[pyo3(signature = (
        symbols,
        start_date,
        end_date,
        resolution=None,
        initial_capital=None,
        name=None,
        commission_bps=None,
        slippage_bps=None,
        latency_ms=None,
        data_source=None,
        csv_data_path=None,
    ))]
    fn new(
        symbols: Vec<String>,
        start_date: &str,
        end_date: &str,
        resolution: Option<&str>,
        initial_capital: Option<f64>,
        name: Option<&str>,
        commission_bps: Option<f64>,
        slippage_bps: Option<f64>,
        latency_ms: Option<u64>,
        data_source: Option<&str>,
        csv_data_path: Option<&str>,
    ) -> PyResult<Self> {
        if symbols.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "symbols list cannot be empty",
            ));
        }

        let start_date = chrono::DateTime::parse_from_rfc3339(start_date)
            .map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Invalid start_date format: {}", e))
            })?
            .with_timezone(&chrono::Utc);

        let end_date = chrono::DateTime::parse_from_rfc3339(end_date)
            .map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Invalid end_date format: {}", e))
            })?
            .with_timezone(&chrono::Utc);

        let resolution = parse_resolution(resolution.unwrap_or("day"))?;
        let initial_capital = Decimal::from_f64(initial_capital.unwrap_or(100_000.0))
            .unwrap_or_else(|| Decimal::from(100_000));

        let rust_symbols: Vec<Symbol> = symbols
            .iter()
            .map(|symbol| Symbol::equity(symbol))
            .collect();

        let mut strategy_config =
            StrategyConfig::new("buy_and_hold".to_string(), "Buy and Hold".to_string());
        strategy_config.symbols = rust_symbols.clone();
        strategy_config.initial_capital = initial_capital;

        let mut config = BacktestConfig::new(
            name.unwrap_or("Python Backtest").to_string(),
            strategy_config.clone(),
        );
        config.start_date = start_date;
        config.end_date = end_date;
        config.initial_capital = initial_capital;
        config.resolution = resolution;
        config.symbols = rust_symbols;
        config.strategy_config = strategy_config;

        if let Some(bps) = commission_bps {
            let pct = Decimal::from_f64(bps / 10_000.0).unwrap_or_default();
            config.execution_settings.commission_per_share = Decimal::ZERO;
            config.execution_settings.commission_percentage = pct;
            config.execution_settings.minimum_commission = Decimal::ZERO;
        }
        if let Some(bps) = slippage_bps {
            let basis_points = if bps.is_sign_negative() {
                0
            } else {
                bps.round() as u32
            };
            config.execution_settings.slippage_model = SlippageModel::Linear { basis_points };
        }
        if let Some(milliseconds) = latency_ms {
            config.execution_settings.latency_model = LatencyModel::Fixed { milliseconds };
        }

        let normalized_data_source = data_source.unwrap_or("default").trim().to_ascii_lowercase();
        config.data_settings.data_source = normalized_data_source.clone();

        let runtime = tokio::runtime::Runtime::new().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to create async runtime: {}",
                e
            ))
        })?;

        let mut inner = runtime
            .block_on(async { RustBacktestEngine::new(config).await })
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "Failed to create backtest engine: {}",
                    e
                ))
            })?;

        if normalized_data_source == "csv" {
            let base_path = csv_data_path.ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(
                    "csv_data_path is required when data_source='csv'",
                )
            })?;
            inner.add_csv_provider(base_path);
        }

        Ok(Self {
            inner: std::sync::Mutex::new(inner),
            runtime,
        })
    }

    fn add_sample_provider(&mut self) -> PyResult<()> {
        let mut inner = self.inner.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire lock: {}", e))
        })?;
        inner.add_sample_provider();
        Ok(())
    }

    fn add_csv_provider(&mut self, base_path: &str) -> PyResult<()> {
        let mut inner = self.inner.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire lock: {}", e))
        })?;
        inner.add_csv_provider(base_path);
        Ok(())
    }

    /// Run a backtest using the built-in buy-and-hold strategy
    fn run_buy_and_hold(&mut self) -> PyResult<PyBacktestResult> {
        self.run_strategy("buy_and_hold", None)
    }

    fn run_strategy(
        &mut self,
        strategy_name: &str,
        params: Option<&Bound<PyDict>>,
    ) -> PyResult<PyBacktestResult> {
        let strategy = build_strategy(strategy_name, params)?;
        let mut inner = self.inner.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire lock: {}", e))
        })?;
        let result = self
            .runtime
            .block_on(inner.run_with_strategy(strategy))
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "Backtest execution failed: {}",
                    e
                ))
            })?;

        Ok(PyBacktestResult::from_backtest_result(result))
    }
}
