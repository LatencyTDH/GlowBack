# Optimization

> **Status:** the `gb-optimizer` Rust crate exists, but the HTTP
> `/optimizations` endpoint is intentionally **disabled** for now and returns
> `501 Not Implemented`.
>
> GlowBack used to fabricate optimization trial metrics with random numbers
> while accepting real-looking `base_backtest` and `ray_cluster` inputs. That
> behavior has been removed. The API now fails closed until it is wired to a
> real execution backend.

## Current API Behavior

| Method | Path                          | Current behavior |
| ------ | ----------------------------- | ---------------- |
| `POST` | `/optimizations`              | Returns `501 Not Implemented` until a real backend exists |
| `GET`  | `/optimizations`              | Lists any in-memory optimization records |
| `GET`  | `/optimizations/{id}`         | Returns status for an existing record |
| `GET`  | `/optimizations/{id}/results` | Returns results for an existing record |
| `POST` | `/optimizations/{id}/cancel`  | Cancels an existing record if it is still pending/running |

### Example: Current Failure Mode

```bash
curl -X POST http://localhost:8000/optimizations \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $API_KEY" \
  -d '{
    "name": "MA Crossover Sweep",
    "search_space": {
      "parameters": [
        {"name": "short_period", "kind": "int_range", "low": 5, "high": 20},
        {"name": "long_period", "kind": "int_range", "low": 20, "high": 60}
      ]
    },
    "strategy": "random",
    "max_trials": 50,
    "concurrency": 4,
    "objective_metric": "sharpe_ratio",
    "direction": "maximize",
    "base_backtest": {
      "symbols": ["AAPL"],
      "start_date": "2020-01-01T00:00:00Z",
      "end_date": "2024-01-01T00:00:00Z",
      "strategy": {"name": "ma_crossover"},
      "initial_capital": 100000
    },
    "ray_cluster": {
      "address": "ray://head-node:10001",
      "namespace": "glowback"
    }
  }'
```

Expected response:

```json
{
  "detail": "Optimization execution is not wired to a real backtest backend yet. GlowBack now fails closed instead of fabricating synthetic trial metrics. POST /optimizations is disabled until the API is connected to the engine and honors base_backtest/ray_cluster inputs."
}
```

## Planned Request Shape

Once the API is connected to a real execution backend, the intended request
shape still looks like this:

```json
{
  "name": "MA Crossover Sweep",
  "search_space": {
    "parameters": [
      {"name": "short_period", "kind": "int_range", "low": 5, "high": 20},
      {"name": "long_period", "kind": "int_range", "low": 20, "high": 60},
      {"name": "position_size", "kind": "float_range", "low": 0.5, "high": 1.0}
    ]
  },
  "strategy": "random",
  "max_trials": 50,
  "concurrency": 4,
  "objective_metric": "sharpe_ratio",
  "direction": "maximize",
  "base_backtest": {
    "symbols": ["AAPL"],
    "start_date": "2020-01-01T00:00:00Z",
    "end_date": "2024-01-01T00:00:00Z",
    "strategy": {"name": "ma_crossover"},
    "initial_capital": 100000
  }
}
```

## Search Strategies

These are the supported strategy concepts in the `gb-optimizer` crate and the
planned API integration.

### Grid Search

Evaluates every combination in a discrete grid. Best for small, discrete
parameter spaces.

```json
{
  "strategy": "grid",
  "search_space": {
    "parameters": [
      {"name": "short_period", "kind": "int_range", "low": 5, "high": 15},
      {"name": "long_period", "kind": "int_range", "low": 20, "high": 30}
    ]
  },
  "grid_steps": 5
}
```

### Random Search

Samples parameter combinations uniformly at random. More efficient than grid
search in high-dimensional spaces.

```json
{
  "strategy": "random",
  "max_trials": 200,
  "search_space": {
    "parameters": [
      {"name": "position_size", "kind": "float_range", "low": 0.5, "high": 1.0},
      {"name": "lookback", "kind": "int_range", "low": 5, "high": 60}
    ]
  }
}
```

### Bayesian Search

Uses a surrogate model to bias sampling toward promising regions. Balances
exploration and exploitation via the `exploration_weight` parameter (0 = pure
exploitation, 1 = pure exploration).

```json
{
  "strategy": "bayesian",
  "exploration_weight": 0.3,
  "max_trials": 100
}
```

## Parameter Types

| Kind          | Description                                | Fields        |
| ------------- | ------------------------------------------ | ------------- |
| `float_range` | Continuous uniform `[low, high]`           | `low`, `high` |
| `int_range`   | Integer range `[low, high]` inclusive      | `low`, `high` |
| `log_uniform` | Log-uniform sampling (e.g. learning rates) | `low`, `high` |
| `choice`      | Categorical set of values                  | `values`      |

## Distributed Execution with Ray

Ray execution is a **planned integration**, not an active API capability. When
GlowBack wires optimization to the engine, the intended request shape for
cluster execution is:

```json
{
  "ray_cluster": {
    "address": "ray://head-node:10001",
    "namespace": "glowback",
    "max_concurrent_tasks": 16,
    "num_cpus": 1.0,
    "num_gpus": 0.0,
    "pip_packages": ["glowback-sdk"]
  }
}
```

At that point, each trial should be dispatched only after the API can prove it
is running a real backtest for the provided `base_backtest` instead of a mock
or RNG placeholder.

## Rust Crate: `gb-optimizer`

The core optimization primitives already live in the `gb-optimizer` Rust crate:

- **`SearchSpace`** — builder for defining parameter dimensions
- **`GridSearch`** / **`RandomSearch`** / **`BayesianSearch`** — strategy impls
- **`Trial`** / **`TrialResult`** — individual trial tracking
- **`OptimizationConfig`** / **`OptimizationStatus`** — run management
- **`RayTaskDescriptor`** / **`WorkerAllocation`** — Ray integration types

```rust
use gb_optimizer::{RandomSearch, SearchSpace, SearchStrategy};

let space = SearchSpace::new()
    .add_int("short_period", 5, 20)
    .add_float("position_size", 0.5, 1.0);

let mut search = RandomSearch::new(space);
let suggestions = search.suggest(10);
```
