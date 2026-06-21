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
    async def execute(self, request, is_cancelled=None, prior_trials=None) -> OptimizationExecution:
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
            diagnostics={
                "objective_metric": "sharpe_ratio",
                "best_trial_generalization_gap": 0.15,
            },
            manifest={
                "manifest_version": "1.0",
                "kind": "optimization_run",
                "diagnostics": {"best_trial_generalization_gap": 0.15},
            },
        )


class SlowCancellationExecutor:
    async def execute(self, request, is_cancelled=None, prior_trials=None) -> OptimizationExecution:
        for _ in range(10):
            if is_cancelled is not None:
                cancelled = is_cancelled()
                if hasattr(cancelled, "__await__"):
                    cancelled = await cancelled
                if cancelled:
                    return OptimizationExecution(state=OptimizationState.cancelled, trials=[])
            await asyncio.sleep(0.01)
        return OptimizationExecution(state=OptimizationState.completed, trials=[])


class ResumeAwareExecutor:
    async def execute(self, request, is_cancelled=None, prior_trials=None) -> OptimizationExecution:
        prior_trials = list(prior_trials or [])
        initial_trial = TrialSummary(
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
        )
        resumed_trial = TrialSummary(
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
        )

        if not prior_trials:
            return OptimizationExecution(
                state=OptimizationState.cancelled,
                trials=[initial_trial],
                best_trial=initial_trial,
                replay_backtest={
                    **request.base_backtest,
                    "strategy": {
                        "name": request.base_backtest["strategy"]["name"],
                        "params": {"short_period": 5, "long_period": 20},
                    },
                },
                diagnostics={
                    "objective_metric": "sharpe_ratio",
                    "resume_supported": True,
                },
                manifest={
                    "manifest_version": "1.0",
                    "kind": "optimization_run",
                    "diagnostics": {"resume_supported": True},
                },
            )

        trials = prior_trials + [resumed_trial]
        return OptimizationExecution(
            state=OptimizationState.completed,
            trials=trials,
            best_trial=resumed_trial,
            replay_backtest={
                **request.base_backtest,
                "strategy": {
                    "name": request.base_backtest["strategy"]["name"],
                    "params": {"short_period": 8, "long_period": 24},
                },
            },
            diagnostics={
                "objective_metric": "sharpe_ratio",
                "resume_supported": True,
            },
            manifest={
                "manifest_version": "1.0",
                "kind": "optimization_run",
                "diagnostics": {"resume_supported": True},
                "execution_plan": {"resume_supported": True, "cancellation_supported": True},
                "trial_lineage": [trial.model_dump(mode="json") for trial in trials],
            },
        )


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
        self.assertEqual(result["diagnostics"]["best_trial_generalization_gap"], 0.15)
        self.assertEqual(result["manifest"]["manifest_version"], "1.0")
        self.assertEqual(len(result["all_trials"]), 2)

    def test_resume_optimization_continues_from_saved_trials(self) -> None:
        original_store = main_module.opt_store
        main_module.opt_store = OptimizationStore(executor=ResumeAwareExecutor())
        client = TestClient(main_module.app)
        try:
            created_response = client.post(
                "/optimizations",
                json=_sample_request().model_dump(mode="json"),
            )
            self.assertEqual(created_response.status_code, 201)
            optimization_id = created_response.json()["optimization_id"]

            for _ in range(50):
                status_response = client.get(f"/optimizations/{optimization_id}")
                self.assertEqual(status_response.status_code, 200)
                if status_response.json()["state"] == "cancelled":
                    break
                time.sleep(0.02)

            resume_response = client.post(f"/optimizations/{optimization_id}/resume")
            self.assertEqual(resume_response.status_code, 200)
            self.assertEqual(resume_response.json()["state"], "running")

            result = None
            for _ in range(50):
                status_response = client.get(f"/optimizations/{optimization_id}")
                self.assertEqual(status_response.status_code, 200)
                if status_response.json()["state"] == "completed":
                    result_response = client.get(f"/optimizations/{optimization_id}/results")
                    self.assertEqual(result_response.status_code, 200)
                    result = result_response.json()
                    break
                time.sleep(0.02)

            self.assertIsNotNone(result)
            assert result is not None
            self.assertEqual(result["state"], "completed")
            self.assertEqual([trial["trial_number"] for trial in result["all_trials"]], [1, 2])
            self.assertEqual(result["best_trial"]["parameters"], {"short_period": 8, "long_period": 24})
            self.assertTrue(result["diagnostics"]["resume_supported"])
            self.assertTrue(result["manifest"]["execution_plan"]["resume_supported"])
        finally:
            main_module.opt_store = original_store


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
