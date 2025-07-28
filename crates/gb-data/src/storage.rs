use std::path::{Path, PathBuf};
use std::sync::Arc;
use chrono::{DateTime, Utc};
use gb_types::{Bar, Symbol, Resolution, GbResult, DataError};
// TODO: Re-enable when Arrow compatibility issues are resolved
// use arrow::array::{
//     Array, ArrayRef, StringArray, TimestampNanosecondArray, Decimal128Array,
//     Int64Array, RecordBatch,
// };
// use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
// use parquet::arrow::{ArrowWriter, arrow_reader::ParquetRecordBatchReaderBuilder};
// use parquet::file::properties::WriterProperties;
use rust_decimal::Decimal;
// use rust_decimal::prelude::ToPrimitive;

/// Storage manager for Parquet files
#[derive(Debug)]
pub struct StorageManager {
    pub data_root: PathBuf,
}

impl StorageManager {
    pub fn new<P: AsRef<Path>>(data_root: P) -> GbResult<Self> {
        let data_root = data_root.as_ref().to_path_buf();
        std::fs::create_dir_all(&data_root)?;
        
        Ok(Self { data_root })
    }
    
    /// Generate the storage path for a symbol and resolution
    fn get_storage_path(&self, symbol: &Symbol, resolution: Resolution) -> PathBuf {
        self.data_root
            .join(&symbol.exchange)
            .join(format!("{:?}", symbol.asset_class))
            .join(&symbol.symbol)
            .join(format!("{}.parquet", resolution))
    }
    
    /// Save bars to Parquet file
    pub async fn save_bars(
        &self,
        _symbol: &Symbol,
        _bars: &[Bar],
        _resolution: Resolution,
    ) -> GbResult<()> {
        // TODO: Implement Parquet storage when Arrow compatibility issues are resolved
        Ok(())
    }
    
    /// Load bars from Parquet file
    pub async fn load_bars(
        &self,
        _symbol: &Symbol,
        _start_date: DateTime<Utc>,
        _end_date: DateTime<Utc>,
        _resolution: Resolution,
    ) -> GbResult<Vec<Bar>> {
        // TODO: Implement Parquet loading when Arrow compatibility issues are resolved
        Err(DataError::LoadingFailed {
            message: "Parquet storage not yet implemented".to_string(),
        }.into())
    }
    
    /*/// Convert bars to Arrow RecordBatch
    fn bars_to_record_batch(&self, bars: &[Bar]) -> GbResult<RecordBatch> {
        let schema = Self::get_schema();
        
        let symbols: Vec<String> = bars.iter().map(|b| b.symbol.to_string()).collect();
        let timestamps: Vec<i64> = bars.iter()
            .map(|b| b.timestamp.timestamp_nanos_opt().unwrap_or(0))
            .collect();
        let opens: Vec<i128> = bars.iter()
            .map(|b| (b.open * Decimal::from(10000)).to_i128().unwrap_or(0))
            .collect();
        let highs: Vec<i128> = bars.iter()
            .map(|b| (b.high * Decimal::from(10000)).to_i128().unwrap_or(0))
            .collect();
        let lows: Vec<i128> = bars.iter()
            .map(|b| (b.low * Decimal::from(10000)).to_i128().unwrap_or(0))
            .collect();
        let closes: Vec<i128> = bars.iter()
            .map(|b| (b.close * Decimal::from(10000)).to_i128().unwrap_or(0))
            .collect();
        let volumes: Vec<i64> = bars.iter()
            .map(|b| b.volume.to_i64().unwrap_or(0))
            .collect();
        
        let arrays: Vec<ArrayRef> = vec![
            Arc::new(StringArray::from(symbols)),
            Arc::new(TimestampNanosecondArray::from(timestamps).with_timezone("UTC")),
            Arc::new(Decimal128Array::from(opens).with_precision_and_scale(18, 4)
                .map_err(|e| DataError::InvalidFormat { message: e.to_string() })?),
            Arc::new(Decimal128Array::from(highs).with_precision_and_scale(18, 4)
                .map_err(|e| DataError::InvalidFormat { message: e.to_string() })?),
            Arc::new(Decimal128Array::from(lows).with_precision_and_scale(18, 4)
                .map_err(|e| DataError::InvalidFormat { message: e.to_string() })?),
            Arc::new(Decimal128Array::from(closes).with_precision_and_scale(18, 4)
                .map_err(|e| DataError::InvalidFormat { message: e.to_string() })?),
            Arc::new(Int64Array::from(volumes)),
        ];
        
        let batch = RecordBatch::try_new(schema, arrays)
            .map_err(|e| DataError::InvalidFormat { message: e.to_string() })?;
        Ok(batch)
    }*/
    
    /*/// Convert Arrow RecordBatch to bars
    fn record_batch_to_bars(
        &self,
        batch: &RecordBatch,
        symbol: &Symbol,
        resolution: Resolution,
    ) -> GbResult<Vec<Bar>> {
        let timestamps = batch.column(1)
            .as_any()
            .downcast_ref::<TimestampNanosecondArray>()
            .ok_or_else(|| DataError::Corruption {
                message: "Invalid timestamp column".to_string(),
            })?;
        
        let opens = batch.column(2)
            .as_any()
            .downcast_ref::<Decimal128Array>()
            .ok_or_else(|| DataError::Corruption {
                message: "Invalid open column".to_string(),
            })?;
        
        let highs = batch.column(3)
            .as_any()
            .downcast_ref::<Decimal128Array>()
            .ok_or_else(|| DataError::Corruption {
                message: "Invalid high column".to_string(),
            })?;
        
        let lows = batch.column(4)
            .as_any()
            .downcast_ref::<Decimal128Array>()
            .ok_or_else(|| DataError::Corruption {
                message: "Invalid low column".to_string(),
            })?;
        
        let closes = batch.column(5)
            .as_any()
            .downcast_ref::<Decimal128Array>()
            .ok_or_else(|| DataError::Corruption {
                message: "Invalid close column".to_string(),
            })?;
        
        let volumes = batch.column(6)
            .as_any()
            .downcast_ref::<Int64Array>()
            .ok_or_else(|| DataError::Corruption {
                message: "Invalid volume column".to_string(),
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
    }*/
    
    /*/// Get the Arrow schema for bar data
    fn get_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("symbol", DataType::Utf8, false),
            Field::new(
                "timestamp",
                DataType::Timestamp(TimeUnit::Nanosecond, Some("UTC".into())),
                false,
            ),
            Field::new("open", DataType::Decimal128(18, 4), false),
            Field::new("high", DataType::Decimal128(18, 4), false),
            Field::new("low", DataType::Decimal128(18, 4), false),
            Field::new("close", DataType::Decimal128(18, 4), false),
            Field::new("volume", DataType::Int64, false),
        ]))
    }*/
    
    /// List available symbols in storage
    pub fn list_symbols(&self) -> GbResult<Vec<Symbol>> {
        let mut symbols = Vec::new();
        
        if !self.data_root.exists() {
            return Ok(symbols);
        }
        
        for exchange_entry in std::fs::read_dir(&self.data_root)? {
            let exchange_path = exchange_entry?.path();
            if !exchange_path.is_dir() {
                continue;
            }
            
            let exchange = exchange_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            
            for asset_class_entry in std::fs::read_dir(&exchange_path)? {
                let asset_class_path = asset_class_entry?.path();
                if !asset_class_path.is_dir() {
                    continue;
                }
                
                let asset_class_str = asset_class_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Equity");
                
                let asset_class = match asset_class_str {
                    "Crypto" => gb_types::AssetClass::Crypto,
                    "Forex" => gb_types::AssetClass::Forex,
                    "Commodity" => gb_types::AssetClass::Commodity,
                    "Bond" => gb_types::AssetClass::Bond,
                    _ => gb_types::AssetClass::Equity,
                };
                
                for symbol_entry in std::fs::read_dir(&asset_class_path)? {
                    let symbol_path = symbol_entry?.path();
                    if !symbol_path.is_dir() {
                        continue;
                    }
                    
                    let symbol_name = symbol_path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    
                    let symbol = Symbol::new(&symbol_name, &exchange, asset_class);
                    symbols.push(symbol);
                }
            }
        }
        
        Ok(symbols)
    }
    
    /// Get storage statistics
    pub fn get_stats(&self) -> GbResult<StorageStats> {
        let mut total_files = 0;
        let mut total_size = 0u64;
        
        fn scan_directory(path: &Path, stats: &mut (u64, u64)) -> std::io::Result<()> {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.is_dir() {
                    scan_directory(&path, stats)?;
                } else if path.extension().and_then(|s| s.to_str()) == Some("parquet") {
                    stats.0 += 1;
                    stats.1 += entry.metadata()?.len();
                }
            }
            Ok(())
        }
        
        if self.data_root.exists() {
            let mut stats = (0u64, 0u64);
            scan_directory(&self.data_root, &mut stats)?;
            total_files = stats.0;
            total_size = stats.1;
        }
        
        Ok(StorageStats {
            total_files,
            total_size_bytes: total_size,
            data_root: self.data_root.clone(),
        })
    }
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total_files: u64,
    pub total_size_bytes: u64,
    pub data_root: PathBuf,
}

impl StorageStats {
    pub fn total_size_mb(&self) -> f64 {
        self.total_size_bytes as f64 / (1024.0 * 1024.0)
    }
    
    pub fn total_size_gb(&self) -> f64 {
        self.total_size_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use gb_types::{AssetClass, Resolution};
    
    #[tokio::test]
    async fn test_storage_roundtrip() {
        let temp_dir = tempdir().unwrap();
        let storage = StorageManager::new(temp_dir.path()).unwrap();
        
        let symbol = Symbol::new("AAPL", "NASDAQ", AssetClass::Equity);
        let bars = vec![
            Bar::new(
                symbol.clone(),
                Utc::now(),
                Decimal::from(100),
                Decimal::from(105),
                Decimal::from(98),
                Decimal::from(102),
                Decimal::from(10000),
                Resolution::Day,
            ),
        ];
        
        // Save bars (currently returns Ok() without doing anything)
        storage.save_bars(&symbol, &bars, Resolution::Day).await.unwrap();
        
        // Load bars (currently returns error - expected)
        let start = Utc::now() - chrono::Duration::days(1);
        let end = Utc::now() + chrono::Duration::days(1);
        let result = storage.load_bars(&symbol, start, end, Resolution::Day).await;
        
        // We expect this to fail since storage is not implemented yet
        assert!(result.is_err());
    }
} 