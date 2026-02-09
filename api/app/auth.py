from __future__ import annotations

import os

from fastapi import HTTPException, Request, status


async def require_api_key(request: Request) -> None:
    api_key = os.getenv("GLOWBACK_API_KEY")
    if not api_key:
        return
    auth_header = request.headers.get("Authorization", "")
    expected = f"Bearer {api_key}"
    if auth_header != expected:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Invalid or missing API key",
        )
