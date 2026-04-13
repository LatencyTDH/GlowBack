from __future__ import annotations

from datetime import datetime, timezone
import unittest

from api.app.adapter import MockEngineAdapter
from api.app.models import BacktestRequest
from api.app.store import RunStore


class BacktestAdapterTests(unittest.IsolatedAsyncioTestCase):
    async def test_mock_adapter_emits_benchmark_curve_and_tearsheet(self) -> None:
        store = RunStore()
        adapter = MockEngineAdapter(store)
        request = BacktestRequest(
            symbols=["AAPL"],
            start_date=datetime(2024, 1, 1, tzinfo=timezone.utc),
            end_date=datetime(2024, 3, 1, tzinfo=timezone.utc),
            benchmark_symbol="SPY",
        )

        status_obj = await store.create_run(request)
        await adapter.run(status_obj.run_id, request)
        result = await store.get_result(status_obj.run_id)

        self.assertIsNotNone(result)
        assert result is not None
        self.assertEqual(result.benchmark_symbol, "SPY")
        self.assertTrue(result.benchmark_curve)
        self.assertIn("benchmark_symbol", result.metrics_summary)
        self.assertIn("information_ratio", result.metrics_summary)
        self.assertIn("benchmark", result.tearsheet)
        self.assertEqual(result.tearsheet["benchmark"]["benchmark_symbol"], "SPY")


if __name__ == "__main__":
    unittest.main()
