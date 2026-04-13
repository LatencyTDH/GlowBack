from __future__ import annotations

import asyncio
import math
import statistics
from datetime import datetime, timedelta
from typing import Protocol

from .models import BacktestRequest, BacktestResult, RunState
from .store import RunStore


TRADING_DAYS_PER_YEAR = 252
RISK_FREE_RATE = 0.02


class EngineAdapter(Protocol):
    async def run(self, run_id: str, request: BacktestRequest) -> None:
        ...


def _build_sample_curve(
    initial_capital: float,
    start_date: datetime,
    end_date: datetime,
    amplitude: float,
    drift: float,
    phase_divisor: float,
    symbol: str | None = None,
) -> tuple[list[dict], list[float]]:
    days = max(2, (end_date - start_date).days + 1)
    steps = min(days, 60)

    equity = initial_capital
    peak = equity
    curve: list[dict] = []
    daily_returns: list[float] = []

    for step in range(steps):
        daily_return = amplitude * math.sin(step / phase_divisor) + drift
        equity *= 1 + daily_return
        peak = max(peak, equity)
        drawdown = (peak - equity) / peak if peak else 0.0

        curve.append(
            {
                "timestamp": (start_date + timedelta(days=step)).isoformat(),
                "value": equity,
                "cash": equity * 0.05,
                "positions": equity * 0.95,
                "total_pnl": equity - initial_capital,
                "returns": (equity - initial_capital) / initial_capital * 100,
                "daily_return": daily_return * 100,
                "drawdown": drawdown * 100,
                **({"symbol": symbol} if symbol else {}),
            }
        )
        daily_returns.append(daily_return)

    return curve, daily_returns


def _calculate_benchmark_metrics(
    equity_curve: list[dict],
    benchmark_curve: list[dict],
) -> dict[str, float]:
    strategy_returns = [point["daily_return"] / 100 for point in equity_curve[1:]]
    benchmark_returns = [point["daily_return"] / 100 for point in benchmark_curve[1:]]
    periods = min(len(strategy_returns), len(benchmark_returns))
    if periods <= 1:
        return {}

    strategy_returns = strategy_returns[:periods]
    benchmark_returns = benchmark_returns[:periods]
    active_returns = [s - b for s, b in zip(strategy_returns, benchmark_returns)]

    benchmark_mean = statistics.mean(benchmark_returns)
    strategy_mean = statistics.mean(strategy_returns)
    benchmark_var = statistics.variance(benchmark_returns) if len(benchmark_returns) > 1 else 0.0

    beta = None
    alpha = None
    if benchmark_var > 0:
        covariance = statistics.covariance(strategy_returns, benchmark_returns)
        beta = covariance / benchmark_var
        alpha_daily = (strategy_mean - RISK_FREE_RATE / TRADING_DAYS_PER_YEAR) - beta * (
            benchmark_mean - RISK_FREE_RATE / TRADING_DAYS_PER_YEAR
        )
        alpha = alpha_daily * TRADING_DAYS_PER_YEAR * 100

    active_std = statistics.pstdev(active_returns) if len(active_returns) > 1 else 0.0
    information_ratio = None
    tracking_error = None
    if active_std > 0:
        information_ratio = statistics.mean(active_returns) / active_std * math.sqrt(TRADING_DAYS_PER_YEAR)
        tracking_error = active_std * math.sqrt(TRADING_DAYS_PER_YEAR) * 100

    benchmark_total_return = benchmark_curve[-1]["returns"]
    strategy_total_return = equity_curve[-1]["returns"]
    benchmark_annualized_return = ((1 + benchmark_total_return / 100) ** (TRADING_DAYS_PER_YEAR / len(benchmark_curve)) - 1) * 100
    strategy_annualized_return = ((1 + strategy_total_return / 100) ** (TRADING_DAYS_PER_YEAR / len(equity_curve)) - 1) * 100

    return {
        "beta": float(beta) if beta is not None else None,
        "alpha": float(alpha) if alpha is not None else None,
        "tracking_error": float(tracking_error) if tracking_error is not None else None,
        "information_ratio": float(information_ratio) if information_ratio is not None else None,
        "excess_return": float(strategy_annualized_return - benchmark_annualized_return),
        "benchmark_total_return": float(benchmark_total_return),
        "benchmark_annualized_return": float(benchmark_annualized_return),
        "strategy_total_return": float(strategy_total_return),
        "strategy_annualized_return": float(strategy_annualized_return),
    }


def _build_sample_results(
    initial_capital: float,
    start_date: datetime,
    end_date: datetime,
    benchmark_symbol: str | None,
) -> tuple[list[dict], list[dict], dict, dict]:
    equity_curve, daily_returns = _build_sample_curve(
        initial_capital=initial_capital,
        start_date=start_date,
        end_date=end_date,
        amplitude=0.002,
        drift=0.0005,
        phase_divisor=4,
    )
    benchmark_curve, benchmark_daily_returns = _build_sample_curve(
        initial_capital=initial_capital,
        start_date=start_date,
        end_date=end_date,
        amplitude=0.0016,
        drift=0.00035,
        phase_divisor=5,
        symbol=benchmark_symbol,
    )

    total_return = equity_curve[-1]["returns"]
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

    volatility = stdev_return * math.sqrt(TRADING_DAYS_PER_YEAR) * 100 if stdev_return > 0 else 0.0
    sharpe_ratio = (mean_return / stdev_return) * math.sqrt(TRADING_DAYS_PER_YEAR) if stdev_return > 0 else 0.0

    downside_returns = [r for r in daily_returns if r < 0]
    if downside_returns:
        downside_deviation = math.sqrt(sum(r * r for r in downside_returns) / len(downside_returns))
    else:
        downside_deviation = 0.0
    sortino_ratio = (
        (mean_return / downside_deviation) * math.sqrt(TRADING_DAYS_PER_YEAR)
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
            skewness = sum(((r - mean_return) / stdev_return) ** 3 for r in daily_returns) / len(daily_returns)
        if stdev_return > 0 and len(daily_returns) >= 4:
            kurtosis = (
                sum(((r - mean_return) / stdev_return) ** 4 for r in daily_returns) / len(daily_returns) - 3
            )

    annualized_return = ((1 + total_return / 100) ** (TRADING_DAYS_PER_YEAR / len(equity_curve)) - 1) * 100 if len(equity_curve) > 1 else 0.0
    calmar_ratio = annualized_return / max_drawdown if max_drawdown > 0 else 0.0

    benchmark_metrics = _calculate_benchmark_metrics(equity_curve, benchmark_curve)
    benchmark_metrics["benchmark_symbol"] = benchmark_symbol

    cost_summary = {
        "total_commissions": 0.0,
        "total_slippage_cost": 0.0,
        "total_cost_drag": 0.0,
        "cost_drag_pct_initial": 0.0,
        "turnover_multiple": 0.0,
        "total_notional": 0.0,
    }

    metrics_summary = {
        "initial_capital": initial_capital,
        "final_value": equity_curve[-1]["value"],
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
        **benchmark_metrics,
    }

    tearsheet = {
        "overview": {
            "final_value": metrics_summary["final_value"],
            "total_return": total_return,
            "annualized_return": annualized_return,
            "sharpe_ratio": sharpe_ratio,
            "max_drawdown": max_drawdown,
        },
        "benchmark": benchmark_metrics,
        "costs": cost_summary,
        "top_contributors": [],
        "biggest_detractors": [],
    }

    return equity_curve, benchmark_curve, metrics_summary, tearsheet


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

            benchmark_symbol = request.benchmark_symbol or (request.symbols[0] if request.symbols else "SPY")
            equity_curve, benchmark_curve, metrics_summary, tearsheet = _build_sample_results(
                request.initial_capital,
                request.start_date,
                request.end_date,
                benchmark_symbol,
            )

            result = BacktestResult(
                run_id=run_id,
                metrics_summary=metrics_summary,
                equity_curve=equity_curve,
                benchmark_curve=benchmark_curve,
                benchmark_symbol=benchmark_symbol,
                trades=[],
                exposures=[],
                tearsheet=tearsheet,
                logs=[f"Mock run completed against benchmark {benchmark_symbol}"],
            )
            await self._store.set_result(run_id, result)
        except Exception as exc:  # pragma: no cover - safety net
            await self._store.update_state(run_id, RunState.failed, error=str(exc))
