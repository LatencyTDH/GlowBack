use std::collections::{HashMap, HashSet};
use std::path::Path;

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use gb_types::{AssetClass, DataError, GbResult, Resolution, Symbol};
use rusqlite::Connection;

/// Data catalog for managing metadata with SQLite backend
#[derive(Debug)]
pub struct DataCatalog {
    connection: Connection,
    symbols: HashMap<String, SymbolInfo>, // Keep in-memory cache for performance
}

impl DataCatalog {
    pub async fn new<P: AsRef<Path>>(db_path: P) -> GbResult<Self> {
        let connection = Connection::open(db_path).map_err(|e| DataError::DatabaseConnection {
            message: e.to_string(),
        })?;

        // Create tables if they don't exist
        connection
            .execute_batch(
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
            );",
            )
            .map_err(|e| DataError::DatabaseConnection { message: e.to_string() })?;

        let symbols = Self::load_symbols(&connection)?;

        Ok(Self {
            connection,
            symbols,
        })
    }

    pub async fn register_symbol_data(
        &mut self,
        symbol: &Symbol,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        resolution: Resolution,
    ) -> GbResult<()> {
        let key = symbol_cache_key(symbol, resolution);
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

        // Update SQLite
        self.connection
            .execute(
                "INSERT OR REPLACE INTO symbol_metadata 
             (id, symbol, exchange, asset_class, resolution, start_date, end_date, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, CURRENT_TIMESTAMP)",
                rusqlite::params![
                    key,
                    symbol.symbol,
                    symbol.exchange,
                    format!("{:?}", symbol.asset_class),
                    format!("{:?}", resolution),
                    start_date.to_rfc3339(),
                    end_date.to_rfc3339(),
                ],
            )
            .map_err(|e| DataError::DatabaseConnection {
                message: e.to_string(),
            })?;

        tracing::debug!(
            "Registered symbol data: {} from {} to {}",
            symbol,
            start_date,
            end_date
        );
        Ok(())
    }

    pub async fn get_symbol_info(&self, symbol: &Symbol) -> GbResult<Option<SymbolInfo>> {
        // For simplified implementation, just look for any resolution
        for info in self.symbols.values() {
            if info.symbol.symbol == symbol.symbol
                && info.symbol.exchange == symbol.exchange
                && info.symbol.asset_class == symbol.asset_class
            {
                return Ok(Some(info.clone()));
            }
        }
        Ok(None)
    }

    pub async fn list_available_symbols(&self) -> GbResult<Vec<Symbol>> {
        let mut symbols = Vec::new();
        let mut seen = HashSet::new();

        for info in self.symbols.values() {
            let key = format!(
                "{}:{}:{:?}",
                info.symbol.symbol, info.symbol.exchange, info.symbol.asset_class
            );
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

        for info in self.symbols.values() {
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

    fn load_symbols(connection: &Connection) -> GbResult<HashMap<String, SymbolInfo>> {
        let mut stmt = connection
            .prepare(
                "SELECT id, symbol, exchange, asset_class, resolution, start_date, end_date, record_count, updated_at
                 FROM symbol_metadata",
            )
            .map_err(|e| DataError::DatabaseConnection { message: e.to_string() })?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, String>(8)?,
                ))
            })
            .map_err(|e| DataError::QueryFailed {
                query: "SELECT symbol_metadata".to_string(),
                error: e.to_string(),
            })?;

        let mut symbols = HashMap::new();
        for row in rows {
            let (
                id,
                symbol,
                exchange,
                asset_class,
                resolution,
                start_date,
                end_date,
                record_count,
                updated_at,
            ) = row.map_err(|e| DataError::QueryFailed {
                query: "SELECT symbol_metadata".to_string(),
                error: e.to_string(),
            })?;

            let asset_class = parse_asset_class(&asset_class)?;
            let resolution = parse_resolution(&resolution)?;
            let first_date = parse_catalog_datetime(&start_date)?;
            let last_date = parse_catalog_datetime(&end_date)?;
            let last_updated = parse_catalog_datetime(&updated_at)?;

            symbols.insert(
                id,
                SymbolInfo {
                    symbol: Symbol::new(&symbol, &exchange, asset_class),
                    first_date,
                    last_date,
                    resolution,
                    record_count: record_count as u64,
                    last_updated,
                },
            );
        }

        Ok(symbols)
    }
}

fn symbol_cache_key(symbol: &Symbol, resolution: Resolution) -> String {
    format!(
        "{}:{}:{:?}:{}",
        symbol.symbol, symbol.exchange, symbol.asset_class, resolution
    )
}

fn parse_asset_class(value: &str) -> GbResult<AssetClass> {
    match value {
        "Equity" => Ok(AssetClass::Equity),
        "Crypto" => Ok(AssetClass::Crypto),
        "Forex" => Ok(AssetClass::Forex),
        "Commodity" => Ok(AssetClass::Commodity),
        "Bond" => Ok(AssetClass::Bond),
        _ => Err(DataError::ParseError {
            message: format!("unknown asset class in catalog metadata: {value}"),
        }
        .into()),
    }
}

fn parse_resolution(value: &str) -> GbResult<Resolution> {
    match value {
        "Tick" => Ok(Resolution::Tick),
        "Second" => Ok(Resolution::Second),
        "Minute" => Ok(Resolution::Minute),
        "FiveMinute" => Ok(Resolution::FiveMinute),
        "FifteenMinute" => Ok(Resolution::FifteenMinute),
        "Hour" => Ok(Resolution::Hour),
        "FourHour" => Ok(Resolution::FourHour),
        "Day" => Ok(Resolution::Day),
        "Week" => Ok(Resolution::Week),
        "Month" => Ok(Resolution::Month),
        _ => Err(DataError::ParseError {
            message: format!("unknown resolution in catalog metadata: {value}"),
        }
        .into()),
    }
}

fn parse_catalog_datetime(value: &str) -> GbResult<DateTime<Utc>> {
    if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
        return Ok(parsed.with_timezone(&Utc));
    }

    if let Ok(parsed) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
        return Ok(Utc.from_utc_datetime(&parsed));
    }

    Err(DataError::ParseError {
        message: format!("unrecognized datetime in catalog metadata: {value}"),
    }
    .into())
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_catalog_reload_restores_symbols_from_sqlite() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("catalog.db");
        let symbol = Symbol::new("AAPL", "NASDAQ", AssetClass::Equity);
        let start = Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 12, 31, 0, 0, 0).unwrap();

        {
            let mut catalog = DataCatalog::new(&db_path).await.unwrap();
            catalog
                .register_symbol_data(&symbol, start, end, Resolution::Day)
                .await
                .unwrap();
        }

        let reopened = DataCatalog::new(&db_path).await.unwrap();

        let info = reopened.get_symbol_info(&symbol).await.unwrap().unwrap();
        assert_eq!(info.symbol, symbol);
        assert_eq!(info.first_date, start);
        assert_eq!(info.last_date, end);
        assert_eq!(info.resolution, Resolution::Day);

        let symbols = reopened.list_available_symbols().await.unwrap();
        assert_eq!(symbols, vec![symbol.clone()]);

        let stats = reopened.get_catalog_stats().await.unwrap();
        assert_eq!(stats.total_symbols, 1);
        assert_eq!(stats.asset_classes, 1);
        assert_eq!(stats.exchanges, 1);
        assert_eq!(stats.total_records, 0);
        assert_eq!(stats.earliest_date, Some(start));
        assert_eq!(stats.latest_date, Some(end));
    }
}
