# Contributing to GlowBack

Thank you for your interest in contributing! This guide covers everything you
need to get started.

## Code of Conduct

All participants are expected to follow our [Code of Conduct](community/code-of-conduct.md).

## Getting Started

1. Fork the repository and clone your fork.
2. Install prerequisites:
   - **Rust** (stable toolchain — see `rust-toolchain.toml`)
   - **Python 3.10+** for requirements-based API/UI/docs workflows (CI uses 3.12; `ui/pyproject.toml` pins 3.12 for `uv`)
   - **maturin** when building the local `gb-python` extension (`python -m pip install maturin`)
3. Build and run the test suite:

```bash
cargo test --workspace --locked
```

4. Launch the Streamlit UI locally:

```bash
cd ui
pip install -r requirements.txt
streamlit run app.py
```

## Development Workflow

1. Create a branch from `main`:
   ```bash
   git checkout -b feat/my-feature main
   ```
2. Make focused, well-scoped changes.
3. Write or update tests for any new behavior.
4. Update documentation if user-facing behavior changes.
5. Run the full test suite before pushing.
6. Open a pull request with a clear summary and test notes.

## Releases

GlowBack releases are cut manually from a previously built CI artifact so a
maintainer can publish a version without rebuilding during the release step.
See [Releasing GlowBack](releasing.md) for the exact workflow and inputs.

## Repository Layout

| Directory       | Description                             |
| --------------- | --------------------------------------- |
| `crates/`       | Rust workspace crates                   |
| `crates/gb-types/` | Core types, strategies, and metrics |
| `crates/gb-engine/` | Backtest simulation engine          |
| `crates/gb-data/`   | Data ingestion and storage          |
| `crates/gb-python/`  | PyO3 Python bindings               |
| `ui/`           | Streamlit web UI                        |
| `docs/`         | MkDocs documentation source             |
| `docker/`       | Dockerfiles for services                |
| `.github/`      | CI workflows and issue templates        |

## Testing

### Rust

```bash
# Run all workspace tests
cargo test --workspace --locked

# Run tests for a specific crate
cargo test -p gb-types --locked
cargo test -p gb-engine --locked
```

### Python / API / UI

```bash
python -m pip install -r api/requirements.txt -r ui/requirements.txt httpx
PYTHONPATH="$PWD" python -m compileall api ui
PYTHONPATH="$PWD" python -m unittest discover -s api/tests -v
PYTHONPATH="$PWD" python -m unittest discover -s ui/tests -v
```

To run the API locally from the `api/` directory, include the repository root on `PYTHONPATH` so `glowback_runtime.py` is importable:

```bash
cd api
PYTHONPATH=.. uvicorn app.main:app --reload
```

### CI Checks

Every pull request runs:

- **Rust Build Pipeline** (`rust.yml`) — crate-scoped Rust build/tests, Python API/UI tests, and release artifact packaging on eligible `main` builds
- **Docs Guard** (`docs-guard.yml`) — requires a docs update when a PR changes user-facing code unless maintainers apply `no-docs`
- **Docs Smoke** (`docs-smoke.yml`) — runs `./scripts/quickstart.sh`, `./scripts/python_sdk_quickstart.sh`, `./scripts/csv_data_tutorial.sh`, `./scripts/replay_manifest_tutorial.sh`, and `mkdocs build --strict` for docs/example changes
- **Docs Deploy** (`docs.yml`) — builds and publishes the MkDocs site from `main`

## Pull Request Guidelines

- Fill out the PR template (summary, checklist, docs note).
- Keep PRs focused — one logical change per PR.
- Reference related issues (e.g., `Fixes #14`).
- Ensure all CI checks pass before requesting review.
- Delete your feature branch after merge.

## Reporting Issues

- **Bugs**: Use the [Bug Report](https://github.com/LatencyTDH/GlowBack/issues/new?template=bug_report.md) template.
- **Features**: Use the [Feature Request](https://github.com/LatencyTDH/GlowBack/issues/new?template=feature_request.md) template.
- **Security**: See [Security Policy](community/security.md) — do not open public issues for vulnerabilities.

## Documentation

We use [MkDocs Material](https://squidfunk.github.io/mkdocs-material/). To
preview docs locally:

```bash
python -m pip install mkdocs-material pymdown-extensions
python -m mkdocs serve
```

The site auto-deploys to GitHub Pages when changes merge to `main`.

## License

By contributing, you agree that your contributions will be licensed under the
same license as the project (see [LICENSE](https://github.com/LatencyTDH/GlowBack/blob/main/LICENSE)).
