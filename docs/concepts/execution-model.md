# Execution Model

The engine simulates realistic execution:

- **Latency**: configurable delay between order submission and fill
- **Slippage**: basis‑point or custom models
- **Commission**: per‑share or percentage models
- **Order types**: market, limit, stop, stop‑limit
- **Time in force**: GTC, day, IOC, and FOK handling in the backtest loop
- **Liquidity participation**: fills are capped by `execution_settings.max_volume_participation`, so oversized orders can partially fill instead of teleporting through the bar
- **Lifecycle events**: backtest results now retain submitted / filled / canceled / rejected / expired order events for auditability

The simulator processes events in time order across symbols to avoid look‑ahead bias.

## Order lifecycle slice

GlowBack now applies a first execution-realism slice in the engine itself:

- orders are marked submitted when strategies place them
- GTC orders can remain open after a partial fill
- IOC orders cancel any remainder immediately after a partial fill
- FOK orders cancel when the full requested size cannot be filled inside the configured participation limit
- day orders expire when their conditions are not met on the execution bar or when only a partial slice is available

This keeps the current engine deterministic while making order outcomes visible to Python and API consumers.

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
