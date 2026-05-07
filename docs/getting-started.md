# Getting Started

## Prerequisites

- Rust (latest stable)
- Python 3.8+ (for docs, API, and UI workflows)

## 5-Minute Quickstart

Clone the repo and run the checked-in quickstart script:

```bash
git clone https://github.com/LatencyTDH/GlowBack.git
cd GlowBack
./scripts/quickstart.sh
```

That script is the same one used by CI. It builds and runs the `gb-types` basic usage example from a clean checkout, then verifies the expected success markers:

- `✅ All basic functionality working!`
- `🎊 Strategy library complete with 4 different strategies!`

If you want to inspect the underlying command directly, it is:

```bash
cargo run --locked --example basic_usage -p gb-types
```

## What the Quickstart Proves

After the script succeeds, you have verified that this checkout can:

- build the core Rust crates needed for the example
- exercise sample-data loading and portfolio operations
- instantiate the built-in strategy library
- finish a runnable end-to-end smoke path without hidden setup

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

- Read the runnable [Examples](examples/index.md) page for the exact quickstart output snapshot.
- Review [Assumptions and Limitations](assumptions-and-limitations.md) before moving from sample data to real research data.
- Try the [Reproducing a Run](tutorials/reproducing-a-run.md) tutorial once you have a completed backtest result.
