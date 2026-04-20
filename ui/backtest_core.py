"""
Shared backtest execution and analytics helpers for the Streamlit UI.
"""

from __future__ import annotations

from collections import defaultdict, deque
from datetime import datetime
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import Any, Iterable
import math
import sys
import time

import numpy as np
import pandas as pd

ROOT_DIR = Path(__file__).resolve().parents[1]
if str(ROOT_DIR) not in sys.path:
    sys.path.insert(0, str(ROOT_DIR))

from glowback_runtime import normalize_strategy_name, run_backtest as run_engine_backtest


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


def _coerce_float(value: Any, default: float = 0.0) -> float:
    try:
        return float(value)
    except (TypeError, ValueError):
        return default


def _round_share_quantity(shares: float) -> float:
    if shares <= 0:
        return 0.0
    return math.floor(shares * 1_000_000) / 1_000_000


def _calculate_weight_map(
    positions: dict[str, float],
    latest_prices: dict[str, float],
    portfolio_value: float,
) -> dict[str, float]:
    if portfolio_value <= 0:
        return {}

    return {
        symbol: float(shares * latest_prices.get(symbol, 0.0) / portfolio_value)
        for symbol, shares in positions.items()
        if latest_prices.get(symbol, 0.0) > 0 and shares != 0
    }


def _normalize_portfolio_construction_config(
    raw_config: dict[str, Any] | None,
    market_data: pd.DataFrame,
) -> dict[str, Any] | None:
    if not raw_config or not raw_config.get("enabled"):
        return None

    available_symbols = {
        str(symbol).strip().upper()
        for symbol in market_data.get("symbol", pd.Series(dtype=str)).dropna().tolist()
    }
    cleaned_weights: dict[str, float] = {}
    validation_errors: list[str] = []
    for raw_symbol, raw_weight in dict(raw_config.get("target_weights") or {}).items():
        symbol = str(raw_symbol).strip().upper()
        weight = _coerce_float(raw_weight)
        if not symbol or weight <= 0:
            continue
        if available_symbols and symbol not in available_symbols:
            validation_errors.append(f"Target weight symbol {symbol} is not present in the loaded market data.")
            continue
        cleaned_weights[symbol] = weight

    total_weight = sum(cleaned_weights.values())
    if total_weight <= 0:
        return {
            "enabled": True,
            "validation_errors": validation_errors or ["Portfolio construction requires at least one positive target weight."],
            "target_weights": {},
        }

    normalized = {symbol: weight / total_weight for symbol, weight in cleaned_weights.items()}
    cash_floor_fraction = min(max(_coerce_float(raw_config.get("cash_floor_pct")) / 100.0, 0.0), 0.95)
    investable_fraction = max(0.0, 1.0 - cash_floor_fraction)
    max_weight_fraction = None
    if raw_config.get("max_weight_pct") not in (None, ""):
        max_weight_fraction = min(max(_coerce_float(raw_config.get("max_weight_pct")) / 100.0, 0.0), 1.0)

    target_weights: dict[str, float] = {}
    upfront_constraint_hits: list[dict[str, Any]] = []
    for symbol, weight in normalized.items():
        scaled_weight = weight * investable_fraction
        if max_weight_fraction is not None and scaled_weight > max_weight_fraction:
            upfront_constraint_hits.append(
                {
                    "type": "max_weight_cap",
                    "symbol": symbol,
                    "requested_weight_pct": round(scaled_weight * 100, 2),
                    "applied_weight_pct": round(max_weight_fraction * 100, 2),
                }
            )
            scaled_weight = max_weight_fraction
        target_weights[symbol] = scaled_weight

    return {
        "enabled": True,
        "method": "target_weights",
        "target_weights": target_weights,
        "rebalance_frequency": str(raw_config.get("rebalance_frequency") or "weekly").lower(),
        "drift_threshold": min(max(_coerce_float(raw_config.get("drift_threshold_pct")) / 100.0, 0.0), 1.0),
        "max_weight": max_weight_fraction,
        "max_turnover": min(max(_coerce_float(raw_config.get("max_turnover_pct")) / 100.0, 0.0), 5.0),
        "cash_floor": cash_floor_fraction,
        "max_drawdown": min(max(_coerce_float(raw_config.get("max_drawdown_pct")) / 100.0, 0.0), 1.0),
        "validation_errors": validation_errors,
        "upfront_constraint_hits": upfront_constraint_hits,
        "target_weights_pct": {symbol: round(weight * 100, 2) for symbol, weight in target_weights.items()},
    }


def _rebalance_due(
    last_rebalance_at: pd.Timestamp | None,
    current_timestamp: pd.Timestamp,
    frequency: str,
) -> bool:
    if last_rebalance_at is None:
        return True
    if frequency == "daily":
        return current_timestamp.normalize() > last_rebalance_at.normalize()
    if frequency == "monthly":
        return current_timestamp.to_period("M") != last_rebalance_at.to_period("M")
    return current_timestamp.to_period("W") != last_rebalance_at.to_period("W")


def _apply_target_weight_rebalance(
    portfolio: SimplePortfolio,
    latest_prices: dict[str, float],
    target_weights: dict[str, float],
    timestamp: pd.Timestamp,
    max_turnover: float | None,
    log_queue,
) -> tuple[float, list[dict[str, Any]]]:
    portfolio_value = portfolio.calculate_value(latest_prices)
    if portfolio_value <= 0:
        return 0.0, []

    desired_shares: dict[str, float] = {}
    for symbol, target_weight in target_weights.items():
        price = _coerce_float(latest_prices.get(symbol))
        if price <= 0:
            continue
        desired_shares[symbol] = (portfolio_value * target_weight) / price

    symbols = set(portfolio.positions) | set(desired_shares)
    planned_deltas: dict[str, float] = {
        symbol: desired_shares.get(symbol, 0.0) - _coerce_float(portfolio.positions.get(symbol))
        for symbol in symbols
    }
    planned_turnover_notional = sum(
        abs(delta) * _coerce_float(latest_prices.get(symbol))
        for symbol, delta in planned_deltas.items()
        if _coerce_float(latest_prices.get(symbol)) > 0
    )

    scale = 1.0
    constraint_hits: list[dict[str, Any]] = []
    if max_turnover and planned_turnover_notional > 0:
        allowed_turnover = portfolio_value * max_turnover
        if planned_turnover_notional > allowed_turnover:
            scale = allowed_turnover / planned_turnover_notional
            constraint_hits.append(
                {
                    "type": "turnover_cap",
                    "requested_turnover_pct": round(planned_turnover_notional / portfolio_value * 100, 2),
                    "applied_turnover_pct": round(max_turnover * 100, 2),
                }
            )

    trade_start_index = len(portfolio.trades)

    for symbol in sorted(symbols):
        delta = planned_deltas.get(symbol, 0.0)
        price = _coerce_float(latest_prices.get(symbol))
        if delta >= 0 or price <= 0:
            continue
        shares_to_sell = min(_round_share_quantity(abs(delta) * scale), _coerce_float(portfolio.positions.get(symbol)))
        if shares_to_sell > 0:
            portfolio.sell(symbol, shares_to_sell, price, timestamp)
            _queue_put(log_queue, f"{timestamp:%Y-%m-%d}: Rebalanced SELL {shares_to_sell:.4f} {symbol} @ {price:.2f}")

    for symbol in sorted(symbols):
        delta = planned_deltas.get(symbol, 0.0)
        price = _coerce_float(latest_prices.get(symbol))
        if delta <= 0 or price <= 0:
            continue
        target_shares = _round_share_quantity(delta * scale)
        if target_shares <= 0:
            continue
        per_share_cost = price * (1 + portfolio._slippage_multiplier()) * (1 + portfolio.commission_rate)
        affordable_shares = portfolio.cash / per_share_cost if per_share_cost > 0 else 0.0
        shares_to_buy = _round_share_quantity(min(target_shares, affordable_shares))
        if shares_to_buy > 0:
            portfolio.buy(symbol, shares_to_buy, price, timestamp)
            _queue_put(log_queue, f"{timestamp:%Y-%m-%d}: Rebalanced BUY {shares_to_buy:.4f} {symbol} @ {price:.2f}")

    actual_turnover_notional = sum(
        _coerce_float(trade.get("gross_notional"))
        for trade in portfolio.trades[trade_start_index:]
    )
    turnover_pct = actual_turnover_notional / portfolio_value * 100 if portfolio_value > 0 else 0.0
    return turnover_pct, constraint_hits


def _finalize_backtest_results(
    market_data: pd.DataFrame,
    portfolio: SimplePortfolio,
    equity_curve: list[dict[str, Any]],
    latest_prices: dict[str, float],
    all_logs: list[str],
    benchmark_symbol: str | None,
    portfolio_construction: dict[str, Any] | None = None,
    portfolio_diagnostics: list[dict[str, Any]] | None = None,
    constraint_hits: list[dict[str, Any]] | None = None,
    log_queue=None,
) -> dict[str, Any]:
    final_value = equity_curve[-1]["value"]
    total_return = (final_value - portfolio.initial_cash) / portfolio.initial_cash * 100
    annualized_return = calculate_annualized_return_pct(equity_curve)
    sharpe_ratio = calculate_sharpe_ratio(equity_curve)
    max_drawdown = calculate_max_drawdown(equity_curve) * 100
    win_rate = calculate_closed_trade_win_rate(portfolio.trades)

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

    diagnostics = portfolio_diagnostics or []
    constraint_events = constraint_hits or []
    portfolio_metrics = {}
    if diagnostics:
        rebalances = [row for row in diagnostics if row.get("rebalanced")]
        portfolio_metrics = {
            "portfolio_rebalances": float(len(rebalances)),
            "average_turnover_pct": float(np.mean([row.get("turnover_pct", 0.0) for row in diagnostics])) if diagnostics else 0.0,
            "max_weight_drift_pct": float(max((row.get("max_abs_drift_pct", 0.0) for row in diagnostics), default=0.0)),
            "constraint_hit_count": float(len(constraint_events)),
        }

    results = {
        "equity_curve": equity_curve,
        "benchmark_curve": benchmark_curve or [],
        "trades": portfolio.trades,
        "exposures": diagnostics,
        "portfolio_construction": portfolio_construction or {},
        "portfolio_diagnostics": diagnostics,
        "constraint_hits": constraint_events,
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
        **portfolio_metrics,
        **benchmark_metrics,
    }
    results["tearsheet"] = build_tearsheet(results)
    return results


def _run_portfolio_construction_backtest(market_data, config, portfolio_config, progress_queue, log_queue):
    commission_rate = float(config.get("commission", 0.0) or 0.0)
    slippage_bps = float(config.get("slippage_bps", config.get("slippage", 0.0)) or 0.0)
    portfolio = SimplePortfolio(
        config.get("initial_capital", 100000),
        commission_rate=commission_rate,
        slippage_bps=slippage_bps,
    )

    _queue_put(log_queue, "Started backtest: Portfolio Construction (target weights)")
    _queue_put(log_queue, f"Initial capital: ${portfolio.initial_cash:,.2f}")
    _queue_put(log_queue, f"Execution costs: commission={commission_rate:.4f}, slippage={slippage_bps:.2f} bps")

    validation_errors = portfolio_config.get("validation_errors") or []
    if not portfolio_config.get("target_weights"):
        for error in validation_errors or ["Portfolio construction requires at least one valid target weight."]:
            _queue_put(log_queue, f"ERROR: {error}")
        return None

    all_logs: list[str] = []
    for warning in validation_errors:
        _queue_put(log_queue, f"WARNING: {warning}")
        all_logs.append(f"WARNING: {warning}")
    latest_prices: dict[str, float] = {}
    equity_curve: list[dict[str, Any]] = []
    portfolio_diagnostics: list[dict[str, Any]] = []
    constraint_hits: list[dict[str, Any]] = []
    previous_value = float(portfolio.initial_cash)
    last_rebalance_at: pd.Timestamp | None = None
    peak_value = float(portfolio.initial_cash)

    for hit in portfolio_config.get("upfront_constraint_hits") or []:
        stamped_hit = {"timestamp": pd.Timestamp(market_data["timestamp"].min()), **hit}
        constraint_hits.append(stamped_hit)

    grouped = list(market_data.sort_values(["timestamp", "symbol"]).groupby("timestamp", sort=True))
    total_steps = len(grouped)
    if total_steps == 0:
        _queue_put(log_queue, "ERROR: No market data available for portfolio construction backtest")
        return None

    for index, (timestamp, rows) in enumerate(grouped, start=1):
        current_timestamp = pd.Timestamp(timestamp)
        for row in rows.itertuples(index=False):
            latest_prices[str(row.symbol)] = float(row.close)

        portfolio_value_before = portfolio.calculate_value(latest_prices)
        peak_value = max(peak_value, portfolio_value_before)
        drawdown_pct = ((peak_value - portfolio_value_before) / peak_value * 100) if peak_value > 0 else 0.0
        current_weights = _calculate_weight_map(portfolio.positions, latest_prices, portfolio_value_before)
        drift_map = {
            symbol: current_weights.get(symbol, 0.0) - portfolio_config["target_weights"].get(symbol, 0.0)
            for symbol in set(current_weights) | set(portfolio_config["target_weights"])
        }
        max_abs_drift_pct = max((abs(weight) * 100 for weight in drift_map.values()), default=0.0)

        rebalance_reason = None
        effective_target_weights = dict(portfolio_config["target_weights"])
        step_constraint_hits: list[dict[str, Any]] = []
        max_drawdown_fraction = portfolio_config.get("max_drawdown") or 0.0
        if max_drawdown_fraction and drawdown_pct >= max_drawdown_fraction * 100 and portfolio.positions:
            effective_target_weights = {}
            rebalance_reason = "drawdown_guard"
            step_constraint_hits.append(
                {
                    "timestamp": current_timestamp,
                    "type": "max_drawdown_guard",
                    "observed_drawdown_pct": round(drawdown_pct, 2),
                    "limit_pct": round(max_drawdown_fraction * 100, 2),
                }
            )
        elif _rebalance_due(last_rebalance_at, current_timestamp, portfolio_config.get("rebalance_frequency", "weekly")):
            rebalance_reason = "initial_allocation" if last_rebalance_at is None else f"{portfolio_config.get('rebalance_frequency', 'weekly')}_schedule"
        elif portfolio_config.get("drift_threshold") and max_abs_drift_pct >= portfolio_config["drift_threshold"] * 100:
            rebalance_reason = "drift_threshold"

        turnover_pct = 0.0
        if rebalance_reason is not None:
            turnover_pct, turnover_hits = _apply_target_weight_rebalance(
                portfolio,
                latest_prices,
                effective_target_weights,
                current_timestamp,
                portfolio_config.get("max_turnover"),
                log_queue,
            )
            step_constraint_hits.extend({"timestamp": current_timestamp, **hit} for hit in turnover_hits)
            last_rebalance_at = current_timestamp

        portfolio_value_after = portfolio.calculate_value(latest_prices)
        peak_value = max(peak_value, portfolio_value_after)
        realized_weights = _calculate_weight_map(portfolio.positions, latest_prices, portfolio_value_after)
        realized_drift_map = {
            symbol: realized_weights.get(symbol, 0.0) - portfolio_config["target_weights"].get(symbol, 0.0)
            for symbol in set(realized_weights) | set(portfolio_config["target_weights"])
        }
        realized_max_abs_drift_pct = max((abs(weight) * 100 for weight in realized_drift_map.values()), default=0.0)
        position_value = portfolio.calculate_position_value(latest_prices)
        period_return = 0.0 if previous_value == 0 else (portfolio_value_after - previous_value) / previous_value
        cumulative_return_pct = (
            0.0
            if portfolio.initial_cash == 0
            else (portfolio_value_after - portfolio.initial_cash) / portfolio.initial_cash * 100
        )

        equity_curve.append(
            {
                "timestamp": current_timestamp,
                "value": portfolio_value_after,
                "cash": portfolio.cash,
                "positions": sum(portfolio.positions.values()),
                "position_value": position_value,
                "returns": cumulative_return_pct,
                "period_return": period_return,
                "exposures": realized_weights,
            }
        )
        previous_value = portfolio_value_after

        portfolio_diagnostics.append(
            {
                "timestamp": current_timestamp,
                "portfolio_value": portfolio_value_after,
                "target_weights": {symbol: round(weight * 100, 2) for symbol, weight in portfolio_config["target_weights"].items()},
                "realized_weights": {symbol: round(weight * 100, 2) for symbol, weight in realized_weights.items()},
                "drift_by_symbol_pct": {symbol: round(weight * 100, 2) for symbol, weight in realized_drift_map.items()},
                "max_abs_drift_pct": round(realized_max_abs_drift_pct, 2),
                "turnover_pct": round(turnover_pct, 2),
                "rebalanced": rebalance_reason is not None,
                "rebalance_reason": rebalance_reason,
                "cash_weight_pct": round((portfolio.cash / portfolio_value_after * 100) if portfolio_value_after > 0 else 0.0, 2),
                "drawdown_pct": round(((peak_value - portfolio_value_after) / peak_value * 100) if peak_value > 0 else 0.0, 2),
                "constraint_hits": step_constraint_hits,
            }
        )
        constraint_hits.extend(step_constraint_hits)

        progress = index / total_steps
        _queue_put(progress_queue, progress)
        time.sleep(0.01)

    _queue_put(log_queue, "Backtest completed!")
    all_logs.extend([f"Portfolio construction method: {portfolio_config.get('method', 'target_weights')}" if portfolio_config else ""])
    benchmark_symbol = config.get("benchmark_symbol")
    portfolio_summary = {
        "method": portfolio_config.get("method", "target_weights"),
        "rebalance_frequency": portfolio_config.get("rebalance_frequency", "weekly"),
        "target_weights": portfolio_config.get("target_weights_pct") or {symbol: round(weight * 100, 2) for symbol, weight in portfolio_config.get("target_weights", {}).items()},
        "cash_floor_pct": round((portfolio_config.get("cash_floor") or 0.0) * 100, 2),
        "max_weight_pct": None if portfolio_config.get("max_weight") is None else round(portfolio_config["max_weight"] * 100, 2),
        "max_turnover_pct": None if not portfolio_config.get("max_turnover") else round(portfolio_config["max_turnover"] * 100, 2),
        "drift_threshold_pct": None if not portfolio_config.get("drift_threshold") else round(portfolio_config["drift_threshold"] * 100, 2),
        "max_drawdown_pct": None if not portfolio_config.get("max_drawdown") else round(portfolio_config["max_drawdown"] * 100, 2),
    }
    return _finalize_backtest_results(
        market_data,
        portfolio,
        equity_curve,
        latest_prices,
        [line for line in all_logs if line],
        benchmark_symbol,
        portfolio_construction=portfolio_summary,
        portfolio_diagnostics=portfolio_diagnostics,
        constraint_hits=constraint_hits,
        log_queue=log_queue,
    )


def build_tearsheet(results: dict[str, Any]) -> dict[str, Any]:
    """Assemble a reusable institutional-style tearsheet payload."""
    benchmark_metrics = results.get("benchmark_metrics") or {}
    cost_summary = results.get("cost_summary") or {}
    attribution = results.get("attribution") or []
    portfolio_summary = results.get("portfolio_construction") or {}
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
        "portfolio": portfolio_summary,
        "costs": cost_summary,
        "top_contributors": top_contributors,
        "biggest_detractors": biggest_detractors,
    }


def _queue_put(queue_obj, value):
    if queue_obj is not None:
        queue_obj.put(value)


def _detect_strategy_name(strategy_code: str, config: dict[str, Any]) -> str:
    configured = config.get("strategy_type") or config.get("strategy_template") or config.get("name")
    if configured:
        try:
            return normalize_strategy_name(configured)
        except ValueError:
            pass

    normalized_code = (strategy_code or "").lower()
    if "movingaveragecrossover" in normalized_code or "ma crossover" in normalized_code:
        return "ma_crossover"
    if "meanreversionstrategy" in normalized_code or "mean reversion" in normalized_code:
        return "mean_reversion"
    if "momentumstrategy" in normalized_code or "momentum" in normalized_code:
        return "momentum"
    if "rsistrategy" in normalized_code or "rsi" in normalized_code:
        return "rsi"
    if "buyandhold" in normalized_code or "buy and hold" in normalized_code:
        return "buy_and_hold"

    raise ValueError(
        "The real engine-backed runner supports built-in strategies only: buy_and_hold, ma_crossover, momentum, mean_reversion, rsi."
    )


def _strategy_params_for_engine(strategy_name: str, config: dict[str, Any]) -> dict[str, Any]:
    params = dict(config.get("strategy_params") or {})

    if strategy_name == "ma_crossover":
        params.setdefault("short_period", int(config.get("short_period", 10)))
        params.setdefault("long_period", int(config.get("long_period", 20)))
    elif strategy_name == "momentum":
        params.setdefault("lookback_period", int(config.get("lookback_period", 10)))
        params.setdefault("momentum_threshold", float(config.get("momentum_threshold", 0.05)))
    elif strategy_name == "mean_reversion":
        params.setdefault("lookback_period", int(config.get("lookback_period", 20)))
        params.setdefault("entry_threshold", float(config.get("entry_threshold", 2.0)))
        params.setdefault("exit_threshold", float(config.get("exit_threshold", 1.0)))
    elif strategy_name == "rsi":
        params.setdefault("lookback_period", int(config.get("lookback_period", 14)))
        params.setdefault("oversold_threshold", float(config.get("oversold_threshold", 30.0)))
        params.setdefault("overbought_threshold", float(config.get("overbought_threshold", 70.0)))

    return params


def _write_market_data_bundle(temp_dir: str, market_data: pd.DataFrame, resolution: str) -> list[str]:
    frame = market_data.copy()
    frame["timestamp"] = pd.to_datetime(frame["timestamp"], utc=True)
    resolution_suffix = {"day": "1d", "hour": "1h", "minute": "1m"}.get(resolution, resolution)
    symbols: list[str] = []

    for symbol, group in frame.groupby("symbol"):
        output = group.sort_values("timestamp")[["timestamp", "open", "high", "low", "close", "volume"]].copy()
        output["timestamp"] = output["timestamp"].dt.strftime("%Y-%m-%dT%H:%M:%SZ")
        output.to_csv(Path(temp_dir) / f"{symbol}_{resolution_suffix}.csv", index=False)
        symbols.append(symbol)

    return symbols


def _latest_prices_from_market_data(market_data: pd.DataFrame) -> dict[str, float]:
    if market_data is None or market_data.empty:
        return {}

    ordered = market_data.sort_values(["timestamp", "symbol"])
    latest: dict[str, float] = {}
    for row in ordered.itertuples(index=False):
        latest[str(row.symbol)] = float(row.close)
    return latest


def _enrich_real_engine_results(
    result: dict[str, Any],
    market_data: pd.DataFrame,
    benchmark_symbol: str | None,
    log_queue=None,
) -> dict[str, Any]:
    enriched = dict(result)
    enriched["logs"] = list(enriched.get("logs") or [])
    enriched["trades"] = list(enriched.get("trades") or [])
    enriched["exposures"] = list(enriched.get("exposures") or [])
    enriched["final_positions"] = dict(enriched.get("final_positions") or {})

    metrics_summary = dict(enriched.get("metrics_summary") or {})
    initial_capital = float(enriched.get("initial_capital") or metrics_summary.get("initial_capital") or 0.0)
    final_value = float(enriched.get("final_value") or metrics_summary.get("final_value") or 0.0)
    total_return = float(enriched.get("total_return") or metrics_summary.get("total_return") or 0.0)
    sharpe_ratio = float(enriched.get("sharpe_ratio") or metrics_summary.get("sharpe_ratio") or 0.0)
    max_drawdown = float(enriched.get("max_drawdown") or metrics_summary.get("max_drawdown") or 0.0)
    total_trades = float(enriched.get("total_trades") or metrics_summary.get("total_trades") or len(enriched["trades"]))

    latest_prices = _latest_prices_from_market_data(market_data)
    benchmark_name = (benchmark_symbol or enriched.get("benchmark_symbol") or None)
    benchmark_curve = list(enriched.get("benchmark_curve") or [])
    if not benchmark_curve and benchmark_name:
        benchmark_curve = build_buy_and_hold_benchmark_curve(market_data, benchmark_name, initial_capital) or []
        if benchmark_name and not benchmark_curve:
            message = f"Benchmark symbol {benchmark_name} was not present in the loaded market data; benchmark metrics were skipped."
            enriched["logs"].append(message)
            _queue_put(log_queue, message)

    benchmark_metrics = dict(enriched.get("benchmark_metrics") or {})
    if not benchmark_metrics and benchmark_curve:
        benchmark_metrics = calculate_benchmark_metrics(enriched.get("equity_curve") or [], benchmark_curve)

    cost_summary = dict(enriched.get("cost_summary") or {})
    if not cost_summary:
        cost_summary = calculate_trading_cost_summary(enriched["trades"], initial_capital, final_value)

    attribution = list(enriched.get("attribution") or [])
    if not attribution:
        attribution = calculate_symbol_attribution(
            enriched["trades"],
            enriched["final_positions"],
            latest_prices,
            initial_capital,
        )

    enriched["initial_capital"] = initial_capital
    enriched["final_value"] = final_value
    enriched["total_return"] = total_return
    enriched["sharpe_ratio"] = sharpe_ratio
    enriched["max_drawdown"] = max_drawdown
    enriched["total_trades"] = total_trades
    enriched["benchmark_symbol"] = benchmark_name
    enriched["benchmark_curve"] = benchmark_curve
    enriched["benchmark_metrics"] = benchmark_metrics
    enriched["cost_summary"] = cost_summary
    enriched["attribution"] = attribution
    enriched.setdefault("portfolio_construction", {})
    enriched.setdefault("portfolio_diagnostics", [])
    enriched.setdefault("constraint_hits", [])

    metrics_summary.setdefault("initial_capital", initial_capital)
    metrics_summary.setdefault("final_value", final_value)
    metrics_summary.setdefault("total_return", total_return)
    metrics_summary.setdefault("sharpe_ratio", sharpe_ratio)
    metrics_summary.setdefault("max_drawdown", max_drawdown)
    metrics_summary.setdefault("total_trades", total_trades)
    metrics_summary["total_commissions"] = cost_summary.get("total_commissions", 0.0)
    metrics_summary["total_slippage_cost"] = cost_summary.get("total_slippage_cost", 0.0)
    metrics_summary.update(benchmark_metrics)
    enriched["metrics_summary"] = metrics_summary
    enriched["tearsheet"] = build_tearsheet(enriched)
    return enriched


def _run_real_engine_backtest(strategy_name, strategy_code, market_data, config, progress_queue, log_queue):
    if market_data is None or market_data.empty:
        raise ValueError("No market data available for the real engine runner")

    strategy_params = _strategy_params_for_engine(strategy_name, config)
    resolution = str(
        config.get("resolution")
        or (market_data["resolution"].iloc[0] if "resolution" in market_data.columns and not market_data.empty else "day")
        or "day"
    )
    normalized_resolution = {"1d": "day", "1h": "hour", "1m": "minute", "5m": "minute"}.get(
        resolution,
        resolution,
    )

    start_date = pd.to_datetime(config.get("start_date") or market_data["timestamp"].min(), utc=True)
    end_date = pd.to_datetime(config.get("end_date") or market_data["timestamp"].max(), utc=True)

    filtered = market_data.copy()
    filtered["timestamp"] = pd.to_datetime(filtered["timestamp"], utc=True)
    filtered = filtered[(filtered["timestamp"] >= start_date) & (filtered["timestamp"] <= end_date)]
    if filtered.empty:
        raise ValueError("No market data remains after applying the selected date range")

    commission_bps = config.get("commission_bps")
    if commission_bps is None and config.get("commission") is not None:
        commission_bps = float(config.get("commission") or 0.0) * 10000
    slippage_bps = config.get("slippage_bps")
    if slippage_bps is None and config.get("slippage") is not None:
        slippage_bps = float(config.get("slippage") or 0.0)

    _queue_put(log_queue, f"Using real engine strategy: {strategy_name}")
    _queue_put(log_queue, f"Loaded {len(filtered)} bars across {filtered['symbol'].nunique()} symbols")
    _queue_put(progress_queue, 0.1)

    with TemporaryDirectory(prefix="glowback_ui_") as temp_dir:
        symbols = _write_market_data_bundle(temp_dir, filtered, normalized_resolution)
        _queue_put(log_queue, f"Prepared CSV bundle for symbols: {', '.join(symbols)}")
        _queue_put(progress_queue, 0.35)

        result = run_engine_backtest(
            symbols=symbols,
            start_date=start_date.to_pydatetime(),
            end_date=end_date.to_pydatetime(),
            resolution=normalized_resolution,
            strategy_name=strategy_name,
            strategy_params=strategy_params,
            initial_capital=float(config.get("initial_capital", 100000.0)),
            run_name=config.get("display_name") or config.get("name") or "UI Backtest",
            commission_bps=float(commission_bps) if commission_bps is not None else None,
            slippage_bps=float(slippage_bps) if slippage_bps is not None else None,
            latency_ms=int(config["latency_ms"]) if config.get("latency_ms") is not None else None,
            data_source="csv",
            csv_data_path=temp_dir,
        )

    _queue_put(progress_queue, 1.0)
    for line in result.get("logs", []):
        _queue_put(log_queue, line)
    return _enrich_real_engine_results(result, filtered, config.get("benchmark_symbol"), log_queue=log_queue)


def _run_local_strategy_backtest(strategy_code, market_data, config, progress_queue, log_queue):
    namespace = {}
    exec(strategy_code, namespace)

    strategy_classes = [obj for obj in namespace.values() if isinstance(obj, type) and hasattr(obj, "on_bar")]
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
            0.0 if portfolio.initial_cash == 0 else (portfolio_value - portfolio.initial_cash) / portfolio.initial_cash * 100
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

    return _finalize_backtest_results(
        market_data,
        portfolio,
        equity_curve,
        latest_prices,
        all_logs,
        config.get("benchmark_symbol"),
        log_queue=log_queue,
    )


def run_backtest(strategy_code, market_data, config, progress_queue, log_queue):
    """Run a UI backtest using portfolio mode, the real engine, or the local Python runner."""
    try:
        portfolio_config = _normalize_portfolio_construction_config(config.get("portfolio_construction"), market_data)
        if portfolio_config is not None:
            return _run_portfolio_construction_backtest(market_data, config, portfolio_config, progress_queue, log_queue)

        try:
            strategy_name = _detect_strategy_name(strategy_code, config)
        except ValueError:
            _queue_put(log_queue, "Falling back to the local Python strategy runner for custom strategy code.")
            return _run_local_strategy_backtest(strategy_code, market_data, config, progress_queue, log_queue)

        return _run_real_engine_backtest(strategy_name, strategy_code, market_data, config, progress_queue, log_queue)
    except Exception as exc:
        _queue_put(log_queue, f"FATAL ERROR: {str(exc)}")
        return None
