from __future__ import annotations

import asyncio
import logging
import time
import uuid

from fastapi import Depends, FastAPI, HTTPException, Query, Request, WebSocket, WebSocketDisconnect, status

from .adapter import MockEngineAdapter
from .auth import require_api_key, validate_api_key
from .models import BacktestRequest, BacktestResult, BacktestStatus, RunState
from .store import RunStore

logger = logging.getLogger("glowback.api")

store = RunStore()
adapter = MockEngineAdapter(store)

app = FastAPI(
    title="GlowBack Gateway API",
    version="0.1.0",
    dependencies=[Depends(require_api_key)],
)


def _apply_security_headers(response) -> None:
    headers = response.headers
    headers.setdefault("X-Content-Type-Options", "nosniff")
    headers.setdefault("X-Frame-Options", "DENY")
    headers.setdefault("Referrer-Policy", "no-referrer")
    headers.setdefault("Permissions-Policy", "geolocation=(), microphone=(), camera=()")
    headers.setdefault("Cache-Control", "no-store")


@app.middleware("http")
async def audit_middleware(request: Request, call_next):
    request_id = request.headers.get("x-request-id") or str(uuid.uuid4())
    request.state.request_id = request_id
    client_host = request.client.host if request.client else "unknown"
    start = time.monotonic()
    try:
        response = await call_next(request)
    except Exception:
        duration_ms = int((time.monotonic() - start) * 1000)
        logger.exception(
            "request_failed request_id=%s method=%s path=%s client_ip=%s duration_ms=%s",
            request_id,
            request.method,
            request.url.path,
            client_host,
            duration_ms,
        )
        raise
    duration_ms = int((time.monotonic() - start) * 1000)
    response.headers.setdefault("X-Request-ID", request_id)
    _apply_security_headers(response)
    logger.info(
        "request_completed request_id=%s method=%s path=%s status=%s client_ip=%s duration_ms=%s",
        request_id,
        request.method,
        request.url.path,
        response.status_code,
        client_host,
        duration_ms,
    )
    return response


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
    request_id = websocket.headers.get("x-request-id") or str(uuid.uuid4())
    websocket.state.request_id = request_id
    client_host = websocket.client.host if websocket.client else "unknown"

    authorized, provided = validate_api_key(websocket.headers, websocket.query_params)
    if not authorized:
        redacted = f"***{provided[-4:]}" if provided and len(provided) > 4 else "***"
        logger.warning(
            "ws_api_key_rejected request_id=%s path=%s client_ip=%s provided=%s",
            request_id,
            websocket.url.path,
            client_host,
            redacted,
        )
        await websocket.close(code=status.WS_1008_POLICY_VIOLATION)
        return

    await websocket.accept()
    logger.info(
        "ws_connected request_id=%s path=%s client_ip=%s run_id=%s",
        request_id,
        websocket.url.path,
        client_host,
        run_id,
    )

    queue = None
    disconnect_reason = "server"
    try:
        status_obj = await store.get_status(run_id)
        if not status_obj:
            disconnect_reason = "run_not_found"
            logger.warning(
                "ws_run_not_found request_id=%s path=%s client_ip=%s run_id=%s",
                request_id,
                websocket.url.path,
                client_host,
                run_id,
            )
            await websocket.close(code=status.WS_1008_POLICY_VIOLATION)
            return

        backlog = await store.get_events_after(run_id, last_event_id)
        for event in backlog:
            await websocket.send_json(event.model_dump())

        queue = await store.subscribe(run_id)
        if not queue:
            disconnect_reason = "subscribe_failed"
            logger.warning(
                "ws_subscribe_failed request_id=%s path=%s client_ip=%s run_id=%s",
                request_id,
                websocket.url.path,
                client_host,
                run_id,
            )
            await websocket.close(code=status.WS_1008_POLICY_VIOLATION)
            return

        while True:
            event = await queue.get()
            await websocket.send_json(event.model_dump())
    except WebSocketDisconnect:
        disconnect_reason = "client"
    finally:
        if queue:
            await store.unsubscribe(run_id, queue)
        logger.info(
            "ws_disconnected request_id=%s path=%s client_ip=%s run_id=%s reason=%s",
            request_id,
            websocket.url.path,
            client_host,
            run_id,
            disconnect_reason,
        )
