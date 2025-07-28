use async_trait::async_trait;
use chrono::{DateTime, Utc};
use gb_types::{Bar, Symbol, Resolution, GbResult, DataError};
use rust_decimal::Decimal;
use std::path::Path;
use csv::ReaderBuilder;
use serde::Deserialize;

/// Trait for data providers (CSV, APIs, databases, etc.)
#[async_trait]
pub trait DataProvider: Send + Sync + std::fmt::Debug {
    /// Check if this provider supports the given symbol
    fn supports_symbol(&self, symbol: &Symbol) -> bool;
    
    /// Fetch bar data for the given parameters
    async fn fetch_bars(
        &mut self,
        symbol: &Symbol,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        resolution: Resolution,
    ) -> GbResult<Vec<Bar>>;
    
    /// Get provider name
    fn name(&self) -> &str;
    
    /// Get provider configuration
    fn config(&self) -> serde_json::Value;
}

/// CSV data provider for loading local CSV files
#[derive(Debug)]
pub struct CsvDataProvider {
    pub name: String,
    pub data_directory: std::path::PathBuf,
    pub file_pattern: String,
}

#[derive(Debug, Deserialize)]
struct CsvRecord {
    #[serde(alias = "Date", alias = "date")]
    timestamp: String,
    #[serde(alias = "Open", alias = "open")]
    open: f64,
    #[serde(alias = "High", alias = "high")]
    high: f64,
    #[serde(alias = "Low", alias = "low")]
    low: f64,
    #[serde(alias = "Close", alias = "close")]
    close: f64,
    #[serde(alias = "Volume", alias = "volume")]
    volume: f64,
}

impl CsvDataProvider {
    pub fn new<P: AsRef<Path>>(data_directory: P) -> Self {
        Self {
            name: "CSV Provider".to_string(),
            data_directory: data_directory.as_ref().to_path_buf(),
            file_pattern: "{symbol}_{resolution}.csv".to_string(),
        }
    }
    
    pub fn with_pattern(mut self, pattern: &str) -> Self {
        self.file_pattern = pattern.to_string();
        self
    }
    
    fn get_file_path(&self, symbol: &Symbol, resolution: Resolution) -> std::path::PathBuf {
        let filename = self.file_pattern
            .replace("{symbol}", &symbol.symbol)
            .replace("{resolution}", &resolution.to_string())
            .replace("{exchange}", &symbol.exchange);
        
        self.data_directory.join(filename)
    }
}

#[async_trait]
impl DataProvider for CsvDataProvider {
    fn supports_symbol(&self, symbol: &Symbol) -> bool {
        let path = self.get_file_path(symbol, Resolution::Day);
        path.exists()
    }
    
    async fn fetch_bars(
        &mut self,
        symbol: &Symbol,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        resolution: Resolution,
    ) -> GbResult<Vec<Bar>> {
        let file_path = self.get_file_path(symbol, resolution);
        
        if !file_path.exists() {
            return Err(DataError::SourceNotFound(
                file_path.to_string_lossy().to_string()
            ).into());
        }
        
        let file = std::fs::File::open(&file_path)?;
        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(file);
        
        let mut bars = Vec::new();
        
        for result in reader.deserialize() {
            let record: CsvRecord = result.map_err(|e| {
                DataError::ParseError {
                    message: format!("CSV parsing error: {}", e),
                }
            })?;
            
            let timestamp = chrono::DateTime::parse_from_rfc3339(&record.timestamp)
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(&record.timestamp, "%Y-%m-%d")
                    .map(|dt| dt.and_utc().into()))
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(&record.timestamp, "%Y-%m-%d %H:%M:%S")
                    .map(|dt| dt.and_utc().into()))
                .map_err(|e| DataError::ParseError {
                    message: format!("Date parsing error: {}", e),
                })?
                .with_timezone(&Utc);
            
            if timestamp >= start_date && timestamp <= end_date {
                let bar = Bar::new(
                    symbol.clone(),
                    timestamp,
                    Decimal::from_f64_retain(record.open).unwrap_or_default(),
                    Decimal::from_f64_retain(record.high).unwrap_or_default(),
                    Decimal::from_f64_retain(record.low).unwrap_or_default(),
                    Decimal::from_f64_retain(record.close).unwrap_or_default(),
                    Decimal::from_f64_retain(record.volume).unwrap_or_default(),
                    resolution,
                );
                bars.push(bar);
            }
        }
        
        bars.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        Ok(bars)
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn config(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "csv",
            "directory": self.data_directory,
            "pattern": self.file_pattern
        })
    }
}

/// Sample data provider for testing and demo purposes
#[derive(Debug)]
pub struct SampleDataProvider {
    pub name: String,
}

impl SampleDataProvider {
    pub fn new() -> Self {
        Self {
            name: "Sample Data Provider".to_string(),
        }
    }
}

impl Default for SampleDataProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DataProvider for SampleDataProvider {
    fn supports_symbol(&self, symbol: &Symbol) -> bool {
        // Support common test symbols
        matches!(symbol.symbol.as_str(), "AAPL" | "GOOGL" | "MSFT" | "TSLA" | "SPY" | "BTC-USD" | "ETH-USD")
    }
    
    async fn fetch_bars(
        &mut self,
        symbol: &Symbol,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        resolution: Resolution,
    ) -> GbResult<Vec<Bar>> {
        if !self.supports_symbol(symbol) {
            return Err(DataError::SymbolNotFound {
                symbol: symbol.to_string(),
            }.into());
        }
        
        // Generate synthetic data for demo
        let mut bars = Vec::new();
        let mut current_date = start_date;
        let mut price = match symbol.symbol.as_str() {
            "AAPL" => Decimal::from(150),
            "GOOGL" => Decimal::from(2500),
            "MSFT" => Decimal::from(300),
            "TSLA" => Decimal::from(800),
            "SPY" => Decimal::from(400),
            "BTC-USD" => Decimal::from(45000),
            "ETH-USD" => Decimal::from(3000),
            _ => Decimal::from(100),
        };
        
        let increment = match resolution {
            Resolution::Minute => chrono::Duration::minutes(1),
            Resolution::FiveMinute => chrono::Duration::minutes(5),
            Resolution::FifteenMinute => chrono::Duration::minutes(15),
            Resolution::Hour => chrono::Duration::hours(1),
            Resolution::FourHour => chrono::Duration::hours(4),
            Resolution::Day => chrono::Duration::days(1),
            Resolution::Week => chrono::Duration::weeks(1),
            Resolution::Month => chrono::Duration::days(30),
            _ => chrono::Duration::days(1),
        };
        
        let mut rng_state = 12345u64; // Simple PRNG
        
        while current_date <= end_date {
            // Simple random walk
            rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
            let random = (rng_state >> 16) as f64 / 65536.0 - 0.5; // -0.5 to 0.5
            
            let change_pct = Decimal::from_f64_retain(random * 0.02).unwrap_or_default(); // ±2%
            let new_price = price * (Decimal::ONE + change_pct);
            
            let volatility = Decimal::from_f64_retain(0.01).unwrap_or_default(); // 1% intraday volatility
            let high = new_price * (Decimal::ONE + volatility);
            let low = new_price * (Decimal::ONE - volatility);
            
            let volume = match symbol.symbol.as_str() {
                "AAPL" => Decimal::from(80000000),
                "SPY" => Decimal::from(50000000),
                "BTC-USD" => Decimal::from(1000),
                _ => Decimal::from(10000000),
            };
            
            let bar = Bar::new(
                symbol.clone(),
                current_date,
                price,
                high,
                low,
                new_price,
                volume,
                resolution,
            );
            
            bars.push(bar);
            price = new_price;
            current_date += increment;
        }
        
        Ok(bars)
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn config(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "sample",
            "supported_symbols": ["AAPL", "GOOGL", "MSFT", "TSLA", "SPY", "BTC-USD", "ETH-USD"]
        })
    }
}

/// Alpha Vantage API provider (placeholder for future implementation)
#[derive(Debug)]
pub struct AlphaVantageProvider {
    pub name: String,
    pub api_key: String,
    pub client: reqwest::Client,
}

impl AlphaVantageProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            name: "Alpha Vantage".to_string(),
            api_key,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl DataProvider for AlphaVantageProvider {
    fn supports_symbol(&self, symbol: &Symbol) -> bool {
        // Alpha Vantage supports most US equities
        matches!(symbol.asset_class, gb_types::AssetClass::Equity)
    }
    
    async fn fetch_bars(
        &mut self,
        _symbol: &Symbol,
        _start_date: DateTime<Utc>,
        _end_date: DateTime<Utc>,
        _resolution: Resolution,
    ) -> GbResult<Vec<Bar>> {
        // TODO: Implement Alpha Vantage API integration
        Err(DataError::LoadingFailed {
            message: "Alpha Vantage integration not yet implemented".to_string(),
        }.into())
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn config(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "alpha_vantage",
            "api_key_set": !self.api_key.is_empty()
        })
    }
} 