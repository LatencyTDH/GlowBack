# Performance

## Goals

- Backtest 10 years of daily data for ~500 equities in < 60 seconds on an 8-core dev machine.
- Keep the engine hot paths measurable so performance work is repeatable instead of anecdotal.

## Current maintained benchmark slice

The first maintained benchmark suite focuses on the `gb-engine` strategy-context update hot path. It compares the legacy full-context rebuild approach with the current incremental buffer update path across two representative workloads.

| Scenario | Legacy full rebuild | Incremental buffers | Speedup | Throughput gain |
| --- | ---: | ---: | ---: | ---: |
| 10 symbols × 126 days | ≈7.8 ms | ≈287 µs | ≈29x | ≈29x |
| 50 symbols × 252 days | ≈235 ms | ≈4.9 ms | ≈48x | ≈48x |

These numbers come from `cargo bench -p gb-engine --bench strategy_context -- --noplot` on the current codebase. Treat them as a baseline snapshot, not a universal hardware promise. The scheduled CI benchmark artifact is the source of truth for run-to-run comparisons.

## What gets reported

`./scripts/run-engine-benchmarks.sh` now produces a benchmark artifact bundle with:

- `summary.md` — human-readable table for CI job summaries and quick review
- `summary.json` — machine-readable benchmark metadata for later automation
- `raw/criterion/` — copied Criterion output for deeper inspection

The summary includes mean runtime, 95% confidence intervals, throughput, and the incremental-vs-legacy speedup for each scenario.

## How to run locally

```bash
./scripts/run-engine-benchmarks.sh artifacts/benchmarks/local
```

That command will:

1. run `cargo bench -p gb-engine --bench strategy_context -- --noplot`
2. collect the Criterion output under `target/criterion/strategy_context_scaling`
3. generate `summary.md` and `summary.json` in the requested artifact directory

If you only want the raw Criterion output, you can still run the benchmark directly:

```bash
cargo bench -p gb-engine --bench strategy_context -- --noplot
```

## CI reporting

Benchmark reporting is intentionally non-blocking for normal PRs.

- `.github/workflows/benchmarks.yml` runs on a weekly schedule and via manual dispatch.
- The workflow uploads the benchmark artifact bundle so baselines stay visible without making ordinary CI noisy.
- Once enough history exists, we can add regression thresholds on top of the generated `summary.json` instead of guessing.
