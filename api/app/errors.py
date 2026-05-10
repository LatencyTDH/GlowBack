from __future__ import annotations

from http import HTTPStatus
from typing import Any

from fastapi.encoders import jsonable_encoder
from fastapi.responses import JSONResponse
from pydantic import BaseModel, Field


class ApiErrorBody(BaseModel):
    code: str = Field(description="Stable machine-readable error code")
    message: str = Field(description="Human-readable summary of the failure")
    details: Any | None = Field(default=None, description="Optional structured validation or debugging details")


class ApiErrorEnvelope(BaseModel):
    error: ApiErrorBody
    request_id: str | None = Field(default=None, description="Request correlation id echoed from X-Request-ID when available")


VERSIONED_ERROR_RESPONSES = {
    401: {"model": ApiErrorEnvelope, "description": "Invalid or missing API key"},
    404: {"model": ApiErrorEnvelope, "description": "Requested resource was not found"},
    409: {"model": ApiErrorEnvelope, "description": "Request conflicts with current resource state"},
    413: {"model": ApiErrorEnvelope, "description": "Request body exceeded the configured size limit"},
    422: {"model": ApiErrorEnvelope, "description": "Request validation failed"},
    429: {"model": ApiErrorEnvelope, "description": "Client exceeded the configured rate limit"},
    500: {"model": ApiErrorEnvelope, "description": "Unexpected server error"},
}


def is_versioned_path(path: str) -> bool:
    return path == "/v1" or path.startswith("/v1/")


def _status_code_name(status_code: int) -> str:
    try:
        phrase = HTTPStatus(status_code).phrase.lower().replace(" ", "_")
    except ValueError:
        phrase = "http_error"
    return phrase


def build_error_payload(
    *,
    status_code: int,
    message: str,
    request_id: str | None,
    details: Any | None = None,
    code: str | None = None,
) -> dict[str, Any]:
    envelope = ApiErrorEnvelope(
        error=ApiErrorBody(
            code=code or _status_code_name(status_code),
            message=message,
            details=details,
        ),
        request_id=request_id,
    )
    return jsonable_encoder(envelope)


def build_error_response(
    *,
    status_code: int,
    message: str,
    request_id: str | None,
    details: Any | None = None,
    code: str | None = None,
    headers: dict[str, str] | None = None,
) -> JSONResponse:
    return JSONResponse(
        status_code=status_code,
        content=build_error_payload(
            status_code=status_code,
            message=message,
            request_id=request_id,
            details=details,
            code=code,
        ),
        headers=headers,
    )
