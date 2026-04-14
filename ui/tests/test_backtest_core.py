import queue
import sys
import unittest
from pathlib import Path

import pandas as pd

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from backtest_core import (  # noqa: E402
    calculate_closed_trade_win_rate,
    calculate_period_return_series,
    run_backtest,
)


class BacktestCoreTests(unittest.TestCase):
    def test_run_backtest_marks_multi_symbol_positions_with_last_known_prices(self):
        strategy_code = """
class BuyAndHoldTwoSymbols:
    name = "Buy and Hold Two Symbols"

    def on_bar(self, bar, portfolio):
        if bar.symbol == 'AAPL' and portfolio.get_position('AAPL') == 0:
            portfolio.buy('AAPL', 10, bar.close, bar.timestamp)
        elif bar.symbol == 'MSFT' and portfolio.get_position('MSFT') == 0:
            portfolio.buy('MSFT', 10, bar.close, bar.timestamp)
        return []
"""
        market_data = pd.DataFrame(
            [
                {
                    'timestamp': pd.Timestamp('2026-01-01'),
                    'symbol': 'AAPL',
                    'open': 100.0,
                    'high': 100.0,
                    'low': 100.0,
                    'close': 100.0,
                    'volume': 1000,
                    'resolution': 'day',
                },
                {
                    'timestamp': pd.Timestamp('2026-01-01'),
                    'symbol': 'MSFT',
                    'open': 50.0,
                    'high': 50.0,
                    'low': 50.0,
                    'close': 50.0,
                    'volume': 1000,
                    'resolution': 'day',
                },
                {
                    'timestamp': pd.Timestamp('2026-01-02'),
                    'symbol': 'AAPL',
                    'open': 110.0,
                    'high': 110.0,
                    'low': 110.0,
                    'close': 110.0,
                    'volume': 1000,
                    'resolution': 'day',
                },
                {
                    'timestamp': pd.Timestamp('2026-01-02'),
                    'symbol': 'MSFT',
                    'open': 55.0,
                    'high': 55.0,
                    'low': 55.0,
                    'close': 55.0,
                    'volume': 1000,
                    'resolution': 'day',
                },
            ]
        )

        results = run_backtest(
            strategy_code,
            market_data,
            {'initial_capital': 2000.0},
            queue.Queue(),
            queue.Queue(),
        )

        self.assertIsNotNone(results)
        self.assertEqual(results['equity_curve'][2]['value'], 2100.0)
        self.assertEqual(results['equity_curve'][3]['value'], 2150.0)
        self.assertEqual(results['final_value'], 2150.0)

    def test_period_returns_are_derived_from_portfolio_value_not_cumulative_return_percent(self):
        equity_curve = [
            {'timestamp': '2026-01-01', 'value': 100.0, 'returns': 0.0},
            {'timestamp': '2026-01-02', 'value': 110.0, 'returns': 10.0},
            {'timestamp': '2026-01-03', 'value': 120.0, 'returns': 20.0},
        ]

        period_returns = calculate_period_return_series(equity_curve).dropna().tolist()

        self.assertEqual(len(period_returns), 2)
        self.assertAlmostEqual(period_returns[0], 0.10, places=8)
        self.assertAlmostEqual(period_returns[1], 120.0 / 110.0 - 1.0, places=8)

    def test_closed_trade_win_rate_uses_realized_pnl(self):
        trades = [
            {'symbol': 'AAPL', 'action': 'BUY', 'shares': 10, 'price': 100.0},
            {'symbol': 'AAPL', 'action': 'SELL', 'shares': 10, 'price': 110.0},
            {'symbol': 'MSFT', 'action': 'BUY', 'shares': 10, 'price': 50.0},
            {'symbol': 'MSFT', 'action': 'SELL', 'shares': 10, 'price': 45.0},
        ]

        self.assertEqual(calculate_closed_trade_win_rate(trades), 50.0)
        self.assertIsNone(
            calculate_closed_trade_win_rate([
                {'symbol': 'AAPL', 'action': 'BUY', 'shares': 10, 'price': 100.0}
            ])
        )

    def test_run_backtest_supports_target_weight_portfolio_construction(self):
        market_data = pd.DataFrame(
            [
                {
                    'timestamp': pd.Timestamp('2026-01-01'),
                    'symbol': 'AAPL',
                    'open': 100.0,
                    'high': 100.0,
                    'low': 100.0,
                    'close': 100.0,
                    'volume': 1000,
                    'resolution': 'day',
                },
                {
                    'timestamp': pd.Timestamp('2026-01-01'),
                    'symbol': 'MSFT',
                    'open': 50.0,
                    'high': 50.0,
                    'low': 50.0,
                    'close': 50.0,
                    'volume': 1000,
                    'resolution': 'day',
                },
                {
                    'timestamp': pd.Timestamp('2026-01-08'),
                    'symbol': 'AAPL',
                    'open': 110.0,
                    'high': 110.0,
                    'low': 110.0,
                    'close': 110.0,
                    'volume': 1000,
                    'resolution': 'day',
                },
                {
                    'timestamp': pd.Timestamp('2026-01-08'),
                    'symbol': 'MSFT',
                    'open': 55.0,
                    'high': 55.0,
                    'low': 55.0,
                    'close': 55.0,
                    'volume': 1000,
                    'resolution': 'day',
                },
            ]
        )

        results = run_backtest(
            '',
            market_data,
            {
                'initial_capital': 1000.0,
                'benchmark_symbol': 'AAPL',
                'portfolio_construction': {
                    'enabled': True,
                    'target_weights': {'AAPL': 0.6, 'MSFT': 0.4},
                    'rebalance_frequency': 'weekly',
                    'cash_floor_pct': 0.0,
                    'max_weight_pct': 70.0,
                    'max_turnover_pct': 100.0,
                    'drift_threshold_pct': 0.0,
                    'max_drawdown_pct': 0.0,
                },
            },
            queue.Queue(),
            queue.Queue(),
        )

        self.assertIsNotNone(results)
        self.assertTrue(results['portfolio_construction'])
        self.assertTrue(results['portfolio_diagnostics'])
        self.assertEqual(results['portfolio_construction']['rebalance_frequency'], 'weekly')
        self.assertEqual(results['portfolio_diagnostics'][0]['rebalance_reason'], 'initial_allocation')
        self.assertAlmostEqual(results['portfolio_diagnostics'][0]['target_weights']['AAPL'], 60.0, places=2)
        self.assertGreater(results['metrics_summary']['portfolio_rebalances'], 0)
        self.assertIn('portfolio', results['tearsheet'])

    def test_run_backtest_surfaces_real_benchmark_metrics_and_cost_drag(self):
        strategy_code = """
class BuyThenSell:
    name = "Buy Then Sell"

    def on_bar(self, bar, portfolio):
        if portfolio.get_position(bar.symbol) == 0:
            portfolio.buy(bar.symbol, 5, bar.close, bar.timestamp)
        elif bar.close >= 110:
            portfolio.sell(bar.symbol, portfolio.get_position(bar.symbol), bar.close, bar.timestamp)
        return []
"""
        market_data = pd.DataFrame(
            [
                {
                    'timestamp': pd.Timestamp('2026-01-01'),
                    'symbol': 'AAPL',
                    'open': 100.0,
                    'high': 100.0,
                    'low': 100.0,
                    'close': 100.0,
                    'volume': 1000,
                    'resolution': 'day',
                },
                {
                    'timestamp': pd.Timestamp('2026-01-02'),
                    'symbol': 'AAPL',
                    'open': 110.0,
                    'high': 110.0,
                    'low': 110.0,
                    'close': 110.0,
                    'volume': 1000,
                    'resolution': 'day',
                },
                {
                    'timestamp': pd.Timestamp('2026-01-03'),
                    'symbol': 'AAPL',
                    'open': 108.0,
                    'high': 108.0,
                    'low': 108.0,
                    'close': 108.0,
                    'volume': 1000,
                    'resolution': 'day',
                },
            ]
        )

        results = run_backtest(
            strategy_code,
            market_data,
            {
                'initial_capital': 1000.0,
                'commission': 0.001,
                'slippage_bps': 5,
                'benchmark_symbol': 'AAPL',
            },
            queue.Queue(),
            queue.Queue(),
        )

        self.assertIsNotNone(results)
        self.assertEqual(results['benchmark_symbol'], 'AAPL')
        self.assertTrue(results['benchmark_curve'])
        self.assertIn('beta', results['benchmark_metrics'])
        self.assertGreater(results['cost_summary']['total_commissions'], 0.0)
        self.assertGreater(results['cost_summary']['total_slippage_cost'], 0.0)
        self.assertTrue(results['attribution'])
        self.assertIn('overview', results['tearsheet'])


if __name__ == '__main__':
    unittest.main()
