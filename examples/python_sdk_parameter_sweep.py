#!/usr/bin/env python3
from __future__ import annotations

from itertools import product

import glowback

SYMBOLS = ["AAPL"]
START_DATE = "2024-01-01T00:00:00Z"
END_DATE = "2024-06-30T23:59:59Z"
INITIAL_CAPITAL = 100000.0
DATA_SOURCE = "sample"
SWEEP = list(product([8, 12, 16], [0.03, 0.05, 0.08]))


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def run_trial(lookback_period: int, momentum_threshold: float) -> dict[str, object]:
    strategy_params = {
        "lookback_period": lookback_period,
        "momentum_threshold": momentum_threshold,
    }
    result = glowback.run_builtin_strategy(
        symbols=SYMBOLS,
        start_date=START_DATE,
        end_date=END_DATE,
        strategy_name="momentum",
        strategy_params=strategy_params,
        resolution="day",
        initial_capital=INITIAL_CAPITAL,
        name=f"Momentum sweep lookback={lookback_period} threshold={momentum_threshold}",
        data_source=DATA_SOURCE,
        data_quality_mode="warn",
        commission_bps=5,
        slippage_bps=2,
        latency_ms=0,
    )

    manifest = result.manifest
    require(manifest is not None, "manifest should be present")
    require(
        manifest["replay_request"]["strategy_name"] == "momentum",
        "strategy name drifted",
    )
    replay_params = manifest["replay_request"]["strategy_params"]
    require(
        replay_params["lookback_period"] == lookback_period,
        "lookback_period drifted",
    )
    require(
        abs(float(replay_params["momentum_threshold"]) - momentum_threshold) < 1e-12,
        "momentum_threshold drifted",
    )
    require(
        manifest["execution"]["data_settings"]["data_source"] == DATA_SOURCE,
        "data source drifted",
    )

    metrics = result.metrics_summary
    return {
        "score": metrics["sharpe_ratio"],
        "final_value": metrics["final_value"],
        "lookback_period": lookback_period,
        "momentum_threshold": momentum_threshold,
        "result": result,
    }


if __name__ == "__main__":
    ranked_trials: list[dict[str, object]] = []

    for lookback_period, momentum_threshold in SWEEP:
        trial = run_trial(lookback_period, momentum_threshold)
        ranked_trials.append(trial)
        print(
            f"trial lookback={lookback_period} threshold={momentum_threshold:.2f} "
            f"sharpe={trial['score']:.4f} final_value=${trial['final_value']:.2f}"
        )

    ranked_trials.sort(
        key=lambda trial: (
            float(trial["score"]),
            float(trial["final_value"]),
        ),
        reverse=True,
    )
    best = ranked_trials[0]

    print(
        "Best momentum sweep:",
        f"lookback={best['lookback_period']}",
        f"threshold={best['momentum_threshold']:.2f}",
        f"sharpe={best['score']:.4f}",
        f"final_value=${best['final_value']:.2f}",
    )
    print("Top 3 sweep results:")
    for trial in ranked_trials[:3]:
        print(
            f"- lookback={trial['lookback_period']} "
            f"threshold={trial['momentum_threshold']:.2f} "
            f"sharpe={trial['score']:.4f} "
            f"final_value=${trial['final_value']:.2f}"
        )

    print("✅ Python SDK parameter sweep completed successfully")
