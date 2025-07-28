use thiserror::Error;

/// Main error type for the GlowBack system
#[derive(Error, Debug)]
pub enum GbError {
    #[error("Data error: {0}")]
    Data(#[from] DataError),
    
    #[error("Strategy error: {0}")]
    Strategy(#[from] StrategyError),
    
    #[error("Order error: {0}")]
    Order(#[from] OrderError),
    
    #[error("Portfolio error: {0}")]
    Portfolio(#[from] PortfolioError),
    
    #[error("Backtest error: {0}")]
    Backtest(#[from] BacktestError),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Arrow error: {0}")]
    Arrow(String),
    
    #[error("Parquet error: {0}")]
    Parquet(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
}

/// Data-related errors
#[derive(Error, Debug)]
pub enum DataError {
    #[error("Data source not found: {0}")]
    SourceNotFound(String),
    
    #[error("Symbol not found: {symbol}")]
    SymbolNotFound { symbol: String },
    
    #[error("No data available for symbol {symbol} in date range {start} to {end}")]
    NoDataInRange {
        symbol: String,
        start: String,
        end: String,
    },
    
    #[error("Invalid data format: {message}")]
    InvalidFormat { message: String },
    
    #[error("Data corruption detected: {message}")]
    Corruption { message: String },
    
    #[error("Insufficient data: {message}")]
    InsufficientData { message: String },
    
    #[error("Data loading failed: {message}")]
    LoadingFailed { message: String },
    
    #[error("Data parsing error: {message}")]
    ParseError { message: String },
    
    #[error("Database connection failed: {message}")]
    DatabaseConnection { message: String },
    
    #[error("Query execution failed: {query}, error: {error}")]
    QueryFailed { query: String, error: String },
}

/// Strategy-related errors
#[derive(Error, Debug)]
pub enum StrategyError {
    #[error("Strategy not found: {strategy_id}")]
    NotFound { strategy_id: String },
    
    #[error("Strategy initialization failed: {message}")]
    InitializationFailed { message: String },
    
    #[error("Strategy execution error: {message}")]
    ExecutionError { message: String },
    
    #[error("Invalid strategy configuration: {message}")]
    InvalidConfig { message: String },
    
    #[error("Strategy parameter error: {parameter}, message: {message}")]
    ParameterError { parameter: String, message: String },
    
    #[error("Strategy state error: {message}")]
    StateError { message: String },
    
    #[error("Strategy compilation error: {message}")]
    CompilationError { message: String },
    
    #[error("Strategy timeout: operation took longer than {timeout_seconds} seconds")]
    Timeout { timeout_seconds: u64 },
}

/// Order-related errors
#[derive(Error, Debug)]
pub enum OrderError {
    #[error("Order not found: {order_id}")]
    NotFound { order_id: String },
    
    #[error("Invalid order: {message}")]
    Invalid { message: String },
    
    #[error("Order rejection: {reason}")]
    Rejected { reason: String },
    
    #[error("Insufficient funds: required {required}, available {available}")]
    InsufficientFunds {
        required: rust_decimal::Decimal,
        available: rust_decimal::Decimal,
    },
    
    #[error("Position limit exceeded: {message}")]
    PositionLimitExceeded { message: String },
    
    #[error("Risk limit violation: {message}")]
    RiskLimitViolation { message: String },
    
    #[error("Order already filled: {order_id}")]
    AlreadyFilled { order_id: String },
    
    #[error("Order already canceled: {order_id}")]
    AlreadyCanceled { order_id: String },
    
    #[error("Market closed: cannot execute order for {symbol}")]
    MarketClosed { symbol: String },
    
    #[error("Unsupported order type: {order_type}")]
    UnsupportedOrderType { order_type: String },
}

/// Portfolio-related errors
#[derive(Error, Debug)]
pub enum PortfolioError {
    #[error("Position not found for symbol: {symbol}")]
    PositionNotFound { symbol: String },
    
    #[error("Insufficient position: trying to sell {requested} shares, but only have {available}")]
    InsufficientPosition {
        requested: rust_decimal::Decimal,
        available: rust_decimal::Decimal,
    },
    
    #[error("Portfolio calculation error: {message}")]
    CalculationError { message: String },
    
    #[error("Risk limit exceeded: {limit_type}, current: {current}, limit: {limit}")]
    RiskLimitExceeded {
        limit_type: String,
        current: rust_decimal::Decimal,
        limit: rust_decimal::Decimal,
    },
    
    #[error("Portfolio state inconsistency: {message}")]
    StateInconsistency { message: String },
    
    #[error("Currency mismatch: expected {expected}, got {actual}")]
    CurrencyMismatch { expected: String, actual: String },
}

/// Backtest-related errors
#[derive(Error, Debug)]
pub enum BacktestError {
    #[error("Backtest not found: {backtest_id}")]
    NotFound { backtest_id: String },
    
    #[error("Invalid backtest configuration: {message}")]
    InvalidConfig { message: String },
    
    #[error("Backtest already running: {backtest_id}")]
    AlreadyRunning { backtest_id: String },
    
    #[error("Backtest execution failed: {message}")]
    ExecutionFailed { message: String },
    
    #[error("Backtest canceled: {backtest_id}")]
    Canceled { backtest_id: String },
    
    #[error("Invalid date range: start {start} is after end {end}")]
    InvalidDateRange { start: String, end: String },
    
    #[error("No symbols specified for backtest")]
    NoSymbols,
    
    #[error("Engine initialization failed: {message}")]
    EngineInitFailed { message: String },
    
    #[error("Simulation error: {message}")]
    SimulationError { message: String },
    
    #[error("Results processing error: {message}")]
    ResultsProcessingError { message: String },
}

/// Result type alias for GlowBack operations
pub type GbResult<T> = Result<T, GbError>;

/// Helper trait for converting string errors
pub trait IntoGbError {
    fn into_gb_error(self) -> GbError;
}

impl IntoGbError for String {
    fn into_gb_error(self) -> GbError {
        GbError::Internal(self)
    }
}

impl IntoGbError for &str {
    fn into_gb_error(self) -> GbError {
        GbError::Internal(self.to_string())
    }
}

/// Macro for creating validation errors
#[macro_export]
macro_rules! validation_error {
    ($($arg:tt)*) => {
        GbError::Validation(format!($($arg)*))
    };
}

/// Macro for creating internal errors
#[macro_export]
macro_rules! internal_error {
    ($($arg:tt)*) => {
        GbError::Internal(format!($($arg)*))
    };
}

/// Macro for creating configuration errors
#[macro_export]
macro_rules! config_error {
    ($($arg:tt)*) => {
        GbError::Config(format!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn test_error_display() {
        let error = OrderError::InsufficientFunds {
            required: Decimal::from(1000),
            available: Decimal::from(500),
        };
        
        assert!(error.to_string().contains("Insufficient funds"));
        assert!(error.to_string().contains("1000"));
        assert!(error.to_string().contains("500"));
    }
    
    #[test]
    fn test_error_conversion() {
        let order_error = OrderError::Invalid {
            message: "test".to_string(),
        };
        let gb_error: GbError = order_error.into();
        
        match gb_error {
            GbError::Order(_) => (),
            _ => panic!("Expected Order error"),
        }
    }
    
    #[test]
    fn test_macros() {
        let _validation_err = validation_error!("Invalid value: {}", 42);
        let _internal_err = internal_error!("Something went wrong");
        let _config_err = config_error!("Missing required field: {}", "symbol");
    }
} 