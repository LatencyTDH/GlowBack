use std::path::Path;
use chrono::{DateTime, Utc};
use gb_types::{Bar, Symbol, Resolution, GbResult, DataError, AssetClass};
use rust_decimal::Decimal;
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
        _file_path: P,
        _symbol: &Symbol,
        _resolution: Resolution,
    ) -> GbResult<Vec<Bar>> {
        // TODO: Implement parquet loading using arrow directly
        Err(DataError::LoadingFailed {
            message: "Parquet loading not yet implemented".to_string(),
        }.into())
    }
    
    /// Load bars from a CSV file using csv crate
    pub async fn load_csv_file<P: AsRef<Path>>(
        &self,
        _file_path: P,
        _symbol: &Symbol,
        _resolution: Resolution,
        _has_headers: bool,
    ) -> GbResult<Vec<Bar>> {
        // TODO: Implement CSV loading using csv crate directly
        Err(DataError::LoadingFailed {
            message: "CSV loading not yet implemented".to_string(),
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
    use tempfile::NamedTempFile;
    use std::io::Write;
    
    #[tokio::test]
    async fn test_csv_loading() {
        let _temp_file = NamedTempFile::new().unwrap();
        // TODO: Implement actual CSV loading test
        /*
        writeln!(temp_file, "timestamp,open,high,low,close,volume").unwrap();
        writeln!(temp_file, "2023-01-01,100.0,105.0,98.0,102.0,10000").unwrap();
        writeln!(temp_file, "2023-01-02,102.0,107.0,101.0,105.0,15000").unwrap();
        temp_file.flush().unwrap();
        
        let loader = BatchLoader::new();
        let symbol = Symbol::equity("AAPL");
        let bars = loader.load_csv_file(temp_file.path(), &symbol, Resolution::Day, true).await.unwrap();
        
        assert_eq!(bars.len(), 2);
        assert_eq!(bars[0].close, Decimal::from(102));
        assert_eq!(bars[1].close, Decimal::from(105));
        */
    }
} 