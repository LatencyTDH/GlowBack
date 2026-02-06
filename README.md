# GlowBack

High‑performance quantitative backtesting platform built in Rust with Python bindings and a Streamlit UI.

[![CI](https://github.com/LatencyTDH/GlowBack/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/LatencyTDH/GlowBack/actions/workflows/rust.yml)
[![Tests](https://img.shields.io/badge/tests-25%20passing-brightgreen)](#testing)
[![Docs](https://github.com/LatencyTDH/GlowBack/actions/workflows/docs.yml/badge.svg?branch=main)](https://latencytdh.github.io/GlowBack/)
[![Rust Version](https://img.shields.io/badge/rust-1.70%2B-blue)](#development-setup)
[![Python Support](https://img.shields.io/badge/python-3.8%2B-blue)](#python-bindings)
[![License](https://img.shields.io/badge/license-MIT-green)](#license)

## Overview

GlowBack provides a fast, realistic backtesting engine with data management, storage, and analytics. It includes:

- Event‑driven simulation engine with slippage/latency/commission models
- Data ingestion (CSV, Alpha Vantage, sample data)
- Arrow/Parquet columnar storage and DuckDB metadata catalog
- Strategy library (4 built‑in strategies)
- Python bindings (async support)
- Streamlit UI for strategy development and analysis

## Project Status

Phase 0+ (Production Infrastructure) is complete. Phase 1 (Alpha) is in progress.

## Architecture

### Core Crates

| Crate | Description |
|------|-------------|
| **gb-types** | Core data structures, orders, portfolio, strategy library |
| **gb-data** | Data ingestion, providers, DuckDB catalog, Parquet storage/loader |
| **gb-engine** | Event‑driven backtesting engine and market simulation |
| **gb-python** | Python bindings with async support |

### UI

- Streamlit interface for data loading, strategy editing, running backtests, and result analysis

## Features

- Realistic market simulation with configurable market hours and resolution
- Multi‑symbol backtesting with chronological event ordering
- Performance analytics (Sharpe, Sortino, Calmar, CAGR, Max Drawdown, etc.)
- Risk analytics (VaR, CVaR, skewness, kurtosis)
- Strategy library: Buy & Hold, Moving Average Crossover, Momentum, Mean Reversion
- Storage: Arrow/Parquet with batch loading and round‑trip I/O
- Catalog: DuckDB metadata with indexed queries

## Getting Started

### Development Setup

Prerequisites:

- Rust 1.70+
- Python 3.8+ (for Python bindings)

```bash
# Clone
git clone <repository-url>
cd glowback

# Run tests
cargo test --workspace
```

### Run Examples

```bash
# Basic usage
cargo run --example basic_usage -p gb-types

# Market simulator tests
cargo test -p gb-engine simulator

# Parquet loader tests
cargo test -p gb-data parquet
```

### Launch the UI

```bash
cd ui && python setup.py
# Opens http://localhost:8501
```

## Python Bindings

```python
import glowback

manager = glowback.PyDataManager()
manager.add_sample_provider()
manager.add_csv_provider("/path/to/data")
manager.add_alpha_vantage_provider("your_api_key")

bars = manager.load_data(symbol, "2023-01-01T00:00:00Z", "2023-12-31T23:59:59Z", "day")
```

## Testing

```bash
cargo test --workspace
# 25 passed; 0 failed
```

## Roadmap

**In progress**
- Performance benchmarking and optimization
- Additional strategies (RSI, Bollinger Bands, pairs trading)

**Planned**
- Advanced analytics (drawdown, factor exposure)
- Parameter sweep and walk‑forward optimization
- Expanded documentation

## Contributing

Contributions are welcome. Focus areas:

1. Performance optimization
2. Additional strategies and analytics
3. UI enhancements
4. Documentation and examples

Open an issue or PR with a clear description of the change.

## License

MIT License — see [LICENSE](LICENSE).
