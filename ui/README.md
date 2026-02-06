# GlowBack UI

Streamlit interface for local strategy development, backtesting, and analysis.

## Overview

The UI provides a full research loop: load data, edit strategies, run backtests, and review results. It’s designed to run locally and connects to the GlowBack core where available.

## Features

- Data loader (sample data, CSV, Alpha Vantage)
- Strategy editor with templates and validation
- Backtest runner with progress and logs
- Results dashboard (equity curve, drawdowns, metrics)
- Portfolio analyzer (risk metrics, scenario analysis)

## Quick Start

### Prerequisites

- Python 3.8+
- Rust toolchain (optional, required for full core integration)

### Install & Launch

```bash
cd ui
python setup.py
# Opens http://localhost:8501
```

Manual alternative:

```bash
pip install -r requirements.txt
streamlit run app.py
```

## Usage

1. **Load data** in the Data Loader (sample, CSV, or Alpha Vantage).
2. **Create or edit a strategy** in the Strategy Editor.
3. **Run a backtest** in the Backtest Runner.
4. **Analyze results** in the Results Dashboard and Portfolio Analyzer.

## Troubleshooting

- **Port in use**: `streamlit run app.py --server.port=8502`
- **Missing dependencies**: `pip install -r requirements.txt --upgrade`
- **Rust bindings not found**: the UI runs in Python‑only mode, but some features may be limited.

## Contributing

- Strategy templates: `strategy_editor.py`
- Data sources: `data_loader.py`
- Charts: `results_dashboard.py`
- Analytics: `portfolio_analyzer.py`

## License

MIT License — see the main repository for details.
