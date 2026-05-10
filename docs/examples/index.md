# Examples

These examples are checked in, runnable, and tied to real validation paths.

## Quickstart smoke example

This repo includes an executable quickstart script that proves a clean checkout can run a complete smoke path.

```bash
./scripts/quickstart.sh
```

Under the hood it runs:

```bash
cargo run --locked --example basic_usage -p gb-types
```

Expected success markers:

```text
✅ All basic functionality working!
🎊 Strategy library complete with 4 different strategies!
```

The quickstart example exercises:

- symbol, bar, cache, and portfolio primitives
- sample data provider wiring
- built-in strategy construction
- basic error handling

## Rust engine lifecycle template

- File: `crates/gb-engine/examples/strategy_lifecycle_template.rs`
- Command:

```bash
cargo run --example strategy_lifecycle_template -p gb-engine --locked
```

What it proves:

- the full Rust strategy lifecycle executes end-to-end
- a custom strategy can submit orders through the real engine
- hook counts and final portfolio state are inspectable after the run

## Python-facing lifecycle template

- File: `ui/examples/lifecycle_strategy.py`
- Validation path:

```bash
python -m unittest ui.tests.test_backtest_core -v
```

What it proves:

- the UI local runner supports `on_start`, `on_bar`, `on_day_end`, and `on_finish`
- the example strategy can place trades and emit lifecycle logs
- the saved example stays executable instead of drifting into pseudo-code

## Next examples to add

- Buy & Hold on AAPL with expected metrics snapshot
- Moving Average Crossover on SPY
- Momentum strategy with parameter sweep

## Related docs

- [Strategy Templates & Lifecycle](../tutorials/strategy-templates.md)
- [Python API Reference](../api/python.md)
- [Getting Started](../getting-started.md)
