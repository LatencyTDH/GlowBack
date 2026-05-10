from __future__ import annotations

from datetime import date, datetime, timezone
from typing import Any

SUPPORTED_STRATEGIES: dict[str, str] = {
    "buy_and_hold": "buy_and_hold",
    "buy-and-hold": "buy_and_hold",
    "buy and hold": "buy_and_hold",
    "moving_average_crossover": "ma_crossover",
    "moving-average-crossover": "ma_crossover",
    "moving average crossover": "ma_crossover",
    "ma_crossover": "ma_crossover",
    "momentum": "momentum",
    "mean_reversion": "mean_reversion",
    "mean-reversion": "mean_reversion",
    "mean reversion": "mean_reversion",
    "rsi": "rsi",
}

_MANIFEST_REQUIRED_FIELDS = (
    "manifest_version",
    "generated_at",
    "engine",
    "strategy",
    "dataset",
    "execution",
    "replay_request",
    "metric_snapshot",
)

_REPLAY_REQUIRED_FIELDS = (
    "symbols",
    "start_date",
    "end_date",
    "resolution",
    "strategy_name",
    "strategy_params",
    "initial_capital",
    "data_source",
)

_METRIC_SNAPSHOT_KEYS = (
    "final_value",
    "total_return",
    "max_drawdown",
    "sharpe_ratio",
    "total_trades",
)


def normalize_strategy_name(name: str | None) -> str:
    raw = (name or "buy_and_hold").strip().lower()
    if raw in SUPPORTED_STRATEGIES:
        return SUPPORTED_STRATEGIES[raw]
    raise ValueError(
        "Unsupported strategy. Use one of: buy_and_hold, ma_crossover, momentum, mean_reversion, rsi."
    )


def _coerce_timestamp(value: datetime | date | str) -> str:
    if isinstance(value, str):
        return value
    if isinstance(value, date) and not isinstance(value, datetime):
        value = datetime.combine(value, datetime.min.time(), tzinfo=timezone.utc)
    if value.tzinfo is None:
        value = value.replace(tzinfo=timezone.utc)
    return value.astimezone(timezone.utc).isoformat().replace("+00:00", "Z")


def _load_glowback():
    try:
        import glowback  # type: ignore
    except ImportError as exc:  # pragma: no cover - depends on local build environment
        raise RuntimeError(
            "GlowBack Python bindings are not installed. Build them with "
            "`maturin develop -m crates/gb-python/Cargo.toml` from the repo root."
        ) from exc
    return glowback


def validate_run_manifest(manifest: dict[str, Any] | None) -> dict[str, Any]:
    if not isinstance(manifest, dict):
        raise ValueError("run manifest must be a dictionary")

    missing = [field for field in _MANIFEST_REQUIRED_FIELDS if field not in manifest]
    if missing:
        raise ValueError(f"run manifest missing required fields: {', '.join(missing)}")

    replay_request = manifest.get("replay_request")
    if not isinstance(replay_request, dict):
        raise ValueError("run manifest replay_request must be an object")

    replay_missing = [field for field in _REPLAY_REQUIRED_FIELDS if field not in replay_request]
    if replay_missing:
        raise ValueError(
            f"run manifest replay_request missing required fields: {', '.join(replay_missing)}"
        )

    metric_snapshot = manifest.get("metric_snapshot")
    if not isinstance(metric_snapshot, dict):
        raise ValueError("run manifest metric_snapshot must be an object")

    metric_missing = [field for field in _METRIC_SNAPSHOT_KEYS if field not in metric_snapshot]
    if metric_missing:
        raise ValueError(
            f"run manifest metric_snapshot missing required fields: {', '.join(metric_missing)}"
        )

    return manifest


def replay_manifest(manifest: dict[str, Any]) -> dict[str, Any]:
    validated = validate_run_manifest(manifest)
    replay_request = dict(validated["replay_request"])
    return run_backtest(**replay_request)


def compare_manifest_metrics(
    manifest: dict[str, Any],
    replay_payload: dict[str, Any],
    *,
    tolerance: float = 1e-6,
) -> dict[str, Any]:
    validated = validate_run_manifest(manifest)
    metric_snapshot = validated["metric_snapshot"]
    deltas: dict[str, float] = {}
    within_tolerance = True

    for key in _METRIC_SNAPSHOT_KEYS:
        expected = float(metric_snapshot[key])
        if key == "total_trades":
            actual = float(replay_payload.get(key, replay_payload.get("metrics_summary", {}).get(key, 0.0)))
        else:
            actual = float(
                replay_payload.get(key, replay_payload.get("metrics_summary", {}).get(key, 0.0))
            )
        delta = actual - expected
        deltas[key] = delta
        if abs(delta) > tolerance:
            within_tolerance = False

    return {
        "within_tolerance": within_tolerance,
        "tolerance": tolerance,
        "deltas": deltas,
    }


def run_backtest(
    *,
    symbols: list[str],
    start_date: datetime | date | str,
    end_date: datetime | date | str,
    resolution: str = "day",
    strategy_name: str = "buy_and_hold",
    strategy_params: dict[str, Any] | None = None,
    initial_capital: float = 100000.0,
    run_name: str | None = None,
    commission_bps: float | None = None,
    slippage_bps: float | None = None,
    latency_ms: int | None = None,
    data_source: str = "default",
    csv_data_path: str | None = None,
) -> dict[str, Any]:
    glowback = _load_glowback()
    strategy_id = normalize_strategy_name(strategy_name)
    source = (data_source or "default").strip().lower()
    if source not in {"default", "sample", "csv"}:
        raise ValueError("data_source must be one of: default, sample, csv")
    if source == "csv" and not csv_data_path:
        raise ValueError("csv_data_path is required when data_source='csv'")

    engine = glowback.BacktestEngine(
        symbols,
        _coerce_timestamp(start_date),
        _coerce_timestamp(end_date),
        resolution,
        initial_capital,
        run_name,
        commission_bps,
        slippage_bps,
        latency_ms,
        source,
        csv_data_path,
    )

    result = engine.run_strategy(strategy_id, strategy_params or {})

    metrics_summary = dict(result.metrics_summary)
    equity_curve = list(result.equity_curve)
    trades = list(result.trades)
    exposures = list(result.exposures)
    logs = list(result.logs)
    final_positions = dict(result.final_positions)
    final_cash = float(result.final_cash)
    manifest = dict(result.manifest) if result.manifest is not None else None

    payload: dict[str, Any] = {
        "metrics_summary": metrics_summary,
        "equity_curve": equity_curve,
        "trades": trades,
        "exposures": exposures,
        "logs": logs,
        "final_cash": final_cash,
        "final_positions": final_positions,
        "manifest": manifest,
    }
    payload.update(metrics_summary)
    payload.setdefault("initial_capital", float(initial_capital))
    payload.setdefault("final_value", metrics_summary.get("final_value", 0.0))
    payload.setdefault("total_return", metrics_summary.get("total_return", 0.0))
    payload.setdefault("sharpe_ratio", metrics_summary.get("sharpe_ratio", 0.0))
    payload.setdefault("max_drawdown", metrics_summary.get("max_drawdown", 0.0))
    payload.setdefault("total_trades", metrics_summary.get("total_trades", float(len(trades))))
    return payload
