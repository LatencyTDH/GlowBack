pub mod cache;
pub mod catalog;
pub mod loaders;
pub mod providers;
pub mod sources;
pub mod storage;
pub mod validation;

pub use cache::*;
pub use catalog::*;
pub use loaders::*;
pub use providers::*;
pub use sources::*;
pub use storage::*;
pub use validation::*;

use gb_types::{DataValidationSummary, DatasetKind, GbResult, PriceAdjustmentMode};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Data manager coordinates all data operations
#[derive(Debug)]
pub struct DataManager {
    pub catalog: catalog::DataCatalog,
    pub storage: storage::StorageManager,
    pub cache: cache::CacheManager,
    pub providers: Vec<Box<dyn providers::DataProvider>>,
}

impl DataManager {
    pub async fn new() -> GbResult<Self> {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("glowback");
        Self::new_with_data_dir(data_dir).await
    }

    pub async fn new_with_data_dir<P: AsRef<Path>>(data_dir: P) -> GbResult<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&data_dir)?;

        let catalog = catalog::DataCatalog::new(&data_dir.join("catalog.db")).await?;
        let storage = storage::StorageManager::new(&data_dir)?;
        let cache = cache::CacheManager::new()?;

        Ok(Self {
            catalog,
            storage,
            cache,
            providers: Vec::new(),
        })
    }

    pub async fn new_ephemeral(prefix: &str) -> GbResult<Self> {
        let data_dir = std::env::temp_dir().join(format!("{prefix}-{}", Uuid::new_v4()));
        Self::new_with_data_dir(data_dir).await
    }

    pub fn add_provider(&mut self, provider: Box<dyn providers::DataProvider>) {
        self.providers.push(provider);
    }

    pub async fn get_validation_summary(
        &self,
        symbol: &gb_types::Symbol,
        resolution: gb_types::Resolution,
    ) -> GbResult<Option<DataValidationSummary>> {
        Ok(self
            .catalog
            .get_symbol_info_for_resolution(symbol, resolution)
            .await?
            .and_then(|info| info.validation_summary))
    }

    pub async fn load_data(
        &mut self,
        symbol: &gb_types::Symbol,
        start_date: chrono::DateTime<chrono::Utc>,
        end_date: chrono::DateTime<chrono::Utc>,
        resolution: gb_types::Resolution,
    ) -> GbResult<Vec<gb_types::Bar>> {
        // Check cache first
        if let Some(data) = self
            .cache
            .get_bars(symbol, start_date, end_date, resolution)
            .await?
        {
            return Ok(data);
        }

        // Try storage
        if let Ok(data) = self
            .storage
            .load_bars(symbol, start_date, end_date, resolution)
            .await
        {
            if !data.is_empty() {
                let existing_info = self
                    .catalog
                    .get_symbol_info_for_resolution(symbol, resolution)
                    .await?;
                let dataset_kind = existing_info
                    .as_ref()
                    .map(|info| info.dataset_kind)
                    .unwrap_or(DatasetKind::External);
                let price_adjustment = existing_info
                    .as_ref()
                    .map(|info| info.price_adjustment)
                    .unwrap_or(PriceAdjustmentMode::Raw);
                let validation_summary = existing_info
                    .and_then(|info| info.validation_summary)
                    .unwrap_or_else(|| {
                        summarize_bars(&data, symbol, resolution, dataset_kind, price_adjustment)
                    });
                let actual_start = data.first().map(|bar| bar.timestamp).unwrap_or(start_date);
                let actual_end = data.last().map(|bar| bar.timestamp).unwrap_or(end_date);

                self.catalog
                    .register_symbol_data(
                        symbol,
                        actual_start,
                        actual_end,
                        resolution,
                        data.len() as u64,
                        dataset_kind,
                        price_adjustment,
                        Some(&validation_summary),
                    )
                    .await?;

                // Cache for future use
                self.cache.store_bars(symbol, &data, resolution).await?;
                return Ok(data);
            }
        }

        // Fetch from providers
        for provider in &mut self.providers {
            if provider.supports_symbol(symbol) {
                if let Ok(data) = provider
                    .fetch_bars(symbol, start_date, end_date, resolution)
                    .await
                {
                    let dataset_kind = provider.dataset_kind();
                    let price_adjustment = provider.price_adjustment_mode();
                    let validation_summary =
                        summarize_bars(&data, symbol, resolution, dataset_kind, price_adjustment);

                    // Store and then reload the merged/deduped view for downstream consumers.
                    self.storage.save_bars(symbol, &data, resolution).await?;
                    let stored_data = self
                        .storage
                        .load_bars(symbol, start_date, end_date, resolution)
                        .await?;
                    self.cache
                        .store_bars(symbol, &stored_data, resolution)
                        .await?;

                    let actual_start = stored_data
                        .first()
                        .map(|bar| bar.timestamp)
                        .unwrap_or(start_date);
                    let actual_end = stored_data
                        .last()
                        .map(|bar| bar.timestamp)
                        .unwrap_or(end_date);

                    self.catalog
                        .register_symbol_data(
                            symbol,
                            actual_start,
                            actual_end,
                            resolution,
                            stored_data.len() as u64,
                            dataset_kind,
                            price_adjustment,
                            Some(&validation_summary),
                        )
                        .await?;

                    return Ok(stored_data);
                }
            }
        }

        Err(gb_types::DataError::NoDataInRange {
            symbol: symbol.to_string(),
            start: start_date.to_rfc3339(),
            end: end_date.to_rfc3339(),
        }
        .into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use gb_types::{Resolution, Symbol};

    #[tokio::test]
    async fn load_data_falls_back_to_providers_when_storage_is_empty() {
        let mut manager = DataManager::new_ephemeral("gb-data-provider-fallback")
            .await
            .unwrap();
        manager.add_provider(Box::new(SampleDataProvider::new()));

        let symbol = Symbol::equity("AAPL");
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 1, 5, 0, 0, 0).unwrap();

        let bars = manager
            .load_data(&symbol, start, end, Resolution::Day)
            .await
            .unwrap();

        assert!(!bars.is_empty());
        assert_eq!(bars[0].symbol, symbol);
    }
}
