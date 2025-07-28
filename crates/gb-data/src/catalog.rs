use std::path::Path;
use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};
use gb_types::{Symbol, Resolution, GbResult, DataError};
use duckdb::Connection;

/// Data catalog for managing metadata with DuckDB backend
#[derive(Debug)]
pub struct DataCatalog {
    connection: Connection,
    symbols: HashMap<String, SymbolInfo>, // Keep in-memory cache for performance
}

impl DataCatalog {
    pub async fn new<P: AsRef<Path>>(db_path: P) -> GbResult<Self> {
        let connection = Connection::open(db_path)
            .map_err(|e| DataError::DatabaseConnection { message: e.to_string() })?;
        
        // Create tables if they don't exist
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS symbol_metadata (
                id TEXT PRIMARY KEY,
                symbol TEXT NOT NULL,
                exchange TEXT NOT NULL,
                asset_class TEXT NOT NULL,
                resolution TEXT NOT NULL,
                start_date TEXT NOT NULL,
                end_date TEXT NOT NULL,
                record_count INTEGER DEFAULT 0,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT DEFAULT CURRENT_TIMESTAMP
            );
            
            CREATE INDEX IF NOT EXISTS idx_symbol_metadata_symbol ON symbol_metadata(symbol);
            CREATE INDEX IF NOT EXISTS idx_symbol_metadata_exchange ON symbol_metadata(exchange);
            CREATE INDEX IF NOT EXISTS idx_symbol_metadata_asset_class ON symbol_metadata(asset_class);
            
            CREATE TABLE IF NOT EXISTS data_sources (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                source_type TEXT NOT NULL,
                config TEXT,
                enabled BOOLEAN DEFAULT TRUE,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            );"
        ).map_err(|e| DataError::DatabaseConnection { message: e.to_string() })?;
        
        Ok(Self {
            connection,
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
        
        // Update in-memory cache
        self.symbols.insert(key.clone(), info);
        
        // Update DuckDB
        let mut stmt = self.connection.prepare(
            "INSERT OR REPLACE INTO symbol_metadata 
             (id, symbol, exchange, asset_class, resolution, start_date, end_date, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)"
        ).map_err(|e| DataError::DatabaseConnection { message: e.to_string() })?;
        
        stmt.execute([
            &key,
            &symbol.symbol,
            &symbol.exchange,
            &format!("{:?}", symbol.asset_class),
            &format!("{:?}", resolution),
            &start_date.to_rfc3339(),
            &end_date.to_rfc3339(),
        ]).map_err(|e| DataError::DatabaseConnection { message: e.to_string() })?;
        
        tracing::debug!("Registered symbol data: {} from {} to {}", symbol, start_date, end_date);
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