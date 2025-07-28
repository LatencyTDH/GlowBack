use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a financial symbol with exchange information
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Symbol {
    pub symbol: String,
    pub exchange: String,
    pub asset_class: AssetClass,
}

impl Symbol {
    pub fn new(symbol: &str, exchange: &str, asset_class: AssetClass) -> Self {
        Self {
            symbol: symbol.to_string(),
            exchange: exchange.to_string(),
            asset_class,
        }
    }
    
    pub fn equity(symbol: &str) -> Self {
        Self::new(symbol, "NASDAQ", AssetClass::Equity)
    }
    
    pub fn crypto(symbol: &str) -> Self {
        Self::new(symbol, "BINANCE", AssetClass::Crypto)
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.exchange, self.symbol)
    }
}

/// Asset classes supported by the platform
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetClass {
    Equity,
    Crypto,
    Forex,
    Commodity,
    Bond,
}

/// OHLCV bar data with volume and timestamp
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bar {
    pub symbol: Symbol,
    pub timestamp: DateTime<Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub resolution: Resolution,
}

impl Bar {
    pub fn new(
        symbol: Symbol,
        timestamp: DateTime<Utc>,
        open: Decimal,
        high: Decimal,
        low: Decimal,
        close: Decimal,
        volume: Decimal,
        resolution: Resolution,
    ) -> Self {
        Self {
            symbol,
            timestamp,
            open,
            high,
            low,
            close,
            volume,
            resolution,
        }
    }
    
    /// Calculate typical price (HLC/3)
    pub fn typical_price(&self) -> Decimal {
        (self.high + self.low + self.close) / Decimal::from(3)
    }
    
    /// Calculate true range
    pub fn true_range(&self, prev_close: Option<Decimal>) -> Decimal {
        let high_low = self.high - self.low;
        match prev_close {
            Some(prev) => {
                let high_prev = (self.high - prev).abs();
                let low_prev = (self.low - prev).abs();
                high_low.max(high_prev).max(low_prev)
            }
            None => high_low,
        }
    }
}

/// Tick data for high-frequency analysis
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tick {
    pub symbol: Symbol,
    pub timestamp: DateTime<Utc>,
    pub price: Decimal,
    pub size: Decimal,
    pub tick_type: TickType,
}

/// Type of tick (trade vs quote)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TickType {
    Trade,
    BidQuote,
    AskQuote,
}

/// Time resolution for market data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Resolution {
    Tick,
    Second,
    Minute,
    FiveMinute,
    FifteenMinute,
    Hour,
    FourHour,
    Day,
    Week,
    Month,
}

impl Resolution {
    pub fn to_seconds(&self) -> Option<u64> {
        match self {
            Resolution::Tick => None,
            Resolution::Second => Some(1),
            Resolution::Minute => Some(60),
            Resolution::FiveMinute => Some(300),
            Resolution::FifteenMinute => Some(900),
            Resolution::Hour => Some(3600),
            Resolution::FourHour => Some(14400),
            Resolution::Day => Some(86400),
            Resolution::Week => Some(604800),
            Resolution::Month => Some(2629746), // Average month
        }
    }
}

impl fmt::Display for Resolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Resolution::Tick => "tick",
            Resolution::Second => "1s",
            Resolution::Minute => "1m",
            Resolution::FiveMinute => "5m",
            Resolution::FifteenMinute => "15m",
            Resolution::Hour => "1h",
            Resolution::FourHour => "4h",
            Resolution::Day => "1d",
            Resolution::Week => "1w",
            Resolution::Month => "1M",
        };
        write!(f, "{}", s)
    }
}

/// Market data event for the event-driven engine
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MarketEvent {
    Bar(Bar),
    Tick(Tick),
    Quote { symbol: Symbol, timestamp: DateTime<Utc>, bid: Decimal, ask: Decimal, bid_size: Decimal, ask_size: Decimal },
}

impl MarketEvent {
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            MarketEvent::Bar(bar) => bar.timestamp,
            MarketEvent::Tick(tick) => tick.timestamp,
            MarketEvent::Quote { timestamp, .. } => *timestamp,
        }
    }
    
    pub fn symbol(&self) -> &Symbol {
        match self {
            MarketEvent::Bar(bar) => &bar.symbol,
            MarketEvent::Tick(tick) => &tick.symbol,
            MarketEvent::Quote { symbol, .. } => symbol,
        }
    }
} 