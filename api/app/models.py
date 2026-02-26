from __future__ import annotations

from datetime import datetime
from enum import Enum
from typing import Any

from pydantic import BaseModel, Field


class StrategyConfig(BaseModel):
    name: str = Field(
        default="buy_and_hold",
        description="Strategy identifier",
        min_length=1,
        max_length=128,
        pattern=r"^[a-z0-9_]+$",
    )
    params: dict[str, Any] = Field(default_factory=dict, description="Strategy parameters")


class ExecutionConfig(BaseModel):
    slippage_bps: float | None = Field(default=None, description="Slippage in basis points")
    commission_bps: float | None = Field(default=None, description="Commission in basis points")
    latency_ms: int | None = Field(default=None, description="Latency in milliseconds")


class BacktestRequest(BaseModel):
    symbols: list[str] = Field(min_length=1, max_length=100)
    start_date: datetime
    end_date: datetime
    resolution: str = Field(
        default="day",
        description="tick|second|minute|hour|day",
        pattern=r"^(tick|second|minute|hour|day)$",
    )
    strategy: StrategyConfig = Field(default_factory=StrategyConfig)
    execution: ExecutionConfig = Field(default_factory=ExecutionConfig)
    initial_capital: float = Field(default=1_000_000.0, gt=0, le=1e12)
    currency: str = Field(default="USD", min_length=3, max_length=5, pattern=r"^[A-Z]{3,5}$")
    timezone: str = Field(default="UTC", max_length=64)


class RunState(str, Enum):
    queued = "queued"
    running = "running"
    completed = "completed"
    failed = "failed"


class BacktestStatus(BaseModel):
    run_id: str
    state: RunState
    progress: float = Field(default=0.0, ge=0.0, le=1.0)
    created_at: datetime
    started_at: datetime | None = None
    finished_at: datetime | None = None
    error: str | None = None


class EventType(str, Enum):
    log = "log"
    progress = "progress"
    metric = "metric"
    state = "state"


class BacktestEvent(BaseModel):
    event_id: int
    run_id: str
    type: EventType
    timestamp: datetime
    payload: dict[str, Any]


class BacktestResult(BaseModel):
    run_id: str
    metrics_summary: dict[str, float] = Field(default_factory=dict)
    equity_curve: list[dict[str, Any]] = Field(default_factory=list)
    trades: list[dict[str, Any]] = Field(default_factory=list)
    exposures: list[dict[str, Any]] = Field(default_factory=list)
    logs: list[str] = Field(default_factory=list)
