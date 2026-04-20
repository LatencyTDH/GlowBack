from __future__ import annotations

from datetime import datetime, timezone
from typing import Any
from uuid import uuid4

import pandas as pd

from api.app.experiment_registry import ExperimentRegistry, get_default_registry, sha256_text, stable_json_dumps


def _strategy_name(config: dict[str, Any]) -> str:
    name = str(config.get("name") or "streamlit_strategy").strip()
    return name or "streamlit_strategy"


def build_streamlit_run_request(
    market_data: pd.DataFrame,
    config: dict[str, Any],
    strategy_code: str,
) -> dict[str, Any]:
    if market_data.empty:
        raise ValueError("market_data must not be empty")

    timestamps = pd.to_datetime(market_data["timestamp"], utc=True)
    symbols = sorted({str(symbol) for symbol in market_data.get("symbol", []) if symbol})
    resolutions = sorted({str(resolution) for resolution in market_data.get("resolution", []) if resolution})

    dataset_fingerprint = sha256_text(
        stable_json_dumps(
            {
                "bars": int(len(market_data)),
                "symbols": symbols,
                "start": timestamps.min().isoformat(),
                "end": timestamps.max().isoformat(),
                "resolutions": resolutions,
            }
        )
    )

    initial_capital = float(config.get("initial_capital", 100000.0))
    commission_bps = None
    if config.get("commission") is not None:
        commission_bps = float(config["commission"]) * 10_000

    slippage_bps = None
    if config.get("slippage") is not None:
        slippage_bps = float(config["slippage"])

    return {
        "symbols": symbols or ["UNKNOWN"],
        "start_date": timestamps.min().isoformat(),
        "end_date": timestamps.max().isoformat(),
        "resolution": resolutions[0] if resolutions else "day",
        "strategy": {
            "name": _strategy_name(config),
            "params": {
                "max_position_size": config.get("max_position_size"),
            },
        },
        "execution": {
            "commission_bps": commission_bps,
            "slippage_bps": slippage_bps,
            "latency_ms": None,
        },
        "initial_capital": initial_capital,
        "currency": "USD",
        "timezone": "UTC",
        "provenance": {
            "runner": "streamlit-local",
            "bar_count": int(len(market_data)),
            "symbol_count": len(symbols) or 1,
            "dataset_fingerprint": dataset_fingerprint,
            "strategy_code_hash": sha256_text(strategy_code),
            "strategy_config_hash": sha256_text(stable_json_dumps(config)),
        },
    }


def persist_streamlit_run(
    result: dict[str, Any],
    market_data: pd.DataFrame,
    config: dict[str, Any],
    strategy_code: str,
    *,
    registry: ExperimentRegistry | None = None,
    label: str | None = None,
    created_at: str | None = None,
    started_at: str | None = None,
    finished_at: str | None = None,
) -> dict[str, Any]:
    registry = registry or get_default_registry()
    run_id = str(result.get("run_id") or uuid4())
    now = datetime.now(timezone.utc).isoformat()

    status = {
        "run_id": run_id,
        "state": "completed",
        "progress": 1.0,
        "created_at": created_at or now,
        "started_at": started_at or created_at or now,
        "finished_at": finished_at or now,
        "error": None,
    }

    request = build_streamlit_run_request(market_data, config, strategy_code)
    strategy_name = _strategy_name(config)
    metadata = {
        "runner": "streamlit-local",
        "comparison_ready": True,
        "saved_via": "advanced_analytics" if label else None,
        "result_summary": {
            "total_return": result.get("total_return"),
            "sharpe_ratio": result.get("sharpe_ratio"),
            "max_drawdown": result.get("max_drawdown"),
            "final_value": result.get("final_value"),
            "total_trades": result.get("total_trades"),
        },
    }

    persisted = registry.upsert_run(
        run_id=run_id,
        source="ui",
        status=status,
        request=request,
        result={**result, "run_id": run_id},
        metadata=metadata,
        strategy_name=strategy_name,
        strategy_code=strategy_code,
        strategy_config=config,
        label=label,
    )
    return persisted


def list_streamlit_runs(*, registry: ExperimentRegistry | None = None) -> list[dict[str, Any]]:
    registry = registry or get_default_registry()
    return registry.list_runs(source="ui", state="completed", limit=200)


def list_saved_strategies(*, registry: ExperimentRegistry | None = None) -> list[dict[str, Any]]:
    registry = registry or get_default_registry()
    return registry.list_strategies(limit=200)


def save_strategy_snapshot(
    name: str,
    code: str,
    config: dict[str, Any] | None = None,
    *,
    registry: ExperimentRegistry | None = None,
) -> dict[str, Any]:
    registry = registry or get_default_registry()
    return registry.upsert_strategy(
        name=name,
        code=code,
        config=config or {},
        metadata={
            "saved_from": "streamlit-strategy-editor",
            "strategy_config_hash": sha256_text(stable_json_dumps(config or {})),
        },
    )


def get_saved_strategy(name: str, *, registry: ExperimentRegistry | None = None) -> dict[str, Any] | None:
    registry = registry or get_default_registry()
    return registry.get_strategy(name)


def delete_saved_strategy(name: str, *, registry: ExperimentRegistry | None = None) -> bool:
    registry = registry or get_default_registry()
    return registry.delete_strategy(name)


def rename_saved_run(run_id: str, label: str | None, *, registry: ExperimentRegistry | None = None) -> dict[str, Any] | None:
    registry = registry or get_default_registry()
    return registry.rename_run(run_id, label)


def delete_saved_run(run_id: str, *, registry: ExperimentRegistry | None = None) -> bool:
    registry = registry or get_default_registry()
    return registry.delete_run(run_id)


def run_display_name(record: dict[str, Any]) -> str:
    label = str(record.get("label") or "").strip()
    if label:
        return label

    strategy_name = record.get("strategy_name") or (record.get("request") or {}).get("strategy", {}).get("name") or "Run"
    created_at = record.get("created_at") or ""
    if created_at:
        try:
            parsed = pd.to_datetime(created_at, utc=True)
            return f"{strategy_name} · {parsed.strftime('%Y-%m-%d %H:%M UTC')}"
        except Exception:  # pragma: no cover - defensive formatting fallback
            pass
    return str(strategy_name)


def run_summary_rows(records: list[dict[str, Any]]) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for record in records:
        result = record.get("result") or {}
        request = record.get("request") or {}
        rows.append(
            {
                "Run": run_display_name(record),
                "Run ID": record["run_id"],
                "Symbols": ", ".join(request.get("symbols", [])),
                "Saved": record.get("created_at"),
                "Total Return (%)": result.get("total_return"),
                "Sharpe": result.get("sharpe_ratio"),
                "Max DD (%)": result.get("max_drawdown"),
                "Trades": result.get("total_trades"),
                "Final Value ($)": result.get("final_value"),
            }
        )
    return rows
