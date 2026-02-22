# Data Model

GlowBack uses explicit, typed structures for market data and execution.

## Bars

Bars represent OHLCV data with nanosecond timestamps in UTC.

## Symbols

Symbols identify instruments across multiple asset classes. Each symbol carries
an `AssetClass` and exchange identifier.

### Asset Classes

| Asset Class | 24/7 Trading | Fractional Qty | Default Exchange |
|-------------|:------------:|:--------------:|------------------|
| **Equity** | No | No | NASDAQ |
| **Crypto** | Yes | Yes | BINANCE |
| **Forex** | No (weekdays) | Yes | FOREX |
| **Commodity** | No | No | CME |
| **Bond** | No | No | NYSE |

### Crypto Symbols

Crypto symbols support both exchange conventions:

- Slash-separated: `BTC-USD`, `ETH-USD`, `SOL-USD`
- Concatenated: `BTCUSDT`, `ETHUSDT`, `SOLUSDT`

Use `Symbol::crypto("BTC-USD")` to create a crypto symbol with sensible defaults.

## Resolution

Resolution specifies the bar interval (Tick, Second, Minute, Hour, Day).

## Storage

- **Arrow/Parquet** for columnar storage
- **SQLite** for metadata and queryable catalogs
