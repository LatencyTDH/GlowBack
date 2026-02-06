# GlowBack

High‑performance quantitative backtesting platform built in Rust with Python bindings and a Streamlit UI.

## Highlights

- Event‑driven simulation engine with realistic execution models
- Data ingestion (CSV, Alpha Vantage, sample data)
- Arrow/Parquet storage with DuckDB metadata catalog
- Strategy library (4 built‑in strategies)
- Python bindings with async support
- Streamlit UI for strategy development and analysis

## Quickstart

```bash
git clone https://github.com/LatencyTDH/GlowBack.git
cd GlowBack
cargo test --workspace
cargo run --example basic_usage -p gb-types
```

```bash
cd ui
python setup.py
# Opens http://localhost:8501
```

## Where to go next

- **Getting Started** for setup and first run
- **Concepts** for data model and execution details
- **Tutorials** for common workflows
- **API Reference** for bindings and crate docs
