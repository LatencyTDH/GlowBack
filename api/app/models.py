from __future__ import annotations

from datetime import datetime
from enum import Enum
from typing import Any

from pydantic import BaseModel, Field, model_validator


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


class PortfolioConstructionConfig(BaseModel):
    method: str = Field(default="target_weights", pattern=r"^target_weights$")
    target_weights: dict[str, float] = Field(default_factory=dict, description="Symbol -> weight fraction")
    rebalance_frequency: str = Field(
        default="weekly",
        description="daily|weekly|monthly",
        pattern=r"^(daily|weekly|monthly)$",
    )
    drift_threshold_pct: float | None = Field(default=None, ge=0.0, le=100.0)
    max_weight_pct: float | None = Field(default=None, gt=0.0, le=100.0)
    max_turnover_pct: float | None = Field(default=None, ge=0.0, le=500.0)
    cash_floor_pct: float = Field(default=0.0, ge=0.0, le=95.0)
    max_drawdown_pct: float | None = Field(default=None, gt=0.0, le=100.0)

    @model_validator(mode="after")
    def validate_target_weights(self) -> "PortfolioConstructionConfig":
        cleaned: dict[str, float] = {}
        total_weight = 0.0
        for raw_symbol, raw_weight in self.target_weights.items():
            symbol = str(raw_symbol).strip().upper()
            weight = float(raw_weight)
            if not symbol or weight <= 0:
                continue
            cleaned[symbol] = weight
            total_weight += weight

        if not cleaned:
            raise ValueError("portfolio_construction.target_weights must include at least one positive symbol weight")
        if total_weight <= 0:
            raise ValueError("portfolio_construction.target_weights must sum to a positive value")

        self.target_weights = cleaned
        return self


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
    benchmark_symbol: str | None = Field(default=None, min_length=1, max_length=16)
    portfolio_construction: PortfolioConstructionConfig | None = None
    data_source: str = Field(default="default", pattern=r"^(default|sample|csv)$")
    csv_data_path: str | None = Field(default=None, description="Directory containing {symbol}_{resolution}.csv files")


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
    metrics_summary: dict[str, Any] = Field(default_factory=dict)
    equity_curve: list[dict[str, Any]] = Field(default_factory=list)
    benchmark_curve: list[dict[str, Any]] = Field(default_factory=list)
    benchmark_symbol: str | None = None
    trades: list[dict[str, Any]] = Field(default_factory=list)
    exposures: list[dict[str, Any]] = Field(default_factory=list)
    order_events: list[dict[str, Any]] = Field(default_factory=list)
    portfolio_construction: dict[str, Any] = Field(default_factory=dict)
    portfolio_diagnostics: list[dict[str, Any]] = Field(default_factory=list)
    constraint_hits: list[dict[str, Any]] = Field(default_factory=list)
    tearsheet: dict[str, Any] = Field(default_factory=dict)
    logs: list[str] = Field(default_factory=list)
    final_cash: float | None = None
    final_positions: dict[str, float] = Field(default_factory=dict)
    manifest: dict[str, Any] | None = None
