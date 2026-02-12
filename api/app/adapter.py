from __future__ import annotations

import asyncio
import math
import statistics
from datetime import datetime, timezone, timedelta
from typing import Protocol

from .models import BacktestRequest, BacktestResult, RunState
from .store import RunStore


class EngineAdapter(Protocol):
    async def run(self, run_id: str, request: BacktestRequest) -> None:
        ...


def _build_sample_results(initial_capital: float, start_date: datetime, end_date: datetime) -> tuple[list[dict], dict]:
    days = max(2, (end_date - start_date).days + 1)
    steps = min(days, 60)

    equity = initial_capital
    peak = equity
    equity_curve: list[dict] = []
    daily_returns: list[float] = []

    for step in range(steps):
        daily_return = 0.002 * math.sin(step / 4) + 0.0005
        equity *= 1 + daily_return
        peak = max(peak, equity)
        drawdown = (peak - equity) / peak if peak else 0.0

        cash = equity * 0.05
        positions = equity - cash
        cumulative_return = (equity - initial_capital) / initial_capital * 100
        total_pnl = equity - initial_capital

        equity_curve.append(
            {
                "timestamp": (start_date + timedelta(days=step)).isoformat(),
                "value": equity,
                "cash": cash,
                "positions": positions,
                "total_pnl": total_pnl,
                "returns": cumulative_return,
                "daily_return": daily_return * 100,
                "drawdown": drawdown * 100,
            }
        )
        daily_returns.append(daily_return)

    total_return = (equity - initial_capital) / initial_capital * 100
    max_drawdown = max(point["drawdown"] for point in equity_curve)

    max_drawdown_duration_days = 0
    current_drawdown_duration = 0
    for point in equity_curve:
        if point["drawdown"] > 0:
            current_drawdown_duration += 1
            max_drawdown_duration_days = max(max_drawdown_duration_days, current_drawdown_duration)
        else:
            current_drawdown_duration = 0

    if len(daily_returns) > 1:
        mean_return = statistics.mean(daily_returns)
        stdev_return = statistics.pstdev(daily_returns)
    else:
        mean_return = 0.0
        stdev_return = 0.0

    volatility = stdev_return * math.sqrt(252) * 100 if stdev_return > 0 else 0.0
    sharpe_ratio = (mean_return / stdev_return) * math.sqrt(252) if stdev_return > 0 else 0.0

    downside_returns = [r for r in daily_returns if r < 0]
    if downside_returns:
        downside_deviation = math.sqrt(sum(r * r for r in downside_returns) / len(downside_returns))
    else:
        downside_deviation = 0.0
    sortino_ratio = (
        (mean_return / downside_deviation) * math.sqrt(252)
        if downside_deviation > 0
        else 0.0
    )

    var_95 = 0.0
    cvar_95 = 0.0
    skewness = 0.0
    kurtosis = 0.0
    if daily_returns:
        sorted_returns = sorted(daily_returns)
        var_index = min(len(sorted_returns) - 1, int(len(sorted_returns) * 0.05))
        var_95 = -sorted_returns[var_index]
        tail_returns = sorted_returns[: var_index + 1]
        cvar_95 = -statistics.mean(tail_returns) if tail_returns else 0.0

        if stdev_return > 0 and len(daily_returns) >= 3:
            skewness = sum(
                ((r - mean_return) / stdev_return) ** 3 for r in daily_returns
            ) / len(daily_returns)
        if stdev_return > 0 and len(daily_returns) >= 4:
            kurtosis = (
                sum(((r - mean_return) / stdev_return) ** 4 for r in daily_returns)
                / len(daily_returns)
                - 3
            )

    annualized_return = 0.0
    if steps > 1:
        annualized_return = ((1 + total_return / 100) ** (252 / steps) - 1) * 100

    calmar_ratio = 0.0
    if max_drawdown > 0:
        calmar_ratio = annualized_return / max_drawdown

    metrics_summary = {
        "initial_capital": initial_capital,
        "final_value": equity,
        "total_return": total_return,
        "annualized_return": annualized_return,
        "volatility": volatility,
        "sharpe_ratio": sharpe_ratio,
        "sortino_ratio": sortino_ratio,
        "max_drawdown": max_drawdown,
        "max_drawdown_duration_days": float(max_drawdown_duration_days),
        "calmar_ratio": calmar_ratio,
        "var_95": var_95,
        "cvar_95": cvar_95,
        "skewness": skewness,
        "kurtosis": kurtosis,
        "total_trades": 0.0,
        "win_rate": 0.0,
        "profit_factor": 0.0,
        "average_win": 0.0,
        "average_loss": 0.0,
        "largest_win": 0.0,
        "largest_loss": 0.0,
        "total_commissions": 0.0,
    }

    return equity_curve, metrics_summary


class MockEngineAdapter:
    def __init__(self, store: RunStore) -> None:
        self._store = store

    async def run(self, run_id: str, request: BacktestRequest) -> None:
        try:
            await self._store.update_state(run_id, RunState.running)
            total_steps = 5
            for step in range(1, total_steps + 1):
                await asyncio.sleep(0.25)
                progress = step / total_steps
                await self._store.update_progress(run_id, progress, message=f"Step {step}/{total_steps}")

            equity_curve, metrics_summary = _build_sample_results(
                request.initial_capital,
                request.start_date,
                request.end_date,
            )

            result = BacktestResult(
                run_id=run_id,
                metrics_summary=metrics_summary,
                equity_curve=equity_curve,
                trades=[],
                exposures=[],
                logs=["Mock run completed"],
            )
            await self._store.set_result(run_id, result)
        except Exception as exc:  # pragma: no cover - safety net
            await self._store.update_state(run_id, RunState.failed, error=str(exc))
