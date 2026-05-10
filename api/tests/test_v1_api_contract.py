from __future__ import annotations

import time
import unittest

from fastapi.testclient import TestClient

import api.app.main as main_module
from api.app.adapter import RealEngineAdapter
from api.app.models import RunState
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
from api.app.store import RunStore


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


class VersionedApiContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.original_store = main_module.store
        self.original_adapter = main_module.adapter
        self.original_opt_store = main_module.opt_store

        main_module.store = RunStore()
        main_module.adapter = RealEngineAdapter(main_module.store)
        main_module.opt_store = OptimizationStore(executor=FakeOptimizationExecutor())
        self.client = TestClient(main_module.app)

    def tearDown(self) -> None:
        main_module.store = self.original_store
        main_module.adapter = self.original_adapter
        main_module.opt_store = self.original_opt_store

    def test_versioned_backtest_list_alias_exists(self) -> None:
        response = self.client.get("/v1/backtests")
        legacy_response = self.client.get("/backtests")

        self.assertEqual(response.status_code, 200)
        self.assertEqual(legacy_response.status_code, 200)
        self.assertEqual(response.json(), legacy_response.json())

    def test_versioned_backtest_not_found_uses_error_envelope(self) -> None:
        response = self.client.get("/v1/backtests/missing-run")

        self.assertEqual(response.status_code, 404)
        self.assertEqual(response.json()["error"]["code"], "not_found")
        self.assertEqual(response.json()["error"]["message"], "Run not found")
        self.assertEqual(response.headers["x-request-id"], response.json()["request_id"])

    def test_legacy_backtest_not_found_keeps_detail_shape(self) -> None:
        response = self.client.get("/backtests/missing-run")

        self.assertEqual(response.status_code, 404)
        self.assertEqual(response.json(), {"detail": "Run not found"})

    def test_versioned_optimization_validation_error_uses_error_envelope(self) -> None:
        payload = _sample_request().model_dump(mode="json")
        payload.pop("name")

        response = self.client.post("/v1/optimizations", json=payload)

        self.assertEqual(response.status_code, 422)
        self.assertEqual(response.json()["error"]["code"], "validation_error")
        self.assertEqual(response.json()["error"]["message"], "Request validation failed")
        self.assertTrue(
            any(detail["loc"][-1] == "name" for detail in response.json()["error"]["details"])
        )

    def test_versioned_optimization_routes_complete_successfully(self) -> None:
        response = self.client.post("/v1/optimizations", json=_sample_request().model_dump(mode="json"))

        self.assertEqual(response.status_code, 201)
        created = response.json()
        self.assertIn(created["state"], {"pending", "running", "completed"})
        optimization_id = created["optimization_id"]

        result = None
        for _ in range(50):
            status_response = self.client.get(f"/v1/optimizations/{optimization_id}")
            self.assertEqual(status_response.status_code, 200)
            self.assertIn(status_response.json()["state"], {state.value for state in OptimizationState})
            if status_response.json()["state"] == OptimizationState.completed.value:
                result_response = self.client.get(f"/v1/optimizations/{optimization_id}/results")
                self.assertEqual(result_response.status_code, 200)
                result = result_response.json()
                break
            time.sleep(0.02)

        self.assertIsNotNone(result)
        assert result is not None
        self.assertEqual(result["state"], OptimizationState.completed.value)
        self.assertEqual(result["best_trial"]["status"], TrialStatus.completed.value)
        self.assertEqual(result["replay_backtest"]["strategy"]["params"], {"short_period": 8, "long_period": 24})


if __name__ == "__main__":
    unittest.main()
