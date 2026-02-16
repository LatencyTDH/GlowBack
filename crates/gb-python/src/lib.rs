use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use num_traits::cast::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;

use gb_engine::BacktestEngine as RustBacktestEngine;
use gb_types::{
    BacktestConfig, BuyAndHoldStrategy, Resolution, StrategyConfig, Symbol,
    BacktestResult as RustBacktestResult,
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

/// Python wrapper for Symbol
#[pyclass]
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
        format!("Symbol(symbol='{}', exchange='{}', asset_class='{:?}')", 
                self.inner.symbol, self.inner.exchange, self.inner.asset_class)
    }
}

/// Python wrapper for DataManager
#[pyclass]
struct PyDataManager {
    inner: std::sync::Mutex<gb_data::DataManager>,
    runtime: tokio::runtime::Runtime,
}

#[pymethods]
impl PyDataManager {
    #[new]
    fn new() -> PyResult<Self> {
        // Create tokio runtime for async operations
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create async runtime: {}", e)))?;
        
        // Create data manager
        let inner = runtime.block_on(async {
            gb_data::DataManager::new().await
        }).map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create data manager: {}", e)))?;

        Ok(Self { 
            inner: std::sync::Mutex::new(inner), 
            runtime 
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
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid start_date format: {}", e)))?
            .with_timezone(&chrono::Utc);
        
        let end_date = chrono::DateTime::parse_from_rfc3339(end_date)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid end_date format: {}", e)))?
            .with_timezone(&chrono::Utc);

        // Parse resolution
        let resolution = match resolution.to_lowercase().as_str() {
            "minute" | "1m" => gb_types::Resolution::Minute,
            "hour" | "1h" => gb_types::Resolution::Hour,
            "day" | "1d" => gb_types::Resolution::Day,
            _ => return Err(pyo3::exceptions::PyValueError::new_err(format!("Invalid resolution: {}", resolution))),
        };

        // Load data asynchronously
        let bars = self.runtime.block_on(async {
            let mut inner = self.inner.lock().map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire lock: {}", e)))?;
            inner.load_data(&symbol.inner, start_date, end_date, resolution).await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to load data: {}", e)))
        })?;

        // Convert to Python bars
        let py_bars = bars.into_iter().map(|bar| PyBar { inner: bar }).collect();
        Ok(py_bars)
    }

    /// Add a sample data provider
    fn add_sample_provider(&mut self) -> PyResult<()> {
        let provider = Box::new(gb_data::SampleDataProvider::new());
        let mut inner = self.inner.lock().map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire lock: {}", e)))?;
        inner.add_provider(provider);
        Ok(())
    }

    /// Add a CSV data provider
    fn add_csv_provider(&mut self, base_path: &str) -> PyResult<()> {
        let provider = Box::new(gb_data::CsvDataProvider::new(base_path));
        let mut inner = self.inner.lock().map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire lock: {}", e)))?;
        inner.add_provider(provider);
        Ok(())
    }

    /// Add an Alpha Vantage provider
    fn add_alpha_vantage_provider(&mut self, api_key: &str) -> PyResult<()> {
        let provider = Box::new(gb_data::AlphaVantageProvider::new(api_key.to_string()));
        let mut inner = self.inner.lock().map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire lock: {}", e)))?;
        inner.add_provider(provider);
        Ok(())
    }

    /// Get catalog statistics
    fn get_catalog_stats(&self) -> PyResult<PyCatalogStats> {
        let stats = self.runtime.block_on(async {
            let inner = self.inner.lock().map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire lock: {}", e)))?;
            inner.catalog.get_catalog_stats().await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to get catalog stats: {}", e)))
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
        let inner = self.inner.lock().map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire lock: {}", e)))?;
        Ok(inner.providers.len())
    }
}

/// Python wrapper for Bar (OHLCV data)
#[pyclass]
struct PyBar {
    inner: gb_types::Bar,
}

#[pymethods]
impl PyBar {
    #[getter]
    fn symbol(&self) -> PySymbol {
        PySymbol { inner: self.inner.symbol.clone() }
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
        format!("Bar({} {} O:{} H:{} L:{} C:{} V:{})",
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
        format!("PyBar(symbol='{}', timestamp='{}', open={}, high={}, low={}, close={}, volume={})",
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
#[pyclass]
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
        format!("CatalogStats(symbols: {}, records: {}, date_range: {} to {})",
            self.total_symbols,
            self.total_records,
            self.date_range_start.as_ref().map(|s| s.as_str()).unwrap_or("None"),
            self.date_range_end.as_ref().map(|s| s.as_str()).unwrap_or("None")
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

#[pyclass]
struct PyBacktestResult {
    metrics_summary: std::collections::HashMap<String, f64>,
    equity_curve: Vec<EquityPoint>,
}

impl PyBacktestResult {
    fn from_backtest_result(result: RustBacktestResult) -> Self {
        let mut metrics_summary = std::collections::HashMap::new();
        metrics_summary.insert(
            "initial_capital".to_string(),
            decimal_to_f64(result.config.initial_capital),
        );

        if let Some(portfolio) = result.final_portfolio.as_ref() {
            metrics_summary.insert(
                "final_value".to_string(),
                decimal_to_f64(portfolio.total_equity),
            );
        }

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
                performance
                    .sharpe_ratio
                    .map(decimal_to_f64)
                    .unwrap_or(0.0),
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
                performance
                    .sortino_ratio
                    .map(decimal_to_f64)
                    .unwrap_or(0.0),
            );
            metrics_summary.insert(
                "calmar_ratio".to_string(),
                performance
                    .calmar_ratio
                    .map(decimal_to_f64)
                    .unwrap_or(0.0),
            );
            metrics_summary.insert(
                "var_95".to_string(),
                performance
                    .var_95
                    .map(decimal_to_f64)
                    .unwrap_or(0.0),
            );
            metrics_summary.insert(
                "cvar_95".to_string(),
                performance
                    .cvar_95
                    .map(decimal_to_f64)
                    .unwrap_or(0.0),
            );
            metrics_summary.insert(
                "skewness".to_string(),
                performance
                    .skewness
                    .map(decimal_to_f64)
                    .unwrap_or(0.0),
            );
            metrics_summary.insert(
                "kurtosis".to_string(),
                performance
                    .kurtosis
                    .map(decimal_to_f64)
                    .unwrap_or(0.0),
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
                performance
                    .profit_factor
                    .map(decimal_to_f64)
                    .unwrap_or(0.0),
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
            .into_iter()
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
            .collect();

        Self {
            metrics_summary,
            equity_curve,
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

    /// Convert the equity curve to a pandas DataFrame (Jupyter-friendly)
    fn to_dataframe(&self, py: Python) -> PyResult<PyObject> {
        let pandas = py.import("pandas").map_err(|_| {
            pyo3::exceptions::PyImportError::new_err(
                "pandas is required for to_dataframe(). Install with `pip install pandas`.",
            )
        })?;

        let data = self.equity_curve(py)?;
        let df = pandas.call_method1("DataFrame", (data,))?;

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
        let df = self.to_dataframe(py)?;
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

    fn __repr__(&self) -> String {
        format!(
            "PyBacktestResult(metrics_summary_keys={:?}, equity_points={})",
            self.metrics_summary.keys().collect::<Vec<_>>(),
            self.equity_curve.len()
        )
    }
}

/// Python wrapper for running backtests
#[pyclass]
struct PyBacktestEngine {
    inner: std::sync::Mutex<RustBacktestEngine>,
    runtime: tokio::runtime::Runtime,
}

#[pymethods]
impl PyBacktestEngine {
    #[new]
    fn new(
        symbols: Vec<String>,
        start_date: &str,
        end_date: &str,
        resolution: Option<&str>,
        initial_capital: Option<f64>,
        name: Option<&str>,
    ) -> PyResult<Self> {
        if symbols.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "symbols list cannot be empty",
            ));
        }

        let start_date = chrono::DateTime::parse_from_rfc3339(start_date)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid start_date format: {}", e)))?
            .with_timezone(&chrono::Utc);

        let end_date = chrono::DateTime::parse_from_rfc3339(end_date)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid end_date format: {}", e)))?
            .with_timezone(&chrono::Utc);

        let resolution = parse_resolution(resolution.unwrap_or("day"))?;
        let initial_capital = Decimal::from_f64(initial_capital.unwrap_or(100_000.0))
            .unwrap_or_else(|| Decimal::from(100_000));

        let rust_symbols: Vec<Symbol> = symbols.iter().map(|symbol| Symbol::equity(symbol)).collect();

        let mut strategy_config = StrategyConfig::new(
            "buy_and_hold".to_string(),
            "Buy and Hold".to_string(),
        );
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

        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create async runtime: {}", e)))?;

        let inner = runtime
            .block_on(async { RustBacktestEngine::new(config).await })
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to create backtest engine: {}", e)))?;

        Ok(Self {
            inner: std::sync::Mutex::new(inner),
            runtime,
        })
    }

    /// Run a backtest using the built-in buy-and-hold strategy
    fn run_buy_and_hold(&mut self) -> PyResult<PyBacktestResult> {
        let result: Result<RustBacktestResult, PyErr> = self.runtime.block_on(async {
            let mut inner = self
                .inner
                .lock()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to acquire lock: {}", e)))?;
            inner
                .run_with_strategy(Box::new(BuyAndHoldStrategy::new()))
                .await
                .map_err(|e| {
                    pyo3::exceptions::PyRuntimeError::new_err(format!(
                        "Backtest execution failed: {}",
                        e
                    ))
                })
        });

        let result = result?;
        Ok(PyBacktestResult::from_backtest_result(result))
    }
}
