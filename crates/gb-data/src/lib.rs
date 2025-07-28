pub mod providers;
pub mod storage;
pub mod catalog;
pub mod loaders;
pub mod cache;
pub mod sources;

pub use providers::*;
pub use storage::*;
pub use catalog::*;
pub use loaders::*;
pub use cache::*;
pub use sources::*;

use gb_types::GbResult;

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
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("glowback");
        
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
    
    pub fn add_provider(&mut self, provider: Box<dyn providers::DataProvider>) {
        self.providers.push(provider);
    }
    
    pub async fn load_data(
        &mut self,
        symbol: &gb_types::Symbol,
        start_date: chrono::DateTime<chrono::Utc>,
        end_date: chrono::DateTime<chrono::Utc>,
        resolution: gb_types::Resolution,
    ) -> GbResult<Vec<gb_types::Bar>> {
        // Check cache first
        if let Some(data) = self.cache.get_bars(symbol, start_date, end_date, resolution).await? {
            return Ok(data);
        }
        
        // Try storage
        if let Ok(data) = self.storage.load_bars(symbol, start_date, end_date, resolution).await {
            // Cache for future use
            self.cache.store_bars(symbol, &data, resolution).await?;
            return Ok(data);
        }
        
        // Fetch from providers
        for provider in &mut self.providers {
            if provider.supports_symbol(symbol) {
                if let Ok(data) = provider.fetch_bars(symbol, start_date, end_date, resolution).await {
                    // Store and cache
                    self.storage.save_bars(symbol, &data, resolution).await?;
                    self.cache.store_bars(symbol, &data, resolution).await?;
                    
                    // Update catalog
                    self.catalog.register_symbol_data(symbol, start_date, end_date, resolution).await?;
                    
                    return Ok(data);
                }
            }
        }
        
        Err(gb_types::DataError::NoDataInRange {
            symbol: symbol.to_string(),
            start: start_date.to_rfc3339(),
            end: end_date.to_rfc3339(),
        }.into())
    }
} 