// TODO: Re-enable when Arrow compatibility issues are resolved - RESOLVED!
use arrow::array::{
    Array, ArrayRef, Decimal128Array, Int64Array, StringArray, TimestampNanosecondArray,
};
use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use arrow::record_batch::RecordBatch;
use chrono::{DateTime, Utc};
use gb_types::{Bar, DataError, GbResult, Resolution, Symbol};
use parquet::arrow::{arrow_reader::ParquetRecordBatchReaderBuilder, ArrowWriter};
use parquet::file::properties::WriterProperties;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

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
        symbol: &Symbol,
        bars: &[Bar],
        resolution: Resolution,
    ) -> GbResult<()> {
        let storage_path = self.get_storage_path(symbol, resolution);

        // Ensure parent directory exists
        if let Some(parent) = storage_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let existing_bars = if storage_path.exists() {
            Self::load_all_bars_from_path(&storage_path, symbol, resolution)?
        } else {
            Vec::new()
        };

        let merged_bars = Self::merge_bars(existing_bars, bars);
        Self::write_bars_atomically(&storage_path, &merged_bars)?;

        tracing::info!(
            "Saved {} merged bars ({} new) to {}",
            merged_bars.len(),
            bars.len(),
            storage_path.display()
        );
        Ok(())
    }

    /// Load bars from Parquet file
    pub async fn load_bars(
        &self,
        symbol: &Symbol,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        resolution: Resolution,
    ) -> GbResult<Vec<Bar>> {
        let storage_path = self.get_storage_path(symbol, resolution);

        if !storage_path.exists() {
            return Err(DataError::SymbolNotFound {
                symbol: symbol.to_string(),
            }
            .into());
        }

        let mut bars = Self::load_all_bars_from_path(&storage_path, symbol, resolution)?;
        bars.retain(|bar| bar.timestamp >= start_date && bar.timestamp <= end_date);

        tracing::info!("Loaded {} bars from {}", bars.len(), storage_path.display());
        Ok(bars)
    }

    fn load_all_bars_from_path(
        storage_path: &Path,
        symbol: &Symbol,
        resolution: Resolution,
    ) -> GbResult<Vec<Bar>> {
        let file = fs::File::open(storage_path)?;
        let reader = ParquetRecordBatchReaderBuilder::try_new(file)
            .map_err(|e| DataError::LoadingFailed {
                message: e.to_string(),
            })?
            .build()
            .map_err(|e| DataError::LoadingFailed {
                message: e.to_string(),
            })?;

        let mut bars = Vec::new();

        for batch_result in reader {
            let batch = batch_result.map_err(|e| DataError::LoadingFailed {
                message: e.to_string(),
            })?;
            bars.extend(Self::record_batch_to_bars(&batch, symbol, resolution)?);
        }

        Ok(bars)
    }

    fn merge_bars(existing_bars: Vec<Bar>, new_bars: &[Bar]) -> Vec<Bar> {
        let mut merged = BTreeMap::new();

        for bar in existing_bars {
            merged.insert(bar.timestamp, bar);
        }

        for bar in new_bars {
            merged.insert(bar.timestamp, bar.clone());
        }

        merged.into_values().collect()
    }

    fn write_bars_atomically(storage_path: &Path, bars: &[Bar]) -> GbResult<()> {
        let temp_path = storage_path.with_extension("parquet.tmp");
        let result = Self::write_bars_to_path(&temp_path, bars);

        if let Err(err) = result {
            let _ = fs::remove_file(&temp_path);
            return Err(err);
        }

        fs::rename(&temp_path, storage_path)?;
        Ok(())
    }

    fn write_bars_to_path(storage_path: &Path, bars: &[Bar]) -> GbResult<()> {
        let schema = Self::get_schema();
        let writer = ArrowWriter::try_new(
            fs::File::create(storage_path)?,
            schema,
            Some(WriterProperties::builder().build()),
        )
        .map_err(|e| DataError::LoadingFailed {
            message: e.to_string(),
        })?;

        let record_batch = Self::bars_to_record_batch(bars)?;
        let mut writer = writer;
        writer
            .write(&record_batch)
            .map_err(|e| DataError::LoadingFailed {
                message: e.to_string(),
            })?;
        writer.close().map_err(|e| DataError::LoadingFailed {
            message: e.to_string(),
        })?;

        Ok(())
    }

    /// Convert bars to Arrow RecordBatch
    fn bars_to_record_batch(bars: &[Bar]) -> GbResult<RecordBatch> {
        let schema = Self::get_schema();

        let symbols: Vec<String> = bars.iter().map(|b| b.symbol.to_string()).collect();
        let timestamps: Vec<i64> = bars
            .iter()
            .map(|b| b.timestamp.timestamp_nanos_opt().unwrap_or(0))
            .collect();
        let opens: Vec<i128> = bars
            .iter()
            .map(|b| (b.open * Decimal::from(10000)).to_i128().unwrap_or(0))
            .collect();
        let highs: Vec<i128> = bars
            .iter()
            .map(|b| (b.high * Decimal::from(10000)).to_i128().unwrap_or(0))
            .collect();
        let lows: Vec<i128> = bars
            .iter()
            .map(|b| (b.low * Decimal::from(10000)).to_i128().unwrap_or(0))
            .collect();
        let closes: Vec<i128> = bars
            .iter()
            .map(|b| (b.close * Decimal::from(10000)).to_i128().unwrap_or(0))
            .collect();
        let volumes: Vec<i64> = bars
            .iter()
            .map(|b| b.volume.to_i64().unwrap_or(0))
            .collect();

        let arrays: Vec<ArrayRef> = vec![
            Arc::new(StringArray::from(symbols)),
            Arc::new(TimestampNanosecondArray::from(timestamps).with_timezone("UTC")),
            Arc::new(
                Decimal128Array::from(opens)
                    .with_precision_and_scale(18, 4)
                    .map_err(|e| DataError::InvalidFormat {
                        message: e.to_string(),
                    })?,
            ),
            Arc::new(
                Decimal128Array::from(highs)
                    .with_precision_and_scale(18, 4)
                    .map_err(|e| DataError::InvalidFormat {
                        message: e.to_string(),
                    })?,
            ),
            Arc::new(
                Decimal128Array::from(lows)
                    .with_precision_and_scale(18, 4)
                    .map_err(|e| DataError::InvalidFormat {
                        message: e.to_string(),
                    })?,
            ),
            Arc::new(
                Decimal128Array::from(closes)
                    .with_precision_and_scale(18, 4)
                    .map_err(|e| DataError::InvalidFormat {
                        message: e.to_string(),
                    })?,
            ),
            Arc::new(Int64Array::from(volumes)),
        ];

        let batch = RecordBatch::try_new(schema, arrays).map_err(|e| DataError::InvalidFormat {
            message: e.to_string(),
        })?;
        Ok(batch)
    }

    /// Convert Arrow RecordBatch to bars
    fn record_batch_to_bars(
        batch: &RecordBatch,
        symbol: &Symbol,
        resolution: Resolution,
    ) -> GbResult<Vec<Bar>> {
        let timestamps = batch
            .column(1)
            .as_any()
            .downcast_ref::<TimestampNanosecondArray>()
            .ok_or_else(|| DataError::Corruption {
                message: "Invalid timestamp column".to_string(),
            })?;

        let opens = batch
            .column(2)
            .as_any()
            .downcast_ref::<Decimal128Array>()
            .ok_or_else(|| DataError::Corruption {
                message: "Invalid open column".to_string(),
            })?;

        let highs = batch
            .column(3)
            .as_any()
            .downcast_ref::<Decimal128Array>()
            .ok_or_else(|| DataError::Corruption {
                message: "Invalid high column".to_string(),
            })?;

        let lows = batch
            .column(4)
            .as_any()
            .downcast_ref::<Decimal128Array>()
            .ok_or_else(|| DataError::Corruption {
                message: "Invalid low column".to_string(),
            })?;

        let closes = batch
            .column(5)
            .as_any()
            .downcast_ref::<Decimal128Array>()
            .ok_or_else(|| DataError::Corruption {
                message: "Invalid close column".to_string(),
            })?;

        let volumes = batch
            .column(6)
            .as_any()
            .downcast_ref::<Int64Array>()
            .ok_or_else(|| DataError::Corruption {
                message: "Invalid volume column".to_string(),
            })?;

        let mut bars = Vec::new();

        for i in 0..batch.num_rows() {
            if timestamps.is_null(i)
                || opens.is_null(i)
                || highs.is_null(i)
                || lows.is_null(i)
                || closes.is_null(i)
                || volumes.is_null(i)
            {
                continue;
            }

            let timestamp_nanos = timestamps.value(i);
            let timestamp = DateTime::from_timestamp(
                timestamp_nanos / 1_000_000_000,
                (timestamp_nanos % 1_000_000_000) as u32,
            )
            .unwrap_or_default();

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

    /// Get the Arrow schema for bar data
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
    }

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

            let exchange = exchange_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            for asset_class_entry in std::fs::read_dir(&exchange_path)? {
                let asset_class_path = asset_class_entry?.path();
                if !asset_class_path.is_dir() {
                    continue;
                }

                let asset_class_str = asset_class_path
                    .file_name()
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

                    let symbol_name = symbol_path
                        .file_name()
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
    use chrono::TimeZone;
    use gb_types::{AssetClass, Resolution};
    use tempfile::tempdir;

    fn sample_bar(symbol: &Symbol, day: u32, close: i64) -> Bar {
        let timestamp = Utc.with_ymd_and_hms(2026, 3, day, 0, 0, 0).unwrap();
        Bar::new(
            symbol.clone(),
            timestamp,
            Decimal::from(close - 1),
            Decimal::from(close + 1),
            Decimal::from(close - 2),
            Decimal::from(close),
            Decimal::from(10_000),
            Resolution::Day,
        )
    }

    #[tokio::test]
    async fn test_storage_roundtrip() {
        let temp_dir = tempdir().unwrap();
        let storage = StorageManager::new(temp_dir.path()).unwrap();

        let symbol = Symbol::new("AAPL", "NASDAQ", AssetClass::Equity);
        let bars = vec![sample_bar(&symbol, 10, 102)];

        storage
            .save_bars(&symbol, &bars, Resolution::Day)
            .await
            .unwrap();

        let start = Utc.with_ymd_and_hms(2026, 3, 9, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 3, 11, 0, 0, 0).unwrap();
        let loaded_bars = storage
            .load_bars(&symbol, start, end, Resolution::Day)
            .await
            .unwrap();

        assert_eq!(loaded_bars, bars);
    }

    #[tokio::test]
    async fn test_save_bars_merges_non_overlapping_history() {
        let temp_dir = tempdir().unwrap();
        let storage = StorageManager::new(temp_dir.path()).unwrap();

        let symbol = Symbol::new("AAPL", "NASDAQ", AssetClass::Equity);
        let first_batch = vec![sample_bar(&symbol, 10, 100)];
        let second_batch = vec![sample_bar(&symbol, 11, 200)];

        storage
            .save_bars(&symbol, &first_batch, Resolution::Day)
            .await
            .unwrap();
        storage
            .save_bars(&symbol, &second_batch, Resolution::Day)
            .await
            .unwrap();

        let start = Utc.with_ymd_and_hms(2026, 3, 10, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 3, 11, 23, 59, 59).unwrap();
        let loaded_bars = storage
            .load_bars(&symbol, start, end, Resolution::Day)
            .await
            .unwrap();

        assert_eq!(loaded_bars.len(), 2);
        assert_eq!(loaded_bars[0], first_batch[0]);
        assert_eq!(loaded_bars[1], second_batch[0]);
    }

    #[tokio::test]
    async fn test_save_bars_dedupes_overlapping_history_by_timestamp() {
        let temp_dir = tempdir().unwrap();
        let storage = StorageManager::new(temp_dir.path()).unwrap();

        let symbol = Symbol::new("AAPL", "NASDAQ", AssetClass::Equity);
        let original = vec![sample_bar(&symbol, 10, 100), sample_bar(&symbol, 11, 200)];
        let replacement = vec![sample_bar(&symbol, 11, 250), sample_bar(&symbol, 12, 300)];

        storage
            .save_bars(&symbol, &original, Resolution::Day)
            .await
            .unwrap();
        storage
            .save_bars(&symbol, &replacement, Resolution::Day)
            .await
            .unwrap();

        let start = Utc.with_ymd_and_hms(2026, 3, 10, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 3, 12, 23, 59, 59).unwrap();
        let loaded_bars = storage
            .load_bars(&symbol, start, end, Resolution::Day)
            .await
            .unwrap();

        assert_eq!(loaded_bars.len(), 3);
        assert_eq!(loaded_bars[0], original[0]);
        assert_eq!(loaded_bars[1], replacement[0]);
        assert_eq!(loaded_bars[2], replacement[1]);
    }
}
