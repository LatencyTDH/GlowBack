# Python API Reference

Python bindings are provided via `gb-python` (PyO3).

## Classes

### `PyBacktestEngine`

Used to configure and run backtests from Python.

```python
import glowback

# Initialize engine
engine = glowback.PyBacktestEngine(
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

### `PyBacktestResult`

Contains the results of a backtest run.

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
curve = result.to_dataframe()
metrics = result.metrics_dataframe()
ax = result.plot_equity()
```

### `PyDataManager`

Used for data ingestion and management.

```python
manager = glowback.PyDataManager()
manager.add_sample_provider()
```
