from __future__ import annotations

import asyncio
import json
import logging
import os
import sys
import time
import uuid

from fastapi import Depends, FastAPI, HTTPException, Query, Request, Response, WebSocket, WebSocketDisconnect, status
from fastapi.exception_handlers import (
    http_exception_handler as default_http_exception_handler,
    request_validation_exception_handler as default_request_validation_exception_handler,
)
from fastapi.exceptions import RequestValidationError
from fastapi.middleware.cors import CORSMiddleware
from starlette.exceptions import HTTPException as StarletteHTTPException

from .adapter import RealEngineAdapter
from .auth import require_api_key, validate_api_key
from .errors import VERSIONED_ERROR_RESPONSES, build_error_response, is_versioned_path
from .models import BacktestRequest, BacktestResult, BacktestStatus, RunState
from .optimization_models import OptimizationRequest, OptimizationResult, OptimizationState, OptimizationStatus
from .optimization_store import OptimizationStore
from .rate_limit import rate_limit_check
from .store import RunStore

# ---------------------------------------------------------------------------
# Structured JSON logging (SOC2: machine-parseable audit trail)
# ---------------------------------------------------------------------------

_LOG_FORMAT = os.getenv("GLOWBACK_LOG_FORMAT", "json")  # "json" or "text"


class _JsonFormatter(logging.Formatter):
    """Emit log records as single-line JSON objects."""

    def format(self, record: logging.LogRecord) -> str:
        payload = {
            "timestamp": self.formatTime(record, datefmt="%Y-%m-%dT%H:%M:%S.%fZ"),
            "level": record.levelname,
            "logger": record.name,
            "message": record.getMessage(),
        }
        if record.exc_info and record.exc_info[0] is not None:
            payload["exception"] = self.formatException(record.exc_info)
        return json.dumps(payload, default=str)


def _configure_logging() -> None:
    root = logging.getLogger()
    root.setLevel(os.getenv("GLOWBACK_LOG_LEVEL", "INFO").upper())
    handler = logging.StreamHandler(sys.stdout)
    if _LOG_FORMAT == "json":
        handler.setFormatter(_JsonFormatter())
    else:
        handler.setFormatter(
            logging.Formatter("%(asctime)s %(levelname)s %(name)s %(message)s")
        )
    root.handlers = [handler]


_configure_logging()
logger = logging.getLogger("glowback.api")

# ---------------------------------------------------------------------------
# Application setup
# ---------------------------------------------------------------------------

_MAX_BODY_BYTES = int(os.getenv("GLOWBACK_MAX_BODY_BYTES", str(1024 * 1024)))  # 1 MiB default

store = RunStore()
adapter = RealEngineAdapter(store)
opt_store = OptimizationStore()

app = FastAPI(
    title="GlowBack Gateway API",
    version="0.2.0",
    dependencies=[Depends(require_api_key), Depends(rate_limit_check)],
)

# ---------------------------------------------------------------------------
# CORS (SOC2: restrict cross-origin access)
# ---------------------------------------------------------------------------

_CORS_ORIGINS = os.getenv("GLOWBACK_CORS_ORIGINS", "").split(",")
_CORS_ORIGINS = [o.strip() for o in _CORS_ORIGINS if o.strip()]

if _CORS_ORIGINS:
    app.add_middleware(
        CORSMiddleware,
        allow_origins=_CORS_ORIGINS,
        allow_credentials=True,
        allow_methods=["GET", "POST"],
        allow_headers=["Authorization", "X-API-Key", "X-Request-ID", "Content-Type"],
        expose_headers=["X-Request-ID", "X-RateLimit-Limit", "X-RateLimit-Remaining", "X-RateLimit-Reset"],
        max_age=600,
    )

# ---------------------------------------------------------------------------
# Security headers helper
# ---------------------------------------------------------------------------


def _apply_security_headers(response) -> None:
    headers = response.headers
    headers.setdefault("X-Content-Type-Options", "nosniff")
    headers.setdefault("X-Frame-Options", "DENY")
    headers.setdefault("Referrer-Policy", "no-referrer")
    headers.setdefault("Permissions-Policy", "geolocation=(), microphone=(), camera=()")
    headers.setdefault("Cache-Control", "no-store")
    headers.setdefault(
        "Strict-Transport-Security", "max-age=63072000; includeSubDomains; preload"
    )
    headers.setdefault(
        "Content-Security-Policy", "default-src 'none'; frame-ancestors 'none'"
    )


# ---------------------------------------------------------------------------
# Audit middleware
# ---------------------------------------------------------------------------


@app.middleware("http")
async def audit_middleware(request: Request, call_next):
    request_id = request.headers.get("x-request-id") or str(uuid.uuid4())
    request.state.request_id = request_id
    client_host = request.client.host if request.client else "unknown"
    start = time.monotonic()

    # --- Enforce request body size limit (SOC2: prevent abuse) ---
    content_length = request.headers.get("content-length")
    if content_length and int(content_length) > _MAX_BODY_BYTES:
        logger.warning(
            "request_body_too_large request_id=%s client_ip=%s content_length=%s max=%s",
            request_id,
            client_host,
            content_length,
            _MAX_BODY_BYTES,
        )
        if is_versioned_path(request.url.path):
            return build_error_response(
                status_code=status.HTTP_413_REQUEST_ENTITY_TOO_LARGE,
                message="Request body too large",
                request_id=request_id,
                code="request_too_large",
            )
        return Response(
            content='{"detail":"Request body too large"}',
            status_code=status.HTTP_413_REQUEST_ENTITY_TOO_LARGE,
            media_type="application/json",
        )

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

    # Apply rate-limit headers if present
    rate_headers = getattr(request.state, "rate_limit_headers", None)
    if rate_headers:
        for key, value in rate_headers.items():
            response.headers.setdefault(key, value)

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


@app.exception_handler(StarletteHTTPException)
async def http_exception_handler(request: Request, exc: StarletteHTTPException):
    if not is_versioned_path(request.url.path):
        return await default_http_exception_handler(request, exc)

    detail = exc.detail
    message = detail if isinstance(detail, str) else "Request failed"
    details = None if isinstance(detail, str) else detail
    return build_error_response(
        status_code=exc.status_code,
        message=message,
        request_id=getattr(request.state, "request_id", None),
        details=details,
        headers=exc.headers,
    )


@app.exception_handler(RequestValidationError)
async def validation_exception_handler(request: Request, exc: RequestValidationError):
    if not is_versioned_path(request.url.path):
        return await default_request_validation_exception_handler(request, exc)

    return build_error_response(
        status_code=status.HTTP_422_UNPROCESSABLE_ENTITY,
        message="Request validation failed",
        request_id=getattr(request.state, "request_id", None),
        details=exc.errors(),
        code="validation_error",
    )


@app.exception_handler(Exception)
async def unhandled_exception_handler(request: Request, exc: Exception):
    if not is_versioned_path(request.url.path):
        return Response(
            content='{"detail":"Internal Server Error"}',
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            media_type="application/json",
        )

    return build_error_response(
        status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
        message="Internal server error",
        request_id=getattr(request.state, "request_id", None),
        code="internal_error",
    )


# ---------------------------------------------------------------------------
# Health check (unauthenticated, excluded from rate limiting via path guard)
# ---------------------------------------------------------------------------


@app.get("/v1/healthz", include_in_schema=True, dependencies=[])
@app.get("/healthz", include_in_schema=True, dependencies=[])
async def health_check() -> dict:
    """Liveness probe — no auth required."""
    return {"status": "healthy", "version": app.version}


# ---------------------------------------------------------------------------
# Backtest endpoints
# ---------------------------------------------------------------------------


@app.post(
    "/v1/backtests",
    response_model=BacktestStatus,
    status_code=status.HTTP_201_CREATED,
    responses=VERSIONED_ERROR_RESPONSES,
)
@app.post("/backtests", response_model=BacktestStatus, status_code=status.HTTP_201_CREATED)
async def create_backtest(request: BacktestRequest) -> BacktestStatus:
    status_obj = await store.create_run(request)
    asyncio.create_task(adapter.run(status_obj.run_id, request))
    return status_obj


@app.get("/v1/backtests", response_model=list[BacktestStatus], responses=VERSIONED_ERROR_RESPONSES)
@app.get("/backtests", response_model=list[BacktestStatus])
async def list_backtests(
    state: RunState | None = Query(default=None),
    limit: int = Query(default=50, ge=1, le=200),
) -> list[BacktestStatus]:
    return await store.list_runs(state=state, limit=limit)


@app.get("/v1/backtests/{run_id}", response_model=BacktestStatus, responses=VERSIONED_ERROR_RESPONSES)
@app.get("/backtests/{run_id}", response_model=BacktestStatus)
async def get_backtest(run_id: str) -> BacktestStatus:
    status_obj = await store.get_status(run_id)
    if not status_obj:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Run not found")
    return status_obj


@app.get(
    "/v1/backtests/{run_id}/results",
    response_model=BacktestResult,
    responses=VERSIONED_ERROR_RESPONSES,
)
@app.get("/backtests/{run_id}/results", response_model=BacktestResult)
async def get_backtest_results(run_id: str) -> BacktestResult:
    result = await store.get_result(run_id)
    if not result:
        raise HTTPException(
            status_code=status.HTTP_409_CONFLICT,
            detail="Results not ready",
        )
    return result


@app.websocket("/v1/backtests/{run_id}/stream")
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
        key_status = "present" if provided else "absent"
        logger.warning(
            "ws_api_key_rejected request_id=%s path=%s client_ip=%s key_status=%s",
            request_id,
            websocket.url.path,
            client_host,
            key_status,
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


# ---------------------------------------------------------------------------
# Optimization endpoints
# ---------------------------------------------------------------------------


@app.post(
    "/v1/optimizations",
    response_model=OptimizationStatus,
    status_code=status.HTTP_201_CREATED,
    responses=VERSIONED_ERROR_RESPONSES,
)
@app.post(
    "/optimizations",
    response_model=OptimizationStatus,
    status_code=status.HTTP_201_CREATED,
)
async def create_optimization(request: OptimizationRequest) -> OptimizationStatus:
    """Create an optimization run and execute it in the background."""
    status_obj = await opt_store.create(request)
    asyncio.create_task(opt_store.run_optimization(status_obj.optimization_id))
    return status_obj


@app.get("/v1/optimizations", response_model=list[OptimizationStatus], responses=VERSIONED_ERROR_RESPONSES)
@app.get("/optimizations", response_model=list[OptimizationStatus])
async def list_optimizations(
    limit: int = Query(default=50, ge=1, le=200),
) -> list[OptimizationStatus]:
    """List optimization runs (most recent first)."""
    return await opt_store.list_optimizations(limit=limit)


@app.get(
    "/v1/optimizations/{opt_id}",
    response_model=OptimizationStatus,
    responses=VERSIONED_ERROR_RESPONSES,
)
@app.get("/optimizations/{opt_id}", response_model=OptimizationStatus)
async def get_optimization(opt_id: str) -> OptimizationStatus:
    """Get current status of an optimization run."""
    status_obj = await opt_store.get_status(opt_id)
    if not status_obj:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND, detail="Optimization not found"
        )
    return status_obj


@app.get(
    "/v1/optimizations/{opt_id}/results",
    response_model=OptimizationResult,
    responses=VERSIONED_ERROR_RESPONSES,
)
@app.get("/optimizations/{opt_id}/results", response_model=OptimizationResult)
async def get_optimization_results(opt_id: str) -> OptimizationResult:
    """Get full results of a completed optimization run."""
    result = await opt_store.get_result(opt_id)
    if not result:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="Optimization not found",
        )
    if result.state not in {
        OptimizationState.completed,
        OptimizationState.failed,
        OptimizationState.cancelled,
    }:
        raise HTTPException(
            status_code=status.HTTP_409_CONFLICT,
            detail="Optimization not yet completed",
        )
    return result


@app.post("/v1/optimizations/{opt_id}/cancel", responses=VERSIONED_ERROR_RESPONSES)
@app.post("/optimizations/{opt_id}/cancel")
async def cancel_optimization(opt_id: str) -> dict:
    """Cancel a running optimization."""
    cancelled = await opt_store.cancel(opt_id)
    if not cancelled:
        raise HTTPException(
            status_code=status.HTTP_409_CONFLICT,
            detail="Cannot cancel — optimization not found or already finished",
        )
    return {"optimization_id": opt_id, "state": "cancelled"}


@app.post("/v1/optimizations/{opt_id}/resume", responses=VERSIONED_ERROR_RESPONSES)
@app.post("/optimizations/{opt_id}/resume")
async def resume_optimization(opt_id: str) -> OptimizationStatus:
    """Resume a canceled or failed optimization run."""
    prepared = await opt_store.prepare_resume(opt_id)
    if not prepared:
        raise HTTPException(
            status_code=status.HTTP_409_CONFLICT,
            detail="Cannot resume — optimization not found or not resumable",
        )

    status_obj, prior_trials = prepared
    asyncio.create_task(opt_store.run_optimization(opt_id, prior_trials=prior_trials))
    return status_obj
