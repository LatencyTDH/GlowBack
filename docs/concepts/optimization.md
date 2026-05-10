# Optimization

> **Status:** `/optimizations` is now wired to real backtest execution through
> the `gb-python` bindings for GlowBack's built-in strategies. The first
> shipping version supports grid, random, and Bayesian search, plus holdout or
> walk-forward validation.

## Current API Behavior

| Method | Path                          | Current behavior |
| ------ | ----------------------------- | ---------------- |
| `POST` | `/optimizations`              | Creates an optimization run and executes it in the background |
| `GET`  | `/optimizations`              | Lists in-memory optimization runs |
| `GET`  | `/optimizations/{id}`         | Returns run status and best-trial summary |
| `GET`  | `/optimizations/{id}/results` | Returns ranked trials and a replayable best-trial backtest payload |
| `POST` | `/optimizations/{id}/cancel`  | Cancels a pending/running optimization |

## Example Request

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
    "strategy": "grid",
    "max_trials": 16,
    "concurrency": 1,
    "objective_metric": "sharpe_ratio",
    "direction": "maximize",
    "validation_mode": "walk_forward",
    "validation_fraction": 0.25,
    "walk_forward_windows": 3,
    "base_backtest": {
      "symbols": ["AAPL"],
      "start_date": "2020-01-01T00:00:00Z",
      "end_date": "2024-01-01T00:00:00Z",
      "resolution": "day",
      "initial_capital": 100000,
      "data_source": "sample",
      "strategy": {
        "name": "ma_crossover",
        "params": {}
      },
      "execution": {
        "commission_bps": 5,
        "slippage_bps": 5
      }
    }
  }'
```

Example create response:

```json
{
  "optimization_id": "8b89f6d0-2ff0-4d58-8f74-bf6eaa4fd316",
  "name": "MA Crossover Sweep",
  "state": "pending",
  "strategy": "grid",
  "objective_metric": "sharpe_ratio",
  "direction": "maximize",
  "max_trials": 16,
  "trials_completed": 0,
  "trials_failed": 0,
  "trials_running": 0,
  "best_trial": null,
  "created_at": "2026-04-11T06:30:00Z",
  "started_at": null,
  "finished_at": null,
  "error": null
}
```

## Result Shape

`GET /optimizations/{id}/results` returns:

- `best_trial` — best completed trial by the requested objective/direction
- `all_trials` — every completed/failed trial with metrics and sampled params
- `replay_backtest` — the best-trial backtest payload, ready to reuse as a
  normal backtest request/config
- `validation_mode` — `holdout` or `walk_forward`

This makes the best run replayable instead of trapping the winning parameters
inside the optimizer.

## Validation Modes

### Holdout

Splits the requested date range into train + validation segments and ranks each
trial by the validation metric.

```json
{
  "validation_mode": "holdout",
  "validation_fraction": 0.25
}
```

### Walk-forward

Uses the trailing validation slice as multiple sequential windows and scores
trials by the mean validation metric across windows.

```json
{
  "validation_mode": "walk_forward",
  "validation_fraction": 0.30,
  "walk_forward_windows": 4
}
```

Returned trial metrics include validation-specific keys such as:

- `validation_<objective_metric>`
- `train_<objective_metric>`
- `full_<objective_metric>`
- `validation_windows`

## Search Strategies

### Grid Search

Evaluates every combination in a discrete grid, capped by `max_trials`.

```json
{
  "strategy": "grid",
  "grid_steps": 5
}
```

### Random Search

Samples independent points from the search space with deterministic seeding.

```json
{
  "strategy": "random",
  "max_trials": 100,
  "random_seed": 42
}
```

### Bayesian Search

Starts with exploratory random samples, then biases future suggestions toward
regions near the best completed trials while still respecting
`exploration_weight`.

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
| `log_uniform` | Log-uniform sampling (e.g. thresholds)     | `low`, `high` |
| `choice`      | Categorical set of values                  | `values`      |

## Built-in Strategy Support

The first shipping backend supports the built-in strategies exposed through
`gb-python`:

- `buy_and_hold`
- `ma_crossover`
- `momentum`
- `mean_reversion`
- `rsi`

The `base_backtest.strategy.params` map is merged with sampled trial
parameters before each real engine run.

## Distributed Execution with Ray

`ray_cluster` remains present in the API model for future distributed
execution, but this first shipping path runs trials locally through the Python
bindings. Once Ray orchestration is live, the API should preserve the same
request/response contract while changing only the execution substrate.

## Rust Crate: `gb-optimizer`

GlowBack's Rust optimizer crate still provides the long-term search primitives
and orchestration concepts:

- **`SearchSpace`** — parameter-dimension builder
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
