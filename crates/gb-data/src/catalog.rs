use std::path::Path;
use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};
use gb_types::{Symbol, Resolution, GbResult, DataError};
// use duckdb::{Connection, Result as DuckResult};

/// Data catalog for managing metadata (simplified in-memory implementation)
#[derive(Debug)]
pub struct DataCatalog {
    // connection: Connection, // TODO: Re-enable when DuckDB dependency is fixed
    symbols: HashMap<String, SymbolInfo>,
}

impl DataCatalog {
    pub async fn new<P: AsRef<Path>>(_db_path: P) -> GbResult<Self> {
        // TODO: Re-implement with DuckDB when dependency conflicts are resolved
        Ok(Self {
            symbols: HashMap::new(),
        })
    }
    
    pub async fn register_symbol_data(
        &mut self,
        symbol: &Symbol,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        resolution: Resolution,
    ) -> GbResult<()> {
        let key = format!("{}:{}:{:?}:{}", symbol.symbol, symbol.exchange, symbol.asset_class, resolution);
        let info = SymbolInfo {
            symbol: symbol.clone(),
            first_date: start_date,
            last_date: end_date,
            resolution,
            record_count: 0,
            last_updated: Utc::now(),
        };
        self.symbols.insert(key, info);
        Ok(())
    }
    
    pub async fn get_symbol_info(&self, symbol: &Symbol) -> GbResult<Option<SymbolInfo>> {
        // For simplified implementation, just look for any resolution
        for (_, info) in &self.symbols {
            if info.symbol.symbol == symbol.symbol 
                && info.symbol.exchange == symbol.exchange 
                && info.symbol.asset_class == symbol.asset_class {
                return Ok(Some(info.clone()));
            }
        }
        Ok(None)
    }
    
    pub async fn list_available_symbols(&self) -> GbResult<Vec<Symbol>> {
        let mut symbols = Vec::new();
        let mut seen = HashSet::new();
        
        for (_, info) in &self.symbols {
            let key = format!("{}:{}:{:?}", info.symbol.symbol, info.symbol.exchange, info.symbol.asset_class);
            if seen.insert(key) {
                symbols.push(info.symbol.clone());
            }
        }
        
        symbols.sort_by(|a, b| a.symbol.cmp(&b.symbol));
        Ok(symbols)
    }
    
    pub async fn get_catalog_stats(&self) -> GbResult<CatalogStats> {
        let mut asset_classes = HashSet::new();
        let mut exchanges = HashSet::new();
        let mut total_records = 0u64;
        let mut earliest_date = None;
        let mut latest_date = None;
        
        for (_, info) in &self.symbols {
            asset_classes.insert(format!("{:?}", info.symbol.asset_class));
            exchanges.insert(info.symbol.exchange.clone());
            total_records += info.record_count;
            
            if earliest_date.is_none() || info.first_date < earliest_date.unwrap() {
                earliest_date = Some(info.first_date);
            }
            if latest_date.is_none() || info.last_date > latest_date.unwrap() {
                latest_date = Some(info.last_date);
            }
        }
        
        Ok(CatalogStats {
            total_symbols: self.symbols.len() as u64,
            asset_classes: asset_classes.len() as u64,
            exchanges: exchanges.len() as u64,
            total_records,
            earliest_date,
            latest_date,
        })
    }
}

#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub symbol: Symbol,
    pub first_date: DateTime<Utc>,
    pub last_date: DateTime<Utc>,
    pub resolution: Resolution,
    pub record_count: u64,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct CatalogStats {
    pub total_symbols: u64,
    pub asset_classes: u64,
    pub exchanges: u64,
    pub total_records: u64,
    pub earliest_date: Option<DateTime<Utc>>,
    pub latest_date: Option<DateTime<Utc>>,
}

// TODO: Re-enable when DuckDB is added back
// impl From<duckdb::Error> for gb_types::DataError {
//     fn from(err: duckdb::Error) -> Self {
//         gb_types::DataError::DatabaseConnection {
//             message: err.to_string(),
//         }
//     }
// } 