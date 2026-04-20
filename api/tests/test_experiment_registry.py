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
