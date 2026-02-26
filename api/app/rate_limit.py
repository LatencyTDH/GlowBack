"""Simple in-memory per-IP rate limiter for SOC2 compliance."""

from __future__ import annotations

import os
import time
from collections import defaultdict
from dataclasses import dataclass, field
from threading import Lock

from fastapi import HTTPException, Request, status


@dataclass
class _Bucket:
    """Token bucket for a single client."""

    tokens: float
    last_refill: float


class RateLimiter:
    """Per-IP token-bucket rate limiter.

    Configurable via environment variables:
      - GLOWBACK_RATE_LIMIT  – max requests per window (default 100)
      - GLOWBACK_RATE_WINDOW – window in seconds (default 60)
    """

    def __init__(self) -> None:
        self._max_tokens = int(os.getenv("GLOWBACK_RATE_LIMIT", "100"))
        self._window = float(os.getenv("GLOWBACK_RATE_WINDOW", "60"))
        self._buckets: dict[str, _Bucket] = defaultdict(
            lambda: _Bucket(tokens=self._max_tokens, last_refill=time.monotonic())
        )
        self._lock = Lock()

    def _refill(self, bucket: _Bucket) -> None:
        now = time.monotonic()
        elapsed = now - bucket.last_refill
        bucket.tokens = min(
            self._max_tokens,
            bucket.tokens + elapsed * (self._max_tokens / self._window),
        )
        bucket.last_refill = now

    def check(self, client_ip: str) -> tuple[bool, dict[str, str]]:
        """Return (allowed, headers). Headers are always set for observability."""
        with self._lock:
            bucket = self._buckets[client_ip]
            self._refill(bucket)

            headers = {
                "X-RateLimit-Limit": str(self._max_tokens),
                "X-RateLimit-Remaining": str(max(0, int(bucket.tokens) - 1)),
                "X-RateLimit-Reset": str(int(bucket.last_refill + self._window)),
            }

            if bucket.tokens >= 1:
                bucket.tokens -= 1
                return True, headers

            return False, headers


_rate_limiter = RateLimiter()


async def rate_limit_check(request: Request) -> None:
    """FastAPI dependency that enforces per-IP rate limiting."""
    client_ip = request.client.host if request.client else "unknown"
    allowed, headers = _rate_limiter.check(client_ip)
    # Stash headers so the audit middleware can apply them
    request.state.rate_limit_headers = headers
    if not allowed:
        raise HTTPException(
            status_code=status.HTTP_429_TOO_MANY_REQUESTS,
            detail="Rate limit exceeded",
            headers=headers,
        )
