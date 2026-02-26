# Security Configuration

GlowBack follows SOC2 alignment practices. This page documents the security
controls available in the API gateway and how to configure them.

## Authentication

The API requires an API key for all endpoints except `/healthz`.

Set the allowed keys via the `GLOWBACK_API_KEY` environment variable
(comma-separated for multiple keys):

```bash
export GLOWBACK_API_KEY="key-one,key-two"
```

Keys can be sent as:

- `Authorization: Bearer <key>` header (preferred)
- `X-API-Key: <key>` header
- `?api_key=<key>` query parameter (not recommended for production)

If `GLOWBACK_API_KEY` is **unset**, authentication is disabled (development
mode only).

## CORS

Set `GLOWBACK_CORS_ORIGINS` to a comma-separated list of allowed origins:

```bash
export GLOWBACK_CORS_ORIGINS="https://app.example.com,https://admin.example.com"
```

When unset, no CORS middleware is applied (requests from other origins are
blocked by browsers by default).

## Rate Limiting

Per-IP token-bucket rate limiting is enforced on all authenticated endpoints.

| Variable              | Default | Description            |
| --------------------- | ------- | ---------------------- |
| `GLOWBACK_RATE_LIMIT` | 100     | Max requests per window |
| `GLOWBACK_RATE_WINDOW` | 60     | Window in seconds       |

Rate limit headers are returned on every response:

- `X-RateLimit-Limit`
- `X-RateLimit-Remaining`
- `X-RateLimit-Reset`

## Request Body Size

The maximum request body size defaults to **1 MiB**. Override with:

```bash
export GLOWBACK_MAX_BODY_BYTES=2097152  # 2 MiB
```

Requests exceeding the limit receive a `413 Request Entity Too Large` response.

## Security Headers

All HTTP responses include the following headers:

| Header                      | Value                                                  |
| --------------------------- | ------------------------------------------------------ |
| `X-Content-Type-Options`    | `nosniff`                                              |
| `X-Frame-Options`           | `DENY`                                                 |
| `Referrer-Policy`           | `no-referrer`                                          |
| `Permissions-Policy`        | `geolocation=(), microphone=(), camera=()`             |
| `Cache-Control`             | `no-store`                                             |
| `Strict-Transport-Security` | `max-age=63072000; includeSubDomains; preload`         |
| `Content-Security-Policy`   | `default-src 'none'; frame-ancestors 'none'`           |
| `X-Request-ID`              | Per-request UUID (or client-provided `X-Request-ID`)   |

## Structured Logging

Logs are emitted as single-line JSON by default for easy ingestion into SIEM
systems. Override with `GLOWBACK_LOG_FORMAT=text` for human-readable output.

Set the log level with `GLOWBACK_LOG_LEVEL` (default `INFO`).

Every HTTP request is logged with:

- `request_id`
- `method`, `path`, `status`
- `client_ip`
- `duration_ms`

Failed authentication and rate-limit violations are logged at `WARNING` level.

## Health Check

`GET /healthz` returns `{"status": "healthy", "version": "..."}` with no
authentication required. Use this for load balancer and Kubernetes liveness
probes.

## Audit Trail

All requests receive a unique `X-Request-ID` that threads through logs and
response headers, enabling full request traceability for incident response.

## Input Validation

- **Strategy names** are restricted to lowercase alphanumeric + underscore
  (`^[a-z0-9_]+$`), max 128 characters.
- **Currency** must be 3–5 uppercase letters (`^[A-Z]{3,5}$`).
- **Resolution** is constrained to the enum
  `tick|second|minute|hour|day`.
- **Symbols list** is capped at 100 entries.
- **Initial capital** must be positive and ≤ 1 trillion.
