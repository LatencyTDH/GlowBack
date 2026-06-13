#!/usr/bin/env python3
from __future__ import annotations

import glowback

HELPER_SYMBOLS = ["AAPL"]
ENGINE_SYMBOLS = ["AAPL", "MSFT"]
START_DATE = "2024-01-01T00:00:00Z"
END_DATE = "2024-06-30T23:59:59Z"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def inspect_result(label: str, result, *, expected_strategy: str, expected_symbols: list[str]) -> None:
    metrics = result.metrics_summary
    equity_curve = result.equity_curve
    trades = result.trades
    manifest = result.manifest

    require(manifest is not None, f"{label}: manifest should be present")
    require(metrics["initial_capital"] == 100000.0, f"{label}: unexpected initial capital")
    require(metrics["final_value"] > 0.0, f"{label}: final value should be positive")
    require(len(equity_curve) > 0, f"{label}: equity curve should not be empty")
    require(manifest["manifest_version"] == "1.0", f"{label}: manifest_version drifted")
    require(
        manifest["replay_request"]["strategy_name"] == expected_strategy,
        f"{label}: wrong manifest strategy",
    )
    require(
        manifest["replay_request"]["symbols"] == expected_symbols,
        f"{label}: wrong manifest symbols",
    )
    require(
        manifest["execution"]["data_settings"]["data_source"] == "sample",
        f"{label}: expected sample data source",
    )
    require(isinstance(result.logs, list), f"{label}: logs should be a list")
    require(isinstance(result.final_positions, dict), f"{label}: final_positions should be a dict")

    print(
        f"{label}: final_value=${metrics['final_value']:.2f} "
        f"equity_points={len(equity_curve)} trades={len(trades)}"
    )


if __name__ == "__main__":
    print("GlowBack exports:", glowback.__all__)
    print("Built-in strategies:", glowback.BUILTIN_STRATEGIES)

    require("BacktestEngine" in glowback.__all__, "BacktestEngine missing from __all__")
    require("BacktestResult" in glowback.__all__, "BacktestResult missing from __all__")
    require(
        list(glowback.BUILTIN_STRATEGIES) == [
            "buy_and_hold",
            "ma_crossover",
            "momentum",
            "mean_reversion",
            "rsi",
            "covered_call",
        ],
        "BUILTIN_STRATEGIES drifted",
    )

    helper_result = glowback.run_builtin_strategy(
        symbols=HELPER_SYMBOLS,
        start_date=START_DATE,
        end_date=END_DATE,
        strategy_name="ma_crossover",
        strategy_params={"short_period": 5, "long_period": 20},
        data_source="sample",
        commission_bps=5,
        slippage_bps=2,
        latency_ms=0,
        name="Python SDK helper quickstart",
    )
    inspect_result(
        "helper",
        helper_result,
        expected_strategy="ma_crossover",
        expected_symbols=HELPER_SYMBOLS,
    )

    engine = glowback.BacktestEngine(
        symbols=ENGINE_SYMBOLS,
        start_date=START_DATE,
        end_date=END_DATE,
        initial_capital=100000.0,
        data_source="sample",
        commission_bps=5,
        slippage_bps=2,
        latency_ms=0,
        name="Python SDK engine quickstart",
    )
    engine_result = engine.run_buy_and_hold()
    inspect_result(
        "engine",
        engine_result,
        expected_strategy="buy_and_hold",
        expected_symbols=ENGINE_SYMBOLS,
    )

    print("✅ Python SDK quickstart completed successfully")
