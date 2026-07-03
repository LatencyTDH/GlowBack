from __future__ import annotations

from datetime import datetime, timezone
import unittest

from api.app.optimization_models import (
    ObjectiveDirection,
    OptimizationState,
    RayClusterConfig,
    ParameterDef,
    ParameterKind,
    SearchSpaceConfig,
    SearchStrategyName,
    ValidationMode,
    OptimizationRequest,
    TrialStatus,
    TrialSummary,
)
from api.app.optimization_runtime import OptimizationExecutor


class ParamAwareBacktestExecutor:
    def run(self, backtest_config: dict) -> dict:
        params = backtest_config["strategy"]["params"]
        short_period = int(params["short_period"])
        long_period = int(params["long_period"])

        start = datetime.fromisoformat(backtest_config["start_date"].replace("Z", "+00:00")).astimezone(timezone.utc)
        end = datetime.fromisoformat(backtest_config["end_date"].replace("Z", "+00:00")).astimezone(timezone.utc)
        days = max(1, (end.date() - start.date()).days + 1)

        if days > 900:
            duration_bonus = 0.45
        elif days > 300:
            duration_bonus = 0.2
        else:
            duration_bonus = -0.15

        sharpe_ratio = 2.5 - abs(short_period - 8) * 0.2 - abs(long_period - 24) * 0.05 + duration_bonus
        total_return = sharpe_ratio * 12

        return {
            "metrics_summary": {
                "sharpe_ratio": sharpe_ratio,
                "total_return": total_return,
                "total_commissions": abs(long_period - short_period) / 10,
            }
        }


class RecordingBacktestExecutor:
    def __init__(self) -> None:
        self.calls: list[dict[str, int]] = []

    def run(self, backtest_config: dict) -> dict:
        params = dict(backtest_config["strategy"]["params"])
        self.calls.append(params)
        score = float(int(params["short_period"]) * 10 + int(params["long_period"]))
        return {
            "metrics_summary": {
                "sharpe_ratio": score,
                "total_return": score * 2,
            }
        }


def _request() -> OptimizationRequest:
    return OptimizationRequest(
        name="Optimizer hardening regression",
        description="Validate diagnostics + manifest generation",
        search_space=SearchSpaceConfig(
            parameters=[
                ParameterDef(name="short_period", kind=ParameterKind.int_range, low=6, high=8),
                ParameterDef(name="long_period", kind=ParameterKind.int_range, low=20, high=24),
            ]
        ),
        strategy=SearchStrategyName.grid,
        max_trials=4,
        concurrency=1,
        objective_metric="sharpe_ratio",
        direction=ObjectiveDirection.maximize,
        validation_mode=ValidationMode.walk_forward,
        validation_fraction=0.25,
        walk_forward_windows=2,
        random_seed=7,
        grid_steps=2,
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


class OptimizationRuntimeTests(unittest.IsolatedAsyncioTestCase):
    async def test_executor_emits_overfit_diagnostics_and_manifest(self) -> None:
        executor = OptimizationExecutor(backtest_executor=ParamAwareBacktestExecutor())

        result = await executor.execute(_request())

        self.assertEqual(result.state, OptimizationState.completed)
        self.assertIsNotNone(result.best_trial)
        assert result.best_trial is not None
        self.assertEqual(result.best_trial.parameters, {"short_period": 8, "long_period": 24})
        self.assertGreater(result.best_trial.metrics["generalization_gap_sharpe_ratio"], 0.0)
        self.assertGreater(result.best_trial.metrics["validation_full_gap_sharpe_ratio"], 0.0)

        self.assertIsNotNone(result.diagnostics)
        assert result.diagnostics is not None
        self.assertEqual(result.diagnostics["objective_metric"], "sharpe_ratio")
        self.assertIn("parameter_stability", result.diagnostics)
        self.assertEqual(
            result.diagnostics["parameter_stability"]["short_period"]["best_value"],
            8,
        )
        self.assertTrue(result.diagnostics["resume_supported"])

        self.assertIsNotNone(result.manifest)
        assert result.manifest is not None
        self.assertEqual(result.manifest["manifest_version"], "1.0")
        self.assertEqual(result.manifest["request"]["random_seed"], 7)
        self.assertEqual(result.manifest["execution_plan"]["mode"], "local_python")
        self.assertTrue(result.manifest["execution_plan"]["resume_supported"])
        self.assertEqual(len(result.manifest["trial_lineage"]), 4)
        self.assertEqual(
            result.manifest["best_trial"]["parameters"],
            {"short_period": 8, "long_period": 24},
        )

    async def test_executor_continues_from_prior_trials(self) -> None:
        executor = OptimizationExecutor(backtest_executor=RecordingBacktestExecutor())
        prior_trials = [
            TrialSummary(
                trial_id="trial-1",
                trial_number=1,
                status=TrialStatus.completed,
                parameters={"short_period": 6, "long_period": 20},
                objective=80.0,
                metrics={"sharpe_ratio": 80.0},
                duration_seconds=0,
            ),
            TrialSummary(
                trial_id="trial-2",
                trial_number=2,
                status=TrialStatus.completed,
                parameters={"short_period": 6, "long_period": 24},
                objective=84.0,
                metrics={"sharpe_ratio": 84.0},
                duration_seconds=0,
            ),
        ]
        resume_request = OptimizationRequest(
            name="Resume regression",
            description="Resume from trial lineage",
            search_space=_request().search_space,
            strategy=SearchStrategyName.grid,
            max_trials=4,
            concurrency=1,
            objective_metric="sharpe_ratio",
            direction=ObjectiveDirection.maximize,
            validation_mode=ValidationMode.holdout,
            validation_fraction=0.25,
            walk_forward_windows=1,
            random_seed=7,
            grid_steps=2,
            base_backtest={
                "symbols": ["AAPL"],
                "start_date": "2024-01-01T00:00:00Z",
                "end_date": "2024-01-05T00:00:00Z",
                "resolution": "day",
                "strategy": {"name": "ma_crossover", "params": {}},
                "initial_capital": 100000,
                "data_source": "sample",
            },
        )

        result = await executor.execute(resume_request, prior_trials=prior_trials)

        self.assertEqual(result.state, OptimizationState.completed)
        self.assertEqual(len(result.trials), 4)
        self.assertEqual([trial.trial_number for trial in result.trials], [1, 2, 3, 4])
        self.assertEqual(result.trials[:2], prior_trials)
        self.assertEqual(executor._backtest_executor.calls, [
            {"short_period": 8, "long_period": 20},
            {"short_period": 8, "long_period": 24},
        ])
        self.assertTrue(result.diagnostics["resume_supported"])
        self.assertTrue(result.manifest["execution_plan"]["resume_supported"])

    async def test_executor_emits_ray_execution_preview(self) -> None:
        executor = OptimizationExecutor(backtest_executor=ParamAwareBacktestExecutor())
        request = _request().model_copy(
            update={
                "concurrency": 4,
                "ray_cluster": RayClusterConfig(
                    address="ray://example:10001",
                    namespace="glowback-qa",
                    max_concurrent_tasks=2,
                    num_cpus=2.5,
                    num_gpus=0.0,
                    pip_packages=["ray==2.46.0"],
                )
            }
        )

        result = await executor.execute(request)

        self.assertIsNotNone(result.manifest)
        assert result.manifest is not None
        execution_plan = result.manifest["execution_plan"]
        self.assertTrue(execution_plan["ray_cluster_requested"])
        self.assertIn("distributed_preview", execution_plan)
        preview = execution_plan["distributed_preview"]
        self.assertEqual(preview["scheduler"], "ray")
        self.assertEqual(preview["status"], "preview")
        self.assertEqual(preview["task_count"], 4)
        self.assertEqual(preview["planned_workers"], 2)
        self.assertEqual(preview["cluster"]["namespace"], "glowback-qa")
        self.assertEqual(preview["cluster"]["max_concurrent_tasks"], 2)
        self.assertEqual(preview["trial_numbers"], [1, 2, 3, 4])


if __name__ == "__main__":
    unittest.main()
