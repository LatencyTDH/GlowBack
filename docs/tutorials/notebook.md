# Jupyter Notebook Workflow

GlowBack’s Python bindings are designed to work cleanly in notebooks. Use the helpers below to explore results inline.

If you want a reproducible starting point before opening Jupyter, run the checked-in companion smoke path first:

```bash
./scripts/python_sdk_quickstart.sh
```

That command creates an isolated virtualenv, builds `gb-python`, and runs `examples/python_sdk_quickstart.py` so the notebook snippets below start from a known-good install.

## Install Notebook Dependencies

```bash
pip install jupyter pandas matplotlib
```

## Run a Backtest

```python
import glowback

# One-liner helper
result = glowback.run_buy_and_hold(
    symbols=["AAPL", "MSFT"],
    start_date="2024-01-01T00:00:00Z",
    end_date="2024-12-31T23:59:59Z",
    initial_capital=100000.0,
)

# Or use the engine directly for more control
engine = glowback.BacktestEngine(
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
curve = result.to_dataframe(index="timestamp")
curve.head()

# Metrics summary table
metrics = result.metrics_dataframe()
metrics

# Quick notebook summary (metrics + curve, optional plot)
summary = result.summary(plot=True, index="timestamp")

# Plot the equity curve
ax = result.plot_equity()
```

## Companion example

- Checked-in script: `examples/python_sdk_quickstart.py`
- Smoke wrapper: `scripts/python_sdk_quickstart.sh`
- Sweep example: `examples/python_sdk_parameter_sweep.py`
- Replay example: `examples/replay_manifest_tutorial.py`

Use the script when you want a copy-pasteable starting point outside Jupyter, then lift the same calls into a notebook cell.

## Notebook gallery

Use these small notebook-sized workflows as the durable starting point for the Python SDK:

- First backtest: `glowback.run_buy_and_hold(...)` from the quickstart example
- Custom parameterized strategy: `glowback.run_builtin_strategy(...)` with `momentum` or `ma_crossover` parameters
- Parameter sweep: the checked-in `examples/python_sdk_parameter_sweep.py`
- Reproducible replay: the checked-in `examples/replay_manifest_tutorial.py`

The notebook-friendly API stays the same across these paths: run a backtest, inspect `result.metrics_summary`, `result.equity_curve`, and `result.manifest`, then copy the same calls into a notebook cell for interactive analysis.

## Notes

- `BacktestEngine`/`BacktestResult` are friendly aliases for `PyBacktestEngine`/`PyBacktestResult`.
- `to_dataframe()` and `metrics_dataframe()` require **pandas**.
- `plot_equity()` requires **matplotlib**.
- For custom visualizations, you can also use `result.equity_curve` directly (list of dicts).
