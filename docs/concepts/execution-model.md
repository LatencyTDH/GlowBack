# Execution Model

The engine simulates realistic execution:

- **Latency**: configurable delay between order submission and fill
- **Slippage**: basis‑point or custom models
- **Commission**: per‑share or percentage models
- **Order types**: market, limit, stop, stop‑limit

The simulator processes events in time order across symbols to avoid look‑ahead bias.
