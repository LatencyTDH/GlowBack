# Changelog

## Unreleased

- **Breaking (build only):** Replaced DuckDB with SQLite (`rusqlite`) for catalog metadata storage. No schema or behavior changes — same SQL, dramatically faster builds (~20 min → ~47s) and smaller artifacts (~3.5 GB → ~1.1 GB).
- Added `[profile.dev] debug = "line-tables-only"` to reduce debug build size.
- Docs site created and expanded
- CI and workflow improvements
- Added a manual GitHub release workflow that publishes versioned releases from a previously built CI artifact.
- Fixed Parquet storage writes to merge existing and incoming bar history by timestamp instead of truncating previously stored data on refresh.
