# Jupyter Notebook Workflow

GlowBack’s Python bindings are designed to work cleanly in notebooks. Use the helpers below to explore results inline.

## Install Notebook Dependencies

```bash
pip install jupyter pandas matplotlib
```

## Run a Backtest

```python
import glowback

engine = glowback.PyBacktestEngine(
    symbols=["AAPL", "MSFT"],
    start_date="2024-01-01T00:00:00Z",
    end_date="2024-12-31T23:59:59Z",
    initial_capital=100000.0,
)

result = engine.run_buy_and_hold()
```

## Explore Results Inline

```python
# Equity curve as a DataFrame
curve = result.to_dataframe()
curve.head()

# Metrics summary table
metrics = result.metrics_dataframe()
metrics

# Plot the equity curve
ax = result.plot_equity()
```

## Notes

- `to_dataframe()` and `metrics_dataframe()` require **pandas**.
- `plot_equity()` requires **matplotlib**.
- For custom visualizations, you can also use `result.equity_curve` directly (list of dicts).
