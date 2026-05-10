from __future__ import annotations

import asyncio
from dataclasses import dataclass, field
from datetime import datetime, timezone
from typing import Iterable
from uuid import uuid4

from .experiment_registry import ExperimentRegistry, get_default_registry
from .models import BacktestEvent, BacktestRequest, BacktestResult, BacktestStatus, EventType, RunState


@dataclass
class RunRecord:
    request: BacktestRequest
    status: BacktestStatus
    result: BacktestResult | None = None
    events: list[BacktestEvent] = field(default_factory=list)
    subscribers: set[asyncio.Queue] = field(default_factory=set)
    next_event_id: int = 1


class RunStore:
    def __init__(self, registry: ExperimentRegistry | None = None) -> None:
        self._runs: dict[str, RunRecord] = {}
        self._lock = asyncio.Lock()
        self._registry = registry or get_default_registry()
        self._hydrate_from_registry()

    def _hydrate_from_registry(self) -> None:
        for payload in self._registry.list_runs(source="api", limit=500):
            request_payload = payload.get("request")
            status_payload = payload.get("status")
            if not request_payload or not status_payload:
                continue

            request = BacktestRequest.model_validate(request_payload)
            status = BacktestStatus.model_validate(status_payload)
            result_payload = payload.get("result")
            result = BacktestResult.model_validate(result_payload) if result_payload else None
            event_payloads = payload.get("events") or []
            events = [BacktestEvent.model_validate(event) for event in event_payloads]
            next_event_id = max((event.event_id for event in events), default=0) + 1
            self._runs[status.run_id] = RunRecord(
                request=request,
                status=status,
                result=result,
                events=events,
                next_event_id=next_event_id,
            )

    def _persist_record(self, run_id: str) -> None:
        record = self._runs.get(run_id)
        if not record:
            return
        self._registry.upsert_run(
            run_id=run_id,
            source="api",
            status=record.status,
            request=record.request,
            result=record.result,
            events=record.events,
            metadata={
                "subscriber_count": len(record.subscribers),
                "event_count": len(record.events),
            },
            strategy_name=record.request.strategy.name,
            strategy_config=record.request.strategy,
        )

    async def create_run(self, request: BacktestRequest) -> BacktestStatus:
        run_id = str(uuid4())
        now = datetime.now(timezone.utc)
        status = BacktestStatus(
            run_id=run_id,
            state=RunState.queued,
            progress=0.0,
            created_at=now,
        )
        record = RunRecord(request=request, status=status)
        async with self._lock:
            self._runs[run_id] = record
        self._persist_record(run_id)
        await self._publish_event(
            run_id,
            EventType.state,
            {"state": RunState.queued, "message": "Run queued"},
        )
        return status

    async def list_runs(self, state: RunState | None = None, limit: int = 50) -> list[BacktestStatus]:
        async with self._lock:
            records = list(self._runs.values())
        if state:
            records = [record for record in records if record.status.state == state]
        records.sort(key=lambda record: record.status.created_at, reverse=True)
        return [record.status for record in records[:limit]]

    async def get_status(self, run_id: str) -> BacktestStatus | None:
        async with self._lock:
            record = self._runs.get(run_id)
            return record.status if record else None

    async def get_result(self, run_id: str) -> BacktestResult | None:
        async with self._lock:
            record = self._runs.get(run_id)
            return record.result if record else None

    async def get_events_after(self, run_id: str, last_event_id: int | None) -> list[BacktestEvent]:
        async with self._lock:
            record = self._runs.get(run_id)
            if not record:
                return []
            if last_event_id is None:
                return list(record.events)
            return [event for event in record.events if event.event_id > last_event_id]

    async def subscribe(self, run_id: str) -> asyncio.Queue | None:
        queue: asyncio.Queue[BacktestEvent] = asyncio.Queue(maxsize=100)
        async with self._lock:
            record = self._runs.get(run_id)
            if not record:
                return None
            record.subscribers.add(queue)
            self._persist_record(run_id)
        return queue

    async def unsubscribe(self, run_id: str, queue: asyncio.Queue) -> None:
        async with self._lock:
            record = self._runs.get(run_id)
            if not record:
                return
            record.subscribers.discard(queue)
            self._persist_record(run_id)

    async def update_state(self, run_id: str, state: RunState, error: str | None = None) -> None:
        now = datetime.now(timezone.utc)
        async with self._lock:
            record = self._runs.get(run_id)
            if not record:
                return
            record.status.state = state
            record.status.error = error
            if state == RunState.running and record.status.started_at is None:
                record.status.started_at = now
            if state in {RunState.completed, RunState.failed}:
                record.status.finished_at = now
            self._persist_record(run_id)
        await self._publish_event(
            run_id,
            EventType.state,
            {"state": state, "error": error},
        )

    async def update_progress(self, run_id: str, progress: float, message: str | None = None) -> None:
        async with self._lock:
            record = self._runs.get(run_id)
            if not record:
                return
            record.status.progress = max(0.0, min(progress, 1.0))
            self._persist_record(run_id)
        payload = {"progress": progress}
        if message:
            payload["message"] = message
        await self._publish_event(run_id, EventType.progress, payload)

    async def set_result(self, run_id: str, result: BacktestResult) -> None:
        async with self._lock:
            record = self._runs.get(run_id)
            if not record:
                return
            record.result = result
            self._persist_record(run_id)
        await self.update_state(run_id, RunState.completed)

    async def _publish_event(self, run_id: str, event_type: EventType, payload: dict) -> None:
        async with self._lock:
            record = self._runs.get(run_id)
            if not record:
                return
            event_id = record.next_event_id
            record.next_event_id += 1
            event = BacktestEvent(
                event_id=event_id,
                run_id=run_id,
                type=event_type,
                timestamp=datetime.now(timezone.utc),
                payload=payload,
            )
            record.events.append(event)
            subscribers = list(record.subscribers)
            self._persist_record(run_id)
        await self._fanout(subscribers, event)

    async def _fanout(self, subscribers: Iterable[asyncio.Queue], event: BacktestEvent) -> None:
        for queue in subscribers:
            try:
                queue.put_nowait(event)
            except asyncio.QueueFull:
                continue
