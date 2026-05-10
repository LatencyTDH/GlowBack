# Strategy Templates & Lifecycle

GlowBack now ships two concrete strategy-authoring templates:

- a **Rust engine template** that shows the real lifecycle hooks used by `gb-engine`
- a **Python-facing UI template** that shows the optional local-runner hooks for custom strategies

## Built-in vs custom strategy paths

- **Built-in strategies** (`buy_and_hold`, `ma_crossover`, `momentum`, `mean_reversion`, `rsi`) run through the real Rust engine.
- **Custom Python strategies** in the Strategy Editor run through the local UI runner today.
- The templates below make that distinction explicit so users can start with the right contract instead of guessing.

## Rust engine lifecycle contract

Custom Rust strategies implement the `Strategy` trait in `gb-types` and participate in this lifecycle:

| Hook | When it runs | Typical use |
| --- | --- | --- |
| `initialize` | Once before the first bar | load config, validate parameters, seed state |
| `on_market_event` | For each market event/bar | generate orders or logs |
| `on_order_event` | After order lifecycle updates | react to fills, cancels, rejects |
| `on_day_end` | After the final event of each trading day | rebalance counters, end-of-day bookkeeping |
| `on_stop` | Once at shutdown | cleanup and final summaries |

### Runnable Rust template

Source: `crates/gb-engine/examples/strategy_lifecycle_template.rs`

Run it locally:

```bash
cargo run --example strategy_lifecycle_template -p gb-engine --locked
```

The example:

- creates a minimal custom strategy
- records every lifecycle hook invocation
- submits one market order through the real engine
- prints the hook counts and final portfolio summary

CI also executes this exact example in `.github/workflows/rust.yml`.

## Python-facing lifecycle template

Source: `ui/examples/lifecycle_strategy.py`

The Strategy Editor now includes a **Lifecycle Template** that demonstrates these hooks for local Python strategies:

- `on_start(portfolio, metadata)`
- `on_bar(bar, portfolio)`
- `on_day_end(trading_day, portfolio)`
- `on_finish(portfolio, summary)`

### Available helper payloads

`metadata` includes:

- `symbols`
- `start`
- `end`
- `resolution`
- `bars`

`summary` includes:

- `final_cash`
- `final_positions`
- `final_value`
- `total_trades`

The UI example intentionally stays simple: it selects a primary symbol in `on_start`, enters once in `on_bar`, emits a daily checkpoint in `on_day_end`, and reports the final state in `on_finish`.

That example is covered by `ui/tests/test_backtest_core.py`, so the Python-facing path is exercised in CI too.

## Recommendation

If you want the most realistic execution path today, start in **Rust** and use the engine template.
If you want to sketch logic quickly in the UI, start with the **Lifecycle Template** and treat it as a lighter-weight authoring surface until custom strategies run end-to-end through the Rust engine.
