use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Data source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceConfig {
    pub name: String,
    pub source_type: DataSourceType,
    pub connection_params: HashMap<String, String>,
    pub enabled: bool,
    pub priority: i32,
}

/// Types of data sources supported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataSourceType {
    Local,
    Http,
    Database,
    CloudStorage,
    WebSocket,
}

impl DataSourceConfig {
    pub fn local(name: &str, path: &str) -> Self {
        let mut params = HashMap::new();
        params.insert("path".to_string(), path.to_string());
        
        Self {
            name: name.to_string(),
            source_type: DataSourceType::Local,
            connection_params: params,
            enabled: true,
            priority: 1,
        }
    }
    
    pub fn http(name: &str, base_url: &str, api_key: Option<&str>) -> Self {
        let mut params = HashMap::new();
        params.insert("base_url".to_string(), base_url.to_string());
        
        if let Some(key) = api_key {
            params.insert("api_key".to_string(), key.to_string());
        }
        
        Self {
            name: name.to_string(),
            source_type: DataSourceType::Http,
            connection_params: params,
            enabled: true,
            priority: 2,
        }
    }
    
    pub fn database(name: &str, connection_string: &str) -> Self {
        let mut params = HashMap::new();
        params.insert("connection_string".to_string(), connection_string.to_string());
        
        Self {
            name: name.to_string(),
            source_type: DataSourceType::Database,
            connection_params: params,
            enabled: true,
            priority: 3,
        }
    }
}

/// Configuration for common data sources
pub struct DataSources;

impl DataSources {
    pub fn sample_data() -> DataSourceConfig {
        DataSourceConfig::local("sample", "./data/sample")
    }
    
    pub fn csv_files(path: &str) -> DataSourceConfig {
        DataSourceConfig::local("csv_files", path)
    }
    
    pub fn alpha_vantage(api_key: &str) -> DataSourceConfig {
        DataSourceConfig::http("alpha_vantage", "https://www.alphavantage.co/query", Some(api_key))
    }
    
    pub fn yahoo_finance() -> DataSourceConfig {
        DataSourceConfig::http("yahoo_finance", "https://query1.finance.yahoo.com/v8/finance/chart", None)
    }
    
    pub fn polygon_io(api_key: &str) -> DataSourceConfig {
        DataSourceConfig::http("polygon", "https://api.polygon.io", Some(api_key))
    }
} 