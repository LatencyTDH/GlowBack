# FAQ

## Which quickstart should I run first?

Run `./scripts/quickstart.sh` from the repository root. It is the same smoke path used by CI and verifies the core Rust example before you spend time on API/UI setup.

## Why does the quickstart say four strategies if the docs list six?

The Rust/Python strategy library currently includes six built-ins: `buy_and_hold`, `ma_crossover`, `momentum`, `mean_reversion`, `rsi`, and the experimental `covered_call`. The quickstart smoke example still exercises four strategies and keeps its historical success marker so CI can detect drift.

## Do I need Rust installed to use the UI?

Not strictly. The Streamlit UI can run in Python-only mode for local exploration, but engine-backed workflows and API optimization runs require the `gb-python` extension built with Rust/maturin.

## Why does the API quickstart set `PYTHONPATH=..`?

`api/app` is a package below the repository root, while `glowback_runtime.py` lives at the root. Running Uvicorn from `api/` needs the parent directory on `PYTHONPATH` so the gateway can import the shared runtime helper.

## Which API paths should new clients use?

Use `/v1/...` paths. Unversioned aliases still exist for local compatibility, but the versioned contract is the public surface documented for clients.

## Where do I file bugs?

Open an issue in the GitHub repository with reproduction steps. For security issues, follow the [Security Policy](community/security.md) and do not open a public issue.
