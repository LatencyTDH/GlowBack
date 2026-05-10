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

## Versioning

`/v1/...` is the canonical public API surface.
The legacy unversioned routes remain available as compatibility aliases for existing local callers:

- `/v1/backtests` ↔ `/backtests`
- `/v1/optimizations` ↔ `/optimizations`
- `/v1/healthz` ↔ `/healthz`

Prefer the versioned routes for new integrations so the contract can evolve without breaking older clients.

## Endpoints

- `POST /v1/backtests` → create a run (returns `run_id` + status)
- `GET /v1/backtests` → list runs (filter by state)
- `GET /v1/backtests/{run_id}` → run status
- `GET /v1/backtests/{run_id}/results` → results payload
- `GET /v1/backtests/{run_id}/stream` → WebSocket stream (events)
- `POST /v1/optimizations` → create an optimization run
- `GET /v1/optimizations` → list optimization runs
- `GET /v1/optimizations/{optimization_id}` → optimization status
- `GET /v1/optimizations/{optimization_id}/results` → optimization result payload
- `POST /v1/optimizations/{optimization_id}/cancel` → cancel a running optimization

## Error envelope

Versioned routes return a stable error envelope instead of the legacy FastAPI `detail` payload:

```json
{
  "error": {
    "code": "not_found",
    "message": "Optimization not found",
    "details": null
  },
  "request_id": "1d4d0ac9-4d45-4f8e-9023-3f682d7d48d4"
}
```

Validation failures use `error.code = "validation_error"` with structured `details` entries from FastAPI/Pydantic.

## Notes

- Backtest run metadata, event history, and completed results are persisted in a local SQLite experiment registry so `GET /backtests` still shows historical runs after restart.
- Backtests execute through the same Rust engine path used by the Python bindings/UI.
- Use `data_source: "sample"` to opt into the built-in sample provider, or `data_source: "csv"` with `csv_data_path` pointing at a directory of `{symbol}_{resolution}.csv` files.
- `GLOWBACK_API_KEY` enables a stub auth check (`Authorization: Bearer <token>` or `X-API-Key: <token>`).
  WebSocket clients can pass `?api_key=<token>`.
- HTTP responses include `X-Request-ID` for log correlation.
