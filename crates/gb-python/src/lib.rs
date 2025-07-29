use pyo3::prelude::*;
use num_traits::cast::ToPrimitive;

/// GlowBack Python module
#[pymodule]
fn glowback(_py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    m.add("__version__", "0.1.0")?;
    m.add_class::<PySymbol>()?;
    m.add_class::<PyDataManager>()?;
    m.add_class::<PyBar>()?;
    m.add_class::<PyCatalogStats>()?;
    Ok(())
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