from __future__ import annotations

import asyncio
from fastapi import Depends, FastAPI, HTTPException, Query, WebSocket, WebSocketDisconnect, status
from fastapi.middleware.cors import CORSMiddleware

from .adapter import MockEngineAdapter
from .auth import require_api_key
from .models import BacktestRequest, BacktestResult, BacktestStatus, RunState
from .store import RunStore

store = RunStore()
adapter = MockEngineAdapter(store)

app = FastAPI(
    title="GlowBack Gateway API",
    version="0.1.0",
    dependencies=[Depends(require_api_key)],
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


@app.post("/backtests", response_model=BacktestStatus, status_code=status.HTTP_201_CREATED)
async def create_backtest(request: BacktestRequest) -> BacktestStatus:
    status_obj = await store.create_run(request)
    asyncio.create_task(adapter.run(status_obj.run_id, request))
    return status_obj


@app.get("/backtests", response_model=list[BacktestStatus])
async def list_backtests(
    state: RunState | None = Query(default=None),
    limit: int = Query(default=50, ge=1, le=200),
) -> list[BacktestStatus]:
    return await store.list_runs(state=state, limit=limit)


@app.get("/backtests/{run_id}", response_model=BacktestStatus)
async def get_backtest(run_id: str) -> BacktestStatus:
    status_obj = await store.get_status(run_id)
    if not status_obj:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Run not found")
    return status_obj


@app.get("/backtests/{run_id}/results", response_model=BacktestResult)
async def get_backtest_results(run_id: str) -> BacktestResult:
    result = await store.get_result(run_id)
    if not result:
        raise HTTPException(
            status_code=status.HTTP_409_CONFLICT,
            detail="Results not ready",
        )
    return result


@app.websocket("/backtests/{run_id}/stream")
async def stream_backtest(
    websocket: WebSocket,
    run_id: str,
    last_event_id: int | None = Query(default=None),
) -> None:
    await websocket.accept()
    status_obj = await store.get_status(run_id)
    if not status_obj:
        await websocket.close(code=status.WS_1008_POLICY_VIOLATION)
        return

    backlog = await store.get_events_after(run_id, last_event_id)
    for event in backlog:
        await websocket.send_json(event.model_dump())

    queue = await store.subscribe(run_id)
    if not queue:
        await websocket.close(code=status.WS_1008_POLICY_VIOLATION)
        return

    try:
        while True:
            event = await queue.get()
            await websocket.send_json(event.model_dump())
    except WebSocketDisconnect:
        return
    finally:
        await store.unsubscribe(run_id, queue)
