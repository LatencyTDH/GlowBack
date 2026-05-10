"""
Results Dashboard Page - Comprehensive backtest results analysis
"""

import streamlit as st
import pandas as pd
import plotly.graph_objects as go
import plotly.express as px
from plotly.subplots import make_subplots
import numpy as np

from backtest_core import (
    calculate_annualized_return_pct,
    calculate_closed_trade_win_rate,
    calculate_period_return_series,
    prepare_equity_curve_frame,
)

def show():
    """Main results dashboard page"""
    
    st.title("📈 Results Dashboard")
    st.markdown("Comprehensive analysis of your backtest results.")
    
    # Check if results exist
    if not st.session_state.backtest_results:
        st.warning("⚠️ No backtest results available. Please run a backtest first.")
        if st.button("🚀 Go to Backtest Runner"):
            # Force navigation by setting query params (Streamlit navigation)
            st.info("💡 Please use the sidebar navigation to go to 'Backtest Runner'")
        return
    
    results = st.session_state.backtest_results
    
    # Validate results structure using utility function
    from utils import validate_backtest_results
    is_valid, message, details = validate_backtest_results(results)
    
    if not is_valid:
        st.error(f"❌ {message}")
        if details:
            st.write("Details:", details)
        st.info("💡 This might be due to a backtest error or incomplete results.")
        return
    
    # Show performance overview
    show_performance_overview(results)
    
    # Show performance charts
    show_performance_charts(results)
    
    # Show trade analysis
    show_trade_analysis(results)
    
    # Show risk analysis
    show_risk_analysis(results)

def show_performance_overview(results):
    """Show performance overview metrics."""
    st.subheader("🎯 Performance Overview")

    equity_curve = prepare_equity_curve_frame(results['equity_curve'])
    benchmark_metrics = results.get('benchmark_metrics') or {}
    period_returns = calculate_period_return_series(equity_curve).dropna()
    win_rate = calculate_closed_trade_win_rate(results.get('trades', []))
    annualized_return = calculate_annualized_return_pct(equity_curve)

    col1, col2, col3, col4, col5 = st.columns(5)

    with col1:
        st.metric(
            "Total Return",
            f"{results['total_return']:.2f}%",
            delta=f"{results['total_return']:.2f}%" if results['total_return'] > 0 else None,
        )

    with col2:
        st.metric(
            "Sharpe Ratio",
            f"{results['sharpe_ratio']:.2f}",
            delta="Good" if results['sharpe_ratio'] > 1.0 else "Poor" if results['sharpe_ratio'] < 0.5 else "Fair",
        )

    with col3:
        st.metric(
            "Max Drawdown",
            f"{results['max_drawdown']:.2f}%",
            delta="High Risk" if results['max_drawdown'] > 20 else "Moderate" if results['max_drawdown'] > 10 else "Low Risk",
        )

    with col4:
        st.metric(
            "Total Trades",
            results['total_trades'],
            delta=f"${results['final_value'] - results.get('initial_capital', 100000):,.0f}" if results['total_trades'] > 0 else None,
        )

    with col5:
        st.metric(
            "Win Rate",
            "N/A" if win_rate is None else f"{win_rate:.1f}%",
            delta=None if win_rate is None else "Good" if win_rate > 60 else "Poor" if win_rate < 40 else "Fair",
        )

    col1, col2 = st.columns(2)

    with col1:
        st.markdown("**📊 Return Metrics**")

        if len(equity_curve) > 1:
            metrics_data = {
                "Metric": [
                    "Initial Capital",
                    "Final Value",
                    "Total Return",
                    "Annualized Return",
                    "Volatility (Annual)",
                    "Best Period",
                    "Worst Period",
                ],
                "Value": [
                    f"${results.get('initial_capital', 100000):,.2f}",
                    f"${results['final_value']:,.2f}",
                    f"{results['total_return']:.2f}%",
                    f"{annualized_return:.2f}%",
                    f"{period_returns.std() * np.sqrt(252) * 100:.2f}%" if len(period_returns) > 0 else "N/A",
                    f"{period_returns.max() * 100:.2f}%" if len(period_returns) > 0 else "N/A",
                    f"{period_returns.min() * 100:.2f}%" if len(period_returns) > 0 else "N/A",
                ],
            }

            st.dataframe(pd.DataFrame(metrics_data), use_container_width=True, hide_index=True)

    with col2:
        st.markdown("**⚖️ Risk Metrics**")

        risk_data = {
            "Metric": [
                "Sharpe Ratio",
                "Max Drawdown",
                "Drawdown Duration",
                "Value at Risk (95%)",
                "Expected Shortfall",
                "Beta (vs Benchmark)",
                "Alpha (Annual)",
                "Tracking Error",
                "Information Ratio",
            ],
            "Value": [
                f"{results['sharpe_ratio']:.2f}",
                f"{results['max_drawdown']:.2f}%",
                "N/A",
                f"{np.percentile(period_returns * 100, 5):.2f}%" if len(period_returns) > 0 else "N/A",
                f"{period_returns[period_returns <= period_returns.quantile(0.05)].mean() * 100:.2f}%"
                if len(period_returns) > 0 and not period_returns[period_returns <= period_returns.quantile(0.05)].empty
                else "N/A",
                "N/A" if benchmark_metrics.get('beta') is None else f"{benchmark_metrics['beta']:.2f}",
                "N/A" if benchmark_metrics.get('alpha') is None else f"{benchmark_metrics['alpha']:.2f}%",
                "N/A" if benchmark_metrics.get('tracking_error') is None else f"{benchmark_metrics['tracking_error']:.2f}%",
                "N/A" if benchmark_metrics.get('information_ratio') is None else f"{benchmark_metrics['information_ratio']:.2f}",
            ],
        }

        st.dataframe(pd.DataFrame(risk_data), use_container_width=True, hide_index=True)

def show_performance_charts(results):
    """Show performance charts."""
    st.subheader("📊 Performance Charts")

    if 'equity_curve' not in results:
        st.error("❌ No equity curve data found in results")
        st.write("Available keys:", list(results.keys()))
        return

    equity_curve = prepare_equity_curve_frame(results['equity_curve'])

    required_columns = ['timestamp', 'value']
    missing_columns = [col for col in required_columns if col not in equity_curve.columns]
    if missing_columns:
        st.error(f"❌ Missing required columns: {missing_columns}")
        st.write("Available columns:", list(equity_curve.columns))
        st.info("💡 This might be due to a different data format. Please check the backtest results.")
        return

    period_returns = calculate_period_return_series(equity_curve) * 100

    fig = make_subplots(
        rows=3,
        cols=1,
        subplot_titles=('Portfolio Value', 'Returns', 'Drawdown'),
        vertical_spacing=0.08,
        specs=[[{"secondary_y": False}], [{"secondary_y": False}], [{"secondary_y": False}]],
    )

    fig.add_trace(
        go.Scatter(
            x=equity_curve['timestamp'],
            y=equity_curve['value'],
            mode='lines',
            name='Portfolio Value',
            line=dict(color='#1f77b4', width=2),
        ),
        row=1,
        col=1,
    )

    if len(period_returns.dropna()) > 0:
        fig.add_trace(
            go.Scatter(
                x=equity_curve.loc[period_returns.notna(), 'timestamp'],
                y=period_returns.dropna(),
                mode='lines',
                name='Period Returns (%)',
                line=dict(color='#ff7f0e', width=1),
            ),
            row=2,
            col=1,
        )

    peak = equity_curve['value'].cummax()
    drawdown = (equity_curve['value'] - peak) / peak * 100

    fig.add_trace(
        go.Scatter(
            x=equity_curve['timestamp'],
            y=drawdown,
            mode='lines',
            name='Drawdown (%)',
            fill='tozeroy',
            fillcolor='rgba(255, 0, 0, 0.3)',
            line=dict(color='red', width=1),
        ),
        row=3,
        col=1,
    )

    fig.update_layout(height=800, title_text="Portfolio Performance Analysis", showlegend=True)
    fig.update_xaxes(title_text="Date", row=3, col=1)
    fig.update_yaxes(title_text="Value ($)", row=1, col=1)
    fig.update_yaxes(title_text="Return (%)", row=2, col=1)
    fig.update_yaxes(title_text="Drawdown (%)", row=3, col=1)

    st.plotly_chart(fig, use_container_width=True)

    benchmark_curve = prepare_equity_curve_frame(results.get('benchmark_curve', []))
    if not benchmark_curve.empty and {'timestamp', 'value'}.issubset(benchmark_curve.columns):
        benchmark_chart = benchmark_curve[['timestamp', 'value']].rename(columns={'value': 'benchmark_value'})
        comparison = equity_curve[['timestamp', 'value']].merge(benchmark_chart, on='timestamp', how='inner')
        if len(comparison) > 1:
            comparison['strategy_index'] = comparison['value'] / comparison['value'].iloc[0] * 100
            comparison['benchmark_index'] = comparison['benchmark_value'] / comparison['benchmark_value'].iloc[0] * 100
            benchmark_name = (results.get('benchmark_metrics') or {}).get('benchmark_symbol') or results.get('benchmark_symbol') or 'Benchmark'

            st.markdown("**📏 Strategy vs Benchmark**")
            comp_fig = go.Figure()
            comp_fig.add_trace(go.Scatter(x=comparison['timestamp'], y=comparison['strategy_index'], mode='lines', name='Strategy'))
            comp_fig.add_trace(go.Scatter(x=comparison['timestamp'], y=comparison['benchmark_index'], mode='lines', name=benchmark_name))
            comp_fig.update_layout(height=350, yaxis_title='Indexed Value (Start = 100)')
            st.plotly_chart(comp_fig, use_container_width=True)

    col1, col2 = st.columns(2)

    with col1:
        show_monthly_returns_heatmap(equity_curve)

    with col2:
        show_portfolio_composition(equity_curve.copy())

def show_monthly_returns_heatmap(equity_curve):
    """Show monthly returns heatmap without mutating the shared equity frame."""
    st.markdown("**📅 Monthly Returns Heatmap**")

    if len(equity_curve) < 30:
        st.info("Not enough data for monthly analysis (need at least 30 days)")
        return

    if 'timestamp' not in equity_curve.columns or 'value' not in equity_curve.columns:
        st.error("❌ Missing required columns for monthly analysis")
        return

    monthly_frame = equity_curve.copy()
    monthly_frame['timestamp'] = pd.to_datetime(monthly_frame['timestamp'])
    monthly_values = monthly_frame.set_index('timestamp')['value'].resample('M').last()
    monthly_returns = monthly_values.pct_change() * 100

    if len(monthly_returns) > 1:
        returns_df = monthly_returns.to_frame('returns')
        returns_df['year'] = returns_df.index.year
        returns_df['month'] = returns_df.index.month

        pivot_table = returns_df.pivot_table(values='returns', index='year', columns='month', aggfunc='first')

        fig = px.imshow(
            pivot_table.values,
            labels=dict(x="Month", y="Year", color="Return (%)"),
            x=[f"{i:02d}" for i in pivot_table.columns],
            y=pivot_table.index,
            color_continuous_scale='RdYlGn',
            color_continuous_midpoint=0,
        )

        fig.update_layout(height=300)
        st.plotly_chart(fig, use_container_width=True)
    else:
        st.info("Not enough monthly data points for heatmap")

def show_portfolio_composition(equity_curve):
    """Show portfolio composition over time"""
    st.markdown("**💼 Portfolio Composition**")
    
    # Check if required columns exist
    required_columns = ['timestamp', 'value', 'cash']
    missing_columns = [col for col in required_columns if col not in equity_curve.columns]
    
    if missing_columns:
        st.error(f"❌ Missing required columns for portfolio composition: {missing_columns}")
        st.write("Available columns:", list(equity_curve.columns))
        return
    
    # Create a simple composition chart
    fig = go.Figure()
    
    fig.add_trace(go.Scatter(
        x=equity_curve['timestamp'],
        y=equity_curve['cash'],
        mode='lines',
        stackgroup='one',
        name='Cash',
        line=dict(color='green')
    ))
    
    # Calculate position value (simplified)
    position_value = equity_curve['value'] - equity_curve['cash']
    fig.add_trace(go.Scatter(
        x=equity_curve['timestamp'],
        y=position_value,
        mode='lines',
        stackgroup='one',
        name='Positions',
        line=dict(color='blue')
    ))
    
    fig.update_layout(
        title="Portfolio Composition Over Time",
        xaxis_title="Date",
        yaxis_title="Value ($)",
        height=300
    )
    
    st.plotly_chart(fig, use_container_width=True)

def show_trade_analysis(results):
    """Show trade analysis"""
    st.subheader("📋 Trade Analysis")
    
    trades = results.get('trades', [])
    
    if not trades:
        st.info("No trades executed during the backtest.")
        return
    
    trades_df = pd.DataFrame(trades)
    
    # Trade summary
    col1, col2, col3, col4 = st.columns(4)
    
    with col1:
        buy_trades = len(trades_df[trades_df['action'] == 'BUY'])
        st.metric("Buy Orders", buy_trades)
    
    with col2:
        sell_trades = len(trades_df[trades_df['action'] == 'SELL'])
        st.metric("Sell Orders", sell_trades)
    
    with col3:
        avg_trade_size = trades_df['shares'].mean() if 'shares' in trades_df.columns else 0
        st.metric("Avg Trade Size", f"{avg_trade_size:.0f}")
    
    with col4:
        win_rate = calculate_closed_trade_win_rate(trades)
        st.metric("Win Rate", "N/A" if win_rate is None else f"{win_rate:.1f}%")
    
    # Trade history table
    st.markdown("**📊 Trade History**")
    
    # Format trades for display
    display_trades = trades_df.copy()
    if 'timestamp' in display_trades.columns:
        display_trades['timestamp'] = pd.to_datetime(display_trades['timestamp']).dt.strftime('%Y-%m-%d %H:%M')
    
    if 'price' in display_trades.columns:
        display_trades['price'] = display_trades['price'].apply(lambda x: f"${x:.2f}")
    
    if 'cost' in display_trades.columns:
        display_trades['cost'] = display_trades['cost'].apply(lambda x: f"${x:.2f}")
    
    if 'proceeds' in display_trades.columns:
        display_trades['proceeds'] = display_trades['proceeds'].apply(lambda x: f"${x:.2f}")
    
    st.dataframe(display_trades, use_container_width=True)
    
    # Trade timing analysis
    col1, col2 = st.columns(2)
    
    with col1:
        # Trade frequency chart
        if len(trades_df) > 1:
            trades_df['timestamp'] = pd.to_datetime(trades_df['timestamp'])
            trades_by_date = trades_df.groupby(trades_df['timestamp'].dt.date).size()
            
            fig = px.bar(
                x=trades_by_date.index,
                y=trades_by_date.values,
                title="Trades per Day"
            )
            fig.update_layout(height=300)
            st.plotly_chart(fig, use_container_width=True)
    
    with col2:
        # Trade size distribution
        if 'shares' in trades_df.columns:
            fig = px.histogram(
                trades_df,
                x='shares',
                title="Trade Size Distribution",
                nbins=20
            )
            fig.update_layout(height=300)
            st.plotly_chart(fig, use_container_width=True)

def show_risk_analysis(results):
    """Show risk analysis."""
    st.subheader("⚖️ Risk Analysis")

    equity_curve = prepare_equity_curve_frame(results['equity_curve'])
    if len(equity_curve) < 2:
        st.info("Not enough data for risk analysis.")
        return

    returns = calculate_period_return_series(equity_curve).dropna()

    col1, col2 = st.columns(2)

    with col1:
        st.markdown("**📊 Return Distribution**")

        if len(returns) > 0:
            fig = px.histogram(
                x=returns * 100,
                nbins=30,
                title="Daily Returns Distribution",
                labels={'x': 'Return (%)', 'y': 'Frequency'},
            )

            mean_return = returns.mean() * 100
            std_return = returns.std() * 100
            if std_return > 0:
                x_norm = np.linspace(returns.min() * 100, returns.max() * 100, 100)
                y_norm = (1 / (std_return * np.sqrt(2 * np.pi))) * np.exp(
                    -0.5 * ((x_norm - mean_return) / std_return) ** 2
                )
                y_norm = y_norm * len(returns) * (returns.max() - returns.min()) * 100 / 30

                fig.add_trace(
                    go.Scatter(
                        x=x_norm,
                        y=y_norm,
                        mode='lines',
                        name='Normal Distribution',
                        line=dict(color='red', dash='dash'),
                    )
                )

            fig.update_layout(height=400)
            st.plotly_chart(fig, use_container_width=True)

    with col2:
        st.markdown("**📈 Rolling Metrics**")

        if len(returns) > 20:
            rolling_mean = returns.rolling(20).mean()
            rolling_std = returns.rolling(20).std().replace(0, np.nan)
            rolling_sharpe = rolling_mean / rolling_std * np.sqrt(252)
            rolling_frame = equity_curve.loc[returns.index].copy()
            rolling_frame['rolling_sharpe'] = rolling_sharpe.values
            rolling_frame = rolling_frame.dropna(subset=['rolling_sharpe'])

            fig = go.Figure()
            fig.add_trace(
                go.Scatter(
                    x=rolling_frame['timestamp'],
                    y=rolling_frame['rolling_sharpe'],
                    mode='lines',
                    name='20-Day Rolling Sharpe',
                    line=dict(color='blue'),
                )
            )
            fig.add_hline(y=1.0, line_dash='dash', line_color='red', annotation_text='Sharpe = 1.0')
            fig.update_layout(
                title='Rolling Sharpe Ratio (20-Day Window)',
                xaxis_title='Date',
                yaxis_title='Sharpe Ratio',
                height=400,
            )
            st.plotly_chart(fig, use_container_width=True)

    st.markdown("**🎯 Risk Summary**")

    if len(returns) > 0:
        risk_metrics = {
            'Metric': [
                'Daily Volatility',
                'Annual Volatility',
                'Skewness',
                'Kurtosis',
                '95% VaR (Daily)',
                '99% VaR (Daily)',
                'Max Daily Loss',
                'Max Daily Gain',
            ],
            'Value': [
                f"{returns.std() * 100:.2f}%",
                f"{returns.std() * np.sqrt(252) * 100:.2f}%",
                f"{returns.skew():.2f}",
                f"{returns.kurtosis():.2f}",
                f"{np.percentile(returns * 100, 5):.2f}%",
                f"{np.percentile(returns * 100, 1):.2f}%",
                f"{returns.min() * 100:.2f}%",
                f"{returns.max() * 100:.2f}%",
            ],
        }

        col1, col2 = st.columns([1, 1])
        with col1:
            st.dataframe(pd.DataFrame(risk_metrics), use_container_width=True, hide_index=True)
