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

    /// Parse Alpha Vantage daily response
    fn parse_daily_response(&self, response: serde_json::Value, symbol: &Symbol) -> GbResult<Vec<Bar>> {
        let time_series = response
            .get("Time Series (Daily)")
            .ok_or_else(|| DataError::ParseError {
                message: "Missing 'Time Series (Daily)' in response".to_string(),
            })?
            .as_object()
            .ok_or_else(|| DataError::ParseError {
                message: "Time Series is not an object".to_string(),
            })?;

        let mut bars = Vec::new();

        for (date_str, data) in time_series {
            let timestamp = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                .map_err(|e| DataError::ParseError {
                    message: format!("Failed to parse date '{}': {}", date_str, e),
                })?
                .and_hms_opt(16, 0, 0) // Market close time (4 PM EST)
                .ok_or_else(|| DataError::ParseError {
                    message: "Failed to create timestamp".to_string(),
                })?;

            let timestamp = DateTime::<Utc>::from_naive_utc_and_offset(timestamp, Utc);

            let data_obj = data.as_object().ok_or_else(|| DataError::ParseError {
                message: format!("Data for {} is not an object", date_str),
            })?;

            let open = self.parse_price_field(data_obj, "1. open")?;
            let high = self.parse_price_field(data_obj, "2. high")?;
            let low = self.parse_price_field(data_obj, "3. low")?;
            let close = self.parse_price_field(data_obj, "4. close")?;
            let volume = self.parse_volume_field(data_obj, "5. volume")?;

            let bar = Bar::new(
                symbol.clone(),
                timestamp,
                open,
                high,
                low,
                close,
                volume,
                Resolution::Day,
            );

            bars.push(bar);
        }

        // Sort by timestamp (oldest first)
        bars.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(bars)
    }

    /// Parse a price field from Alpha Vantage response
    fn parse_price_field(&self, data: &serde_json::Map<String, serde_json::Value>, field: &str) -> GbResult<rust_decimal::Decimal> {
        let value_str = data
            .get(field)
            .ok_or_else(|| DataError::ParseError {
                message: format!("Missing field '{}'", field),
            })?
            .as_str()
            .ok_or_else(|| DataError::ParseError {
                message: format!("Field '{}' is not a string", field),
            })?;

        value_str.parse::<rust_decimal::Decimal>()
            .map_err(|e| DataError::ParseError {
                message: format!("Failed to parse {} value '{}': {}", field, value_str, e),
            }.into())
    }

    /// Parse a volume field from Alpha Vantage response
    fn parse_volume_field(&self, data: &serde_json::Map<String, serde_json::Value>, field: &str) -> GbResult<rust_decimal::Decimal> {
        let value_str = data
            .get(field)
            .ok_or_else(|| DataError::ParseError {
                message: format!("Missing field '{}'", field),
            })?
            .as_str()
            .ok_or_else(|| DataError::ParseError {
                message: format!("Field '{}' is not a string", field),
            })?;

        value_str.parse::<rust_decimal::Decimal>()
            .map_err(|e| DataError::ParseError {
                message: format!("Failed to parse {} value '{}': {}", field, value_str, e),
            }.into())
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
        symbol: &Symbol,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        resolution: Resolution,
    ) -> GbResult<Vec<Bar>> {
        tracing::info!("Fetching data from Alpha Vantage for {} ({:?})", symbol, resolution);

        // Alpha Vantage mainly supports daily data for free tier
        let function = match resolution {
            Resolution::Day => "TIME_SERIES_DAILY",
            _ => {
                return Err(DataError::LoadingFailed {
                    message: format!("Resolution {:?} not supported by Alpha Vantage free tier", resolution),
                }.into());
            }
        };

        let url = format!(
            "https://www.alphavantage.co/query?function={}&symbol={}&apikey={}",
            function, symbol.symbol, self.api_key
        );
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| DataError::LoadingFailed {
                message: format!("HTTP request failed: {}", e),
            })?;

        if !response.status().is_success() {
            return Err(DataError::LoadingFailed {
                message: format!("HTTP error: {}", response.status()),
            }.into());
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| DataError::LoadingFailed {
                message: format!("Failed to parse JSON response: {}", e),
            })?;

        // Check for API errors
        if let Some(error) = json.get("Error Message") {
            return Err(DataError::LoadingFailed {
                message: format!("API error: {}", error),
            }.into());
        }

        if let Some(note) = json.get("Note") {
            return Err(DataError::LoadingFailed {
                message: format!("API limit exceeded: {}", note),
            }.into());
        }

        let mut bars = self.parse_daily_response(json, symbol)?;

        // Filter by date range
        bars.retain(|bar| bar.timestamp >= start_date && bar.timestamp <= end_date);

        tracing::info!("Retrieved {} bars from Alpha Vantage for {}", bars.len(), symbol);
        Ok(bars)
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