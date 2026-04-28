# FastAPI Gateway

GlowBack exposes an HTTP + WebSocket gateway for backtests. This service is the API surface between clients (SDK/UI) and the Rust engine. Backtests execute through the real engine via the `gb-python` bindings, while run metadata is persisted in a local SQLite experiment registry.

## Quickstart

```bash
cd api
python -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
maturin develop -m ../crates/gb-python/Cargo.toml

uvicorn app.main:app --reload
```

Interactive docs are available at `/docs`.

## Authentication (stub)

If `GLOWBACK_API_KEY` is set in the environment, requests must include either:

```
Authorization: Bearer <token>
```

or

```
X-API-Key: <token>
```

Multiple keys are supported via a comma-separated `GLOWBACK_API_KEY` value.

WebSocket clients that cannot set headers (e.g., browsers) can pass `?api_key=<token>` in the URL.

## Request IDs & Security Headers

HTTP responses include `X-Request-ID` (use the header from your client to correlate logs).
The gateway also sets basic security headers (`X-Content-Type-Options`, `X-Frame-Options`,
`Referrer-Policy`, `Permissions-Policy`, `Cache-Control: no-store`).

## REST Endpoints

- `POST /backtests` → create run
- `GET /backtests` → list runs (filter by state)
- `GET /backtests/{run_id}` → run status
- `GET /backtests/{run_id}/results` → results payload

### Create Run

```json
POST /backtests
{
  "symbols": ["AAPL", "MSFT"],
  "start_date": "2024-01-01T00:00:00Z",
  "end_date": "2024-12-31T23:59:59Z",
  "resolution": "day",
  "strategy": {"name": "buy_and_hold"},
  "execution": {"slippage_bps": 1.0, "commission_bps": 0.5},
  "benchmark_symbol": "SPY",
  "portfolio_construction": {
    "method": "target_weights",
    "target_weights": {"AAPL": 0.6, "MSFT": 0.4},
    "rebalance_frequency": "weekly",
    "drift_threshold_pct": 5.0,
    "max_weight_pct": 40.0,
    "max_turnover_pct": 50.0,
    "cash_floor_pct": 5.0,
    "max_drawdown_pct": 20.0
  },
  "data_source": "sample"
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
  "benchmark_symbol": "SPY",
  "benchmark_curve": [
    {
      "timestamp": "2024-01-01T00:00:00+00:00",
      "symbol": "SPY",
      "value": 1003200.0,
      "returns": 0.32,
      "daily_return": 0.32,
      "drawdown": 0.0
    }
  ],
  "trades": [],
  "exposures": [],
  "portfolio_construction": {
    "method": "target_weights",
    "rebalance_frequency": "weekly",
    "target_weights": {"AAPL": 57.0, "MSFT": 38.0},
    "cash_floor_pct": 5.0,
    "max_weight_pct": 40.0,
    "max_turnover_pct": 50.0,
    "drift_threshold_pct": 5.0,
    "max_drawdown_pct": 20.0
  },
  "portfolio_diagnostics": [
    {
      "timestamp": "2024-01-01T00:00:00+00:00",
      "target_weights": {"AAPL": 57.0, "MSFT": 38.0},
      "realized_weights": {"AAPL": 57.0, "MSFT": 38.0},
      "max_abs_drift_pct": 0.0,
      "turnover_pct": 50.0,
      "rebalanced": true,
      "rebalance_reason": "initial_allocation",
      "cash_weight_pct": 5.0
    }
  ],
  "constraint_hits": [],
  "tearsheet": {
    "overview": {"final_value": 1012345.67},
    "benchmark": {"beta": 0.94, "alpha": 1.85, "information_ratio": 0.42},
    "portfolio": {"method": "target_weights", "rebalance_frequency": "weekly"},
    "costs": {"total_cost_drag": 0.0}
  },
  "logs": [],
  "final_cash": 50000.0,
  "final_positions": {"AAPL": 6333.3333},
  "manifest": {
    "manifest_version": "1.0",
    "engine": {"crate_name": "gb-engine", "version": "0.1.0"},
    "dataset": {
      "data_source": "sample",
      "resolution": "day",
      "symbols": ["AAPL", "MSFT"],
      "total_bars": 504
    },
    "replay_request": {
      "symbols": ["AAPL", "MSFT"],
      "strategy_name": "buy_and_hold",
      "resolution": "day",
      "data_source": "sample"
    },
    "metric_snapshot": {
      "final_value": 1012345.67,
      "total_return": 1.23,
      "max_drawdown": 0.65,
      "sharpe_ratio": 0.84,
      "total_trades": 0
    }
  }
}
```

Common `metrics_summary` keys include:
- `initial_capital`, `final_value`
- `total_return`, `annualized_return`, `volatility`
- `sharpe_ratio`, `sortino_ratio`, `calmar_ratio`
- `max_drawdown`, `max_drawdown_duration_days`
- `var_95`, `cvar_95`
- `skewness`, `kurtosis`
- `total_trades`, `win_rate`, `profit_factor`
- `average_win`, `average_loss`, `largest_win`, `largest_loss`
- `total_commissions`
- portfolio construction metrics such as `portfolio_rebalances`, `average_turnover_pct`, `max_weight_drift_pct`, and `constraint_hit_count`
- benchmark-relative metrics such as `beta`, `alpha`, `tracking_error`, `information_ratio`, and `excess_return`

Additional top-level result fields include:
- `benchmark_symbol`, `benchmark_curve`
- `portfolio_construction`, `portfolio_diagnostics`, `constraint_hits`
- `tearsheet`
- `manifest` (deterministic run lineage + replay request)

Notes:
- `returns`, `daily_return`, `max_drawdown`, and `volatility` are expressed as percentages.
- `total_pnl` is an absolute value in account currency.

## WebSocket Streaming

`GET /backtests/{run_id}/stream`

- Emits ordered events with `event_id`, `type`, and `payload`.
- Clients can pass `?last_event_id=<id>` to resume from a specific event.

## Notes

- Backtest runs, event history, and completed results are persisted in a local SQLite experiment registry, so `/backtests` survives service restarts.
- Backtests execute through the same Rust engine-backed path used by the embedded Python runtime.
- Request `data_source: "sample"` for the built-in sample provider, or `data_source: "csv"` plus `csv_data_path` for local CSV bundles.
- Benchmark-relative metrics are computed from the returned strategy and benchmark curves, and portfolio-construction fields remain part of the API contract and result payloads when available.
- `/optimizations` uses the same real `gb-python` execution path for built-in strategies.
- The `manifest` payload is designed to be replayed locally with `glowback_runtime.replay_manifest(...)`; see the "Reproducing a Run" tutorial.

