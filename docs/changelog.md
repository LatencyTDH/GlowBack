# Changelog

## Unreleased

- **Run manifests + replayability:** Engine-backed API backtests now emit a typed `manifest` contract with dataset lineage, execution settings, replay-ready request payloads, and headline metric snapshots; Python/runtime helpers can replay the manifest locally and compare results against the captured metrics.
- **Experiment durability:** GlowBack now persists Streamlit strategy snapshots, completed UI backtest runs, and API backtest history in a shared SQLite-backed experiment registry so saved strategies, comparison runs, and historical `/backtests` listings survive restarts and remain exportable/deletable intentionally.
- **Engine scaling:** `gb-engine` now keeps per-symbol `StrategyContext` market buffers incrementally, reuses the live context across hot-path callbacks, and ships a Criterion benchmark covering representative 10-symbol/6-month and 50-symbol/1-year workloads instead of rebuilding full-history buffers on every callback.
- **Catalog durability:** `gb-data` now reloads persisted `symbol_metadata` rows when `DataCatalog` starts, so symbol listings, metadata lookups, and catalog stats survive process restarts instead of disappearing from the in-memory cache.
- **UI backtest correctness:** The Streamlit backtester now keeps last-known prices for every held symbol, analytics derive returns from the equity curve instead of `pct_change()` on cumulative return percentages, the dashboard no longer mutates the shared equity DataFrame in-place, and win rate is computed from realized closed trades (or shown as unavailable when nothing has closed yet).
- **Portfolio correctness:** `Position::apply_fill` now preserves the residual leg when a fill crosses through flat, so long→short and short→long reversals keep the reopened position with the crossing fill's basis; added reversal and exact-close regression coverage in `gb-types`.
- **Live risk safety:** Total exposure checks in `gb-live` now price each held symbol from its own latest mark instead of reusing the incoming order's price across the whole book; added multi-symbol regression tests for both false-approve and false-reject cases.
- **CI stability:** Sample/demo backtests now use isolated ephemeral data directories, preventing concurrent test runs from sharing one on-disk catalog/parquet store and intermittently failing multi-symbol crypto tests.
- **Optimization workflow:** `/optimizations` now runs real engine-backed optimization trials for built-in strategies via `gb-python`, supports grid/random/Bayesian search, adds holdout or walk-forward validation, returns replayable best-trial backtest payloads, and replaces the Streamlit optimizer placeholder with a live API-driven workflow.
- **CI coverage:** Normal PR CI now includes `gb-live`, `gb-risk`, and `gb-optimizer` in the Rust test matrix, adds lightweight API/UI Python validation, and gates release artifacts on those checks passing.
- **Breaking (build only):** Replaced DuckDB with SQLite (`rusqlite`) for catalog metadata storage. No schema or behavior changes — same SQL, dramatically faster builds (~20 min → ~47s) and smaller artifacts (~3.5 GB → ~1.1 GB).
- Added `[profile.dev] debug = "line-tables-only"` to reduce debug build size.
- Docs site created and expanded
- CI and workflow improvements
- Added a manual GitHub release workflow that publishes versioned releases from a previously built CI artifact.
- Fixed Parquet storage writes to merge existing and incoming bar history by timestamp instead of truncating previously stored data on refresh.
