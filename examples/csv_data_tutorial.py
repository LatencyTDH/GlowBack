#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path

import glowback

DATA_DIR = Path(__file__).resolve().parent / "data"
START_DATE = "2025-01-02T00:00:00Z"
END_DATE = "2025-01-31T23:59:59Z"
EXPECTED_BAR_COUNT = 20


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


if __name__ == "__main__":
    manager = glowback.DataManager()
    manager.add_csv_provider(str(DATA_DIR))

    symbol = glowback.Symbol("AAPL", "NASDAQ", "equity")
    bars = manager.load_data(symbol, START_DATE, END_DATE, "day")

    require(len(bars) == EXPECTED_BAR_COUNT, f"expected {EXPECTED_BAR_COUNT} bars, found {len(bars)}")
    require(bars[0].timestamp.startswith("2025-01-02"), "unexpected first bar timestamp")
    require(round(bars[0].close, 2) == 243.26, "unexpected first bar close")
    require(bars[-1].timestamp.startswith("2025-01-31"), "unexpected last bar timestamp")
    require(round(bars[-1].close, 2) == 235.43, "unexpected last bar close")

    print(
        f"Loaded {len(bars)} AAPL bars from checked-in CSV fixture "
        f"({bars[0].timestamp} -> {bars[-1].timestamp})."
    )

    engine = glowback.BacktestEngine(
        symbols=["AAPL"],
        start_date=START_DATE,
        end_date=END_DATE,
        initial_capital=100000.0,
        name="CSV data tutorial",
        commission_bps=5,
        slippage_bps=2,
        latency_ms=0,
        data_source="csv",
        csv_data_path=str(DATA_DIR),
    )
    result = engine.run_buy_and_hold()

    metrics = result.metrics_summary
    manifest = result.manifest
    require(manifest is not None, "manifest should be present")
    require(metrics["initial_capital"] == 100000.0, "unexpected initial capital")
    require(metrics["final_value"] > 0.0, "final value should be positive")
    require(
        len(result.equity_curve) >= EXPECTED_BAR_COUNT,
        "equity curve should include at least the CSV bar window",
    )
    require(
        manifest["execution"]["data_settings"]["data_source"] == "csv",
        "manifest should record the CSV data source",
    )
    require(
        manifest["replay_request"]["symbols"] == ["AAPL"],
        "manifest should record the tutorial symbol",
    )

    print(
        f"CSV tutorial backtest: final_value=${metrics['final_value']:.2f} "
        f"trades={len(result.trades)} equity_points={len(result.equity_curve)}"
    )
    print("✅ CSV data tutorial completed successfully")
