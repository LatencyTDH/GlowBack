# Changelog

## Unreleased

- **Build optimization:** Switched DuckDB from bundled (C++ source compilation) to pre-built library linking via `DUCKDB_DOWNLOAD_LIB`. Build time: ~20 min → ~60s. Build size: ~3.5 GB → ~1.5 GB. No code changes — same DuckDB, same API.
- Added `.cargo/config.toml` with `DUCKDB_DOWNLOAD_LIB=1` so it's automatic for all developers.
- Added `[profile.dev] debug = "line-tables-only"` to reduce debug build size.
- Docs site created and expanded
- CI and workflow improvements
