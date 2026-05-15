#!/usr/bin/env python3
from __future__ import annotations

import json
from pathlib import Path
from tempfile import TemporaryDirectory

from glowback_runtime import compare_manifest_metrics, replay_manifest, run_backtest, validate_run_manifest

SYMBOLS = ["AAPL", "MSFT"]
START_DATE = "2024-01-01T00:00:00Z"
END_DATE = "2024-06-30T23:59:59Z"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


if __name__ == "__main__":
    original = run_backtest(
        symbols=SYMBOLS,
        start_date=START_DATE,
        end_date=END_DATE,
        strategy_name="ma_crossover",
        strategy_params={"short_period": 5, "long_period": 20},
        initial_capital=100000.0,
        data_source="sample",
        commission_bps=5,
        slippage_bps=2,
        latency_ms=0,
        run_name="Replay Manifest Tutorial",
    )

    manifest = validate_run_manifest(original["manifest"])
    require(manifest["replay_request"]["strategy_name"] == "ma_crossover", "manifest strategy drifted")
    require(manifest["replay_request"]["symbols"] == SYMBOLS, "manifest symbols drifted")

    with TemporaryDirectory() as temp_dir:
        result_path = Path(temp_dir) / "run-result.json"
        result_path.write_text(
            json.dumps(
                {
                    "manifest": manifest,
                    "metrics_summary": original["metrics_summary"],
                },
                indent=2,
                sort_keys=True,
            )
        )
        loaded = json.loads(result_path.read_text())

    replay_payload = replay_manifest(loaded["manifest"])
    comparison = compare_manifest_metrics(loaded["manifest"], replay_payload, tolerance=1e-6)

    require(replay_payload["manifest"] is not None, "replay run should emit a manifest")
    require(comparison["within_tolerance"], f"replay drifted: {comparison['deltas']}")
    require(replay_payload["final_value"] > 0.0, "replay final value should be positive")

    print(
        "Original vs replay final value:",
        f"${original['final_value']:.2f} -> ${replay_payload['final_value']:.2f}",
    )
    print("Metric deltas:", comparison["deltas"])
    print("✅ Replay manifest tutorial completed successfully")
