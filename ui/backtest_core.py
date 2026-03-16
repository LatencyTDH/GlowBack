"""
Shared backtest execution and analytics helpers for the Streamlit UI.
"""

from __future__ import annotations

from collections import defaultdict, deque
from datetime import datetime
from typing import Any, Iterable
import time

import numpy as np
import pandas as pd


class SimplePortfolio:
    """Simple long-only portfolio implementation for UI backtests."""

    def __init__(self, initial_cash: float = 100000):
        self.initial_cash = initial_cash
        self.cash = initial_cash
        self.positions: dict[str, float] = {}
        self.trades: list[dict[str, Any]] = []
        self.equity_curve: list[dict[str, Any]] = []
        self.last_prices: dict[str, float] = {}

    def buy(self, symbol, shares, price, timestamp=None):
        """Buy shares."""
        cost = shares * price
        if cost <= self.cash:
            self.cash -= cost
            self.positions[symbol] = self.positions.get(symbol, 0) + shares
            self.last_prices[symbol] = price
            self.trades.append(
                {
                    "timestamp": timestamp or datetime.now(),
                    "symbol": symbol,
                    "action": "BUY",
                    "shares": shares,
                    "price": price,
                    "cost": cost,
                }
            )
            return True
        return False

    def sell(self, symbol, shares, price, timestamp=None):
        """Sell shares."""
        if self.positions.get(symbol, 0) >= shares:
            proceeds = shares * price
            self.cash += proceeds
            self.positions[symbol] -= shares
            self.last_prices[symbol] = price
            if self.positions[symbol] == 0:
                del self.positions[symbol]
            self.trades.append(
                {
                    "timestamp": timestamp or datetime.now(),
                    "symbol": symbol,
                    "action": "SELL",
                    "shares": shares,
                    "price": price,
                    "proceeds": proceeds,
                }
            )
            return True
        return False

    def get_position(self, symbol):
        """Get position size for symbol."""
        return self.positions.get(symbol, 0)

    def get_positions(self):
        """Get all positions."""
        return self.positions.copy()

    def calculate_position_value(self, current_prices: dict[str, float] | None = None) -> float:
        """Calculate marked-to-market position value using the latest known price per symbol."""
        if current_prices:
            self.last_prices.update({symbol: price for symbol, price in current_prices.items() if price is not None})

        return sum(
            shares * self.last_prices.get(symbol, 0)
            for symbol, shares in self.positions.items()
        )

    def calculate_value(self, current_prices: dict[str, float] | None = None) -> float:
        """Calculate total portfolio value."""
        return self.cash + self.calculate_position_value(current_prices)

    @property
    def value(self):
        """Total portfolio value."""
        return self.calculate_value()

    @property
    def total_equity(self):
        """Total equity."""
        return self.calculate_value()

    @property
    def unrealized_pnl(self):
        """Unrealized P&L."""
        return self.calculate_value() - self.initial_cash

    @property
    def realized_pnl(self):
        """Realized P&L."""
        total_bought = sum(t["cost"] for t in self.trades if t["action"] == "BUY")
        total_sold = sum(t.get("proceeds", 0) for t in self.trades if t["action"] == "SELL")
        return total_sold - total_bought


class SimpleBar:
    """Simple bar data structure."""

    def __init__(self, timestamp, symbol, open_price, high, low, close, volume, resolution):
        self.timestamp = timestamp
        self.symbol = symbol
        self.open = open_price
        self.high = high
        self.low = low
        self.close = close
        self.volume = volume
        self.resolution = resolution


def prepare_equity_curve_frame(equity_curve: Iterable[dict[str, Any]] | pd.DataFrame) -> pd.DataFrame:
    """Convert an equity curve payload into a normalized DataFrame."""
    frame = equity_curve.copy() if isinstance(equity_curve, pd.DataFrame) else pd.DataFrame(list(equity_curve))
    if not frame.empty and "timestamp" in frame.columns:
        frame["timestamp"] = pd.to_datetime(frame["timestamp"])
    return frame


def calculate_period_return_series(equity_curve: Iterable[dict[str, Any]] | pd.DataFrame) -> pd.Series:
    """Calculate per-period returns directly from portfolio value."""
    frame = prepare_equity_curve_frame(equity_curve)
    if frame.empty or "value" not in frame.columns:
        return pd.Series(dtype=float)

    values = pd.to_numeric(frame["value"], errors="coerce")
    returns = values.pct_change(fill_method=None)
    returns = returns.replace([np.inf, -np.inf], np.nan)
    returns.index = frame.index
    return returns


def calculate_sharpe_ratio(equity_curve: Iterable[dict[str, Any]] | pd.DataFrame, periods_per_year: int = 252) -> float:
    """Calculate Sharpe ratio from true per-period returns."""
    period_returns = calculate_period_return_series(equity_curve).dropna()
    if period_returns.empty:
        return 0.0

    std_dev = period_returns.std()
    if pd.isna(std_dev) or std_dev == 0:
        return 0.0

    return float(period_returns.mean() / std_dev * np.sqrt(periods_per_year))


def calculate_max_drawdown(equity_curve: Iterable[dict[str, Any]] | pd.DataFrame) -> float:
    """Calculate max drawdown as a decimal fraction."""
    frame = prepare_equity_curve_frame(equity_curve)
    if frame.empty or "value" not in frame.columns:
        return 0.0

    values = pd.to_numeric(frame["value"], errors="coerce")
    peak = values.cummax()
    drawdown = (peak - values) / peak
    drawdown = drawdown.replace([np.inf, -np.inf], np.nan).fillna(0.0)
    return float(drawdown.max()) if not drawdown.empty else 0.0


def calculate_annualized_return_pct(
    equity_curve: Iterable[dict[str, Any]] | pd.DataFrame,
    periods_per_year: int = 252,
) -> float:
    """Calculate annualized return percentage from the equity curve."""
    frame = prepare_equity_curve_frame(equity_curve)
    if frame.empty or "value" not in frame.columns or len(frame) < 2:
        return 0.0

    start_value = float(frame["value"].iloc[0])
    end_value = float(frame["value"].iloc[-1])
    if start_value <= 0:
        return 0.0

    periods = len(frame) - 1
    if periods <= 0:
        return 0.0

    growth = end_value / start_value
    annualized = growth ** (periods_per_year / periods) - 1
    return float(annualized * 100)


def calculate_closed_trade_win_rate(trades: list[dict[str, Any]]) -> float | None:
    """Calculate win rate from realized sell-side closures using FIFO lots.

    Returns ``None`` when no closed trades exist yet.
    """
    if not trades:
        return None

    open_lots: dict[str, deque[dict[str, float]]] = defaultdict(deque)
    closed_trade_pnls: list[float] = []

    for trade in trades:
        symbol = trade.get("symbol")
        shares = float(trade.get("shares", 0) or 0)
        price = float(trade.get("price", 0) or 0)
        action = trade.get("action")

        if not symbol or shares <= 0:
            continue

        if action == "BUY":
            open_lots[symbol].append({"shares": shares, "price": price})
            continue

        if action != "SELL":
            continue

        remaining = shares
        realized_pnl = 0.0
        closed_any_quantity = False

        while remaining > 0 and open_lots[symbol]:
            lot = open_lots[symbol][0]
            matched_shares = min(remaining, lot["shares"])
            realized_pnl += matched_shares * (price - lot["price"])
            lot["shares"] -= matched_shares
            remaining -= matched_shares
            closed_any_quantity = True

            if lot["shares"] <= 0:
                open_lots[symbol].popleft()

        if closed_any_quantity:
            closed_trade_pnls.append(realized_pnl)

    if not closed_trade_pnls:
        return None

    wins = sum(1 for pnl in closed_trade_pnls if pnl > 0)
    return wins / len(closed_trade_pnls) * 100.0


def _queue_put(queue_obj, value):
    if queue_obj is not None:
        queue_obj.put(value)


def run_backtest(strategy_code, market_data, config, progress_queue, log_queue):
    """Run a simple UI backtest in a worker thread."""
    try:
        namespace = {}
        exec(strategy_code, namespace)

        strategy_classes = [
            obj for obj in namespace.values() if isinstance(obj, type) and hasattr(obj, "on_bar")
        ]
        if not strategy_classes:
            _queue_put(log_queue, "ERROR: No strategy class found")
            return None

        strategy_class = strategy_classes[0]
        strategy = strategy_class()
        portfolio = SimplePortfolio(config.get("initial_capital", 100000))

        _queue_put(log_queue, f"Started backtest: {getattr(strategy, 'name', 'Unknown Strategy')}")
        _queue_put(log_queue, f"Initial capital: ${portfolio.initial_cash:,.2f}")

        total_bars = len(market_data)
        equity_curve = []
        all_logs = []
        latest_prices: dict[str, float] = {}
        previous_value = float(portfolio.initial_cash)

        for i, row in market_data.iterrows():
            bar = SimpleBar(
                timestamp=row["timestamp"],
                symbol=row["symbol"],
                open_price=row["open"],
                high=row["high"],
                low=row["low"],
                close=row["close"],
                volume=row["volume"],
                resolution=row["resolution"],
            )

            try:
                logs = strategy.on_bar(bar, portfolio)
                if logs:
                    for log in logs:
                        log_line = f"{bar.timestamp.strftime('%Y-%m-%d')}: {log}"
                        _queue_put(log_queue, log_line)
                        all_logs.append(log_line)
            except Exception as exc:
                _queue_put(log_queue, f"ERROR on {bar.timestamp}: {str(exc)}")

            latest_prices[row["symbol"]] = row["close"]
            position_value = portfolio.calculate_position_value(latest_prices)
            portfolio_value = portfolio.cash + position_value
            period_return = 0.0 if previous_value == 0 else (portfolio_value - previous_value) / previous_value
            cumulative_return_pct = (
                0.0
                if portfolio.initial_cash == 0
                else (portfolio_value - portfolio.initial_cash) / portfolio.initial_cash * 100
            )

            equity_curve.append(
                {
                    "timestamp": row["timestamp"],
                    "value": portfolio_value,
                    "cash": portfolio.cash,
                    "positions": sum(portfolio.positions.values()),
                    "position_value": position_value,
                    "returns": cumulative_return_pct,
                    "period_return": period_return,
                }
            )
            previous_value = portfolio_value

            progress = (i + 1) / total_bars
            _queue_put(progress_queue, progress)
            time.sleep(0.01)

        _queue_put(log_queue, "Backtest completed!")

        final_value = equity_curve[-1]["value"]
        total_return = (final_value - portfolio.initial_cash) / portfolio.initial_cash * 100
        sharpe_ratio = calculate_sharpe_ratio(equity_curve)
        max_drawdown = calculate_max_drawdown(equity_curve) * 100

        return {
            "equity_curve": equity_curve,
            "trades": portfolio.trades,
            "initial_capital": portfolio.initial_cash,
            "final_value": final_value,
            "total_return": total_return,
            "sharpe_ratio": sharpe_ratio,
            "max_drawdown": max_drawdown,
            "total_trades": len(portfolio.trades),
            "final_cash": portfolio.cash,
            "final_positions": portfolio.positions,
            "logs": all_logs,
        }
    except Exception as exc:
        _queue_put(log_queue, f"FATAL ERROR: {str(exc)}")
        return None
