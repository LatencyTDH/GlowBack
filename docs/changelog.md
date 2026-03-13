# Changelog

## Unreleased

- **API safety:** `/optimizations` now fails closed with `501 Not Implemented` until the endpoint is wired to a real backtest backend, instead of fabricating synthetic trial metrics from RNG while accepting `base_backtest` / `ray_cluster` inputs.
- **Breaking (build only):** Replaced DuckDB with SQLite (`rusqlite`) for catalog metadata storage. No schema or behavior changes — same SQL, dramatically faster builds (~20 min → ~47s) and smaller artifacts (~3.5 GB → ~1.1 GB).
- Added `[profile.dev] debug = "line-tables-only"` to reduce debug build size.
- Docs site created and expanded
- CI and workflow improvements
- Added a manual GitHub release workflow that publishes versioned releases from a previously built CI artifact.
