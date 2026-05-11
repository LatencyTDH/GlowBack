# Rust API Reference

Generated rustdoc is not published yet. Until it is, the crate sources are the canonical Rust reference:

- `crates/gb-types` — market data, orders, portfolio/accounting, strategies, metrics, and result types
- `crates/gb-data` — data providers, validation summaries, SQLite catalog, and Parquet storage
- `crates/gb-engine` — event-driven backtest engine, execution settings, order lifecycle, and manifests
- `crates/gb-options` — option contracts, Black-Scholes pricing, greeks, chains, and option execution helpers
- `crates/gb-optimizer` — search spaces, search strategies, trial/result concepts, and future distributed optimization types
- `crates/gb-risk` / `crates/gb-live` — early risk and live/paper trading surfaces

Useful local commands:

```bash
cargo doc --workspace --no-deps --locked --open
cargo test --workspace --locked
```

When hosted rustdoc is enabled, this page should link to the generated crate documentation.
