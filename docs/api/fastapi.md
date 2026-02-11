# FastAPI Gateway

GlowBack exposes a minimal HTTP + WebSocket gateway for backtests. This service is the API surface between clients (SDK/UI) and the Rust engine. Phase 1 uses a mock adapter + in‑memory storage.

## Quickstart

```bash
cd api
python -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt

uvicorn app.main:app --reload
```

Interactive docs are available at `/docs`.

## Authentication (stub)

If `GLOWBACK_API_KEY` is set in the environment, requests must include:

```
Authorization: Bearer <token>
```

## REST Endpoints

- `POST /backtests` → create run
- `GET /backtests` → list runs (filter by state)
- `GET /backtests/{run_id}` → run status
- `GET /backtests/{run_id}/results` → results payload

### Create Run

```json
POST /backtests
{
  "symbols": ["AAPL"],
  "start_date": "2024-01-01T00:00:00Z",
  "end_date": "2024-12-31T23:59:59Z",
  "resolution": "day",
  "strategy": {"name": "buy_and_hold"},
  "execution": {"slippage_bps": 1.0, "commission_bps": 0.5}
}
```

## Results Payload (sample)

`GET /backtests/{run_id}/results`

```json
{
  "run_id": "<uuid>",
  "metrics_summary": {
    "initial_capital": 1000000.0,
    "final_value": 1012345.67,
    "total_return": 1.23,
    "annualized_return": 3.45,
    "volatility": 9.12,
    "sharpe_ratio": 0.84,
    "max_drawdown": 0.65,
    "max_drawdown_duration_days": 12,
    "calmar_ratio": 5.3,
    "total_trades": 0
  },
  "equity_curve": [
    {
      "timestamp": "2024-01-01T00:00:00+00:00",
      "value": 1005000.0,
      "cash": 50000.0,
      "positions": 1000000.0,
      "total_pnl": 5000.0,
      "returns": 0.5,
      "daily_return": 0.5,
      "drawdown": 0.0
    }
  ],
  "trades": [],
  "exposures": [],
  "logs": []
}
```

Notes:
- `returns`, `daily_return`, `max_drawdown`, and `volatility` are expressed as percentages.
- `total_pnl` is an absolute value in account currency.

## WebSocket Streaming

`GET /backtests/{run_id}/stream`

- Emits ordered events with `event_id`, `type`, and `payload`.
- Clients can pass `?last_event_id=<id>` to resume from a specific event.

## Notes

- Storage is in‑memory; restarting the service clears runs.
- The mock engine emits progress updates and a sample result.
- Replace the mock adapter with `gb-python` bindings or a CLI bridge in Phase 2.
