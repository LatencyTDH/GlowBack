from __future__ import annotations

import unittest
from unittest import mock

from api.app.adapter import RealEngineAdapter
from api.app.models import BacktestRequest, PortfolioConstructionConfig, RunState
from api.app.store import RunStore


class RealEngineAdapterTests(unittest.IsolatedAsyncioTestCase):
    async def test_adapter_uses_real_engine_runtime_and_persists_result(self) -> None:
        store = RunStore()
        adapter = RealEngineAdapter(store)
        request = BacktestRequest(
            symbols=["AAPL"],
            start_date="2024-01-01T00:00:00Z",
            end_date="2024-12-31T00:00:00Z",
            resolution="day",
            strategy={"name": "buy_and_hold", "params": {}},
            execution={"commission_bps": 1.5, "slippage_bps": 4.0, "latency_ms": 250},
            initial_capital=100000.0,
            benchmark_symbol="SPY",
            portfolio_construction=PortfolioConstructionConfig(
                target_weights={"AAPL": 1.0},
                rebalance_frequency="weekly",
                cash_floor_pct=5.0,
                max_turnover_pct=50.0,
            ),
            data_source="sample",
        )
        status_obj = await store.create_run(request)

        with mock.patch(
            "api.app.adapter.run_engine_backtest",
            return_value={
                "metrics_summary": {
                    "initial_capital": 100000.0,
                    "final_value": 101250.0,
                    "benchmark_symbol": "SPY",
                    "information_ratio": 0.42,
                },
                "equity_curve": [{"timestamp": "2024-01-01T00:00:00Z", "value": 101250.0}],
                "benchmark_curve": [{"timestamp": "2024-01-01T00:00:00Z", "symbol": "SPY", "value": 100500.0}],
                "trades": [{"timestamp": "2024-01-01T00:00:00Z", "symbol": "AAPL", "action": "BUY", "shares": 10.0, "price": 100.0}],
                "exposures": [{"timestamp": "2024-01-01T00:00:00Z", "cash_pct": 5.0, "positions_pct": 95.0}],
                "portfolio_construction": {"method": "target_weights", "rebalance_frequency": "weekly"},
                "portfolio_diagnostics": [{"timestamp": "2024-01-01T00:00:00Z", "rebalanced": True}],
                "constraint_hits": [{"type": "max_weight_cap", "symbol": "AAPL"}],
                "tearsheet": {"benchmark": {"benchmark_symbol": "SPY"}},
                "logs": ["Engine-backed backtest completed"],
                "final_cash": 5000.0,
                "final_positions": {"AAPL": 10.0},
                "manifest": {
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
                        "end_date": "2024-12-31T00:00:00Z",
                        "symbols": ["AAPL"],
                        "bar_counts": {"AAPL": 252},
                        "total_bars": 252,
                    },
                    "execution": {
                        "initial_capital": 100000.0,
                        "commission_bps": 1.5,
                        "slippage_bps": 4.0,
                        "latency_ms": 250,
                        "commission_percentage": 0.00015,
                        "minimum_commission": 1.0,
                        "slippage_model": {"Linear": {"basis_points": 4}},
                        "latency_model": {"Fixed": {"milliseconds": 250}},
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
                        "end_date": "2024-12-31T00:00:00Z",
                        "resolution": "day",
                        "strategy_name": "buy_and_hold",
                        "strategy_params": {},
                        "initial_capital": 100000.0,
                        "data_source": "sample",
                        "commission_bps": 1.5,
                        "slippage_bps": 4.0,
                        "latency_ms": 250,
                        "run_name": "API Backtest replay",
                    },
                    "metric_snapshot": {
                        "final_value": 101250.0,
                        "total_return": 1.25,
                        "max_drawdown": 0.0,
                        "sharpe_ratio": 0.42,
                        "total_trades": 1,
                    },
                },
            },
        ) as mocked_run:
            await adapter.run(status_obj.run_id, request)

        mocked_run.assert_called_once()
        kwargs = mocked_run.call_args.kwargs
        self.assertEqual(kwargs["strategy_name"], "buy_and_hold")
        self.assertEqual(kwargs["data_source"], "sample")
        self.assertEqual(kwargs["commission_bps"], 1.5)
        self.assertEqual(kwargs["slippage_bps"], 4.0)
        self.assertEqual(kwargs["latency_ms"], 250)

        status = await store.get_status(status_obj.run_id)
        result = await store.get_result(status_obj.run_id)
        self.assertIsNotNone(status)
        self.assertIsNotNone(result)
        assert status is not None
        assert result is not None
        self.assertEqual(status.state, RunState.completed)
        self.assertEqual(result.benchmark_symbol, "SPY")
        self.assertTrue(result.benchmark_curve)
        self.assertEqual(result.portfolio_construction.get("rebalance_frequency"), "weekly")
        self.assertTrue(result.portfolio_diagnostics)
        self.assertTrue(result.constraint_hits)
        self.assertIn("benchmark_symbol", result.metrics_summary)
        self.assertEqual(result.final_cash, 5000.0)
        self.assertEqual(result.final_positions, {"AAPL": 10.0})
        self.assertEqual(result.trades[0]["action"], "BUY")
        self.assertIsNotNone(result.manifest)
        self.assertEqual(result.manifest["replay_request"]["strategy_name"], "buy_and_hold")


if __name__ == "__main__":
    unittest.main()
