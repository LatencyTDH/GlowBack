"""In-memory store for optimization runs backed by real backtest execution."""

from __future__ import annotations

import asyncio
from dataclasses import dataclass, field
from datetime import datetime, timezone
from typing import Any
from uuid import uuid4

from .optimization_models import (
    OptimizationRequest,
    OptimizationResult,
    OptimizationState,
    OptimizationStatus,
    TrialStatus,
    TrialSummary,
)
from .optimization_runtime import OptimizationExecution, OptimizationExecutor


@dataclass
class TrialRecord:
    trial_id: str
    trial_number: int
    parameters: dict[str, Any]
    status: TrialStatus = TrialStatus.pending
    objective: float | None = None
    metrics: dict[str, float] = field(default_factory=dict)
    duration_seconds: int | None = None
    error: str | None = None

    @classmethod
    def from_summary(cls, summary: TrialSummary) -> "TrialRecord":
        return cls(
            trial_id=summary.trial_id,
            trial_number=summary.trial_number,
            parameters=summary.parameters,
            status=summary.status,
            objective=summary.objective,
            metrics=summary.metrics,
            duration_seconds=summary.duration_seconds,
            error=summary.error,
        )

    def to_summary(self) -> TrialSummary:
        return TrialSummary(
            trial_id=self.trial_id,
            trial_number=self.trial_number,
            status=self.status,
            parameters=self.parameters,
            objective=self.objective,
            metrics=self.metrics,
            duration_seconds=self.duration_seconds,
            error=self.error,
        )


@dataclass
class OptimizationRecord:
    optimization_id: str
    request: OptimizationRequest
    state: OptimizationState = OptimizationState.pending
    trials: list[TrialRecord] = field(default_factory=list)
    best_trial: TrialRecord | None = None
    replay_backtest: dict[str, Any] | None = None
    created_at: datetime = field(default_factory=lambda: datetime.now(timezone.utc))
    started_at: datetime | None = None
    finished_at: datetime | None = None
    error: str | None = None

    def to_status(self) -> OptimizationStatus:
        return OptimizationStatus(
            optimization_id=self.optimization_id,
            name=self.request.name,
            state=self.state,
            strategy=self.request.strategy.value,
            objective_metric=self.request.objective_metric,
            direction=self.request.direction.value,
            max_trials=self.request.max_trials,
            trials_completed=sum(
                1 for t in self.trials if t.status == TrialStatus.completed
            ),
            trials_failed=sum(
                1 for t in self.trials if t.status == TrialStatus.failed
            ),
            trials_running=sum(
                1 for t in self.trials if t.status == TrialStatus.running
            ),
            best_trial=self.best_trial.to_summary() if self.best_trial else None,
            created_at=self.created_at,
            started_at=self.started_at,
            finished_at=self.finished_at,
            error=self.error,
        )

    def to_result(self) -> OptimizationResult:
        total_dur = None
        if self.started_at and self.finished_at:
            total_dur = int((self.finished_at - self.started_at).total_seconds())
        return OptimizationResult(
            optimization_id=self.optimization_id,
            state=self.state,
            best_trial=self.best_trial.to_summary() if self.best_trial else None,
            all_trials=[t.to_summary() for t in self.trials],
            total_duration_seconds=total_dur,
            search_space=self.request.search_space,
            validation_mode=self.request.validation_mode.value,
            replay_backtest=self.replay_backtest,
        )


class OptimizationStore:
    """In-memory store for optimization runs."""

    def __init__(self, executor: OptimizationExecutor | None = None) -> None:
        self._optimizations: dict[str, OptimizationRecord] = {}
        self._lock = asyncio.Lock()
        self._executor = executor or OptimizationExecutor()

    async def create(self, request: OptimizationRequest) -> OptimizationStatus:
        opt_id = str(uuid4())
        record = OptimizationRecord(
            optimization_id=opt_id,
            request=request,
        )
        async with self._lock:
            self._optimizations[opt_id] = record
        return record.to_status()

    async def get_status(self, opt_id: str) -> OptimizationStatus | None:
        async with self._lock:
            record = self._optimizations.get(opt_id)
            return record.to_status() if record else None

    async def get_result(self, opt_id: str) -> OptimizationResult | None:
        async with self._lock:
            record = self._optimizations.get(opt_id)
            return record.to_result() if record else None

    async def list_optimizations(
        self, limit: int = 50
    ) -> list[OptimizationStatus]:
        async with self._lock:
            records = sorted(
                self._optimizations.values(),
                key=lambda r: r.created_at,
                reverse=True,
            )
            return [r.to_status() for r in records[:limit]]

    async def cancel(self, opt_id: str) -> bool:
        async with self._lock:
            record = self._optimizations.get(opt_id)
            if not record:
                return False
            if record.state in {
                OptimizationState.completed,
                OptimizationState.failed,
                OptimizationState.cancelled,
            }:
                return False
            record.state = OptimizationState.cancelled
            record.finished_at = datetime.now(timezone.utc)
            return True

    async def run_optimization(self, opt_id: str) -> None:
        async with self._lock:
            record = self._optimizations.get(opt_id)
            if not record or record.state == OptimizationState.cancelled:
                return
            record.state = OptimizationState.running
            record.started_at = datetime.now(timezone.utc)
            record.error = None
            request = record.request

        try:
            execution = await self._executor.execute(
                request,
                is_cancelled=lambda: self._is_cancelled(opt_id),
            )
        except Exception as exc:
            async with self._lock:
                record = self._optimizations.get(opt_id)
                if not record:
                    return
                record.state = OptimizationState.failed
                record.finished_at = datetime.now(timezone.utc)
                record.error = str(exc)
            return

        await self._apply_execution(opt_id, execution)

    async def _is_cancelled(self, opt_id: str) -> bool:
        async with self._lock:
            record = self._optimizations.get(opt_id)
            return bool(record and record.state == OptimizationState.cancelled)

    async def _apply_execution(
        self,
        opt_id: str,
        execution: OptimizationExecution,
    ) -> None:
        async with self._lock:
            record = self._optimizations.get(opt_id)
            if not record:
                return
            record.trials = [TrialRecord.from_summary(trial) for trial in execution.trials]
            record.best_trial = (
                TrialRecord.from_summary(execution.best_trial)
                if execution.best_trial
                else None
            )
            record.replay_backtest = execution.replay_backtest
            if record.state != OptimizationState.cancelled:
                record.state = execution.state
                record.error = execution.error
                record.finished_at = datetime.now(timezone.utc)
            elif record.finished_at is None:
                record.finished_at = datetime.now(timezone.utc)
