# Examples

## Quickstart smoke example

This repo now includes an executable quickstart script that proves a clean checkout can run a complete smoke path.

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

## Next examples to add

- Buy & Hold on AAPL with expected metrics snapshot
- Moving Average Crossover on SPY
- Momentum strategy with parameter sweep

If you want a specific example, open an issue and it can be added here.
