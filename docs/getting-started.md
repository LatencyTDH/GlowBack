# Getting Started

## Prerequisites

- Rust (latest stable)
- Python 3.8+ (for Python bindings)

## Clone and Test

```bash
git clone https://github.com/LatencyTDH/GlowBack.git
cd GlowBack
cargo test --workspace
```

## Run an Example

```bash
cargo run --example basic_usage -p gb-types
```

## Launch the UI

```bash
cd ui
python setup.py
# Opens http://localhost:8501
```

## Docker (Compose)

```bash
# Copy environment template and customize
cp .env.example .env

# Build and start all services
docker compose up --build -d
```

Note: The engine image builds with `rust:stable-slim` to match `rust-toolchain.toml`.

Services:
- UI: http://localhost:8501
- API: http://localhost:8000 (set `GLOWBACK_API_KEY` to require auth)
- Engine: http://localhost:8081 (health JSON)

All services include health checks, restart policies, and resource limits.
For production deployment details, see the [Deployment Guide](deployment.md).

## Next Steps

- Load sample data via the UI
- Try a built‑in strategy template
- Review results in the dashboard
