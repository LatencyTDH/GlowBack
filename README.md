<p align="center">
  <img src="assets/glowback-banner.jpg" alt="GlowBack Banner" width="80%" />
</p>

# GlowBack

High‑performance quantitative backtesting platform built in Rust with Python bindings and a Streamlit UI.

[![CI](https://github.com/LatencyTDH/GlowBack/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/LatencyTDH/GlowBack/actions/workflows/rust.yml)
[![Docs Smoke](https://github.com/LatencyTDH/GlowBack/actions/workflows/docs-smoke.yml/badge.svg?branch=main)](https://github.com/LatencyTDH/GlowBack/actions/workflows/docs-smoke.yml)
[![Docs](https://github.com/LatencyTDH/GlowBack/actions/workflows/docs.yml/badge.svg?branch=main)](https://latencytdh.github.io/GlowBack/)
[![Rust Version](https://img.shields.io/badge/rust-stable-blue)](#getting-started)
[![Python Support](https://img.shields.io/badge/python-3.10%2B-blue)](#python-bindings)
[![License](https://img.shields.io/badge/license-MIT-green)](#license)

## Overview

GlowBack provides a fast, realistic backtesting engine with data management, storage, and analytics. It includes:

- Event‑driven simulation engine with slippage/latency/commission models, order lifecycle events, and participation-capped partial fills
- Data ingestion (CSV, Alpha Vantage, explicit sample/demo data)
- Arrow/Parquet columnar storage and SQLite metadata catalog
- Strategy library (6 built‑in strategies, including an experimental covered-call workflow; the quickstart smoke path exercises four of them)
- Python bindings (async support)
- Streamlit UI for strategy development and analysis
- Durable experiment registry for saved strategies, historical runs, and cross-session comparisons
- Sandbox paper broker rejects sell orders that exceed held inventory (no implicit naked shorts)

## Project Status

Phase 0+ (Production Infrastructure) is complete. Phase 1 (Alpha) is in progress.

## Architecture

### Core Crates

| Crate | Description |
|------|-------------|
| **gb-types** | Core data structures, orders, portfolio, strategy library |
| **gb-data** | Data ingestion, providers, SQLite catalog, Parquet storage/loader |
| **gb-engine** | Event‑driven backtesting engine and market simulation |
| **gb-python** | Python bindings with async support |

### UI

- Streamlit interface for data loading, strategy editing, running backtests, and result analysis
- Persisted experiment history so runs and saved strategies survive restarts

## Features

- Realistic market simulation with configurable market hours and resolution
- Multi‑asset backtesting: equities and crypto (spot) with asset-class-aware fees, market hours, and fractional quantities
- Portfolio accounting now marks positions with signed market value so short liabilities reduce equity correctly; `gb-types` ships deterministic accounting invariants coverage for long, short, fractional, and multi-asset books
- Multi‑symbol backtesting with chronological event ordering and auditable order submission/fill/cancel/expire traces
- Performance analytics (Sharpe, Sortino, Calmar, CAGR, Max Drawdown, etc.)
- Risk analytics (VaR, CVaR, skewness, kurtosis)
- Built-in strategies: Buy & Hold, Moving Average Crossover, Momentum, Mean Reversion, RSI, and an experimental Covered Call path
- Strategy authoring templates for both the Rust engine lifecycle and the UI's local Python runner lifecycle
- Storage: Arrow/Parquet with batch loading and round‑trip I/O
- Catalog: SQLite metadata with indexed queries

## Getting Started

### 5-Minute Quickstart

Prerequisites:

- Rust (latest stable)
- Python 3.10+ for requirements-based API/UI/docs workflows (CI uses 3.12; the UI project metadata pins 3.12 for `uv`)

```bash
git clone https://github.com/LatencyTDH/GlowBack.git
cd GlowBack
./scripts/quickstart.sh
```

The quickstart is intentionally executable from a clean checkout: it builds and runs the `gb-types` basic usage example, then verifies the success markers in the output.

### Next Runs

```bash
# Re-run the example directly
cargo run --locked --example basic_usage -p gb-types

# Runnable Rust strategy lifecycle template
cargo run --example strategy_lifecycle_template -p gb-engine --locked

# Market simulator tests
cargo test -p gb-engine simulator

# Parquet loader tests
cargo test -p gb-data parquet

# Full workspace tests
cargo test --workspace --locked
```

For the UI-side local strategy lifecycle example, see `ui/examples/lifecycle_strategy.py`
and the matching validation in `ui/tests/test_backtest_core.py`.

### Launch the UI

```bash
cd ui && python setup.py
# Opens http://localhost:8501
```

### Docker (Compose)

```bash
cp .env.example .env   # configure ports, API key, etc.
docker compose up --build -d
```

Services:
- UI: http://localhost:8501
- API: http://localhost:8000 (set `GLOWBACK_API_KEY` to require auth)
- Engine: http://localhost:8081 (health JSON)

All services include health checks, restart policies, and resource limits.
See the [Deployment Guide](docs/deployment.md) for production configuration.

## Python Bindings

Build the local extension from the repo root:

```bash
python -m pip install maturin
maturin develop -m crates/gb-python/Cargo.toml
```

Or run the checked-in clean-venv smoke path:

```bash
./scripts/python_sdk_quickstart.sh
```

The supported public surface is exported via `glowback.__all__`, and canonical built-in strategy IDs are available in `glowback.BUILTIN_STRATEGIES`.
The Python runtime now also returns `order_events`, `option_trades`, and `option_events` when a strategy emits those lifecycle records.

```python
import glowback

print(glowback.BUILTIN_STRATEGIES)

manager = glowback.PyDataManager()
manager.add_csv_provider("/path/to/data")
manager.add_alpha_vantage_provider("your_api_key")

# Sample/demo data must be opted into explicitly.
manager.add_sample_provider()

bars = manager.load_data(symbol, "2023-01-01T00:00:00Z", "2023-12-31T23:59:59Z", "day")
```

`cargo test -p gb-python --locked --no-default-features` includes parity checks that compare the Python helpers with the direct Rust engine for `buy_and_hold`, `ma_crossover`, and the experimental `covered_call` path. The docs smoke workflow also runs `./scripts/python_sdk_quickstart.sh`, which builds `gb-python` in an isolated virtualenv and executes `examples/python_sdk_quickstart.py` end to end.

## Testing

```bash
cargo test --workspace --locked --exclude gb-python
cargo test -p gb-python --locked --no-default-features
./scripts/quickstart.sh
mkdocs build --strict
```

The badges at the top of this README are sourced from GitHub Actions so they stay aligned with current CI status instead of drifting into stale manual counts.

## Assumptions and Limitations

GlowBack is usable today, but some surfaces are intentionally still alpha. Start with the explicit boundaries in [docs/assumptions-and-limitations.md](docs/assumptions-and-limitations.md) before planning a production workflow.

## Benchmarks

```bash
./scripts/run-engine-benchmarks.sh artifacts/benchmarks/local
```

This runs the maintained `gb-engine` hot-path benchmark, generates a compact `summary.md` / `summary.json`, and preserves the raw Criterion output for drill-down.

Scheduled and manual CI benchmark runs upload the same artifact structure from `.github/workflows/benchmarks.yml`, so benchmark history is visible without blocking normal pull requests.

## Roadmap

**In progress**
- Hardening the FastAPI, Streamlit, and Docker deployment paths around the engine-backed workflow
- Expanding benchmark history and regression thresholds for the maintained hot paths
- Improving live/paper parity, safety controls, and audit trails before real-money workflows

**Planned**
- More end-to-end options backtesting coverage on top of `gb-options`
- Additional strategy examples and richer overfit diagnostics
- Hosted API/rustdoc references once the public contract stabilizes

## Contributing

Contributions are welcome. Focus areas:

1. Performance optimization
2. Additional strategies and analytics
3. UI enhancements
4. Documentation and examples

Open an issue or PR with a clear description of the change.

## License

MIT License — see [LICENSE](LICENSE).
