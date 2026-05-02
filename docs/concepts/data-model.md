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

## Dataset Metadata + Validation

GlowBack persists per-symbol, per-resolution dataset metadata in the catalog alongside stored bars. Each catalog entry now records:

- `record_count`
- `dataset_kind` (`external`, `user_provided`, `sample`)
- `price_adjustment` (`raw`, `split_adjusted`, `total_return_adjusted`, `synthetic`, `unknown`)
- optional `validation_summary`

`validation_summary` captures data-quality signals such as duplicate timestamps, missing expected intervals, invalid OHLCV rows, negative prices/volumes, timezone/resolution metadata, and whether the dataset is sample data.

## Data Quality Modes

Backtest `DataSettings` include `data_quality_mode`:

- `warn` (default): keep running, but surface validation warnings/critical issues in result metadata and run manifests
- `fail`: reject datasets that contain critical validation issues before the engine starts

Run manifests now include `dataset.validation_summaries`, keyed by symbol, so downstream replay/audit tooling can see the exact data-quality findings attached to a run.
