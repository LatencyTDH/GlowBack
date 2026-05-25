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

## Python SDK quickstart example

- File: `examples/python_sdk_quickstart.py`
- Command:

```bash
./scripts/python_sdk_quickstart.sh
```

What it proves:

- a clean isolated virtualenv can install `maturin`, build `gb-python`, and import `glowback`
- the supported public surface (`__all__`, `BUILTIN_STRATEGIES`) stays stable and documented
- both the one-shot helper path and the stateful `BacktestEngine` path run successfully against sample data
- result manifests, metrics, logs, and final positions are accessible from Python without reaching into Rust internals

Expected success marker:

```text
✅ Python SDK quickstart completed successfully
```

## Python SDK wheel smoke example

- File: `scripts/python_sdk_wheel_smoke.sh`
- Command:

```bash
./scripts/python_sdk_wheel_smoke.sh
```

What it proves:

- `gb-python` can be packaged as a wheel from a clean checkout instead of only via editable source installs
- the built wheel installs into a fresh virtualenv and still imports `glowback` without repo-local hacks
- the same checked-in Python quickstart example succeeds after wheel installation, so packaging and runtime behavior stay in sync
- the wheel path stays honest locally and in `.github/workflows/python-wheels.yml`

Expected success marker:

```text
✅ Python SDK quickstart completed successfully
```

## CSV data tutorial example

- File: `examples/csv_data_tutorial.py`
- Fixture: `examples/data/AAPL_1d.csv`
- Command:

```bash
./scripts/csv_data_tutorial.sh
```

What it proves:

- the checked-in CSV fixture matches the `CsvDataProvider` naming contract instead of relying on an ad hoc local file layout
- the documented `DataManager.add_csv_provider(...)` and `load_data(...)` flow can ingest a real fixture without pseudo-code gaps
- a CSV-backed `BacktestEngine` run produces a manifest that records the `csv` data source and a usable result payload

Expected success marker:

```text
✅ CSV data tutorial completed successfully
```

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

## Replay-manifest tutorial example

- File: `examples/replay_manifest_tutorial.py`
- Command:

```bash
./scripts/replay_manifest_tutorial.sh
```

What it proves:

- a real sample-data backtest emits a valid replayable manifest
- the documented `glowback_runtime.replay_manifest(...)` helper can rerun that manifest locally
- the replayed headline metrics stay within tolerance of the captured snapshot
- the checked-in reproducibility tutorial stays executable instead of becoming aspirational docs

Expected success marker:

```text
✅ Replay manifest tutorial completed successfully
```

## Next examples to add

- Momentum strategy with parameter sweep
- API workflow smoke path with checked-in request/response fixtures

## Related docs

- [Strategy Templates & Lifecycle](../tutorials/strategy-templates.md)
- [Python API Reference](../api/python.md)
- [Notebook Workflow](../tutorials/notebook.md)
- [Reproducing a Run](../tutorials/reproducing-a-run.md)
- [Getting Started](../getting-started.md)
