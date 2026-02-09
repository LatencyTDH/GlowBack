from __future__ import annotations

import asyncio
from datetime import datetime, timezone
from typing import Protocol

from .models import BacktestRequest, BacktestResult, RunState
from .store import RunStore


class EngineAdapter(Protocol):
    async def run(self, run_id: str, request: BacktestRequest) -> None:
        ...


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

            result = BacktestResult(
                run_id=run_id,
                metrics_summary={
                    "sharpe": 1.42,
                    "cagr": 0.18,
                    "max_drawdown": -0.12,
                },
                equity_curve=[
                    {
                        "timestamp": datetime.now(timezone.utc).isoformat(),
                        "equity": request.initial_capital,
                    }
                ],
                trades=[],
                exposures=[],
                logs=["Mock run completed"],
            )
            await self._store.set_result(run_id, result)
        except Exception as exc:  # pragma: no cover - safety net
            await self._store.update_state(run_id, RunState.failed, error=str(exc))
