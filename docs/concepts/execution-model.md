# Execution Model

The engine simulates realistic execution:

- **Latency**: configurable delay between order submission and fill
- **Slippage**: basis‑point or custom models
- **Commission**: per‑share or percentage models
- **Order types**: market, limit, stop, stop‑limit

The simulator processes events in time order across symbols to avoid look‑ahead bias.

## Fee Models

GlowBack supports two fee models via `FeeModel`:

| Model | Use Case | Fields |
|-------|----------|--------|
| **PerShare** | Equities, commodities | `per_share`, `percentage`, `minimum` |
| **MakerTaker** | Crypto, FX | `maker_fee_pct`, `taker_fee_pct` |

Use `ExecutionConfig::for_asset_class()` for sensible defaults per asset class.

## Market Hours

Market hours are asset-class-aware via `MarketHours::for_asset_class()`:

- **Crypto**: 24/7 — no market close, weekends active
- **Forex**: 24 hours on weekdays (Sunday evening – Friday evening)
- **Equity**: US market hours (14:00–21:00 UTC, weekdays only)
