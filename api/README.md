# GlowBack FastAPI Gateway

This folder contains the FastAPI service that exposes the GlowBack backtesting API contract.
Backtest runs execute through the real Rust engine via the `gb-python` bindings, while run metadata, event history, and completed results are persisted in a local SQLite-backed experiment registry.

## Quickstart

```bash
cd api
python -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
maturin develop -m ../crates/gb-python/Cargo.toml

# optional: require an API key (comma-separated keys supported)
export GLOWBACK_API_KEY="dev-secret"

uvicorn app.main:app --reload
```

The server will be available at http://127.0.0.1:8000 with interactive docs at `/docs`.

## Endpoints

- `POST /backtests` → create a run (returns `run_id` + status)
- `GET /backtests` → list runs (filter by state)
- `GET /backtests/{run_id}` → run status
- `GET /backtests/{run_id}/results` → results payload
- `GET /backtests/{run_id}/stream` → WebSocket stream (events)

## Notes

- Backtest run metadata, event history, and completed results are persisted in a local SQLite experiment registry so `GET /backtests` still shows historical runs after restart.
- Backtests execute through the same Rust engine path used by the Python bindings/UI.
- Use `data_source: "sample"` to opt into the built-in sample provider, or `data_source: "csv"` with `csv_data_path` pointing at a directory of `{symbol}_{resolution}.csv` files.
- `data_quality_mode` accepts `"warn"` (default) or `"fail"`. In `warn` mode, validation findings are attached to result metadata and run manifests. In `fail` mode, critical validation issues stop the backtest before execution.
- Run manifests now include `dataset.validation_summaries`, keyed by symbol, so API consumers can inspect duplicate timestamps, missing intervals, sample-data markers, and adjustment metadata after a run.
- `GLOWBACK_API_KEY` enables a stub auth check (`Authorization: Bearer <token>` or `X-API-Key: <token>`).
  WebSocket clients can pass `?api_key=<token>`.
- HTTP responses include `X-Request-ID` for log correlation.
