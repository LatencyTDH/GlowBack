from __future__ import annotations

import asyncio
import copy
import itertools
import math
import random
import statistics
import time
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from typing import Any, Protocol

from .models import BacktestRequest
from .optimization_models import (
    OptimizationRequest,
    OptimizationState,
    ParameterDef,
    ParameterKind,
    SearchStrategyName,
    TrialStatus,
    TrialSummary,
    ValidationMode,
)


class BacktestExecutor(Protocol):
    def run(self, backtest_config: dict[str, Any]) -> dict[str, Any]:
        ...


class GlowbackBacktestExecutor:
    """Run real backtests through the gb-python bindings."""

    def __init__(self) -> None:
        self._module = None

    def _load_module(self):
        if self._module is None:
            try:
                import glowback  # type: ignore
            except ImportError as exc:  # pragma: no cover - exercised in real runtime
                raise RuntimeError(
                    "The `glowback` Python bindings are not available. Build or install "
                    "`gb-python` before running optimization jobs."
                ) from exc
            self._module = glowback
        return self._module

    def run(self, backtest_config: dict[str, Any]) -> dict[str, Any]:
        glowback = self._load_module()
        normalized = normalize_backtest_config(backtest_config)
        execution = normalized.get("execution") or {}
        strategy = normalized["strategy"]

        result = glowback.run_builtin_strategy(
            symbols=normalized["symbols"],
            start_date=normalized["start_date"],
            end_date=normalized["end_date"],
            strategy_name=strategy["name"],
            strategy_params=strategy.get("params") or {},
            resolution=normalized.get("resolution", "day"),
            initial_capital=float(normalized.get("initial_capital", 100000.0)),
            name=normalized.get("name") or "Optimization Trial",
            data_source=normalized.get("data_source") or "sample",
            commission_bps=execution.get("commission_bps"),
            slippage_bps=execution.get("slippage_bps"),
            latency_ms=execution.get("latency_ms"),
        )
        return {
            "metrics_summary": dict(result.metrics_summary),
            "equity_curve": list(result.equity_curve),
        }


@dataclass
class OptimizationExecution:
    state: OptimizationState
    trials: list[TrialSummary]
    best_trial: TrialSummary | None = None
    replay_backtest: dict[str, Any] | None = None
    error: str | None = None


@dataclass(frozen=True)
class ValidationWindow:
    start: datetime
    end: datetime


class OptimizationExecutor:
    def __init__(self, backtest_executor: BacktestExecutor | None = None) -> None:
        self._backtest_executor = backtest_executor or GlowbackBacktestExecutor()

    async def execute(
        self,
        request: OptimizationRequest,
        is_cancelled=None,
    ) -> OptimizationExecution:
        trials: list[TrialSummary] = []
        completed_trials: list[TrialSummary] = []

        if request.strategy == SearchStrategyName.bayesian:
            parameter_sets: list[dict[str, Any]] | None = None
        else:
            parameter_sets = self._generate_parameter_sets(request)
            if not parameter_sets:
                raise RuntimeError("Search space produced no trial candidates")

        for trial_number in range(1, request.max_trials + 1):
            if await self._is_cancelled(is_cancelled):
                best_trial = self._select_best(completed_trials, request.direction.value)
                return OptimizationExecution(
                    state=OptimizationState.cancelled,
                    trials=trials,
                    best_trial=best_trial,
                    replay_backtest=(
                        build_replay_backtest(request.base_backtest, best_trial.parameters)
                        if best_trial
                        else None
                    ),
                )

            parameters = (
                parameter_sets[trial_number - 1]
                if parameter_sets is not None
                else self._next_bayesian_parameters(request, completed_trials, trial_number)
            )
            trial = await asyncio.to_thread(
                self._evaluate_trial_sync,
                request,
                trial_number,
                parameters,
            )
            trials.append(trial)
            if trial.status == TrialStatus.completed:
                completed_trials.append(trial)

            if parameter_sets is not None and trial_number >= len(parameter_sets):
                break

        best_trial = self._select_best(completed_trials, request.direction.value)
        if not completed_trials:
            return OptimizationExecution(
                state=OptimizationState.failed,
                trials=trials,
                error="All optimization trials failed",
            )

        return OptimizationExecution(
            state=OptimizationState.completed,
            trials=trials,
            best_trial=best_trial,
            replay_backtest=(
                build_replay_backtest(request.base_backtest, best_trial.parameters)
                if best_trial
                else None
            ),
        )

    async def _is_cancelled(self, is_cancelled) -> bool:
        if is_cancelled is None:
            return False
        result = is_cancelled()
        if asyncio.iscoroutine(result):
            result = await result
        return bool(result)

    def _evaluate_trial_sync(
        self,
        request: OptimizationRequest,
        trial_number: int,
        parameters: dict[str, Any],
    ) -> TrialSummary:
        started = time.monotonic()
        trial_id = f"trial-{trial_number}"
        replay_backtest = build_replay_backtest(request.base_backtest, parameters)

        try:
            full_result = self._backtest_executor.run(replay_backtest)
            full_metrics = _coerce_metrics(full_result.get("metrics_summary") or {})
            metric_name = request.objective_metric
            full_objective = _metric_value(full_metrics, metric_name)

            metrics = dict(full_metrics)
            validation_windows = build_validation_windows(replay_backtest, request)
            if validation_windows:
                first_window = validation_windows[0]
                train_end = first_window.start - timedelta(days=1)
                train_backtest = copy.deepcopy(replay_backtest)
                if train_end > _parse_dt(train_backtest["start_date"]):
                    train_backtest["end_date"] = _isoformat(train_end)
                    train_result = self._backtest_executor.run(train_backtest)
                    train_metrics = _coerce_metrics(train_result.get("metrics_summary") or {})
                    metrics[f"train_{metric_name}"] = _metric_value(train_metrics, metric_name)

                validation_scores: list[float] = []
                for window in validation_windows:
                    validation_backtest = copy.deepcopy(replay_backtest)
                    validation_backtest["start_date"] = _isoformat(window.start)
                    validation_backtest["end_date"] = _isoformat(window.end)
                    validation_result = self._backtest_executor.run(validation_backtest)
                    validation_metrics = _coerce_metrics(
                        validation_result.get("metrics_summary") or {}
                    )
                    validation_scores.append(_metric_value(validation_metrics, metric_name))

                objective = statistics.mean(validation_scores)
                metrics[f"validation_{metric_name}"] = objective
                metrics["validation_windows"] = float(len(validation_scores))
                if len(validation_scores) > 1:
                    metrics[f"validation_{metric_name}_stddev"] = statistics.pstdev(
                        validation_scores
                    )
            else:
                objective = full_objective

            metrics[f"full_{metric_name}"] = full_objective
            return TrialSummary(
                trial_id=trial_id,
                trial_number=trial_number,
                status=TrialStatus.completed,
                parameters=parameters,
                objective=objective,
                metrics=metrics,
                duration_seconds=max(0, int(time.monotonic() - started)),
            )
        except Exception as exc:
            return TrialSummary(
                trial_id=trial_id,
                trial_number=trial_number,
                status=TrialStatus.failed,
                parameters=parameters,
                duration_seconds=max(0, int(time.monotonic() - started)),
                error=str(exc),
            )

    def _generate_parameter_sets(self, request: OptimizationRequest) -> list[dict[str, Any]]:
        if request.strategy == SearchStrategyName.grid:
            return self._grid_parameter_sets(request.search_space.parameters, request.grid_steps, request.max_trials)
        return self._random_parameter_sets(request, request.max_trials)

    def _grid_parameter_sets(
        self,
        parameters: list[ParameterDef],
        grid_steps: int,
        max_trials: int,
    ) -> list[dict[str, Any]]:
        axes = []
        for parameter in parameters:
            values = self._grid_values(parameter, grid_steps)
            if not values:
                raise RuntimeError(f"Parameter '{parameter.name}' produced no grid values")
            axes.append([(parameter.name, value) for value in values])

        combos = []
        for combo in itertools.product(*axes):
            combos.append(dict(combo))
            if len(combos) >= max_trials:
                break
        return combos

    def _grid_values(self, parameter: ParameterDef, grid_steps: int) -> list[Any]:
        if parameter.kind == ParameterKind.choice:
            return list(parameter.values or [])

        low = parameter.low
        high = parameter.high
        if low is None or high is None:
            raise RuntimeError(f"Parameter '{parameter.name}' must define low/high bounds")
        if high < low:
            raise RuntimeError(f"Parameter '{parameter.name}' has high < low")

        if parameter.kind == ParameterKind.int_range:
            values = {
                int(round(low + (high - low) * step / max(grid_steps - 1, 1)))
                for step in range(grid_steps)
            }
            return sorted(values)

        if parameter.kind == ParameterKind.float_range:
            if math.isclose(low, high):
                return [float(low)]
            return [
                float(low + (high - low) * step / max(grid_steps - 1, 1))
                for step in range(grid_steps)
            ]

        if parameter.kind == ParameterKind.log_uniform:
            if low <= 0 or high <= 0:
                raise RuntimeError(
                    f"Parameter '{parameter.name}' must be > 0 for log-uniform search"
                )
            if math.isclose(low, high):
                return [float(low)]
            log_low = math.log(low)
            log_high = math.log(high)
            return [
                float(math.exp(log_low + (log_high - log_low) * step / max(grid_steps - 1, 1)))
                for step in range(grid_steps)
            ]

        raise RuntimeError(f"Unsupported parameter kind: {parameter.kind}")

    def _random_parameter_sets(
        self,
        request: OptimizationRequest,
        count: int,
    ) -> list[dict[str, Any]]:
        rng = random.Random(request.random_seed)
        samples: list[dict[str, Any]] = []
        seen: set[str] = set()
        attempts = 0
        max_attempts = max(count * 20, 100)

        while len(samples) < count and attempts < max_attempts:
            attempts += 1
            params = {
                parameter.name: self._sample_parameter(rng, parameter)
                for parameter in request.search_space.parameters
            }
            key = repr(sorted(params.items()))
            if key in seen:
                continue
            seen.add(key)
            samples.append(params)

        return samples

    def _sample_parameter(self, rng: random.Random, parameter: ParameterDef) -> Any:
        if parameter.kind == ParameterKind.choice:
            values = list(parameter.values or [])
            if not values:
                raise RuntimeError(f"Choice parameter '{parameter.name}' must define values")
            return rng.choice(values)

        low = parameter.low
        high = parameter.high
        if low is None or high is None:
            raise RuntimeError(f"Parameter '{parameter.name}' must define low/high bounds")
        if high < low:
            raise RuntimeError(f"Parameter '{parameter.name}' has high < low")

        if parameter.kind == ParameterKind.int_range:
            return rng.randint(int(round(low)), int(round(high)))
        if parameter.kind == ParameterKind.float_range:
            return rng.uniform(low, high)
        if parameter.kind == ParameterKind.log_uniform:
            if low <= 0 or high <= 0:
                raise RuntimeError(
                    f"Parameter '{parameter.name}' must be > 0 for log-uniform search"
                )
            return math.exp(rng.uniform(math.log(low), math.log(high)))

        raise RuntimeError(f"Unsupported parameter kind: {parameter.kind}")

    def _next_bayesian_parameters(
        self,
        request: OptimizationRequest,
        completed_trials: list[TrialSummary],
        trial_number: int,
    ) -> dict[str, Any]:
        rng = random.Random(request.random_seed + trial_number * 9973)
        explore_trials = max(3, min(8, len(request.search_space.parameters) * 2))
        if len(completed_trials) < explore_trials:
            return {
                parameter.name: self._sample_parameter(rng, parameter)
                for parameter in request.search_space.parameters
            }

        best_trial = self._select_best(completed_trials, request.direction.value)
        best_parameters = best_trial.parameters if best_trial else {}
        proposal: dict[str, Any] = {}
        for parameter in request.search_space.parameters:
            best_value = best_parameters.get(parameter.name)
            if best_value is None or rng.random() < request.exploration_weight:
                proposal[parameter.name] = self._sample_parameter(rng, parameter)
                continue

            if parameter.kind == ParameterKind.choice:
                values = list(parameter.values or [])
                if not values:
                    raise RuntimeError(f"Choice parameter '{parameter.name}' must define values")
                proposal[parameter.name] = best_value if rng.random() < 0.7 else rng.choice(values)
                continue

            low = parameter.low
            high = parameter.high
            if low is None or high is None:
                raise RuntimeError(f"Parameter '{parameter.name}' must define low/high bounds")

            if parameter.kind == ParameterKind.int_range:
                span = max(1, int(round((high - low) * 0.2)))
                candidate = int(round(best_value)) + rng.randint(-span, span)
                proposal[parameter.name] = max(int(round(low)), min(int(round(high)), candidate))
            elif parameter.kind == ParameterKind.float_range:
                sigma = max((high - low) * 0.15, 1e-9)
                candidate = float(best_value) + rng.gauss(0.0, sigma)
                proposal[parameter.name] = max(low, min(high, candidate))
            elif parameter.kind == ParameterKind.log_uniform:
                if low <= 0 or high <= 0:
                    raise RuntimeError(
                        f"Parameter '{parameter.name}' must be > 0 for log-uniform search"
                    )
                log_best = math.log(float(best_value))
                sigma = max((math.log(high) - math.log(low)) * 0.2, 1e-9)
                candidate = math.exp(log_best + rng.gauss(0.0, sigma))
                proposal[parameter.name] = max(low, min(high, candidate))
            else:
                raise RuntimeError(f"Unsupported parameter kind: {parameter.kind}")

        return proposal

    def _select_best(
        self,
        trials: list[TrialSummary],
        direction: str,
    ) -> TrialSummary | None:
        if not trials:
            return None
        reverse = direction == "maximize"
        return sorted(
            trials,
            key=lambda trial: float("-inf") if trial.objective is None else trial.objective,
            reverse=reverse,
        )[0]


def normalize_backtest_config(base_backtest: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(base_backtest, dict):
        raise RuntimeError("Optimization base_backtest must be an object")

    validated = BacktestRequest.model_validate(base_backtest)
    normalized = validated.model_dump(mode="json")
    strategy = base_backtest.get("strategy") or {}
    normalized["strategy"] = {
        "name": str(strategy.get("name") or validated.strategy.name).strip().lower(),
        "params": dict(strategy.get("params") or {}),
    }
    normalized["name"] = base_backtest.get("name") or validated.strategy.name or "Optimization Trial"
    normalized["data_source"] = _resolve_data_source(base_backtest)
    execution = base_backtest.get("execution") or {}
    normalized["execution"] = {
        key: execution[key]
        for key in ("commission_bps", "slippage_bps", "latency_ms")
        if key in execution and execution[key] is not None
    }
    return normalized


def build_replay_backtest(base_backtest: dict[str, Any], parameters: dict[str, Any]) -> dict[str, Any]:
    replay_backtest = copy.deepcopy(normalize_backtest_config(base_backtest))
    replay_backtest.setdefault("strategy", {})
    replay_params = dict(replay_backtest["strategy"].get("params") or {})
    replay_params.update(parameters)
    replay_backtest["strategy"]["params"] = replay_params
    return replay_backtest


def build_validation_windows(
    replay_backtest: dict[str, Any],
    request: OptimizationRequest,
) -> list[ValidationWindow]:
    start = _parse_dt(replay_backtest["start_date"])
    end = _parse_dt(replay_backtest["end_date"])
    total_days = max(1, (end.date() - start.date()).days + 1)
    if total_days < 8:
        return []

    validation_days = max(1, int(total_days * request.validation_fraction))
    if validation_days >= total_days:
        return []

    validation_start_offset = total_days - validation_days
    windows = 1 if request.validation_mode == ValidationMode.holdout else min(request.walk_forward_windows, validation_days)
    windows = max(1, windows)
    base_window_days = max(1, validation_days // windows)
    remainder = max(0, validation_days - base_window_days * windows)

    result: list[ValidationWindow] = []
    offset = validation_start_offset
    for index in range(windows):
        size = base_window_days + (1 if index < remainder else 0)
        window_start = start + timedelta(days=offset)
        window_end = min(end, window_start + timedelta(days=size - 1))
        if window_start > window_end:
            break
        if window_start <= start:
            continue
        result.append(ValidationWindow(start=window_start, end=window_end))
        offset += size

    return result


def _resolve_data_source(base_backtest: dict[str, Any]) -> str:
    direct = base_backtest.get("data_source")
    if isinstance(direct, str) and direct.strip():
        return direct.strip()
    data_settings = base_backtest.get("data_settings")
    if isinstance(data_settings, dict):
        value = data_settings.get("data_source")
        if isinstance(value, str) and value.strip():
            return value.strip()
    return "sample"


def _parse_dt(value: Any) -> datetime:
    if isinstance(value, datetime):
        dt = value
    else:
        text = str(value).strip()
        if text.endswith("Z"):
            text = text[:-1] + "+00:00"
        dt = datetime.fromisoformat(text)
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    return dt.astimezone(timezone.utc)


def _isoformat(value: datetime) -> str:
    return value.astimezone(timezone.utc).isoformat().replace("+00:00", "Z")


def _coerce_metrics(raw_metrics: dict[str, Any]) -> dict[str, float]:
    metrics: dict[str, float] = {}
    for key, value in raw_metrics.items():
        try:
            metrics[key] = float(value)
        except (TypeError, ValueError):
            continue
    return metrics


def _metric_value(metrics: dict[str, float], metric_name: str) -> float:
    if metric_name not in metrics:
        available = ", ".join(sorted(metrics.keys())) or "none"
        raise RuntimeError(
            f"Objective metric '{metric_name}' is unavailable for this trial. "
            f"Available metrics: {available}"
        )
    return metrics[metric_name]
