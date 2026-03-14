# Changelog

## Unreleased

- **Live risk safety:** Total exposure checks in `gb-live` now price each held symbol from its own latest mark instead of reusing the incoming order's price across the whole book; added multi-symbol regression tests for both false-approve and false-reject cases.
- **API safety:** `/optimizations` now fails closed with `501 Not Implemented` until the endpoint is wired to a real backtest backend, instead of fabricating synthetic trial metrics from RNG while accepting `base_backtest` / `ray_cluster` inputs.
- **Breaking (build only):** Replaced DuckDB with SQLite (`rusqlite`) for catalog metadata storage. No schema or behavior changes — same SQL, dramatically faster builds (~20 min → ~47s) and smaller artifacts (~3.5 GB → ~1.1 GB).
- Added `[profile.dev] debug = "line-tables-only"` to reduce debug build size.
- Docs site created and expanded
- CI and workflow improvements
- Added a manual GitHub release workflow that publishes versioned releases from a previously built CI artifact.
- Fixed Parquet storage writes to merge existing and incoming bar history by timestamp instead of truncating previously stored data on refresh.
