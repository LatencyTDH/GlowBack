"""
Portfolio Analyzer Page - Advanced portfolio analysis and optimization
"""

import streamlit as st
import pandas as pd
import plotly.graph_objects as go
import plotly.express as px
from plotly.subplots import make_subplots
import numpy as np

def show():
    """Main portfolio analyzer page"""
    
    st.title("💼 Portfolio Analyzer")
    st.markdown("Advanced portfolio analysis, optimization, and risk management tools.")
    
    if not st.session_state.backtest_results:
        st.warning("⚠️ No backtest results available. Please run a backtest first.")
        if st.button("🚀 Go to Backtest Runner"):
            # Force navigation by setting query params (Streamlit navigation)
            st.info("💡 Please use the sidebar navigation to go to 'Backtest Runner'")
        return
    
    # Portfolio analysis tabs
    tab1, tab2, tab3, tab4 = st.tabs(["📊 Performance", "⚖️ Risk Analysis", "🎯 Optimization", "📈 Scenarios"])
    
    with tab1:
        show_performance_analysis()
    
    with tab2:
        show_risk_analysis()
    
    with tab3:
        show_optimization_analysis()
    
    with tab4:
        show_scenario_analysis()

def show_performance_analysis():
    """Show detailed performance analysis"""
    st.subheader("📊 Performance Analysis")
    
    results = st.session_state.backtest_results
    equity_curve = pd.DataFrame(results['equity_curve'])
    
    # Performance attribution
    col1, col2 = st.columns(2)
    
    with col1:
        st.markdown("**🎯 Performance Attribution**")

        attribution_rows = results.get('attribution') or []
        cost_summary = results.get('cost_summary') or {}
        benchmark_metrics = results.get('benchmark_metrics') or {}

        attribution_data = [
            {
                "Component": row.get("component", "Unknown"),
                "Contribution (%)": row.get("contribution_pct", 0.0),
            }
            for row in attribution_rows
        ]
        if cost_summary:
            attribution_data.append(
                {
                    "Component": "Trading Costs",
                    "Contribution (%)": -cost_summary.get("cost_drag_pct_initial", 0.0),
                }
            )

        attr_df = pd.DataFrame(attribution_data)
        if not attr_df.empty:
            fig = px.bar(
                attr_df,
                x="Component",
                y="Contribution (%)",
                color="Contribution (%)",
                color_continuous_scale="RdYlGn",
                title="Performance Attribution",
            )
            st.plotly_chart(fig, use_container_width=True)
        else:
            st.info("Run a backtest with executed trades to see attribution.")

        if benchmark_metrics:
            beta = benchmark_metrics.get('beta')
            alpha = benchmark_metrics.get('alpha')
            information_ratio = benchmark_metrics.get('information_ratio')
            st.caption(
                f"Benchmark: {benchmark_metrics.get('benchmark_symbol') or results.get('benchmark_symbol')} · "
                f"Beta {'N/A' if beta is None else f'{beta:.2f}'} · "
                f"Alpha {'N/A' if alpha is None else f'{alpha:.2f}%'} · "
                f"IR {'N/A' if information_ratio is None else f'{information_ratio:.2f}'}"
            )
    
    with col2:
        st.markdown("**📅 Period Analysis**")
        
        # Period returns
        if len(equity_curve) > 1:
            equity_curve['timestamp'] = pd.to_datetime(equity_curve['timestamp'])
            equity_curve.set_index('timestamp', inplace=True)
            
            # Calculate period returns
            periods = {
                "1 Week": 7,
                "1 Month": 30,
                "3 Months": 90,
                "6 Months": 180,
                "1 Year": 365
            }
            
            period_returns = []
            for period_name, days in periods.items():
                if len(equity_curve) >= days:
                    start_value = equity_curve['value'].iloc[-days]
                    end_value = equity_curve['value'].iloc[-1]
                    period_return = (end_value - start_value) / start_value * 100
                    period_returns.append({"Period": period_name, "Return (%)": period_return})
            
            if period_returns:
                period_df = pd.DataFrame(period_returns)
                st.dataframe(period_df, use_container_width=True, hide_index=True)
    
    # Rolling performance metrics
    st.markdown("**📈 Rolling Performance Metrics**")
    
    if len(equity_curve) > 30:
        # Calculate rolling returns
        returns = equity_curve['value'].pct_change()
        
        # Rolling Sharpe ratio
        rolling_sharpe = returns.rolling(30).mean() / returns.rolling(30).std() * np.sqrt(365)
        
        # Rolling volatility
        rolling_vol = returns.rolling(30).std() * np.sqrt(365) * 100
        
        # Create subplot
        fig = make_subplots(
            rows=2, cols=1,
            subplot_titles=('Rolling 30-Day Sharpe Ratio', 'Rolling 30-Day Volatility (%)'),
            vertical_spacing=0.1
        )
        
        fig.add_trace(
            go.Scatter(
                x=equity_curve.index,
                y=rolling_sharpe,
                mode='lines',
                name='Sharpe Ratio',
                line=dict(color='blue')
            ),
            row=1, col=1
        )
        
        fig.add_trace(
            go.Scatter(
                x=equity_curve.index,
                y=rolling_vol,
                mode='lines',
                name='Volatility (%)',
                line=dict(color='red')
            ),
            row=2, col=1
        )
        
        fig.update_layout(height=500, showlegend=False)
        st.plotly_chart(fig, use_container_width=True)

def show_risk_analysis():
    """Show advanced risk analysis"""
    st.subheader("⚖️ Advanced Risk Analysis")
    
    results = st.session_state.backtest_results
    equity_curve = pd.DataFrame(results['equity_curve'])
    
    if len(equity_curve) < 2:
        st.info("Not enough data for risk analysis.")
        return
    
    # Calculate returns
    returns = equity_curve['value'].pct_change().dropna()
    
    col1, col2 = st.columns(2)
    
    with col1:
        st.markdown("**📊 Value at Risk Analysis**")
        
        if len(returns) > 0:
            # Calculate VaR at different confidence levels
            var_levels = [0.95, 0.99, 0.999]
            var_data = []
            
            for level in var_levels:
                var_daily = np.percentile(returns, (1 - level) * 100)
                var_annual = var_daily * np.sqrt(252)
                
                var_data.append({
                    "Confidence Level": f"{level*100:.1f}%",
                    "Daily VaR": f"{var_daily*100:.2f}%",
                    "Annual VaR": f"{var_annual*100:.2f}%"
                })
            
            var_df = pd.DataFrame(var_data)
            st.dataframe(var_df, use_container_width=True, hide_index=True)
            
            # VaR visualization
            fig = go.Figure()
            
            fig.add_trace(go.Histogram(
                x=returns * 100,
                nbinsx=50,
                name='Return Distribution',
                opacity=0.7
            ))
            
            # Add VaR lines
            colors = ['red', 'orange', 'darkred']
            for i, level in enumerate(var_levels):
                var_value = np.percentile(returns * 100, (1 - level) * 100)
                fig.add_vline(
                    x=var_value,
                    line_dash="dash",
                    line_color=colors[i],
                    annotation_text=f"VaR {level*100:.1f}%"
                )
            
            fig.update_layout(
                title="Value at Risk Analysis",
                xaxis_title="Daily Return (%)",
                yaxis_title="Frequency"
            )
            
            st.plotly_chart(fig, use_container_width=True)
    
    with col2:
        st.markdown("**📉 Drawdown Analysis**")
        
        # Calculate drawdown
        peak = equity_curve['value'].expanding().max()
        drawdown = (equity_curve['value'] - peak) / peak * 100
        
        # Drawdown statistics
        max_dd = drawdown.min()
        current_dd = drawdown.iloc[-1]
        
        # Find drawdown periods
        in_drawdown = drawdown < -0.1  # More than 0.1% drawdown
        dd_periods = []
        
        if in_drawdown.any():
            start_idx = None
            for i, is_dd in enumerate(in_drawdown):
                if is_dd and start_idx is None:
                    start_idx = i
                elif not is_dd and start_idx is not None:
                    dd_periods.append((start_idx, i-1))
                    start_idx = None
            
            # If still in drawdown at the end
            if start_idx is not None:
                dd_periods.append((start_idx, len(in_drawdown)-1))
        
        # Drawdown metrics
        dd_metrics = {
            "Metric": [
                "Maximum Drawdown",
                "Current Drawdown",
                "Avg Drawdown",
                "Drawdown Periods",
                "Longest Drawdown",
                "Recovery Time"
            ],
            "Value": [
                f"{max_dd:.2f}%",
                f"{current_dd:.2f}%",
                f"{drawdown[drawdown < 0].mean():.2f}%" if len(drawdown[drawdown < 0]) > 0 else "N/A",
                len(dd_periods),
                f"{max([(end-start) for start, end in dd_periods]) if dd_periods else 0} days",
                "N/A"  # Would need to calculate
            ]
        }
        
        st.dataframe(pd.DataFrame(dd_metrics), use_container_width=True, hide_index=True)
        
        # Underwater curve
        fig = go.Figure()
        
        fig.add_trace(go.Scatter(
            x=equity_curve.index,
            y=drawdown,
            mode='lines',
            fill='tozeroy',
            fillcolor='rgba(255, 0, 0, 0.3)',
            line=dict(color='red'),
            name='Drawdown'
        ))
        
        fig.update_layout(
            title="Underwater Curve",
            xaxis_title="Date",
            yaxis_title="Drawdown (%)",
            height=300
        )
        
        st.plotly_chart(fig, use_container_width=True)

def show_optimization_analysis():
    """Show portfolio construction and rebalancing diagnostics."""
    st.subheader("🎯 Portfolio Construction & Rebalancing")

    results = st.session_state.backtest_results
    portfolio_summary = results.get('portfolio_construction') or {}
    portfolio_metrics = results.get('metrics_summary') or {}
    diagnostics = pd.DataFrame(results.get('portfolio_diagnostics') or [])
    constraint_hits = results.get('constraint_hits') or []

    if not portfolio_summary:
        st.info(
            "Run a backtest in target-weight portfolio construction mode to unlock rebalance schedules, "
            "drift diagnostics, turnover controls, and constraint-hit reporting."
        )
        return

    col1, col2, col3, col4 = st.columns(4)
    with col1:
        st.metric("Rebalances", int(portfolio_metrics.get('portfolio_rebalances', 0)))
    with col2:
        st.metric("Avg Turnover", f"{portfolio_metrics.get('average_turnover_pct', 0.0):.2f}%")
    with col3:
        st.metric("Max Drift", f"{portfolio_metrics.get('max_weight_drift_pct', 0.0):.2f}%")
    with col4:
        st.metric("Constraint Hits", int(portfolio_metrics.get('constraint_hit_count', 0)))

    target_weights = portfolio_summary.get('target_weights') or {}
    target_df = pd.DataFrame(
        [
            {"Symbol": symbol, "Target Weight (%)": weight}
            for symbol, weight in sorted(target_weights.items())
        ]
    )

    col1, col2 = st.columns([1, 1])
    with col1:
        st.markdown("**🎯 Allocation Policy**")
        if not target_df.empty:
            st.dataframe(target_df, use_container_width=True, hide_index=True)
        else:
            st.info("No target weights captured for this run.")
    with col2:
        st.markdown("**⚙️ Policy Controls**")
        st.write(f"Method: {portfolio_summary.get('method', 'target_weights')}")
        st.write(f"Rebalance Frequency: {portfolio_summary.get('rebalance_frequency', 'weekly')}")
        st.write(f"Cash Floor: {portfolio_summary.get('cash_floor_pct', 0.0):.2f}%")
        st.write(
            f"Max Weight: {portfolio_summary.get('max_weight_pct', 'N/A')}"
            + ("%" if portfolio_summary.get('max_weight_pct') is not None else "")
        )
        st.write(
            f"Drift Threshold: {portfolio_summary.get('drift_threshold_pct', 'N/A')}"
            + ("%" if portfolio_summary.get('drift_threshold_pct') is not None else "")
        )
        st.write(
            f"Max Turnover: {portfolio_summary.get('max_turnover_pct', 'N/A')}"
            + ("%" if portfolio_summary.get('max_turnover_pct') is not None else "")
        )
        st.write(
            f"Drawdown Guardrail: {portfolio_summary.get('max_drawdown_pct', 'N/A')}"
            + ("%" if portfolio_summary.get('max_drawdown_pct') is not None else "")
        )

    if diagnostics.empty:
        st.warning("No portfolio diagnostics were recorded for this run.")
        return

    diagnostics['timestamp'] = pd.to_datetime(diagnostics['timestamp'])

    monitor_fig = make_subplots(
        rows=2,
        cols=1,
        shared_xaxes=True,
        subplot_titles=('Turnover & Drift', 'Drawdown & Cash Buffer'),
        vertical_spacing=0.12,
    )
    monitor_fig.add_trace(
        go.Bar(
            x=diagnostics['timestamp'],
            y=diagnostics['turnover_pct'],
            name='Turnover (%)',
            marker_color='#4C78A8',
        ),
        row=1,
        col=1,
    )
    monitor_fig.add_trace(
        go.Scatter(
            x=diagnostics['timestamp'],
            y=diagnostics['max_abs_drift_pct'],
            mode='lines+markers',
            name='Max drift (%)',
            line=dict(color='#F58518'),
        ),
        row=1,
        col=1,
    )
    monitor_fig.add_trace(
        go.Scatter(
            x=diagnostics['timestamp'],
            y=diagnostics['drawdown_pct'],
            mode='lines',
            name='Drawdown (%)',
            line=dict(color='#E45756'),
        ),
        row=2,
        col=1,
    )
    monitor_fig.add_trace(
        go.Scatter(
            x=diagnostics['timestamp'],
            y=diagnostics['cash_weight_pct'],
            mode='lines',
            name='Cash weight (%)',
            line=dict(color='#72B7B2'),
        ),
        row=2,
        col=1,
    )
    monitor_fig.update_layout(height=550, barmode='group')
    st.plotly_chart(monitor_fig, use_container_width=True)

    weight_rows = []
    for row in diagnostics.itertuples(index=False):
        target_map = getattr(row, 'target_weights', {}) or {}
        realized_map = getattr(row, 'realized_weights', {}) or {}
        for symbol in sorted(set(target_map) | set(realized_map)):
            weight_rows.append(
                {
                    'timestamp': row.timestamp,
                    'symbol': symbol,
                    'target_weight_pct': target_map.get(symbol, 0.0),
                    'realized_weight_pct': realized_map.get(symbol, 0.0),
                }
            )
    weights_df = pd.DataFrame(weight_rows)
    if not weights_df.empty:
        st.markdown("**📊 Target vs Realized Weights**")
        weights_fig = go.Figure()
        for symbol, frame in weights_df.groupby('symbol'):
            weights_fig.add_trace(
                go.Scatter(
                    x=frame['timestamp'],
                    y=frame['realized_weight_pct'],
                    mode='lines+markers',
                    name=f'{symbol} realized',
                )
            )
            weights_fig.add_trace(
                go.Scatter(
                    x=frame['timestamp'],
                    y=frame['target_weight_pct'],
                    mode='lines',
                    name=f'{symbol} target',
                    line=dict(dash='dash'),
                )
            )
        weights_fig.update_layout(height=450, yaxis_title='Weight (%)')
        st.plotly_chart(weights_fig, use_container_width=True)

    rebalance_table = diagnostics.loc[
        :, ['timestamp', 'rebalanced', 'rebalance_reason', 'turnover_pct', 'max_abs_drift_pct', 'cash_weight_pct']
    ].copy()
    rebalance_table['timestamp'] = rebalance_table['timestamp'].dt.strftime('%Y-%m-%d')
    st.markdown("**🗓️ Rebalance Timeline**")
    st.dataframe(rebalance_table, use_container_width=True, hide_index=True)

    if constraint_hits:
        st.markdown("**🧱 Constraint Hits**")
        constraint_df = pd.DataFrame(constraint_hits)
        if 'timestamp' in constraint_df.columns:
            constraint_df['timestamp'] = pd.to_datetime(constraint_df['timestamp']).dt.strftime('%Y-%m-%d')
        st.dataframe(constraint_df, use_container_width=True, hide_index=True)

def show_scenario_analysis():
    """Show scenario analysis"""
    st.subheader("📈 Scenario Analysis")
    
    st.markdown("Analyze how your strategy performs under different market conditions.")
    
    col1, col2 = st.columns(2)
    
    with col1:
        st.markdown("**🎭 Market Scenarios**")
        
        scenario_type = st.selectbox(
            "Scenario Type",
            ["Bull Market", "Bear Market", "Sideways Market", "High Volatility", "Low Volatility", "Custom"]
        )
        
        if scenario_type == "Custom":
            st.markdown("**Custom Scenario Parameters**")
            market_trend = st.slider("Market Trend (% daily)", -2.0, 2.0, 0.0, step=0.1)
            volatility = st.slider("Volatility (% daily)", 0.5, 5.0, 1.5, step=0.1)
            duration = st.number_input("Duration (days)", 30, 365, 90)
        
        # Scenario analysis settings
        st.markdown("**⚙️ Analysis Settings**")
        
        num_simulations = st.number_input("Number of Simulations", 100, 10000, 1000, step=100)
        confidence_interval = st.slider("Confidence Interval", 90, 99, 95)
        
        if st.button("🎭 Run Scenario Analysis"):
            run_scenario_analysis(scenario_type, num_simulations, confidence_interval)
    
    with col2:
        st.markdown("**📊 Monte Carlo Simulation**")
        
        if st.session_state.backtest_results:
            show_monte_carlo_results()
        else:
            st.info("Run a scenario analysis to see Monte Carlo results here.")

def run_scenario_analysis(scenario_type, num_simulations, confidence_interval):
    """Run scenario analysis simulation"""
    
    with st.spinner(f"Running {num_simulations} Monte Carlo simulations..."):
        # Simulate different outcomes
        outcomes = []
        
        for i in range(min(num_simulations, 100)):  # Limit for demo
            # Generate random outcome based on scenario
            if scenario_type == "Bull Market":
                outcome = np.random.normal(15, 10)  # 15% mean, 10% std
            elif scenario_type == "Bear Market":
                outcome = np.random.normal(-10, 15)  # -10% mean, 15% std
            elif scenario_type == "Sideways Market":
                outcome = np.random.normal(2, 8)  # 2% mean, 8% std
            elif scenario_type == "High Volatility":
                outcome = np.random.normal(5, 25)  # 5% mean, 25% std
            elif scenario_type == "Low Volatility":
                outcome = np.random.normal(6, 5)  # 6% mean, 5% std
            else:
                outcome = np.random.normal(0, 15)  # Default
            
            outcomes.append(outcome)
        
        # Calculate statistics
        mean_outcome = np.mean(outcomes)
        std_outcome = np.std(outcomes)
        
        # Confidence intervals
        lower_bound = np.percentile(outcomes, (100 - confidence_interval) / 2)
        upper_bound = np.percentile(outcomes, 100 - (100 - confidence_interval) / 2)
        
        # Display results
        st.success("✅ Scenario analysis completed!")
        
        col1, col2, col3 = st.columns(3)
        with col1:
            st.metric("Expected Return", f"{mean_outcome:.1f}%")
        with col2:
            st.metric("Risk (Std Dev)", f"{std_outcome:.1f}%")
        with col3:
            st.metric(f"{confidence_interval}% Range", f"{lower_bound:.1f}% to {upper_bound:.1f}%")
        
        # Store results for visualization
        st.session_state.scenario_results = {
            'outcomes': outcomes,
            'scenario_type': scenario_type,
            'stats': {
                'mean': mean_outcome,
                'std': std_outcome,
                'lower': lower_bound,
                'upper': upper_bound
            }
        }

def show_monte_carlo_results():
    """Show Monte Carlo simulation results"""
    
    if 'scenario_results' not in st.session_state:
        st.info("No scenario analysis results available. Run a scenario analysis first.")
        return
    
    results = st.session_state.scenario_results
    outcomes = results['outcomes']
    scenario_type = results['scenario_type']
    
    # Distribution chart
    fig = go.Figure()
    
    fig.add_trace(go.Histogram(
        x=outcomes,
        nbinsx=30,
        name=f'{scenario_type} Outcomes',
        opacity=0.7
    ))
    
    # Add confidence interval lines
    stats = results['stats']
    fig.add_vline(x=stats['lower'], line_dash="dash", line_color="red", 
                 annotation_text=f"Lower bound: {stats['lower']:.1f}%")
    fig.add_vline(x=stats['upper'], line_dash="dash", line_color="green", 
                 annotation_text=f"Upper bound: {stats['upper']:.1f}%")
    fig.add_vline(x=stats['mean'], line_dash="solid", line_color="blue", 
                 annotation_text=f"Mean: {stats['mean']:.1f}%")
    
    fig.update_layout(
        title=f"Monte Carlo Results: {scenario_type}",
        xaxis_title="Return (%)",
        yaxis_title="Frequency",
        height=400
    )
    
    st.plotly_chart(fig, use_container_width=True)
    
    # Risk metrics
    st.markdown("**⚖️ Risk Metrics**")
    
    prob_loss = len([x for x in outcomes if x < 0]) / len(outcomes) * 100
    prob_big_loss = len([x for x in outcomes if x < -20]) / len(outcomes) * 100
    
    risk_data = {
        "Metric": [
            "Probability of Loss",
            "Probability of >20% Loss",
            "Value at Risk (95%)",
            "Expected Shortfall",
            "Best Case (95%)",
            "Worst Case (5%)"
        ],
        "Value": [
            f"{prob_loss:.1f}%",
            f"{prob_big_loss:.1f}%",
            f"{np.percentile(outcomes, 5):.1f}%",
            f"{np.mean([x for x in outcomes if x < np.percentile(outcomes, 5)]):.1f}%",
            f"{np.percentile(outcomes, 95):.1f}%",
            f"{np.percentile(outcomes, 5):.1f}%"
        ]
    }
    
    st.dataframe(pd.DataFrame(risk_data), use_container_width=True, hide_index=True) 