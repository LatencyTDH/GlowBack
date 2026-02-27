"""Optimization models for parameter search and distributed optimization."""

from __future__ import annotations

from datetime import datetime
from enum import Enum
from typing import Any

from pydantic import BaseModel, Field


class ParameterKind(str, Enum):
    float_range = "float_range"
    int_range = "int_range"
    log_uniform = "log_uniform"
    choice = "choice"


class ParameterDef(BaseModel):
    name: str = Field(min_length=1, max_length=128)
    kind: ParameterKind
    low: float | None = None
    high: float | None = None
    values: list[Any] | None = None


class SearchSpaceConfig(BaseModel):
    parameters: list[ParameterDef] = Field(min_length=1, max_length=50)


class ObjectiveDirection(str, Enum):
    maximize = "maximize"
    minimize = "minimize"


class SearchStrategyName(str, Enum):
    grid = "grid"
    random = "random"
    bayesian = "bayesian"


class RayClusterConfig(BaseModel):
    address: str = Field(default="ray://localhost:10001")
    namespace: str = Field(default="glowback")
    max_concurrent_tasks: int = Field(default=4, ge=1, le=256)
    num_cpus: float = Field(default=1.0, gt=0)
    num_gpus: float = Field(default=0.0, ge=0)
    pip_packages: list[str] = Field(default_factory=list)


class OptimizationRequest(BaseModel):
    name: str = Field(min_length=1, max_length=256)
    description: str = Field(default="", max_length=1024)
    search_space: SearchSpaceConfig
    strategy: SearchStrategyName = SearchStrategyName.random
    max_trials: int = Field(default=100, ge=1, le=10_000)
    concurrency: int = Field(default=4, ge=1, le=128)
    objective_metric: str = Field(default="sharpe_ratio", max_length=64)
    direction: ObjectiveDirection = ObjectiveDirection.maximize
    base_backtest: dict[str, Any] = Field(
        description="Base backtest config that trials will override with sampled parameters"
    )
    exploration_weight: float = Field(default=0.3, ge=0.0, le=1.0)
    grid_steps: int = Field(default=5, ge=2, le=100)
    ray_cluster: RayClusterConfig | None = Field(
        default=None,
        description="Optional Ray cluster config for distributed execution",
    )


class OptimizationState(str, Enum):
    pending = "pending"
    running = "running"
    completed = "completed"
    failed = "failed"
    cancelled = "cancelled"


class TrialStatus(str, Enum):
    pending = "pending"
    running = "running"
    completed = "completed"
    failed = "failed"


class TrialSummary(BaseModel):
    trial_id: str
    trial_number: int
    status: TrialStatus
    parameters: dict[str, Any] = Field(default_factory=dict)
    objective: float | None = None
    metrics: dict[str, float] = Field(default_factory=dict)
    duration_seconds: int | None = None
    error: str | None = None


class OptimizationStatus(BaseModel):
    optimization_id: str
    name: str
    state: OptimizationState
    strategy: str
    objective_metric: str
    direction: str
    max_trials: int
    trials_completed: int = 0
    trials_failed: int = 0
    trials_running: int = 0
    best_trial: TrialSummary | None = None
    created_at: datetime
    started_at: datetime | None = None
    finished_at: datetime | None = None
    error: str | None = None


class OptimizationResult(BaseModel):
    optimization_id: str
    state: OptimizationState
    best_trial: TrialSummary | None = None
    all_trials: list[TrialSummary] = Field(default_factory=list)
    total_duration_seconds: int | None = None
    search_space: SearchSpaceConfig | None = None
