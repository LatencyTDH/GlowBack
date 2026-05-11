# Deployment Guide

This guide covers the Docker Compose service topology for local deployment and staging-style smoke tests. Treat it as a starting point for production packaging, not a turnkey production runbook.

## Quick Start

```bash
# Clone and enter the repository
git clone https://github.com/LatencyTDH/GlowBack.git
cd GlowBack

# Copy the environment template
cp .env.example .env

# Build and start all services
docker compose up --build -d
```

Services become available at:

| Service | URL                    | Purpose              |
|---------|------------------------|----------------------|
| UI      | http://localhost:8501   | Streamlit dashboard  |
| API     | http://localhost:8000   | FastAPI REST service |
| Engine  | http://localhost:8081   | Rust backtest engine |

## Architecture

```
┌──────────┐     ┌──────────┐     ┌────────────────────┐
│    UI    │────▸│   API    │────▸│ gb-python + engine │
│ :8501    │     │ :8000    │     │ embedded runtime   │
│Streamlit │     │ FastAPI  │     └────────────────────┘
└──────────┘     └──────────┘

┌──────────┐
│ Engine   │  Standalone gb-engine-service exposed on :8081
│ :8081    │
└──────────┘
```

The UI depends on the API, and Compose currently starts the standalone Engine service before the API with health-check-based `depends_on`. Current API backtest requests run through the `gb-python`/shared-runtime path, so a green container health check proves the web process is reachable, not that every backtest path is production-ready. Run the smoke checks below before relying on a deployment.

## Smoke Checks

After Compose reports healthy services, verify both liveness and an engine-backed request path:

```bash
docker compose ps
curl -fsS http://localhost:8000/healthz
```

If you set `GLOWBACK_API_KEY`, include it when creating a sample run:

```bash
curl -fsS -X POST http://localhost:8000/v1/backtests \
  -H 'Content-Type: application/json' \
  -H "X-API-Key: ${GLOWBACK_API_KEY:-}" \
  -d '{
    "symbols": ["AAPL"],
    "start_date": "2024-01-01T00:00:00Z",
    "end_date": "2024-01-31T00:00:00Z",
    "resolution": "day",
    "strategy": {"name": "buy_and_hold"},
    "data_source": "sample"
  }'
```

For local engine/API development outside Docker, prefer the checked-in quickstart plus the [FastAPI Gateway](api/fastapi.md) setup instructions; those commands are closer to the CI-validated path.

## Configuration

Copy `.env.example` to `.env` and edit as needed:

```bash
cp .env.example .env
```

### Ports

| Variable               | Default | Description          |
|------------------------|---------|----------------------|
| `GLOWBACK_ENGINE_PORT` | 8081    | Engine host port     |
| `GLOWBACK_API_PORT`    | 8000    | API host port        |
| `GLOWBACK_UI_PORT`     | 8501    | UI host port         |

### Security

| Variable                | Default | Description                              |
|-------------------------|---------|------------------------------------------|
| `GLOWBACK_API_KEY`      | _(empty)_ | API key for request auth (empty = off) |
| `GLOWBACK_CORS_ORIGINS` | `*`     | Comma-separated allowed CORS origins     |
| `GLOWBACK_RATE_LIMIT`   | 100     | Max requests per IP per window           |
| `GLOWBACK_RATE_WINDOW`  | 60      | Rate limit window in seconds             |
| `GLOWBACK_MAX_BODY_BYTES` | 1048576 | Max request body size (bytes)          |

### Logging

| Variable              | Default | Description                    |
|-----------------------|---------|--------------------------------|
| `GLOWBACK_LOG_FORMAT` | `json`  | Log format: `json` or `text`   |
| `GLOWBACK_LOG_LEVEL`  | `info`  | Python log level               |
| `RUST_LOG`            | `info`  | Rust tracing filter for engine |

## Health Checks

Every service includes a Docker health check:

- **Engine:** Probes `http://localhost:8081/` via `curl`.
- **API:** Probes `http://localhost:8000/healthz` via Python's `urllib`.
- **UI:** Probes Streamlit's `http://localhost:8501/_stcore/health` via Python's
  `urllib`.

Check status at any time:

```bash
docker compose ps
```

The `STATUS` column shows `healthy`, `starting`, or `unhealthy`.

## Data Persistence

Two named volumes preserve data across container restarts:

| Volume        | Mount Point | Purpose                  |
|---------------|-------------|--------------------------|
| `engine-data` | `/app/data` | Engine working data      |
| `api-data`    | `/app/data` | API state and catalogs   |

To back up volumes:

```bash
docker run --rm -v glowback_engine-data:/data -v $(pwd):/backup \
  busybox tar czf /backup/engine-data.tar.gz -C /data .
```

## Resource Limits

Default memory and CPU limits are set in `docker-compose.yml`:

| Service | Memory | CPUs |
|---------|--------|------|
| Engine  | 2 GB   | 2.0  |
| API     | 512 MB | 1.0  |
| UI      | 512 MB | 1.0  |

Adjust these in `docker-compose.yml` under `deploy.resources.limits` for your
hardware.

## Production Hardening Checklist

1. **Set `GLOWBACK_API_KEY`** — never run without authentication outside trusted local development.
2. **Restrict CORS** — change `GLOWBACK_CORS_ORIGINS` from `*` to the exact browser origins you control.
3. **Use a reverse proxy** — put Nginx, Caddy, or Traefik in front for TLS, request logging, and access controls.
4. **Run an engine-backed smoke test** — do not rely on `/healthz` alone; create a sample `/v1/backtests` run and fetch its result.
5. **Monitor logs** — use `docker compose logs -f` locally or ship structured API/engine logs to a log aggregator.
6. **Set resource limits** — tune memory/CPU for your workload and data sizes.
7. **Back up volumes** — schedule periodic backups of named volumes and test restore procedures.

## Updating

```bash
git pull
docker compose up --build -d
```

Compose rebuilds only changed images and restarts affected services.

## Stopping

```bash
# Stop services (keeps volumes)
docker compose down

# Stop and remove volumes (data loss!)
docker compose down -v
```
