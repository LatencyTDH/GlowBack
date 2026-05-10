import sys
import unittest
from pathlib import Path

import pandas as pd

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from research_registry import build_streamlit_run_request  # noqa: E402


class ResearchRegistryTests(unittest.TestCase):
    def test_build_streamlit_run_request_captures_reproducibility_metadata(self):
        market_data = pd.DataFrame(
            [
                {
                    "timestamp": pd.Timestamp("2026-01-01T00:00:00Z"),
                    "symbol": "MSFT",
                    "open": 100.0,
                    "high": 101.0,
                    "low": 99.0,
                    "close": 100.5,
                    "volume": 1000,
                    "resolution": "day",
                },
                {
                    "timestamp": pd.Timestamp("2026-01-02T00:00:00Z"),
                    "symbol": "AAPL",
                    "open": 200.0,
                    "high": 202.0,
                    "low": 198.0,
                    "close": 201.0,
                    "volume": 1500,
                    "resolution": "day",
                },
            ]
        )

        payload = build_streamlit_run_request(
            market_data,
            {
                "name": "Mean Reversion Lab",
                "initial_capital": 50000,
                "commission": 0.001,
                "slippage": 5,
                "max_position_size": 0.25,
            },
            "class MeanReversionLab:\n    pass\n",
        )

        self.assertEqual(payload["symbols"], ["AAPL", "MSFT"])
        self.assertEqual(payload["resolution"], "day")
        self.assertEqual(payload["strategy"]["name"], "Mean Reversion Lab")
        self.assertEqual(payload["provenance"]["bar_count"], 2)
        self.assertEqual(payload["provenance"]["symbol_count"], 2)
        self.assertIn("strategy_code_hash", payload["provenance"])
        self.assertIn("dataset_fingerprint", payload["provenance"])


if __name__ == "__main__":
    unittest.main()
