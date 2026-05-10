from __future__ import annotations

import unittest
from unittest import mock

from glowback_runtime import compare_manifest_metrics, replay_manifest, validate_run_manifest


class RunManifestTests(unittest.TestCase):
    def _manifest(self) -> dict:
        return {
            "manifest_version": "1.0",
            "generated_at": "2026-04-28T00:00:00Z",
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
                "start_date": "2024-01-01T00:00:00Z",
                "end_date": "2024-01-10T00:00:00Z",
                "symbols": ["AAPL"],
                "bar_counts": {"AAPL": 10},
                "total_bars": 10,
            },
            "execution": {
                "initial_capital": 100000.0,
                "commission_bps": 5.0,
                "slippage_bps": 5.0,
                "latency_ms": 100,
                "commission_percentage": 0.0005,
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
                "start_date": "2024-01-01T00:00:00Z",
                "end_date": "2024-01-10T00:00:00Z",
                "resolution": "day",
                "strategy_name": "buy_and_hold",
                "strategy_params": {},
                "initial_capital": 100000.0,
                "data_source": "sample",
                "commission_bps": 5.0,
                "slippage_bps": 5.0,
                "latency_ms": 100,
                "run_name": "Manifest Replay",
            },
            "metric_snapshot": {
                "final_value": 101500.0,
                "total_return": 1.5,
                "max_drawdown": 0.25,
                "sharpe_ratio": 1.1,
                "total_trades": 3,
            },
        }

    def test_validate_run_manifest_rejects_missing_required_fields(self) -> None:
        manifest = self._manifest()
        manifest.pop("replay_request")

        with self.assertRaisesRegex(ValueError, "replay_request"):
            validate_run_manifest(manifest)

    def test_replay_manifest_uses_replay_request_and_compares_metrics(self) -> None:
        manifest = self._manifest()
        with mock.patch(
            "glowback_runtime.run_backtest",
            return_value={
                "final_value": 101500.0,
                "total_return": 1.5,
                "max_drawdown": 0.25,
                "sharpe_ratio": 1.1,
                "total_trades": 3,
                "metrics_summary": {
                    "final_value": 101500.0,
                    "total_return": 1.5,
                    "max_drawdown": 0.25,
                    "sharpe_ratio": 1.1,
                    "total_trades": 3,
                },
            },
        ) as mocked_run:
            replay_payload = replay_manifest(manifest)
            comparison = compare_manifest_metrics(manifest, replay_payload)

        mocked_run.assert_called_once_with(**manifest["replay_request"])
        self.assertTrue(comparison["within_tolerance"])
        self.assertEqual(comparison["deltas"]["final_value"], 0.0)
        self.assertEqual(comparison["deltas"]["total_trades"], 0.0)


if __name__ == "__main__":
    unittest.main()
