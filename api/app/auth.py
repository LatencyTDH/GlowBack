from __future__ import annotations

import logging
import os
from typing import Mapping

from fastapi import HTTPException, Request, status

logger = logging.getLogger("glowback.api.auth")


def _load_api_keys() -> set[str]:
    raw = os.getenv("GLOWBACK_API_KEY", "")
    return {key.strip() for key in raw.split(",") if key.strip()}


def _extract_bearer_token(header_value: str | None) -> str | None:
    if not header_value:
        return None
    prefix = "bearer "
    if header_value.lower().startswith(prefix):
        token = header_value[len(prefix) :].strip()
        return token or None
    return None


def extract_api_key(
    headers: Mapping[str, str],
    query_params: Mapping[str, str] | None = None,
) -> str | None:
    token = _extract_bearer_token(headers.get("authorization"))
    if token:
        return token
    api_key = headers.get("x-api-key")
    if api_key:
        return api_key.strip() or None
    if query_params:
        query_key = query_params.get("api_key")
        if query_key:
            return query_key.strip() or None
    return None


def validate_api_key(
    headers: Mapping[str, str],
    query_params: Mapping[str, str] | None = None,
) -> tuple[bool, bool]:
    keys = _load_api_keys()
    if not keys:
        return True, False
    provided = extract_api_key(headers, query_params)
    if not provided:
        return False, False
    return provided in keys, True


async def require_api_key(request: Request) -> None:
    authorized, provided = validate_api_key(request.headers)
    if not authorized:
        request_id = getattr(request.state, "request_id", None)
        client_host = request.client.host if request.client else "unknown"
        key_status = "present" if provided else "absent"
        logger.warning(
            "api_key_rejected request_id=%s method=%s path=%s client_ip=%s key_status=%s",
            request_id,
            request.method,
            request.url.path,
            client_host,
            key_status,
        )
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Invalid or missing API key",
        )
