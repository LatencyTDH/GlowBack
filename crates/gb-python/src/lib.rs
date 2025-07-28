use pyo3::prelude::*;

/// GlowBack Python module
#[pymodule]
fn glowback(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add("__version__", "0.1.0")?;
    m.add_class::<PySymbol>()?;
    m.add_class::<PyDataManager>()?;
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
    // TODO: Add actual data manager when async support is added
}

#[pymethods]
impl PyDataManager {
    #[new]
    fn new() -> Self {
        Self {}
    }
} 