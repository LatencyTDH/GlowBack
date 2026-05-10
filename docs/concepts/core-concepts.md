# Core Concepts

GlowBack is an event‑driven backtesting system. Understanding a few core concepts helps you reason about results and performance.

## Key Ideas

- **Events**: Market data is processed in chronological order across symbols.
- **Strategies**: Logic that emits orders in response to events.
- **Execution**: Orders fill with realistic slippage, latency, and commission.
- **Portfolio**: Tracks positions, cash, and P&L over time.
- **Results**: Metrics and artifacts (equity curve, trades) for evaluation.

## Supported account modes

| Mode | Current behavior | Constraints / notes |
| --- | --- | --- |
| Backtest portfolio (`gb-types::Portfolio`) | Supports long and short positions, multi-symbol books, fractional quantities, commissions, realized P&L, unrealized P&L, and marked-to-market equity snapshots. | Equity is computed from cash plus signed position market value. Short exposure is modeled as a liability. Margin interest and broker-specific borrowing rules are not modeled yet. |
| Sandbox paper broker (`gb-live::PaperBroker`) | Cash account for live-like dry runs with fills, positions, and account balance snapshots. | Rejects buys that exceed available cash and rejects sell orders that exceed held inventory. No naked shorts or margin borrowing. |

The regression suite now treats these accounting rules as explicit invariants so trade-to-trade portfolio snapshots stay auditable instead of being inferred from aggregate returns alone.
