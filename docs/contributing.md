# Contributing to GlowBack

Thank you for your interest in contributing! This guide covers everything you
need to get started.

## Code of Conduct

All participants are expected to follow our [Code of Conduct](community/code-of-conduct.md).

## Getting Started

1. Fork the repository and clone your fork.
2. Install prerequisites:
   - **Rust** (stable toolchain — see `rust-toolchain.toml`)
   - **Python 3.8+** (for the UI and Python bindings)
   - **Node.js** (optional, for the dashboard)
3. Build and run the test suite:

```bash
cargo test --workspace
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
cargo test --workspace

# Run tests for a specific crate
cargo test -p gb-types
cargo test -p gb-engine
```

### Python / UI

```bash
cd ui
python -c "import py_compile; py_compile.compile('app.py')"
```

### CI Checks

Every pull request runs:

- **Rust CI** (`rust.yml`) — `cargo test --workspace` and `cargo clippy`
- **Docs guard** (`docs-guard.yml`) — ensures documentation builds cleanly
- **Docs deploy** (`docs.yml`) — deploys to GitHub Pages on merge to `main`

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
pip install mkdocs-material pymdown-extensions
mkdocs serve
```

The site auto-deploys to GitHub Pages when changes merge to `main`.

## License

By contributing, you agree that your contributions will be licensed under the
same license as the project (see [LICENSE](https://github.com/LatencyTDH/GlowBack/blob/main/LICENSE)).
