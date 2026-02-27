# Optimization

GlowBack includes a parameter-search and distributed optimization framework for
systematically exploring strategy configurations.

## Search Strategies

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

| Kind           | Description                                  | Fields        |
| -------------- | -------------------------------------------- | ------------- |
| `float_range`  | Continuous uniform `[low, high]`             | `low`, `high` |
| `int_range`    | Integer range `[low, high]` inclusive        | `low`, `high` |
| `log_uniform`  | Log-uniform sampling (e.g. learning rates)   | `low`, `high` |
| `choice`       | Categorical set of values                    | `values`      |

## API Endpoints

| Method   | Path                                | Description                        |
| -------- | ----------------------------------- | ---------------------------------- |
| `POST`   | `/optimizations`                    | Create and start an optimization   |
| `GET`    | `/optimizations`                    | List optimization runs             |
| `GET`    | `/optimizations/{id}`               | Get optimization status            |
| `GET`    | `/optimizations/{id}/results`       | Get full results (when completed)  |
| `POST`   | `/optimizations/{id}/cancel`        | Cancel a running optimization      |

### Example: Create Optimization

```bash
curl -X POST http://localhost:8000/optimizations \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $API_KEY" \
  -d '{
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
  }'
```

## Distributed Execution with Ray

For large-scale parameter sweeps, GlowBack supports distributing trials across
a Ray cluster.  Pass a `ray_cluster` config in the optimization request:

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

Each trial is packaged as a `RayTaskDescriptor` and dispatched as a
`@ray.remote` function call.  The optimizer collects results as futures
resolve, feeds them back to adaptive strategies (Bayesian), and tracks the
best configuration.

## Rust Crate: `gb-optimizer`

The core optimization logic lives in the `gb-optimizer` Rust crate:

- **`SearchSpace`** — builder for defining parameter dimensions
- **`GridSearch`** / **`RandomSearch`** / **`BayesianSearch`** — strategy impls
- **`Trial`** / **`TrialResult`** — individual trial tracking
- **`OptimizationConfig`** / **`OptimizationStatus`** — run management
- **`RayTaskDescriptor`** / **`WorkerAllocation`** — Ray integration types

```rust
use gb_optimizer::{SearchSpace, RandomSearch, SearchStrategy};

let space = SearchSpace::new()
    .add_int("short_period", 5, 20)
    .add_float("position_size", 0.5, 1.0);

let mut search = RandomSearch::new(space);
let suggestions = search.suggest(10);
```
