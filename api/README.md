# GlowBack FastAPI Gateway (Phase 1)

This folder contains a minimal FastAPI service that exposes the GlowBack backtesting API contract.
It currently runs with a mock engine adapter while persisting run history/events/results into a local SQLite-backed experiment registry.

## Quickstart

```bash
cd api
python -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt

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
- The engine adapter is a mock that emits progress events and a sample result.
- `GLOWBACK_API_KEY` enables a stub auth check (`Authorization: Bearer <token>` or `X-API-Key: <token>`).
  WebSocket clients can pass `?api_key=<token>`.
- HTTP responses include `X-Request-ID` for log correlation.
