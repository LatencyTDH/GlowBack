"""
Shared backtest execution and analytics helpers for the Streamlit UI.
"""

from __future__ import annotations

from collections import defaultdict, deque
from datetime import datetime
from typing import Any, Iterable
import math
import time

import numpy as np
import pandas as pd


TRADING_DAYS_PER_YEAR = 252
RISK_FREE_RATE = 0.02


class SimplePortfolio:
    """Simple long-only portfolio implementation for UI backtests."""

    def __init__(
        self,
        initial_cash: float = 100000,
        commission_rate: float = 0.0,
        slippage_bps: float = 0.0,
    ):
        self.initial_cash = initial_cash
        self.cash = initial_cash
        self.positions: dict[str, float] = {}
        self.trades: list[dict[str, Any]] = []
        self.equity_curve: list[dict[str, Any]] = []
        self.last_prices: dict[str, float] = {}
        self.total_commissions = 0.0
        self.total_slippage_cost = 0.0
        self.commission_rate = max(float(commission_rate or 0.0), 0.0)
        self.slippage_bps = max(float(slippage_bps or 0.0), 0.0)

    def _slippage_multiplier(self) -> float:
        return self.slippage_bps / 10000.0

    def buy(self, symbol, shares, price, timestamp=None):
        """Buy shares."""
        if shares <= 0 or price <= 0:
            return False

        gross_notional = shares * price
        fill_price = price * (1 + self._slippage_multiplier())
        executed_notional = shares * fill_price
        slippage_cost = max(executed_notional - gross_notional, 0.0)
        commission = executed_notional * self.commission_rate
        total_cost = executed_notional + commission

        if total_cost <= self.cash:
            self.cash -= total_cost
            self.positions[symbol] = self.positions.get(symbol, 0) + shares
            self.last_prices[symbol] = price
            self.total_commissions += commission
            self.total_slippage_cost += slippage_cost
            self.trades.append(
                {
                    "timestamp": timestamp or datetime.now(),
                    "symbol": symbol,
                    "action": "BUY",
                    "shares": shares,
                    "price": fill_price,
                    "market_price": price,
                    "cost": total_cost,
                    "gross_notional": gross_notional,
                    "executed_notional": executed_notional,
                    "commission": commission,
                    "slippage_cost": slippage_cost,
                    "net_cash_flow": -total_cost,
                }
            )
            return True
        return False

    def sell(self, symbol, shares, price, timestamp=None):
        """Sell shares."""
        if shares <= 0 or price <= 0:
            return False

        if self.positions.get(symbol, 0) >= shares:
            gross_notional = shares * price
            fill_price = price * (1 - self._slippage_multiplier())
            executed_notional = shares * fill_price
            slippage_cost = max(gross_notional - executed_notional, 0.0)
            commission = executed_notional * self.commission_rate
            proceeds = executed_notional - commission
            self.cash += proceeds
            self.positions[symbol] -= shares
            self.last_prices[symbol] = price
            self.total_commissions += commission
            self.total_slippage_cost += slippage_cost
            if self.positions[symbol] == 0:
                del self.positions[symbol]
            self.trades.append(
                {
                    "timestamp": timestamp or datetime.now(),
                    "symbol": symbol,
                    "action": "SELL",
                    "shares": shares,
                    "price": fill_price,
                    "market_price": price,
                    "proceeds": proceeds,
                    "gross_notional": gross_notional,
                    "executed_notional": executed_notional,
                    "commission": commission,
                    "slippage_cost": slippage_cost,
                    "net_cash_flow": proceeds,
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
        frame = frame.sort_values("timestamp").reset_index(drop=True)
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


def calculate_sharpe_ratio(equity_curve: Iterable[dict[str, Any]] | pd.DataFrame, periods_per_year: int = TRADING_DAYS_PER_YEAR) -> float:
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
    periods_per_year: int = TRADING_DAYS_PER_YEAR,
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
        commission = float(trade.get("commission", 0) or 0)
        slippage_cost = float(trade.get("slippage_cost", 0) or 0)

        if not symbol or shares <= 0:
            continue

        if action == "BUY":
            cost_per_share = (shares * price + commission + slippage_cost) / shares
            open_lots[symbol].append({"shares": shares, "price": cost_per_share})
            continue

        if action != "SELL":
            continue

        remaining = shares
        realized_pnl = 0.0
        closed_any_quantity = False
        net_proceeds_per_share = max(shares * price - commission - slippage_cost, 0.0) / shares

        while remaining > 0 and open_lots[symbol]:
            lot = open_lots[symbol][0]
            matched_shares = min(remaining, lot["shares"])
            realized_pnl += matched_shares * (net_proceeds_per_share - lot["price"])
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


def build_buy_and_hold_benchmark_curve(
    market_data: pd.DataFrame,
    benchmark_symbol: str | None,
    initial_capital: float,
) -> list[dict[str, Any]] | None:
    """Build an actual benchmark curve from benchmark price bars."""
    if not benchmark_symbol or market_data.empty:
        return None

    if "symbol" not in market_data.columns or "close" not in market_data.columns or "timestamp" not in market_data.columns:
        return None

    benchmark_rows = market_data.loc[market_data["symbol"] == benchmark_symbol, ["timestamp", "close"]].copy()
    if benchmark_rows.empty:
        return None

    benchmark_rows["timestamp"] = pd.to_datetime(benchmark_rows["timestamp"])
    benchmark_rows["close"] = pd.to_numeric(benchmark_rows["close"], errors="coerce")
    benchmark_rows = benchmark_rows.dropna(subset=["close"]).sort_values("timestamp")
    benchmark_rows = benchmark_rows.drop_duplicates(subset=["timestamp"], keep="last")
    if benchmark_rows.empty:
        return None

    starting_price = float(benchmark_rows["close"].iloc[0])
    if starting_price <= 0:
        return None

    shares = initial_capital / starting_price
    curve: list[dict[str, Any]] = []
    previous_value = initial_capital

    for row in benchmark_rows.itertuples(index=False):
        value = shares * float(row.close)
        period_return = 0.0 if previous_value == 0 else (value - previous_value) / previous_value
        curve.append(
            {
                "timestamp": row.timestamp,
                "symbol": benchmark_symbol,
                "value": value,
                "price": float(row.close),
                "period_return": period_return,
                "returns": (value - initial_capital) / initial_capital * 100,
            }
        )
        previous_value = value

    return curve


def calculate_benchmark_metrics(
    equity_curve: Iterable[dict[str, Any]] | pd.DataFrame,
    benchmark_curve: Iterable[dict[str, Any]] | pd.DataFrame,
    periods_per_year: int = TRADING_DAYS_PER_YEAR,
    risk_free_rate: float = RISK_FREE_RATE,
) -> dict[str, Any]:
    """Calculate benchmark-relative metrics from actual strategy and benchmark curves."""
    strategy_frame = prepare_equity_curve_frame(equity_curve)
    benchmark_frame = prepare_equity_curve_frame(benchmark_curve)
    if len(strategy_frame) < 2 or len(benchmark_frame) < 2:
        return {}

    strategy_series = strategy_frame[["timestamp", "value"]].rename(columns={"value": "strategy_value"})
    benchmark_series = benchmark_frame[["timestamp", "value"]].rename(columns={"value": "benchmark_value"})
    aligned = strategy_series.merge(benchmark_series, on="timestamp", how="inner")
    if len(aligned) < 2:
        return {}

    aligned["strategy_return"] = aligned["strategy_value"].pct_change(fill_method=None)
    aligned["benchmark_return"] = aligned["benchmark_value"].pct_change(fill_method=None)
    aligned = aligned.dropna(subset=["strategy_return", "benchmark_return"]).reset_index(drop=True)
    if aligned.empty:
        return {}

    rf_daily = risk_free_rate / periods_per_year
    strategy_returns = aligned["strategy_return"]
    benchmark_returns = aligned["benchmark_return"]
    active_returns = strategy_returns - benchmark_returns

    benchmark_var = benchmark_returns.var(ddof=1)
    beta = None
    alpha = None
    if not pd.isna(benchmark_var) and benchmark_var > 0:
        beta = float(strategy_returns.cov(benchmark_returns) / benchmark_var)
        alpha_daily = (strategy_returns.mean() - rf_daily) - beta * (benchmark_returns.mean() - rf_daily)
        alpha = float(alpha_daily * periods_per_year * 100)

    active_std = active_returns.std(ddof=1)
    information_ratio = None
    tracking_error = None
    if not pd.isna(active_std) and active_std > 0:
        information_ratio = float(active_returns.mean() / active_std * np.sqrt(periods_per_year))
        tracking_error = float(active_std * np.sqrt(periods_per_year) * 100)

    strategy_annualized = calculate_annualized_return_pct(aligned.rename(columns={"strategy_value": "value"})[["timestamp", "value"]])
    benchmark_annualized = calculate_annualized_return_pct(aligned.rename(columns={"benchmark_value": "value"})[["timestamp", "value"]])
    strategy_total_return = (aligned["strategy_value"].iloc[-1] / aligned["strategy_value"].iloc[0] - 1) * 100
    benchmark_total_return = (aligned["benchmark_value"].iloc[-1] / aligned["benchmark_value"].iloc[0] - 1) * 100

    benchmark_symbol = None
    if "symbol" in benchmark_frame.columns and benchmark_frame["symbol"].notna().any():
        benchmark_symbol = str(benchmark_frame["symbol"].dropna().iloc[0])

    return {
        "benchmark_symbol": benchmark_symbol,
        "beta": beta,
        "alpha": alpha,
        "information_ratio": information_ratio,
        "tracking_error": tracking_error,
        "excess_return": float(strategy_annualized - benchmark_annualized),
        "benchmark_total_return": float(benchmark_total_return),
        "benchmark_annualized_return": float(benchmark_annualized),
        "strategy_total_return": float(strategy_total_return),
        "strategy_annualized_return": float(strategy_annualized),
        "observations": int(len(aligned)),
    }


def calculate_trading_cost_summary(
    trades: list[dict[str, Any]],
    initial_capital: float,
    final_value: float | None = None,
) -> dict[str, float]:
    """Summarize actual commissions, slippage drag, and turnover from trades."""
    total_commissions = float(sum(float(t.get("commission", 0) or 0) for t in trades))
    total_slippage_cost = float(sum(float(t.get("slippage_cost", 0) or 0) for t in trades))
    total_notional = float(sum(float(t.get("gross_notional", 0) or 0) for t in trades))
    total_cost = total_commissions + total_slippage_cost
    return {
        "total_commissions": total_commissions,
        "total_slippage_cost": total_slippage_cost,
        "total_cost_drag": total_cost,
        "cost_drag_pct_initial": (total_cost / initial_capital * 100) if initial_capital else 0.0,
        "turnover_multiple": (total_notional / initial_capital) if initial_capital else 0.0,
        "total_notional": total_notional,
        "ending_equity": float(final_value or 0.0),
    }


def calculate_symbol_attribution(
    trades: list[dict[str, Any]],
    final_positions: dict[str, float],
    latest_prices: dict[str, float],
    initial_capital: float,
) -> list[dict[str, Any]]:
    """Build real per-symbol attribution from trades and marked positions."""
    open_lots: dict[str, deque[dict[str, float]]] = defaultdict(deque)
    buckets: dict[str, dict[str, float]] = defaultdict(
        lambda: {
            "realized_pnl": 0.0,
            "unrealized_pnl": 0.0,
            "commissions": 0.0,
            "slippage_cost": 0.0,
            "gross_notional": 0.0,
        }
    )

    for trade in trades:
        symbol = trade.get("symbol")
        shares = float(trade.get("shares", 0) or 0)
        price = float(trade.get("price", 0) or 0)
        commission = float(trade.get("commission", 0) or 0)
        slippage_cost = float(trade.get("slippage_cost", 0) or 0)
        gross_notional = float(trade.get("gross_notional", shares * price) or 0)
        action = trade.get("action")
        if not symbol or shares <= 0 or price <= 0:
            continue

        bucket = buckets[symbol]
        bucket["commissions"] += commission
        bucket["slippage_cost"] += slippage_cost
        bucket["gross_notional"] += gross_notional

        if action == "BUY":
            total_cost = float(trade.get("cost", shares * price + commission + slippage_cost) or 0)
            open_lots[symbol].append(
                {
                    "shares": shares,
                    "cost_per_share": total_cost / shares,
                }
            )
            continue

        if action != "SELL":
            continue

        proceeds = float(trade.get("proceeds", shares * price - commission - slippage_cost) or 0)
        proceeds_per_share = proceeds / shares
        remaining = shares

        while remaining > 0 and open_lots[symbol]:
            lot = open_lots[symbol][0]
            matched_shares = min(remaining, lot["shares"])
            bucket["realized_pnl"] += matched_shares * (proceeds_per_share - lot["cost_per_share"])
            lot["shares"] -= matched_shares
            remaining -= matched_shares
            if lot["shares"] <= 0:
                open_lots[symbol].popleft()

    for symbol, lots in open_lots.items():
        current_price = float(latest_prices.get(symbol, 0) or 0)
        for lot in lots:
            if current_price <= 0:
                continue
            buckets[symbol]["unrealized_pnl"] += lot["shares"] * (current_price - lot["cost_per_share"])

    attribution: list[dict[str, Any]] = []
    symbols = set(buckets.keys()) | set(final_positions.keys())
    for symbol in symbols:
        bucket = buckets[symbol]
        ending_shares = float(final_positions.get(symbol, 0) or 0)
        ending_price = float(latest_prices.get(symbol, 0) or 0)
        total_pnl = bucket["realized_pnl"] + bucket["unrealized_pnl"]
        attribution.append(
            {
                "component": symbol,
                "realized_pnl": bucket["realized_pnl"],
                "unrealized_pnl": bucket["unrealized_pnl"],
                "total_pnl": total_pnl,
                "contribution_pct": (total_pnl / initial_capital * 100) if initial_capital else 0.0,
                "commissions": bucket["commissions"],
                "slippage_cost": bucket["slippage_cost"],
                "gross_notional": bucket["gross_notional"],
                "ending_shares": ending_shares,
                "ending_market_value": ending_shares * ending_price,
            }
        )

    attribution.sort(key=lambda row: abs(row["contribution_pct"]), reverse=True)
    return attribution


def build_tearsheet(results: dict[str, Any]) -> dict[str, Any]:
    """Assemble a reusable institutional-style tearsheet payload."""
    benchmark_metrics = results.get("benchmark_metrics") or {}
    cost_summary = results.get("cost_summary") or {}
    attribution = results.get("attribution") or []
    top_contributors = sorted(attribution, key=lambda row: row.get("contribution_pct", 0), reverse=True)[:5]
    biggest_detractors = sorted(attribution, key=lambda row: row.get("contribution_pct", 0))[:5]

    return {
        "generated_at": datetime.utcnow().isoformat() + "Z",
        "overview": {
            "initial_capital": results.get("initial_capital"),
            "final_value": results.get("final_value"),
            "total_return": results.get("total_return"),
            "annualized_return": results.get("annualized_return"),
            "sharpe_ratio": results.get("sharpe_ratio"),
            "max_drawdown": results.get("max_drawdown"),
            "total_trades": results.get("total_trades"),
        },
        "benchmark": benchmark_metrics,
        "costs": cost_summary,
        "top_contributors": top_contributors,
        "biggest_detractors": biggest_detractors,
    }


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
        commission_rate = float(config.get("commission", 0.0) or 0.0)
        slippage_bps = float(config.get("slippage_bps", config.get("slippage", 0.0)) or 0.0)
        portfolio = SimplePortfolio(
            config.get("initial_capital", 100000),
            commission_rate=commission_rate,
            slippage_bps=slippage_bps,
        )

        _queue_put(log_queue, f"Started backtest: {getattr(strategy, 'name', 'Unknown Strategy')}")
        _queue_put(log_queue, f"Initial capital: ${portfolio.initial_cash:,.2f}")
        _queue_put(log_queue, f"Execution costs: commission={commission_rate:.4f}, slippage={slippage_bps:.2f} bps")

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

            exposures = {}
            if portfolio_value > 0:
                exposures = {
                    symbol: (shares * latest_prices.get(symbol, 0)) / portfolio_value
                    for symbol, shares in portfolio.positions.items()
                    if latest_prices.get(symbol, 0) is not None
                }

            equity_curve.append(
                {
                    "timestamp": row["timestamp"],
                    "value": portfolio_value,
                    "cash": portfolio.cash,
                    "positions": sum(portfolio.positions.values()),
                    "position_value": position_value,
                    "returns": cumulative_return_pct,
                    "period_return": period_return,
                    "exposures": exposures,
                }
            )
            previous_value = portfolio_value

            progress = (i + 1) / total_bars
            _queue_put(progress_queue, progress)
            time.sleep(0.01)

        _queue_put(log_queue, "Backtest completed!")

        final_value = equity_curve[-1]["value"]
        total_return = (final_value - portfolio.initial_cash) / portfolio.initial_cash * 100
        annualized_return = calculate_annualized_return_pct(equity_curve)
        sharpe_ratio = calculate_sharpe_ratio(equity_curve)
        max_drawdown = calculate_max_drawdown(equity_curve) * 100
        win_rate = calculate_closed_trade_win_rate(portfolio.trades)

        benchmark_symbol = config.get("benchmark_symbol")
        benchmark_curve = build_buy_and_hold_benchmark_curve(market_data, benchmark_symbol, portfolio.initial_cash)
        benchmark_metrics = calculate_benchmark_metrics(equity_curve, benchmark_curve) if benchmark_curve else {}
        if benchmark_symbol and not benchmark_curve:
            _queue_put(log_queue, f"Benchmark symbol {benchmark_symbol} was not present in the loaded market data; benchmark metrics were skipped.")

        cost_summary = calculate_trading_cost_summary(portfolio.trades, portfolio.initial_cash, final_value)
        attribution = calculate_symbol_attribution(
            portfolio.trades,
            portfolio.positions,
            latest_prices,
            portfolio.initial_cash,
        )

        results = {
            "equity_curve": equity_curve,
            "benchmark_curve": benchmark_curve or [],
            "trades": portfolio.trades,
            "initial_capital": portfolio.initial_cash,
            "final_value": final_value,
            "total_return": total_return,
            "annualized_return": annualized_return,
            "sharpe_ratio": sharpe_ratio,
            "max_drawdown": max_drawdown,
            "total_trades": len(portfolio.trades),
            "final_cash": portfolio.cash,
            "final_positions": portfolio.positions,
            "logs": all_logs,
            "benchmark_symbol": benchmark_symbol,
            "benchmark_metrics": benchmark_metrics,
            "cost_summary": cost_summary,
            "attribution": attribution,
            "total_commissions": cost_summary["total_commissions"],
            "total_slippage_cost": cost_summary["total_slippage_cost"],
            "win_rate": win_rate,
        }
        results["metrics_summary"] = {
            "initial_capital": portfolio.initial_cash,
            "final_value": final_value,
            "total_return": total_return,
            "annualized_return": annualized_return,
            "sharpe_ratio": sharpe_ratio,
            "max_drawdown": max_drawdown,
            "total_trades": len(portfolio.trades),
            "win_rate": 0.0 if win_rate is None else win_rate,
            "total_commissions": cost_summary["total_commissions"],
            "total_slippage_cost": cost_summary["total_slippage_cost"],
            **benchmark_metrics,
        }
        results["tearsheet"] = build_tearsheet(results)
        return results
    except Exception as exc:
        _queue_put(log_queue, f"FATAL ERROR: {str(exc)}")
        return None
