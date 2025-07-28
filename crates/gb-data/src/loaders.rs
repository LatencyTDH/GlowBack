use std::path::Path;
use std::fs;
use chrono::{DateTime, Utc};
use gb_types::{Bar, Symbol, Resolution, GbResult, DataError, AssetClass};
use rust_decimal::Decimal;
use arrow::array::{Array, StringArray, TimestampNanosecondArray, Decimal128Array, Int64Array};
use arrow::record_batch::RecordBatch;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
// use polars::prelude::*;

/// Batch data loader for efficient bulk operations
#[derive(Debug)]
pub struct BatchLoader {
    chunk_size: usize,
}

impl BatchLoader {
    pub fn new() -> Self {
        Self {
            chunk_size: 10000, // Process 10k rows at a time
        }
    }
    
    pub fn with_chunk_size(chunk_size: usize) -> Self {
        Self { chunk_size }
    }
    
    /// Load bars from a Parquet file using Arrow for performance
    pub async fn load_parquet_file<P: AsRef<Path>>(
        &self,
        file_path: P,
        symbol: &Symbol,
        resolution: Resolution,
    ) -> GbResult<Vec<Bar>> {
        let path = file_path.as_ref();
        tracing::info!("Loading Parquet data from: {}", path.display());

        if !path.exists() {
            return Err(DataError::SymbolNotFound { 
                symbol: symbol.to_string() 
            }.into());
        }

        let file = fs::File::open(path)?;
        let reader = ParquetRecordBatchReaderBuilder::try_new(file)
            .map_err(|e| DataError::LoadingFailed { 
                message: format!("Failed to create Parquet reader for {}: {}", path.display(), e) 
            })?
            .build()
            .map_err(|e| DataError::LoadingFailed { 
                message: format!("Failed to build Parquet reader: {}", e) 
            })?;

        let mut all_bars = Vec::new();

        for batch_result in reader {
            let batch = batch_result
                .map_err(|e| DataError::LoadingFailed { 
                    message: format!("Failed to read Parquet batch: {}", e) 
                })?;

            let batch_bars = Self::record_batch_to_bars(&batch, symbol, resolution)?;
            all_bars.extend(batch_bars);
        }

        tracing::info!("Loaded {} bars from Parquet file: {}", all_bars.len(), path.display());
        Ok(all_bars)
    }

    /// Convert Arrow RecordBatch to bars (similar to storage.rs implementation)
    fn record_batch_to_bars(
        batch: &RecordBatch,
        symbol: &Symbol,
        resolution: Resolution,
    ) -> GbResult<Vec<Bar>> {
        let timestamps = batch.column(1)
            .as_any()
            .downcast_ref::<TimestampNanosecondArray>()
            .ok_or_else(|| DataError::Corruption {
                message: "Invalid timestamp column in Parquet file".to_string(),
            })?;
        
        let opens = batch.column(2)
            .as_any()
            .downcast_ref::<Decimal128Array>()
            .ok_or_else(|| DataError::Corruption {
                message: "Invalid open column in Parquet file".to_string(),
            })?;
        
        let highs = batch.column(3)
            .as_any()
            .downcast_ref::<Decimal128Array>()
            .ok_or_else(|| DataError::Corruption {
                message: "Invalid high column in Parquet file".to_string(),
            })?;
        
        let lows = batch.column(4)
            .as_any()
            .downcast_ref::<Decimal128Array>()
            .ok_or_else(|| DataError::Corruption {
                message: "Invalid low column in Parquet file".to_string(),
            })?;
        
        let closes = batch.column(5)
            .as_any()
            .downcast_ref::<Decimal128Array>()
            .ok_or_else(|| DataError::Corruption {
                message: "Invalid close column in Parquet file".to_string(),
            })?;
        
        let volumes = batch.column(6)
            .as_any()
            .downcast_ref::<Int64Array>()
            .ok_or_else(|| DataError::Corruption {
                message: "Invalid volume column in Parquet file".to_string(),
            })?;
        
        let mut bars = Vec::new();
        
        for i in 0..batch.num_rows() {
            if timestamps.is_null(i) || opens.is_null(i) || highs.is_null(i) 
                || lows.is_null(i) || closes.is_null(i) || volumes.is_null(i) {
                continue;
            }
            
            let timestamp_nanos = timestamps.value(i);
            let timestamp = DateTime::from_timestamp(
                timestamp_nanos / 1_000_000_000,
                (timestamp_nanos % 1_000_000_000) as u32,
            ).unwrap_or_default();
            
            let open = Decimal::from_i128_with_scale(opens.value(i), 4);
            let high = Decimal::from_i128_with_scale(highs.value(i), 4);
            let low = Decimal::from_i128_with_scale(lows.value(i), 4);
            let close = Decimal::from_i128_with_scale(closes.value(i), 4);
            let volume = Decimal::from(volumes.value(i));
            
            let bar = Bar::new(
                symbol.clone(),
                timestamp,
                open,
                high,
                low,
                close,
                volume,
                resolution,
            );
            
            bars.push(bar);
        }
        
        Ok(bars)
    }
    
    /// Load bars from a CSV file using csv crate
    pub async fn load_csv_file<P: AsRef<Path>>(
        &self,
        file_path: P,
        symbol: &Symbol,
        resolution: Resolution,
        has_headers: bool,
    ) -> GbResult<Vec<Bar>> {
        use csv::ReaderBuilder;
        use std::str::FromStr;
        
        let path = file_path.as_ref();
        tracing::info!("Loading CSV data from: {}", path.display());
        
        let mut bars = Vec::new();
        let mut rdr = ReaderBuilder::new()
            .has_headers(has_headers)
            .from_path(path)
            .map_err(|e| DataError::LoadingFailed {
                message: format!("Failed to open CSV file {}: {}", path.display(), e),
            })?;

        let headers = if has_headers {
            Some(rdr.headers()
                .map_err(|e| DataError::LoadingFailed {
                    message: format!("Failed to read CSV headers: {}", e),
                })?
                .clone())
        } else {
            None
        };

        if let Some(ref h) = headers {
            tracing::debug!("CSV headers: {:?}", h);
        }

        for (line_num, result) in rdr.records().enumerate() {
            let record = result.map_err(|e| DataError::LoadingFailed {
                message: format!("Failed to read CSV record at line {}: {}", line_num + if has_headers { 2 } else { 1 }, e),
            })?;

            match self.parse_csv_record(&record, symbol, resolution, &headers) {
                Ok(bar) => bars.push(bar),
                Err(e) => {
                    tracing::warn!("Skipping invalid record at line {}: {}", line_num + if has_headers { 2 } else { 1 }, e);
                    continue;
                }
            }
        }

        tracing::info!("Loaded {} bars from CSV file", bars.len());
        Ok(bars)
    }

    /// Parse a CSV record into a Bar struct
    fn parse_csv_record(
        &self,
        record: &csv::StringRecord,
        symbol: &Symbol,
        resolution: Resolution,
        headers: &Option<csv::StringRecord>,
    ) -> GbResult<Bar> {
        use std::str::FromStr;
        
        // Default column mapping for standard OHLCV CSV format
        let (timestamp_idx, open_idx, high_idx, low_idx, close_idx, volume_idx) = 
            if let Some(headers) = headers {
                self.detect_csv_columns(headers)?
            } else {
                // Default ordering: timestamp, open, high, low, close, volume
                (0, 1, 2, 3, 4, 5)
            };

        if record.len() <= volume_idx {
            return Err(DataError::ParseError {
                message: format!("CSV record has {} columns, expected at least {}", record.len(), volume_idx + 1),
            }.into());
        }

        // Parse timestamp
        let timestamp_str = record.get(timestamp_idx).unwrap_or("");
        let timestamp = self.parse_timestamp(timestamp_str)?;

        // Parse OHLCV values
        let open = self.parse_decimal(record.get(open_idx).unwrap_or(""), "open")?;
        let high = self.parse_decimal(record.get(high_idx).unwrap_or(""), "high")?;
        let low = self.parse_decimal(record.get(low_idx).unwrap_or(""), "low")?;
        let close = self.parse_decimal(record.get(close_idx).unwrap_or(""), "close")?;
        let volume = self.parse_decimal(record.get(volume_idx).unwrap_or(""), "volume")?;

        // Validate OHLC relationships
        if high < low {
            return Err(DataError::ParseError {
                message: format!("Invalid OHLC: high ({}) < low ({})", high, low),
            }.into());
        }
        if high < open || high < close {
            return Err(DataError::ParseError {
                message: format!("Invalid OHLC: high ({}) < open ({}) or close ({})", high, open, close),
            }.into());
        }
        if low > open || low > close {
            return Err(DataError::ParseError {
                message: format!("Invalid OHLC: low ({}) > open ({}) or close ({})", low, open, close),
            }.into());
        }

        Ok(Bar::new(
            symbol.clone(),
            timestamp,
            open,
            high,
            low,
            close,
            volume,
            resolution,
        ))
    }

    /// Detect CSV column positions from headers
    fn detect_csv_columns(&self, headers: &csv::StringRecord) -> GbResult<(usize, usize, usize, usize, usize, usize)> {
        let mut timestamp_idx = None;
        let mut open_idx = None;
        let mut high_idx = None;
        let mut low_idx = None;
        let mut close_idx = None;
        let mut volume_idx = None;

        for (i, header) in headers.iter().enumerate() {
            let header_lower = header.to_lowercase();
            match header_lower.as_str() {
                "timestamp" | "date" | "datetime" | "time" => timestamp_idx = Some(i),
                "open" => open_idx = Some(i),
                "high" => high_idx = Some(i),
                "low" => low_idx = Some(i),
                "close" | "close_price" => close_idx = Some(i),
                "volume" | "vol" => volume_idx = Some(i),
                _ => {} // Ignore unknown columns
            }
        }

        let timestamp_idx = timestamp_idx.ok_or_else(|| DataError::ParseError {
            message: "Could not find timestamp column in CSV headers".to_string(),
        })?;
        let open_idx = open_idx.ok_or_else(|| DataError::ParseError {
            message: "Could not find open column in CSV headers".to_string(),
        })?;
        let high_idx = high_idx.ok_or_else(|| DataError::ParseError {
            message: "Could not find high column in CSV headers".to_string(),
        })?;
        let low_idx = low_idx.ok_or_else(|| DataError::ParseError {
            message: "Could not find low column in CSV headers".to_string(),
        })?;
        let close_idx = close_idx.ok_or_else(|| DataError::ParseError {
            message: "Could not find close column in CSV headers".to_string(),
        })?;
        let volume_idx = volume_idx.ok_or_else(|| DataError::ParseError {
            message: "Could not find volume column in CSV headers".to_string(),
        })?;

        Ok((timestamp_idx, open_idx, high_idx, low_idx, close_idx, volume_idx))
    }

    /// Parse a timestamp string into DateTime<Utc>
    fn parse_timestamp(&self, timestamp_str: &str) -> GbResult<chrono::DateTime<chrono::Utc>> {
        use chrono::{DateTime, Utc, NaiveDateTime};
        
        // Try parsing as date-only first
        if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(timestamp_str, "%Y-%m-%d") {
            // Convert to datetime at market open (9:30 AM EST = 14:30 UTC)
            if let Some(naive_dt) = naive_date.and_hms_opt(14, 30, 0) {
                return Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc));
            }
        }

        // Try multiple timestamp formats
        let formats = [
            "%Y-%m-%d %H:%M:%S",      // 2023-01-01 10:30:00
            "%Y/%m/%d %H:%M:%S",      // 2023/01/01 10:30:00
            "%Y/%m/%d",               // 2023/01/01
            "%m/%d/%Y %H:%M:%S",      // 01/01/2023 10:30:00
            "%m/%d/%Y",               // 01/01/2023
            "%Y-%m-%dT%H:%M:%S",      // 2023-01-01T10:30:00
            "%Y-%m-%dT%H:%M:%SZ",     // 2023-01-01T10:30:00Z
        ];

        for format in &formats {
            if let Ok(naive_dt) = NaiveDateTime::parse_from_str(timestamp_str, format) {
                return Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc));
            }
        }

        // Try parsing as Unix timestamp
        if let Ok(timestamp) = timestamp_str.parse::<i64>() {
            if let Some(dt) = DateTime::from_timestamp(timestamp, 0) {
                return Ok(dt);
            }
        }

        Err(DataError::ParseError {
            message: format!("Could not parse timestamp: {}", timestamp_str),
        }.into())
    }

    /// Parse a decimal value from string
    fn parse_decimal(&self, value_str: &str, field_name: &str) -> GbResult<rust_decimal::Decimal> {
        use rust_decimal::Decimal;
        
        if value_str.is_empty() {
            return Err(DataError::ParseError {
                message: format!("Empty value for field: {}", field_name),
            }.into());
        }

        value_str.parse::<Decimal>()
            .map_err(|e| DataError::ParseError {
                message: format!("Could not parse {} value '{}': {}", field_name, value_str, e),
            }.into())
    }
    
    /*/// Convert Polars DataFrame to Bar structs
    fn dataframe_to_bars(
        &self,
        df: DataFrame,
        symbol: &Symbol,
        resolution: Resolution,
    ) -> GbResult<Vec<Bar>> {
        let mut bars = Vec::new();
        
        // Get column indices
        let timestamp_col = self.find_timestamp_column(&df)?;
        let open_col = self.find_column(&df, &["open", "Open", "OPEN"])?;
        let high_col = self.find_column(&df, &["high", "High", "HIGH"])?;
        let low_col = self.find_column(&df, &["low", "Low", "LOW"])?;
        let close_col = self.find_column(&df, &["close", "Close", "CLOSE"])?;
        let volume_col = self.find_column(&df, &["volume", "Volume", "VOLUME", "vol", "Vol"])?;
        
        let num_rows = df.height();
        
        for i in 0..num_rows {
            let timestamp = self.extract_timestamp(&df, timestamp_col, i)?;
            let open = self.extract_decimal(&df, open_col, i)?;
            let high = self.extract_decimal(&df, high_col, i)?;
            let low = self.extract_decimal(&df, low_col, i)?;
            let close = self.extract_decimal(&df, close_col, i)?;
            let volume = self.extract_decimal(&df, volume_col, i)?;
            
            let bar = Bar::new(
                symbol.clone(),
                timestamp,
                open,
                high,
                low,
                close,
                volume,
                resolution,
            );
            
            bars.push(bar);
        }
        
        // Sort by timestamp
        bars.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        
        Ok(bars)
    }*/
    
    /*fn find_timestamp_column(&self, df: &DataFrame) -> GbResult<usize> {
        let candidates = ["timestamp", "Timestamp", "TIMESTAMP", "date", "Date", "DATE", "time", "Time"];
        
        for (i, col) in df.get_column_names().iter().enumerate() {
            if candidates.contains(col) {
                return Ok(i);
            }
        }
        
        // Try index 0 if no named timestamp column
        if !df.get_column_names().is_empty() {
            Ok(0)
        } else {
            Err(DataError::InvalidFormat {
                message: "No timestamp column found".to_string(),
            }.into())
        }
    }
    
    fn find_column(&self, df: &DataFrame, candidates: &[&str]) -> GbResult<usize> {
        for (i, col) in df.get_column_names().iter().enumerate() {
            if candidates.contains(col) {
                return Ok(i);
            }
        }
        
        Err(DataError::InvalidFormat {
            message: format!("Column not found, candidates: {:?}", candidates),
        }.into())
    }
    
    fn extract_timestamp(&self, df: &DataFrame, col_idx: usize, row_idx: usize) -> GbResult<DateTime<Utc>> {
        let col = df.get_columns().get(col_idx)
            .ok_or_else(|| DataError::InvalidFormat {
                message: "Invalid column index".to_string(),
            })?;
        
        match col.dtype() {
            DataType::Datetime(_, _) => {
                if let Ok(datetime_chunked) = col.datetime() {
                    if let Some(value) = datetime_chunked.get(row_idx) {
                        // Convert from nanoseconds since epoch
                        let timestamp = DateTime::from_timestamp(value / 1_000_000_000, (value % 1_000_000_000) as u32)
                            .unwrap_or_default()
                            .and_utc();
                        Ok(timestamp)
                    } else {
                        Err(DataError::InvalidFormat {
                            message: "Null timestamp value".to_string(),
                        }.into())
                    }
                } else {
                    Err(DataError::InvalidFormat {
                        message: "Invalid datetime column".to_string(),
                    }.into())
                }
            }
            DataType::String => {
                if let Ok(string_chunked) = col.str() {
                    if let Some(date_str) = string_chunked.get(row_idx) {
                        self.parse_timestamp_string(date_str)
                    } else {
                        Err(DataError::InvalidFormat {
                            message: "Null timestamp string".to_string(),
                        }.into())
                    }
                } else {
                    Err(DataError::InvalidFormat {
                        message: "Invalid string column".to_string(),
                    }.into())
                }
            }
            _ => Err(DataError::InvalidFormat {
                message: format!("Unsupported timestamp column type: {:?}", col.dtype()),
            }.into())
        }
    }
    
    fn parse_timestamp_string(&self, date_str: &str) -> GbResult<DateTime<Utc>> {
        // Try multiple timestamp formats
        let formats = [
            "%Y-%m-%d",
            "%Y-%m-%d %H:%M:%S",
            "%Y-%m-%d %H:%M:%S%.f",
            "%Y-%m-%dT%H:%M:%S",
            "%Y-%m-%dT%H:%M:%S%.f",
            "%Y-%m-%dT%H:%M:%SZ",
            "%Y-%m-%dT%H:%M:%S%.fZ",
        ];
        
        for format in &formats {
            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(date_str, format) {
                return Ok(dt.and_utc());
            }
            if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, format) {
                return Ok(date.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc());
            }
        }
        
        // Try RFC3339 parsing as fallback
        if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
            return Ok(dt.with_timezone(&Utc));
        }
        
        Err(DataError::ParseError {
            message: format!("Unable to parse timestamp: {}", date_str),
        }.into())
    }
    
    fn extract_decimal(&self, df: &DataFrame, col_idx: usize, row_idx: usize) -> GbResult<Decimal> {
        let col = df.get_columns().get(col_idx)
            .ok_or_else(|| DataError::InvalidFormat {
                message: "Invalid column index".to_string(),
            })?;
        
        match col.dtype() {
            DataType::Float64 => {
                if let Ok(float_chunked) = col.f64() {
                    if let Some(value) = float_chunked.get(row_idx) {
                        Ok(Decimal::from_f64_retain(value).unwrap_or_default())
                    } else {
                        Ok(Decimal::ZERO)
                    }
                } else {
                    Err(DataError::InvalidFormat {
                        message: "Invalid float64 column".to_string(),
                    }.into())
                }
            }
            DataType::Float32 => {
                if let Ok(float_chunked) = col.f32() {
                    if let Some(value) = float_chunked.get(row_idx) {
                        Ok(Decimal::from_f32_retain(value).unwrap_or_default())
                    } else {
                        Ok(Decimal::ZERO)
                    }
                } else {
                    Err(DataError::InvalidFormat {
                        message: "Invalid float32 column".to_string(),
                    }.into())
                }
            }
            DataType::Int64 => {
                if let Ok(int_chunked) = col.i64() {
                    if let Some(value) = int_chunked.get(row_idx) {
                        Ok(Decimal::from(value))
                    } else {
                        Ok(Decimal::ZERO)
                    }
                } else {
                    Err(DataError::InvalidFormat {
                        message: "Invalid int64 column".to_string(),
                    }.into())
                }
            }
            DataType::String => {
                if let Ok(string_chunked) = col.str() {
                    if let Some(value_str) = string_chunked.get(row_idx) {
                        value_str.parse::<Decimal>()
                            .map_err(|e| DataError::ParseError {
                                message: format!("Failed to parse decimal: {}", e),
                            }.into())
                    } else {
                        Ok(Decimal::ZERO)
                    }
                } else {
                    Err(DataError::InvalidFormat {
                        message: "Invalid string column".to_string(),
                    }.into())
                }
            }
            _ => Err(DataError::InvalidFormat {
                message: format!("Unsupported numeric column type: {:?}", col.dtype()),
            }.into())
        }
    }*/
}

impl Default for BatchLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility functions for data loading
pub struct DataLoaderUtils;

impl DataLoaderUtils {
    /// Detect file format from extension
    pub fn detect_format<P: AsRef<Path>>(file_path: P) -> Option<DataFormat> {
        let path = file_path.as_ref();
        let extension = path.extension()?.to_str()?;
        
        match extension.to_lowercase().as_str() {
            "csv" => Some(DataFormat::Csv),
            "parquet" => Some(DataFormat::Parquet),
            "json" => Some(DataFormat::Json),
            "jsonl" | "ndjson" => Some(DataFormat::JsonLines),
            _ => None,
        }
    }
    
    /// Create symbol from file path pattern
    pub fn symbol_from_path<P: AsRef<Path>>(
        file_path: P,
        default_exchange: &str,
        default_asset_class: AssetClass,
    ) -> Symbol {
        let path = file_path.as_ref();
        let file_stem = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("UNKNOWN");
        
        // Try to extract symbol from filename
        let symbol_name = file_stem.split('_').next()
            .unwrap_or(file_stem)
            .to_uppercase();
        
        Symbol::new(&symbol_name, default_exchange, default_asset_class)
    }
    
    /// Validate data consistency
    pub fn validate_bars(bars: &[Bar]) -> Vec<String> {
        let mut issues = Vec::new();
        
        if bars.is_empty() {
            issues.push("No data found".to_string());
            return issues;
        }
        
        // Check for negative prices
        for (i, bar) in bars.iter().enumerate() {
            if bar.open < Decimal::ZERO || bar.high < Decimal::ZERO 
                || bar.low < Decimal::ZERO || bar.close < Decimal::ZERO {
                issues.push(format!("Negative price at row {}", i));
            }
            
            if bar.volume < Decimal::ZERO {
                issues.push(format!("Negative volume at row {}", i));
            }
            
            if bar.high < bar.low {
                issues.push(format!("High < Low at row {}", i));
            }
            
            if bar.high < bar.open || bar.high < bar.close {
                issues.push(format!("High price inconsistent at row {}", i));
            }
            
            if bar.low > bar.open || bar.low > bar.close {
                issues.push(format!("Low price inconsistent at row {}", i));
            }
        }
        
        // Check for timestamp ordering
        let mut prev_timestamp = bars[0].timestamp;
        for (i, bar) in bars.iter().enumerate().skip(1) {
            if bar.timestamp < prev_timestamp {
                issues.push(format!("Timestamp out of order at row {}", i));
            }
            prev_timestamp = bar.timestamp;
        }
        
        issues
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DataFormat {
    Csv,
    Parquet,
    Json,
    JsonLines,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::StorageManager;
    use tempfile::{NamedTempFile, TempDir};
    use std::io::Write;
    
    #[tokio::test]
    async fn test_csv_loading() {
        let loader = BatchLoader::new();
        let symbol = Symbol::equity("AAPL");
        
        // Create a temporary CSV file with test data
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "date,open,high,low,close,volume").unwrap();
        writeln!(temp_file, "2023-01-01,100.0,105.0,98.0,102.0,10000").unwrap();
        writeln!(temp_file, "2023-01-02,102.0,107.0,101.0,105.0,15000").unwrap();
        temp_file.flush().unwrap();
        
        let bars = loader.load_csv_file(temp_file.path(), &symbol, Resolution::Day, true).await.unwrap();
        assert_eq!(bars.len(), 2);
        
        // Verify first bar
        let bar1 = &bars[0];
        assert_eq!(bar1.symbol, symbol);
        assert_eq!(bar1.open, rust_decimal::Decimal::from(100));
        assert_eq!(bar1.high, rust_decimal::Decimal::from(105));
        assert_eq!(bar1.low, rust_decimal::Decimal::from(98));
        assert_eq!(bar1.close, rust_decimal::Decimal::from(102));
        assert_eq!(bar1.volume, rust_decimal::Decimal::from(10000));
        assert_eq!(bar1.resolution, Resolution::Day);
        
        // Verify second bar
        let bar2 = &bars[1];
        assert_eq!(bar2.open, rust_decimal::Decimal::from(102));
        assert_eq!(bar2.high, rust_decimal::Decimal::from(107));
        assert_eq!(bar2.low, rust_decimal::Decimal::from(101));
        assert_eq!(bar2.close, rust_decimal::Decimal::from(105));
        assert_eq!(bar2.volume, rust_decimal::Decimal::from(15000));
    }

    #[tokio::test]
    async fn test_parquet_loading() {
        let loader = BatchLoader::new();
        let symbol = Symbol::equity("TSLA");
        
        // Create test data
        let test_bars = vec![
            Bar::new(
                symbol.clone(),
                "2023-06-01T14:30:00Z".parse().unwrap(),
                Decimal::from(250),
                Decimal::from(255),
                Decimal::from(248),
                Decimal::from(252),
                Decimal::from(50000),
                Resolution::Day,
            ),
            Bar::new(
                symbol.clone(),
                "2023-06-02T14:30:00Z".parse().unwrap(),
                Decimal::from(252),
                Decimal::from(258),
                Decimal::from(250),
                Decimal::from(256),
                Decimal::from(75000),
                Resolution::Day,
            ),
            Bar::new(
                symbol.clone(),
                "2023-06-03T14:30:00Z".parse().unwrap(),
                Decimal::from(256),
                Decimal::from(262),
                Decimal::from(254),
                Decimal::from(260),
                Decimal::from(60000),
                Resolution::Day,
            ),
        ];

        // Create temporary directory for storage
        let temp_dir = TempDir::new().unwrap();
        let storage = StorageManager::new(temp_dir.path()).unwrap();

        // Save bars to Parquet file using storage
        storage.save_bars(&symbol, &test_bars, Resolution::Day).await.unwrap();

        // Get the expected Parquet file path (Resolution::Day formats as "1d")
        let storage_path = temp_dir.path()
            .join("NASDAQ")   // exchange
            .join("Equity")   // asset class (Debug format)
            .join("TSLA")     // symbol
            .join("1d.parquet"); // Resolution::Day formats as "1d"

        // Verify the file was created
        assert!(storage_path.exists(), "Parquet file should exist at: {:?}", storage_path);

        // Load bars using the Parquet loader
        let loaded_bars = loader.load_parquet_file(&storage_path, &symbol, Resolution::Day).await.unwrap();

        // Verify the round-trip worked correctly
        assert_eq!(loaded_bars.len(), test_bars.len());

        for (loaded, original) in loaded_bars.iter().zip(test_bars.iter()) {
            assert_eq!(loaded.symbol, original.symbol);
            assert_eq!(loaded.timestamp, original.timestamp);
            assert_eq!(loaded.open, original.open);
            assert_eq!(loaded.high, original.high);
            assert_eq!(loaded.low, original.low);
            assert_eq!(loaded.close, original.close);
            assert_eq!(loaded.volume, original.volume);
            assert_eq!(loaded.resolution, original.resolution);
        }

        tracing::info!("Parquet round-trip test completed successfully: {} bars", loaded_bars.len());
    }

    #[tokio::test]
    async fn test_parquet_loading_nonexistent_file() {
        let loader = BatchLoader::new();
        let symbol = Symbol::equity("NONEXISTENT");
        
        let result = loader.load_parquet_file("/path/that/does/not/exist.parquet", &symbol, Resolution::Day).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            gb_types::GbError::Data(DataError::SymbolNotFound { .. }) => {
                // Expected error type
            }
            other => panic!("Expected SymbolNotFound error, got: {:?}", other),
        }
    }
} 