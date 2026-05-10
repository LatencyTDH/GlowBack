# Examples

These examples are checked-in, runnable, and tied to real validation paths.

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

## Related docs

- [Strategy Templates & Lifecycle](../tutorials/strategy-templates.md)
- [Python API Reference](../api/python.md)
- [Getting Started](../getting-started.md)
