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

- `metrics_summary`: Dictionary of performance metrics (Sharpe, Returns, Drawdown, etc.)
- `equity_curve`: List of daily snapshots (value, cash, positions, returns, drawdown).

### `PyDataManager`

Used for data ingestion and management.

```python
manager = glowback.PyDataManager()
manager.add_sample_provider()
```
