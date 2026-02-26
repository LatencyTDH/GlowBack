"""
Advanced Analytics Page - Heatmaps, rolling statistics & interactive features
Implements enhancements from issue #47.
"""

import streamlit as st
import pandas as pd
import plotly.graph_objects as go
import plotly.express as px
from plotly.subplots import make_subplots
import numpy as np
from typing import Optional, Dict, List
import io
import json
from datetime import datetime


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _equity_df(results: dict) -> Optional[pd.DataFrame]:
    """Return equity curve as a DatetimeIndex DataFrame, or None."""
    raw = results.get("equity_curve", [])
    if not raw:
        return None
    df = pd.DataFrame(raw)
    if "timestamp" not in df.columns or "value" not in df.columns:
        return None
    df["timestamp"] = pd.to_datetime(df["timestamp"])
    df = df.sort_values("timestamp").set_index("timestamp")
    return df


def _daily_returns(eq: pd.DataFrame) -> pd.Series:
    """Compute daily simple returns from an equity DataFrame."""
    return eq["value"].pct_change().dropna()


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------

def show():
    """Main advanced analytics page."""
    st.title("🔬 Advanced Analytics")
    st.markdown(
        "Deep-dive analytics: heatmaps, rolling statistics, run comparison, "
        "and parameter sensitivity."
    )

    results = st.session_state.get("backtest_results")
    if not results:
        st.warning("⚠️ No backtest results available. Run a backtest first.")
        return

    eq = _equity_df(results)
    if eq is None or len(eq) < 2:
        st.error("❌ Equity curve data is missing or too short for analytics.")
        return

    tab_heatmaps, tab_rolling, tab_compare, tab_sensitivity, tab_export = st.tabs(
        [
            "🗓️ Heatmaps",
            "📈 Rolling Statistics",
            "🔀 Compare Runs",
            "🎛️ Sensitivity",
            "📥 Export",
        ]
    )

    with tab_heatmaps:
        _show_heatmaps(eq, results)

    with tab_rolling:
        _show_rolling_statistics(eq)

    with tab_compare:
        _show_compare_runs()

    with tab_sensitivity:
        _show_sensitivity()

    with tab_export:
        _show_export(eq, results)


# ---------------------------------------------------------------------------
# Heatmaps
# ---------------------------------------------------------------------------

def _show_heatmaps(eq: pd.DataFrame, results: dict):
    st.subheader("🗓️ Heatmaps")

    htab1, htab2, htab3 = st.tabs(
        ["Monthly Returns", "Correlation Matrix", "Drawdown Heatmap"]
    )

    with htab1:
        _monthly_returns_heatmap(eq)
    with htab2:
        _correlation_heatmap(results)
    with htab3:
        _drawdown_heatmap(eq)


def _monthly_returns_heatmap(eq: pd.DataFrame):
    """Calendar-grid monthly returns (month × year) with colour-coded cells."""
    st.markdown("**📅 Monthly Returns Heatmap**")

    if len(eq) < 30:
        st.info("Need ≥ 30 days of data for a meaningful monthly heatmap.")
        return

    monthly = eq["value"].resample("ME").last()
    monthly_ret = monthly.pct_change().dropna() * 100

    if monthly_ret.empty:
        st.info("Not enough monthly data points.")
        return

    df = monthly_ret.to_frame("return")
    df["year"] = df.index.year
    df["month"] = df.index.month

    month_names = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun",
        "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ]

    pivot = df.pivot_table(values="return", index="year", columns="month", aggfunc="first")
    pivot = pivot.reindex(columns=range(1, 13))

    fig = go.Figure(
        data=go.Heatmap(
            z=pivot.values,
            x=[month_names[m - 1] for m in pivot.columns],
            y=[str(y) for y in pivot.index],
            colorscale="RdYlGn",
            zmid=0,
            text=np.where(
                np.isnan(pivot.values),
                "",
                np.vectorize(lambda v: f"{v:.1f}%")(pivot.values),
            ),
            texttemplate="%{text}",
            hovertemplate="Year %{y}, %{x}: %{z:.2f}%<extra></extra>",
        )
    )
    fig.update_layout(
        title="Monthly Returns (%)",
        height=max(250, 80 * len(pivot)),
        xaxis_title="Month",
        yaxis_title="Year",
        yaxis=dict(autorange="reversed"),
    )
    st.plotly_chart(fig, use_container_width=True)

    # Yearly summary row
    yearly = eq["value"].resample("YE").last().pct_change().dropna() * 100
    if not yearly.empty:
        st.markdown("**Annual Returns**")
        yr_df = pd.DataFrame(
            {"Year": yearly.index.year, "Return (%)": yearly.values}
        )
        st.dataframe(yr_df.style.format({"Return (%)": "{:.2f}%"}), hide_index=True)


def _correlation_heatmap(results: dict):
    """Correlation matrix for multi-symbol backtests (uses per-symbol returns if available)."""
    st.markdown("**🔗 Correlation Matrix**")

    symbol_returns = results.get("symbol_returns")
    if symbol_returns and isinstance(symbol_returns, dict) and len(symbol_returns) > 1:
        df = pd.DataFrame(symbol_returns)
        corr = df.corr()
    else:
        # Fall back: simulate from trades if multiple symbols present
        trades = results.get("trades", [])
        symbols = list({t.get("symbol", "UNKNOWN") for t in trades if "symbol" in t})
        if len(symbols) < 2:
            st.info(
                "Correlation analysis requires multi-symbol backtest data. "
                "Run a backtest with ≥ 2 symbols to see this heatmap."
            )
            return
        # Try to build returns per symbol from equity components
        st.info("Multi-symbol returns not available in current results format.")
        return

    fig = go.Figure(
        data=go.Heatmap(
            z=corr.values,
            x=corr.columns.tolist(),
            y=corr.index.tolist(),
            colorscale="RdBu_r",
            zmid=0,
            zmin=-1,
            zmax=1,
            text=np.vectorize(lambda v: f"{v:.2f}")(corr.values),
            texttemplate="%{text}",
            hovertemplate="%{x} vs %{y}: %{z:.3f}<extra></extra>",
        )
    )
    fig.update_layout(title="Return Correlation Matrix", height=500)
    st.plotly_chart(fig, use_container_width=True)


def _drawdown_heatmap(eq: pd.DataFrame):
    """Drawdown magnitude as a calendar-day heatmap (week × weekday)."""
    st.markdown("**📉 Drawdown Heatmap**")

    peak = eq["value"].expanding().max()
    dd = (eq["value"] - peak) / peak * 100  # negative values

    if dd.empty:
        st.info("No drawdown data available.")
        return

    # Monthly aggregation — worst drawdown per month
    dd_monthly = dd.resample("ME").min()  # worst dd per month
    df = dd_monthly.to_frame("drawdown")
    df["year"] = df.index.year
    df["month"] = df.index.month

    month_names = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun",
        "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ]

    pivot = df.pivot_table(values="drawdown", index="year", columns="month", aggfunc="first")
    pivot = pivot.reindex(columns=range(1, 13))

    fig = go.Figure(
        data=go.Heatmap(
            z=pivot.values,
            x=[month_names[m - 1] for m in pivot.columns],
            y=[str(y) for y in pivot.index],
            colorscale=[[0, "darkred"], [0.5, "lightyellow"], [1, "white"]],
            zmax=0,
            text=np.where(
                np.isnan(pivot.values),
                "",
                np.vectorize(lambda v: f"{v:.1f}%")(pivot.values),
            ),
            texttemplate="%{text}",
            hovertemplate="Year %{y}, %{x}: %{z:.2f}%<extra></extra>",
        )
    )
    fig.update_layout(
        title="Worst Monthly Drawdown (%)",
        height=max(250, 80 * len(pivot)),
        xaxis_title="Month",
        yaxis_title="Year",
        yaxis=dict(autorange="reversed"),
    )
    st.plotly_chart(fig, use_container_width=True)


# ---------------------------------------------------------------------------
# Rolling Statistics
# ---------------------------------------------------------------------------

def _show_rolling_statistics(eq: pd.DataFrame):
    st.subheader("📈 Rolling Statistics")

    rets = _daily_returns(eq)
    if len(rets) < 30:
        st.info("Need ≥ 30 trading days for rolling statistics.")
        return

    # Configurable window
    col_cfg1, col_cfg2 = st.columns(2)
    with col_cfg1:
        window = st.selectbox(
            "Rolling window (days)",
            options=[30, 60, 90, 252],
            index=0,
            help="Number of trading days for the rolling window.",
        )
    with col_cfg2:
        benchmark_ret_annual = st.number_input(
            "Benchmark annual return (%) for beta",
            value=10.0,
            step=1.0,
            help="Used to generate a synthetic benchmark for beta calculation.",
        )

    # --- Rolling Sharpe ---
    rolling_mean = rets.rolling(window).mean()
    rolling_std = rets.rolling(window).std()
    rolling_sharpe = (rolling_mean / rolling_std) * np.sqrt(252)

    # --- Rolling Volatility (annualised) ---
    rolling_vol = rolling_std * np.sqrt(252) * 100

    # Percentile bands for volatility
    vol_median = rolling_vol.median()
    vol_p25 = rolling_vol.quantile(0.25)
    vol_p75 = rolling_vol.quantile(0.75)

    # --- Rolling Beta (vs synthetic benchmark) ---
    np.random.seed(42)
    bench_daily = benchmark_ret_annual / 100 / 252
    bench_vol = rolling_vol.median() / 100 / np.sqrt(252) if vol_median > 0 else 0.01
    bench_returns = pd.Series(
        np.random.normal(bench_daily, bench_vol, size=len(rets)),
        index=rets.index,
    )
    rolling_cov = rets.rolling(window).cov(bench_returns)
    rolling_bench_var = bench_returns.rolling(window).var()
    rolling_beta = rolling_cov / rolling_bench_var

    # --- Rolling Max Drawdown ---
    rolling_max_dd = pd.Series(index=eq.index, dtype=float)
    values = eq["value"].values
    for i in range(window, len(values)):
        w = values[i - window: i + 1]
        peak = np.maximum.accumulate(w)
        dd = (w - peak) / peak
        rolling_max_dd.iloc[i] = dd.min() * 100

    # Build 2×2 chart grid
    fig = make_subplots(
        rows=2,
        cols=2,
        subplot_titles=(
            f"Rolling Sharpe ({window}d)",
            f"Rolling Volatility ({window}d, annualised)",
            f"Rolling Beta ({window}d)",
            f"Rolling Max Drawdown ({window}d)",
        ),
        vertical_spacing=0.12,
        horizontal_spacing=0.08,
    )

    # Sharpe
    fig.add_trace(
        go.Scatter(
            x=rolling_sharpe.index,
            y=rolling_sharpe,
            mode="lines",
            name="Sharpe",
            line=dict(color="#1f77b4"),
        ),
        row=1,
        col=1,
    )
    fig.add_hline(y=1.0, line_dash="dash", line_color="grey", row=1, col=1)

    # Volatility with percentile bands
    fig.add_trace(
        go.Scatter(
            x=rolling_vol.index,
            y=rolling_vol,
            mode="lines",
            name="Volatility (%)",
            line=dict(color="#ff7f0e"),
        ),
        row=1,
        col=2,
    )
    fig.add_hline(y=vol_p25, line_dash="dot", line_color="green", row=1, col=2,
                  annotation_text="P25")
    fig.add_hline(y=vol_p75, line_dash="dot", line_color="red", row=1, col=2,
                  annotation_text="P75")

    # Beta
    fig.add_trace(
        go.Scatter(
            x=rolling_beta.index,
            y=rolling_beta,
            mode="lines",
            name="Beta",
            line=dict(color="#2ca02c"),
        ),
        row=2,
        col=1,
    )
    fig.add_hline(y=1.0, line_dash="dash", line_color="grey", row=2, col=1)

    # Max Drawdown
    fig.add_trace(
        go.Scatter(
            x=rolling_max_dd.index,
            y=rolling_max_dd,
            mode="lines",
            name="Max DD (%)",
            fill="tozeroy",
            fillcolor="rgba(255,0,0,0.15)",
            line=dict(color="red"),
        ),
        row=2,
        col=2,
    )

    fig.update_layout(height=700, showlegend=False)
    fig.update_yaxes(title_text="Sharpe", row=1, col=1)
    fig.update_yaxes(title_text="Vol (%)", row=1, col=2)
    fig.update_yaxes(title_text="Beta", row=2, col=1)
    fig.update_yaxes(title_text="DD (%)", row=2, col=2)
    st.plotly_chart(fig, use_container_width=True)

    # Summary table
    st.markdown("**📊 Rolling Statistics Summary**")
    summary = {
        "Metric": [
            f"Avg {window}d Sharpe",
            f"Current {window}d Sharpe",
            f"Avg Annualised Vol",
            f"Current Annualised Vol",
            f"Avg Beta",
            f"Worst {window}d Max DD",
        ],
        "Value": [
            f"{rolling_sharpe.mean():.2f}",
            f"{rolling_sharpe.iloc[-1]:.2f}" if not np.isnan(rolling_sharpe.iloc[-1]) else "N/A",
            f"{rolling_vol.mean():.1f}%",
            f"{rolling_vol.iloc[-1]:.1f}%" if not np.isnan(rolling_vol.iloc[-1]) else "N/A",
            f"{rolling_beta.mean():.2f}",
            f"{rolling_max_dd.min():.1f}%",
        ],
    }
    st.dataframe(pd.DataFrame(summary), use_container_width=True, hide_index=True)


# ---------------------------------------------------------------------------
# Compare Runs
# ---------------------------------------------------------------------------

def _show_compare_runs():
    st.subheader("🔀 Compare Backtest Runs")

    saved_runs: Dict[str, dict] = st.session_state.get("saved_runs", {})

    # Save current run
    current = st.session_state.get("backtest_results")
    if current:
        run_name = st.text_input(
            "Save current run as",
            value=f"Run {len(saved_runs) + 1}",
            key="save_run_name",
        )
        if st.button("💾 Save Current Run"):
            saved_runs[run_name] = {
                "results": current,
                "saved_at": datetime.now().isoformat(),
            }
            st.session_state.saved_runs = saved_runs
            st.success(f"Saved as **{run_name}**")

    if len(saved_runs) < 2:
        st.info(
            "Save ≥ 2 backtest runs to compare them side-by-side. "
            "Run different strategies or parameters and save each result."
        )
        if saved_runs:
            st.markdown(f"**Saved runs:** {', '.join(saved_runs.keys())}")
        return

    # Select runs to compare
    selected = st.multiselect(
        "Select runs to compare",
        options=list(saved_runs.keys()),
        default=list(saved_runs.keys())[:2],
    )
    if len(selected) < 2:
        st.warning("Select at least 2 runs.")
        return

    # Metrics comparison table
    rows = []
    for name in selected:
        r = saved_runs[name]["results"]
        rows.append(
            {
                "Run": name,
                "Total Return (%)": f"{r.get('total_return', 0):.2f}",
                "Sharpe": f"{r.get('sharpe_ratio', 0):.2f}",
                "Max DD (%)": f"{r.get('max_drawdown', 0):.2f}",
                "Trades": r.get("total_trades", 0),
                "Final Value ($)": f"{r.get('final_value', 0):,.2f}",
            }
        )
    st.dataframe(pd.DataFrame(rows), use_container_width=True, hide_index=True)

    # Equity overlay chart
    fig = go.Figure()
    for name in selected:
        r = saved_runs[name]["results"]
        edf = _equity_df(r)
        if edf is not None:
            fig.add_trace(
                go.Scatter(
                    x=edf.index,
                    y=edf["value"],
                    mode="lines",
                    name=name,
                )
            )
    fig.update_layout(
        title="Equity Curve Comparison",
        xaxis_title="Date",
        yaxis_title="Portfolio Value ($)",
        height=450,
    )
    st.plotly_chart(fig, use_container_width=True)

    # Delete a saved run
    with st.expander("🗑️ Manage saved runs"):
        to_delete = st.selectbox("Delete a run", options=[""] + list(saved_runs.keys()))
        if to_delete and st.button("Delete"):
            del saved_runs[to_delete]
            st.session_state.saved_runs = saved_runs
            st.rerun()


# ---------------------------------------------------------------------------
# Parameter Sensitivity
# ---------------------------------------------------------------------------

def _show_sensitivity():
    st.subheader("🎛️ Parameter Sensitivity Analysis")

    st.markdown(
        "Visualise how a strategy's performance changes across a grid of parameters. "
        "Provide parameter ranges below and GlowBack will simulate outcomes."
    )

    col1, col2 = st.columns(2)
    with col1:
        param_x_name = st.text_input("Parameter X name", value="SMA Period")
        param_x_min = st.number_input("X min", value=10, step=1)
        param_x_max = st.number_input("X max", value=60, step=1)
        param_x_step = st.number_input("X step", value=10, step=1, min_value=1)
    with col2:
        param_y_name = st.text_input("Parameter Y name", value="Position Size (%)")
        param_y_min = st.number_input("Y min", value=10, step=1)
        param_y_max = st.number_input("Y max", value=100, step=1)
        param_y_step = st.number_input("Y step", value=20, step=1, min_value=1)

    metric_choice = st.selectbox(
        "Metric to plot",
        options=["Total Return (%)", "Sharpe Ratio", "Max Drawdown (%)", "Win Rate (%)"],
    )

    if st.button("🚀 Run Sensitivity Analysis"):
        x_vals = list(range(int(param_x_min), int(param_x_max) + 1, int(param_x_step)))
        y_vals = list(range(int(param_y_min), int(param_y_max) + 1, int(param_y_step)))

        if not x_vals or not y_vals:
            st.error("Parameter ranges produce no values. Check min/max/step.")
            return

        # Simulated results (placeholder — real engine integration would go here)
        current = st.session_state.get("backtest_results", {})
        base_return = current.get("total_return", 5.0)
        base_sharpe = current.get("sharpe_ratio", 1.0)
        base_dd = current.get("max_drawdown", 15.0)

        np.random.seed(7)
        z = np.zeros((len(y_vals), len(x_vals)))
        for j, yv in enumerate(y_vals):
            for i, xv in enumerate(x_vals):
                noise = np.random.normal(0, 2)
                if metric_choice == "Total Return (%)":
                    z[j, i] = base_return + (xv - param_x_min) * 0.15 + noise
                elif metric_choice == "Sharpe Ratio":
                    z[j, i] = max(0, base_sharpe + (xv - param_x_min) * 0.01 + noise * 0.1)
                elif metric_choice == "Max Drawdown (%)":
                    z[j, i] = -(abs(base_dd) + (yv - param_y_min) * 0.1 + noise)
                else:
                    z[j, i] = 50 + (xv - param_x_min) * 0.2 + noise

        fig = go.Figure(
            data=go.Heatmap(
                z=z,
                x=[str(v) for v in x_vals],
                y=[str(v) for v in y_vals],
                colorscale="Viridis",
                text=np.vectorize(lambda v: f"{v:.1f}")(z),
                texttemplate="%{text}",
                hovertemplate=(
                    f"{param_x_name}: %{{x}}<br>"
                    f"{param_y_name}: %{{y}}<br>"
                    f"{metric_choice}: %{{z:.2f}}<extra></extra>"
                ),
            )
        )
        fig.update_layout(
            title=f"Sensitivity: {metric_choice}",
            xaxis_title=param_x_name,
            yaxis_title=param_y_name,
            height=500,
        )
        st.plotly_chart(fig, use_container_width=True)

        st.caption(
            "⚠️ Results are simulated perturbations of the current backtest. "
            "Full engine integration will run actual backtests per parameter combination."
        )


# ---------------------------------------------------------------------------
# Export
# ---------------------------------------------------------------------------

def _show_export(eq: pd.DataFrame, results: dict):
    st.subheader("📥 Export Results")

    col1, col2 = st.columns(2)

    with col1:
        st.markdown("**📄 CSV Export**")
        # Equity curve CSV
        csv_buf = io.StringIO()
        eq.to_csv(csv_buf)
        st.download_button(
            "⬇️ Download Equity Curve (CSV)",
            data=csv_buf.getvalue(),
            file_name="glowback_equity_curve.csv",
            mime="text/csv",
        )

        # Trades CSV
        trades = results.get("trades", [])
        if trades:
            trades_csv = pd.DataFrame(trades).to_csv(index=False)
            st.download_button(
                "⬇️ Download Trades (CSV)",
                data=trades_csv,
                file_name="glowback_trades.csv",
                mime="text/csv",
            )

    with col2:
        st.markdown("**📊 JSON Export**")
        # Full results JSON (exclude large nested structures for brevity)
        export_data = {
            k: v
            for k, v in results.items()
            if k not in ("equity_curve",)  # equity curve exported separately
        }
        export_data["summary"] = {
            "total_return": results.get("total_return"),
            "sharpe_ratio": results.get("sharpe_ratio"),
            "max_drawdown": results.get("max_drawdown"),
            "final_value": results.get("final_value"),
            "total_trades": results.get("total_trades"),
        }
        st.download_button(
            "⬇️ Download Summary (JSON)",
            data=json.dumps(export_data, indent=2, default=str),
            file_name="glowback_summary.json",
            mime="application/json",
        )

    st.markdown("---")
    st.markdown("**🖨️ PDF Report**")
    st.info(
        "PDF export requires a print-friendly view. "
        "Use your browser's **Print → Save as PDF** (Ctrl+P / Cmd+P) "
        "for a quick snapshot, or install `weasyprint` for programmatic PDF generation."
    )
