# GlowBack

High‑performance quantitative backtesting platform built in Rust with Python bindings and a Streamlit UI.

## Highlights

- Event‑driven simulation engine with realistic execution models
- Data ingestion (CSV, Alpha Vantage, sample data)
- Arrow/Parquet storage with SQLite metadata catalog
- Strategy library (4 built‑in strategies)
- Python bindings with async support
- Streamlit UI for strategy development and analysis

## Quickstart

```bash
git clone https://github.com/LatencyTDH/GlowBack.git
cd GlowBack
./scripts/quickstart.sh
```

```bash
cd ui
python setup.py
# Opens http://localhost:8501
```

The quickstart script is exercised in CI and validates the expected success markers from the basic usage example.

## Where to go next

- **Getting Started** for the 5-minute first run and exact success markers
- **Assumptions & Limitations** for the current product boundaries
- **Concepts** for data model and execution details
- **Tutorials** for common workflows
- **API Reference** for bindings and crate docs
