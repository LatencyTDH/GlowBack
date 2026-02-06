# Data Model

GlowBack uses explicit, typed structures for market data and execution.

## Bars

Bars represent OHLCV data with nanosecond timestamps in UTC.

## Symbols

Symbols identify instruments (equities, later crypto/FX). Resolution specifies the bar interval.

## Storage

- **Arrow/Parquet** for columnar storage
- **DuckDB** for metadata and queryable catalogs
