# Deployment Guide

This guide covers deploying GlowBack with Docker Compose for development and
production environments.

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
┌──────────┐     ┌──────────┐     ┌──────────┐
│    UI    │────▸│   API    │────▸│  Engine  │
│ :8501    │     │ :8000    │     │ :8081    │
│Streamlit │     │ FastAPI  │     │  Rust    │
└──────────┘     └──────────┘     └──────────┘
```

The UI depends on the API, and the API depends on the Engine. Docker Compose
enforces this ordering with health-check-based `depends_on` so each service
waits for its dependency to be fully healthy before starting.

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

## Production Checklist

1. **Set `GLOWBACK_API_KEY`** — never run without authentication in production.
2. **Restrict CORS** — change `GLOWBACK_CORS_ORIGINS` from `*` to your domain.
3. **Use a reverse proxy** — put Nginx, Caddy, or Traefik in front for TLS.
4. **Monitor logs** — use `docker compose logs -f` or ship to a log aggregator.
5. **Set resource limits** — tune memory/CPU for your workload.
6. **Back up volumes** — schedule periodic backups of named volumes.

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
