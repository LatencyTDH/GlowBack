# Strategy Templates

The UI includes built‑in templates for common strategies:

- Buy & Hold
- Moving Average Crossover
- Momentum
- Mean Reversion

## Use in UI

1. Open **Strategy Editor**.
2. Choose a template from the dropdown.
3. Adjust parameters and save the configuration.

## Use in Rust (conceptual)

```rust
let strategy = MovingAverageCrossoverStrategy::new()
  .with_fast_period(10)
  .with_slow_period(30);
```
