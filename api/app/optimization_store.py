"""In-memory store for optimization runs and trials."""

from __future__ import annotations

import asyncio
import random
import time
from dataclasses import dataclass, field
from datetime import datetime, timezone
from typing import Any
from uuid import uuid4

from .optimization_models import (
    ObjectiveDirection,
    OptimizationRequest,
    OptimizationResult,
    OptimizationState,
    OptimizationStatus,
    ParameterKind,
    TrialStatus,
    TrialSummary,
)


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
    started_at: datetime | None = None
    finished_at: datetime | None = None

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
        )


def _sample_parameter(param) -> Any:
    """Sample a single parameter value based on its kind."""
    if param.kind == ParameterKind.float_range:
        return round(random.uniform(param.low, param.high), 6)
    elif param.kind == ParameterKind.int_range:
        return random.randint(int(param.low), int(param.high))
    elif param.kind == ParameterKind.log_uniform:
        import math

        log_low = math.log(param.low)
        log_high = math.log(param.high)
        return round(math.exp(random.uniform(log_low, log_high)), 6)
    elif param.kind == ParameterKind.choice:
        return random.choice(param.values)
    return None


def _grid_values(param, steps: int = 5) -> list[Any]:
    """Generate grid values for a parameter."""
    if param.kind == ParameterKind.int_range:
        low, high = int(param.low), int(param.high)
        return list(range(low, high + 1))
    elif param.kind == ParameterKind.float_range:
        return [
            round(param.low + i * (param.high - param.low) / (steps - 1), 6)
            for i in range(steps)
        ]
    elif param.kind == ParameterKind.log_uniform:
        import math

        log_low = math.log(param.low)
        log_high = math.log(param.high)
        return [
            round(math.exp(log_low + i * (log_high - log_low) / (steps - 1)), 6)
            for i in range(steps)
        ]
    elif param.kind == ParameterKind.choice:
        return list(param.values)
    return []


def _generate_grid_combos(
    request: OptimizationRequest,
) -> list[dict[str, Any]]:
    """Generate all grid combinations (cartesian product)."""
    import itertools

    axes = []
    names = []
    for param in request.search_space.parameters:
        names.append(param.name)
        axes.append(_grid_values(param, request.grid_steps))
    combos = []
    for vals in itertools.product(*axes):
        combos.append(dict(zip(names, vals)))
    return combos[: request.max_trials]


def _generate_trial_params(
    request: OptimizationRequest, count: int
) -> list[dict[str, Any]]:
    """Generate parameter combinations for trials."""
    if request.strategy.value == "grid":
        return _generate_grid_combos(request)
    # random and bayesian both start with random samples
    results = []
    for _ in range(count):
        params = {}
        for param in request.search_space.parameters:
            params[param.name] = _sample_parameter(param)
        results.append(params)
    return results


class OptimizationStore:
    """In-memory store for optimization runs."""

    def __init__(self) -> None:
        self._optimizations: dict[str, OptimizationRecord] = {}
        self._lock = asyncio.Lock()

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
        """Execute the optimization run (simulated backtest trials).

        In a production setup, each trial would dispatch a real backtest to
        the Rust engine (or a Ray cluster).  Here we simulate the execution
        with randomized metrics to demonstrate the orchestration flow.
        """
        async with self._lock:
            record = self._optimizations.get(opt_id)
            if not record:
                return
            record.state = OptimizationState.running
            record.started_at = datetime.now(timezone.utc)

        request = record.request
        all_params = _generate_trial_params(request, request.max_trials)

        # Create trial records
        trials: list[TrialRecord] = []
        for i, params in enumerate(all_params):
            trial = TrialRecord(
                trial_id=str(uuid4()),
                trial_number=i,
                parameters=params,
            )
            trials.append(trial)

        async with self._lock:
            record.trials = trials

        # Process trials in batches (simulated concurrency)
        batch_size = request.concurrency
        for batch_start in range(0, len(trials), batch_size):
            # Check for cancellation
            async with self._lock:
                if record.state == OptimizationState.cancelled:
                    return

            batch = trials[batch_start : batch_start + batch_size]

            # Mark batch as running
            async with self._lock:
                for trial in batch:
                    trial.status = TrialStatus.running
                    trial.started_at = datetime.now(timezone.utc)

            # Simulate backtest execution
            await asyncio.sleep(0.05)  # Simulate work

            # Complete batch with simulated results
            async with self._lock:
                for trial in batch:
                    start_ts = time.monotonic()
                    # Simulated metrics — in production, these come from the
                    # Rust backtesting engine.
                    trial.metrics = {
                        "total_return": round(random.uniform(-0.3, 0.8), 4),
                        "sharpe_ratio": round(random.uniform(-1.0, 3.0), 4),
                        "max_drawdown": round(random.uniform(0.01, 0.5), 4),
                        "volatility": round(random.uniform(0.05, 0.6), 4),
                        "win_rate": round(random.uniform(0.3, 0.7), 4),
                    }
                    obj_metric = request.objective_metric
                    trial.objective = trial.metrics.get(obj_metric, 0.0)
                    trial.status = TrialStatus.completed
                    trial.finished_at = datetime.now(timezone.utc)
                    trial.duration_seconds = max(1, int(time.monotonic() - start_ts))

                    # Update best trial
                    if record.best_trial is None:
                        record.best_trial = trial
                    else:
                        is_better = (
                            request.direction == ObjectiveDirection.maximize
                            and (trial.objective or 0) > (record.best_trial.objective or 0)
                        ) or (
                            request.direction == ObjectiveDirection.minimize
                            and (trial.objective or 0) < (record.best_trial.objective or 0)
                        )
                        if is_better:
                            record.best_trial = trial

        # Mark completed
        async with self._lock:
            if record.state == OptimizationState.running:
                record.state = OptimizationState.completed
                record.finished_at = datetime.now(timezone.utc)
