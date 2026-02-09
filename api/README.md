# GlowBack FastAPI Gateway (Phase 1)

This folder contains a minimal FastAPI service that exposes the GlowBack backtesting API contract.
It currently runs with a mock engine adapter and in‑memory storage while the Rust engine bindings are integrated.

## Quickstart

```bash
cd api
python -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt

# optional: require an API key
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

- Storage is in‑memory (process‑local).
- The engine adapter is a mock that emits progress events and a sample result.
- `GLOWBACK_API_KEY` enables a stub auth check (`Authorization: Bearer <token>`).
