from __future__ import annotations

import asyncio
import time
import unittest

from fastapi.testclient import TestClient

import api.app.main as main_module
from api.app.optimization_models import (
    ObjectiveDirection,
    OptimizationRequest,
    OptimizationState,
    ParameterDef,
    ParameterKind,
    SearchSpaceConfig,
    SearchStrategyName,
    TrialStatus,
    TrialSummary,
    ValidationMode,
)
from api.app.optimization_runtime import OptimizationExecution
from api.app.optimization_store import OptimizationStore


class FakeOptimizationExecutor:
    async def execute(self, request, is_cancelled=None) -> OptimizationExecution:
        if is_cancelled is not None:
            cancelled = is_cancelled()
            if hasattr(cancelled, "__await__"):
                cancelled = await cancelled
            if cancelled:
                return OptimizationExecution(state=OptimizationState.cancelled, trials=[])

        trials = [
            TrialSummary(
                trial_id="trial-1",
                trial_number=1,
                status=TrialStatus.completed,
                parameters={"short_period": 5, "long_period": 20},
                objective=1.25,
                metrics={
                    "sharpe_ratio": 1.1,
                    "validation_sharpe_ratio": 1.25,
                    "full_sharpe_ratio": 1.1,
                },
                duration_seconds=0,
            ),
            TrialSummary(
                trial_id="trial-2",
                trial_number=2,
                status=TrialStatus.completed,
                parameters={"short_period": 8, "long_period": 24},
                objective=1.55,
                metrics={
                    "sharpe_ratio": 1.4,
                    "validation_sharpe_ratio": 1.55,
                    "full_sharpe_ratio": 1.4,
                },
                duration_seconds=0,
            ),
        ]
        return OptimizationExecution(
            state=OptimizationState.completed,
            trials=trials,
            best_trial=trials[1],
            replay_backtest={
                **request.base_backtest,
                "strategy": {
                    "name": request.base_backtest["strategy"]["name"],
                    "params": {"short_period": 8, "long_period": 24},
                },
            },
        )


class SlowCancellationExecutor:
    async def execute(self, request, is_cancelled=None) -> OptimizationExecution:
        for _ in range(10):
            if is_cancelled is not None:
                cancelled = is_cancelled()
                if hasattr(cancelled, "__await__"):
                    cancelled = await cancelled
                if cancelled:
                    return OptimizationExecution(state=OptimizationState.cancelled, trials=[])
            await asyncio.sleep(0.01)
        return OptimizationExecution(state=OptimizationState.completed, trials=[])


def _sample_request() -> OptimizationRequest:
    return OptimizationRequest(
        name="MA Crossover Sweep",
        description="Regression fixture",
        search_space=SearchSpaceConfig(
            parameters=[
                ParameterDef(
                    name="short_period",
                    kind=ParameterKind.int_range,
                    low=5,
                    high=20,
                ),
                ParameterDef(
                    name="long_period",
                    kind=ParameterKind.int_range,
                    low=20,
                    high=60,
                ),
            ]
        ),
        strategy=SearchStrategyName.random,
        max_trials=2,
        concurrency=1,
        objective_metric="sharpe_ratio",
        direction=ObjectiveDirection.maximize,
        validation_mode=ValidationMode.walk_forward,
        validation_fraction=0.25,
        walk_forward_windows=2,
        base_backtest={
            "symbols": ["AAPL"],
            "start_date": "2020-01-01T00:00:00Z",
            "end_date": "2024-01-01T00:00:00Z",
            "resolution": "day",
            "strategy": {"name": "ma_crossover", "params": {}},
            "initial_capital": 100000,
            "data_source": "sample",
        },
    )


class OptimizationApiTests(unittest.TestCase):
    def setUp(self) -> None:
        self.original_store = main_module.opt_store
        main_module.opt_store = OptimizationStore(executor=FakeOptimizationExecutor())
        self.client = TestClient(main_module.app)

    def tearDown(self) -> None:
        main_module.opt_store = self.original_store

    def test_create_optimization_runs_in_background_and_returns_result(self) -> None:
        response = self.client.post("/optimizations", json=_sample_request().model_dump(mode="json"))

        self.assertEqual(response.status_code, 201)
        created = response.json()
        self.assertIn(created["state"], {"pending", "running", "completed"})
        optimization_id = created["optimization_id"]

        result = None
        for _ in range(50):
            status_response = self.client.get(f"/optimizations/{optimization_id}")
            self.assertEqual(status_response.status_code, 200)
            if status_response.json()["state"] == "completed":
                result_response = self.client.get(f"/optimizations/{optimization_id}/results")
                self.assertEqual(result_response.status_code, 200)
                result = result_response.json()
                break
            time.sleep(0.02)

        self.assertIsNotNone(result)
        assert result is not None
        self.assertEqual(result["state"], "completed")
        self.assertEqual(result["best_trial"]["parameters"], {"short_period": 8, "long_period": 24})
        self.assertEqual(result["validation_mode"], "walk_forward")
        self.assertEqual(result["replay_backtest"]["strategy"]["params"], {"short_period": 8, "long_period": 24})
        self.assertEqual(len(result["all_trials"]), 2)


class OptimizationStoreTests(unittest.IsolatedAsyncioTestCase):
    async def test_cancel_marks_run_cancelled(self) -> None:
        store = OptimizationStore(executor=SlowCancellationExecutor())
        created = await store.create(_sample_request())

        task = asyncio.create_task(store.run_optimization(created.optimization_id))
        await asyncio.sleep(0.02)
        cancelled = await store.cancel(created.optimization_id)
        await task

        self.assertTrue(cancelled)
        status_obj = await store.get_status(created.optimization_id)
        self.assertIsNotNone(status_obj)
        assert status_obj is not None
        self.assertEqual(status_obj.state, OptimizationState.cancelled)


if __name__ == "__main__":
    unittest.main()
