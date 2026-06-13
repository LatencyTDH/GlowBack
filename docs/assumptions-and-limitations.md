# Assumptions and Limitations

GlowBack already covers a meaningful research workflow, but it is still an alpha system. This page makes the current boundaries explicit so the quickstart and tutorials stay honest.

## Execution and fills

- Backtests use deterministic bar-based execution with configurable commissions, slippage, latency, and participation caps.
- The engine models realistic order lifecycle events, but it is not a tick-by-tick exchange simulator.
- Paper/live parity work is still in progress; do not treat paper execution as broker-grade behavior yet.

## Data quality and sourcing

- Sample/demo data is available for smoke tests and tutorials, but production research still depends on the quality of the input data you load.
- CSV and provider ingestion validate structure, yet corporate actions, survivorship-bias controls, and deeper dataset provenance are still evolving.
- Alpha Vantage examples are intentionally conservative because free-tier rate limits can dominate the experience.

## Portfolio accounting

- Core long/short/fractional accounting invariants are covered by tests, including signed market value for short liabilities.
- More advanced cash management edge cases should still be validated with your own scenarios before relying on them for trading decisions.
- Multi-asset support is strongest for equities and spot crypto; other asset classes remain narrower.

## Optimization workflow

- GlowBack optimization results now surface validation gaps, stability summaries, and an optimization manifest with seed/trial lineage, but resume semantics and true scale-out orchestration remain active roadmap work.
- Treat optimization results as research aids, not proof of live robustness.

## Live and paper trading

- The `gb-live` crate exists, but real broker adapters are intentionally not positioned as production-ready.
- `PaperBroker` now records an append-only audit trail for broker events and inventory/risk rejections, and CI includes a backtest-order-stream replay check against the paper broker on sample data.
- Broader safety controls, parity checks, and auditability still remain prerequisites before any real-money workflow should be trusted.

## Options workflow

- The `gb-options` crate now has one documented end-to-end engine path: an experimental covered-call workflow that buys 100 shares, prices a short call with Black-Scholes greeks, and records option lifecycle events in Rust/Python/API result payloads.
- Broader multi-leg options accounting, mark-to-market liability treatment, and richer exercise/assignment flows are still incomplete.
- Use options support as an experimental surface until the documented engine/accounting path is broader than the current covered-call slice.

## How to use this page

- Start with the [Getting Started](getting-started.md) quickstart to verify your checkout.
- Use the [Examples](examples/index.md) and tutorials as the source of truth for currently exercised workflows.
- Check the [Roadmap](roadmap.md) before planning around a feature that sits near one of the boundaries above.
