# Python API Reference

Python bindings are provided via `gb-python` (PyO3).

## Install locally

From the repository root:

```bash
python -m pip install maturin
maturin develop -m crates/gb-python/Cargo.toml
```

For a clean-checkout smoke path that creates an isolated virtualenv, builds the extension, and runs a checked-in example end to end:

```bash
./scripts/python_sdk_quickstart.sh
```

## Supported public surface

The module publishes its supported contract via `glowback.__all__`, and the canonical built-in strategy IDs are exposed in `glowback.BUILTIN_STRATEGIES`.

```python
import glowback

print(glowback.__all__)
print(glowback.BUILTIN_STRATEGIES)
```

CI parity coverage for the binding lives in `cargo test -p gb-python --locked --no-default-features`, including direct Python-vs-Rust checks for `buy_and_hold` and `ma_crossover`. The docs smoke workflow also runs `./scripts/python_sdk_quickstart.sh` so the documented install/build path stays executable.

## Quick Helper

The checked-in companion example lives at `examples/python_sdk_quickstart.py` and exercises both the helper and `BacktestEngine` paths against sample data.


```python
import glowback

result = glowback.run_buy_and_hold(
    symbols=["AAPL", "MSFT"],
    start_date="2024-01-01T00:00:00Z",
    end_date="2024-12-31T23:59:59Z",
    initial_capital=100000.0
)
```

## Built-in Strategy Helper

`run_builtin_strategy(...)` runs the real Rust engine for the built-in strategy
set used by the optimization API.

```python
import glowback

result = glowback.run_builtin_strategy(
    symbols=["AAPL"],
    start_date="2024-01-01T00:00:00Z",
    end_date="2024-06-30T00:00:00Z",
    strategy_name="ma_crossover",
    strategy_params={"short_period": 10, "long_period": 30},
    data_source="sample",
    commission_bps=5,
    slippage_bps=5,
)
```

Supported built-ins:

- `buy_and_hold`
- `ma_crossover`
- `momentum`
- `mean_reversion`
- `rsi`

## Classes

### `BacktestEngine` (alias: `PyBacktestEngine`)

Used to configure and run backtests from Python.

```python
import glowback

# Initialize engine
engine = glowback.BacktestEngine(
    symbols=["AAPL", "MSFT"],
    start_date="2024-01-01T00:00:00Z",
    end_date="2024-12-31T23:59:59Z",
    initial_capital=100000.0
)

# Run a buy-and-hold backtest
result = engine.run_buy_and_hold()

# Access metrics
print(result.metrics_summary["sharpe_ratio"])

# Access equity curve
for point in result.equity_curve[:5]:
    print(f"{point['timestamp']}: {point['value']}")
```

### `BacktestResult` (alias: `PyBacktestResult`)

Contains the results of a backtest run.

- `manifest`: Replayable run-lineage payload with engine version, dataset summary,
  execution settings, replay request, and headline metrics.
- `metrics_summary`: Dictionary of performance metrics. Common keys include:
  - `initial_capital`, `final_value`
  - `total_return`, `annualized_return`, `volatility`
  - `sharpe_ratio`, `sortino_ratio`, `calmar_ratio`
  - `max_drawdown`, `max_drawdown_duration_days`
  - `var_95`, `cvar_95`
  - `skewness`, `kurtosis`
  - `total_trades`, `win_rate`, `profit_factor`
  - `average_win`, `average_loss`, `largest_win`, `largest_loss`
  - `total_commissions`
- `equity_curve`: List of daily snapshots (`value`, `cash`, `positions`, `total_pnl`, `returns`, `daily_return`, `drawdown`).

Notebook helpers (requires pandas/matplotlib):

```python
curve = result.to_dataframe(index="timestamp")
metrics = result.metrics_dataframe()
summary = result.summary(plot=True, index="timestamp")
ax = result.plot_equity()
manifest = result.manifest
```

### `DataManager` (alias: `PyDataManager`)

Used for data ingestion and management.

```python
manager = glowback.DataManager()
manager.add_sample_provider()
```
