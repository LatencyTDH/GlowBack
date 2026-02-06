# AGENTS.md

Guidance for AI agents contributing to this repository.

## Repository Overview

GlowBack is a Rust‑first quantitative backtesting platform with Python bindings and a Streamlit UI.

**Core crates:**
- `crates/gb-types` — core types, orders, portfolio, strategies
- `crates/gb-data` — data ingestion, providers, DuckDB catalog, Parquet storage/loader
- `crates/gb-engine` — event‑driven backtesting engine and market simulation
- `crates/gb-python` — PyO3 Python bindings
- `ui/` — Streamlit UI

## Development Standards

- **Keep changes scoped.** Prefer small, reviewable commits.
- **Document behavior.** If you change APIs or user behavior, update the relevant README or docs.
- **Respect invariants.** Financial data uses `Decimal` and nanosecond UTC timestamps.
- **Avoid breaking tests.** Run targeted tests for the crate you touch.

## Local Commands

```bash
# Whole workspace tests
cargo test --workspace

# Common targeted tests
cargo test -p gb-engine
cargo test -p gb-data
cargo test -p gb-types
cargo test -p gb-python
```

UI:
```bash
cd ui
python setup.py
# or
pip install -r requirements.txt
streamlit run app.py
```

## Key Architectural Notes

- **Event‑driven engine:** chronological event ordering across symbols.
- **Execution realism:** slippage, latency, and commission models are part of the core engine.
- **Storage:** Arrow/Parquet for columnar data; DuckDB for local metadata queries.
- **Python bindings:** PyO3 with async support.

## Testing & Quality

- Prefer adding or updating unit tests with feature changes.
- Maintain deterministic tests when possible.
- Keep formatting consistent with existing code.

## Contribution Workflow

1. Create a branch from the current working branch.
2. Make focused changes.
3. Run relevant tests.
4. Update docs if public behavior changes.
5. Open a PR with a clear summary and test notes.

## Avoid

- Massive “cleanup” or refactor PRs without a specific goal.
- Introducing new dependencies without a clear need.
- Changing public APIs without updating docs and examples.
