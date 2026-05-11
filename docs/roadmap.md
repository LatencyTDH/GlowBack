# Roadmap

GlowBack is in an alpha stage: the core research loop is usable, but several surfaces are still being hardened before they should be treated as production trading infrastructure.

## Current focus

- **Engine-backed research loop:** keep the Rust engine, Python bindings, FastAPI gateway, and Streamlit UI aligned around the same executable paths.
- **Documentation and examples:** expand runnable examples, clarify alpha boundaries, and keep setup instructions validated by CI smoke tests.
- **Performance baselines:** continue collecting benchmark artifacts for maintained hot paths and add regression thresholds once enough history exists.
- **Data quality and reproducibility:** strengthen dataset validation, run manifests, replay helpers, and experiment-registry workflows.

## Near-term improvements

- Publish hosted Rust API docs once the crate-level public surface stabilizes.
- Add more end-to-end examples for built-in strategies, CSV datasets, and API replay.
- Extend optimization diagnostics beyond best-trial selection to better expose overfit risk.
- Tighten Docker/API packaging so deployment instructions are covered by the same level of smoke testing as the local quickstart.

## Longer-term themes

- More complete options backtesting on top of `gb-options`.
- Safer paper/live trading parity in `gb-live`, including broker adapters, risk limits, and audit trails.
- Expanded risk analytics and real-time monitoring.
- A richer hosted/dashboard experience after the local Streamlit workflow is mature.

See [Assumptions and Limitations](assumptions-and-limitations.md) for the current boundaries that matter before planning production use.
