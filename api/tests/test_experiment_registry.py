from __future__ import annotations

import tempfile
import unittest
from datetime import datetime, timezone
from pathlib import Path

from api.app.experiment_registry import ExperimentRegistry, sha256_text
from api.app.models import BacktestRequest, BacktestResult, RunState, StrategyConfig
from api.app.store import RunStore


class ExperimentRegistryTests(unittest.IsolatedAsyncioTestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.registry = ExperimentRegistry(Path(self.tempdir.name) / "registry.sqlite3")

    def tearDown(self) -> None:
        self.registry.close()
        self.tempdir.cleanup()

    async def test_run_store_rehydrates_persisted_api_runs(self) -> None:
        store = RunStore(registry=self.registry)
        request = BacktestRequest(
            symbols=["AAPL"],
            start_date=datetime(2026, 1, 1, tzinfo=timezone.utc),
            end_date=datetime(2026, 1, 5, tzinfo=timezone.utc),
            strategy=StrategyConfig(name="buy_and_hold"),
            initial_capital=100000,
        )

        status_obj = await store.create_run(request)
        await store.update_state(status_obj.run_id, RunState.running)
        await store.set_result(
            status_obj.run_id,
            BacktestResult(
                run_id=status_obj.run_id,
                metrics_summary={"final_value": 101234.5},
                equity_curve=[{"timestamp": "2026-01-01T00:00:00Z", "value": 100000.0}],
                trades=[],
                exposures=[],
                logs=["completed"],
                manifest={
                    "manifest_version": "1.0",
                    "generated_at": "2026-01-05T00:00:00Z",
                    "engine": {"crate_name": "gb-engine", "version": "0.1.0"},
                    "strategy": {
                        "strategy_id": "buy_and_hold",
                        "name": "buy_and_hold",
                        "parameters": {},
                        "code_hash": None,
                    },
                    "dataset": {
                        "data_source": "sample",
                        "resolution": "day",
                        "start_date": "2026-01-01T00:00:00Z",
                        "end_date": "2026-01-05T00:00:00Z",
                        "symbols": ["AAPL"],
                        "bar_counts": {"AAPL": 5},
                        "total_bars": 5,
                    },
                    "execution": {
                        "initial_capital": 100000.0,
                        "commission_bps": 0.0,
                        "slippage_bps": 5.0,
                        "latency_ms": 100,
                        "commission_percentage": 0.0,
                        "minimum_commission": 1.0,
                        "slippage_model": {"Linear": {"basis_points": 5}},
                        "latency_model": {"Fixed": {"milliseconds": 100}},
                        "market_impact_model": {"SquareRoot": {"factor": "0.0001"}},
                        "data_settings": {
                            "data_source": "sample",
                            "adjust_for_splits": True,
                            "adjust_for_dividends": True,
                            "fill_gaps": False,
                            "survivor_bias_free": True,
                            "max_bars_in_memory": 10000,
                        },
                    },
                    "replay_request": {
                        "symbols": ["AAPL"],
                        "start_date": "2026-01-01T00:00:00Z",
                        "end_date": "2026-01-05T00:00:00Z",
                        "resolution": "day",
                        "strategy_name": "buy_and_hold",
                        "strategy_params": {},
                        "initial_capital": 100000.0,
                        "data_source": "sample",
                        "commission_bps": 0.0,
                        "slippage_bps": 5.0,
                        "latency_ms": 100,
                        "run_name": "Registry Replay",
                    },
                    "metric_snapshot": {
                        "final_value": 101234.5,
                        "total_return": 1.2345,
                        "max_drawdown": 0.0,
                        "sharpe_ratio": 0.0,
                        "total_trades": 0,
                    },
                },
            ),
        )

        reloaded = RunStore(registry=self.registry)
        persisted_status = await reloaded.get_status(status_obj.run_id)
        persisted_result = await reloaded.get_result(status_obj.run_id)
        events = await reloaded.get_events_after(status_obj.run_id, None)

        self.assertIsNotNone(persisted_status)
        self.assertIsNotNone(persisted_result)
        self.assertEqual(persisted_status.state, RunState.completed)
        self.assertEqual(persisted_result.metrics_summary["final_value"], 101234.5)
        self.assertEqual(persisted_result.manifest["dataset"]["total_bars"], 5)
        self.assertGreaterEqual(len(events), 3)

    async def test_registry_persists_strategy_snapshots_with_hashes(self) -> None:
        strategy = self.registry.upsert_strategy(
            name="Momentum Lab",
            code="class MomentumLab:\n    pass\n",
            config={"initial_capital": 250000, "commission": 0.001},
            metadata={"saved_from": "test"},
        )

        loaded = self.registry.get_strategy("Momentum Lab")
        listed = self.registry.list_strategies()

        self.assertIsNotNone(loaded)
        self.assertEqual(strategy["code_hash"], sha256_text("class MomentumLab:\n    pass\n"))
        self.assertEqual(loaded["config"]["initial_capital"], 250000)
        self.assertEqual(listed[0]["name"], "Momentum Lab")


if __name__ == "__main__":
    unittest.main()
