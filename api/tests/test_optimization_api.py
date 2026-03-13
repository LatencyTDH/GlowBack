from __future__ import annotations

import unittest

from fastapi.testclient import TestClient

from api.app.main import app
from api.app.optimization_models import (
    ObjectiveDirection,
    OptimizationRequest,
    OptimizationState,
    ParameterDef,
    ParameterKind,
    SearchSpaceConfig,
    SearchStrategyName,
)
from api.app.optimization_store import (
    OPTIMIZATION_BACKEND_UNAVAILABLE_ERROR,
    OptimizationStore,
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
        max_trials=10,
        concurrency=2,
        objective_metric="sharpe_ratio",
        direction=ObjectiveDirection.maximize,
        base_backtest={
            "symbols": ["AAPL"],
            "start_date": "2020-01-01T00:00:00Z",
            "end_date": "2024-01-01T00:00:00Z",
            "strategy": {"name": "ma_crossover"},
            "initial_capital": 100000,
        },
    )


class OptimizationApiTests(unittest.TestCase):
    def setUp(self) -> None:
        self.client = TestClient(app)

    def test_create_optimization_returns_501_until_real_backend_exists(self) -> None:
        response = self.client.post("/optimizations", json=_sample_request().model_dump(mode="json"))

        self.assertEqual(response.status_code, 501)
        detail = response.json()["detail"]
        self.assertIn("not wired to a real backtest backend", detail)
        self.assertIn("base_backtest/ray_cluster", detail)
        self.assertIn("synthetic trial metrics", detail)


class OptimizationStoreTests(unittest.IsolatedAsyncioTestCase):
    async def test_run_optimization_marks_record_failed_instead_of_generating_trials(self) -> None:
        store = OptimizationStore()
        created = await store.create(_sample_request())

        await store.run_optimization(created.optimization_id)

        status_obj = await store.get_status(created.optimization_id)
        result = await store.get_result(created.optimization_id)

        self.assertIsNotNone(status_obj)
        self.assertIsNotNone(result)
        self.assertEqual(status_obj.state, OptimizationState.failed)
        self.assertEqual(result.state, OptimizationState.failed)
        self.assertEqual(result.all_trials, [])
        self.assertIsNone(result.best_trial)
        self.assertEqual(status_obj.error, OPTIMIZATION_BACKEND_UNAVAILABLE_ERROR)
        self.assertEqual(result.total_duration_seconds, 0)


if __name__ == "__main__":
    unittest.main()
