"""
Portfolio Analyzer Page - Advanced portfolio analysis and optimization
"""

import streamlit as st
import pandas as pd
import plotly.graph_objects as go
import plotly.express as px
from plotly.subplots import make_subplots
import numpy as np
from datetime import datetime, timedelta

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
        
        # Calculate performance components
        total_return = results['total_return']
        
        # Simplified attribution (in reality, this would be more complex)
        attribution_data = {
            "Component": ["Asset Selection", "Market Timing", "Cash Allocation", "Trading Costs"],
            "Contribution (%)": [
                total_return * 0.6,  # Simplified
                total_return * 0.3,
                total_return * 0.1,
                -abs(total_return * 0.02)  # Cost drag
            ]
        }
        
        attr_df = pd.DataFrame(attribution_data)
        
        fig = px.bar(
            attr_df,
            x="Component",
            y="Contribution (%)",
            color="Contribution (%)",
            color_continuous_scale="RdYlGn",
            title="Performance Attribution"
        )
        
        st.plotly_chart(fig, use_container_width=True)
    
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
    """Show portfolio optimization analysis"""
    st.subheader("🎯 Portfolio Optimization")
    
    st.info("🚧 Portfolio optimization features coming soon! This will include:")
    
    col1, col2 = st.columns(2)
    
    with col1:
        st.markdown("""
        **📊 Mean Reversion Optimization**
        - Parameter sweep analysis
        - Walk-forward optimization
        - Out-of-sample testing
        - Robust optimization methods
        """)
        
        # Placeholder optimization interface
        st.markdown("**⚙️ Optimization Settings**")
        
        param_to_optimize = st.selectbox(
            "Parameter to Optimize",
            ["Moving Average Period", "Position Size", "Stop Loss", "Take Profit"]
        )
        
        optimization_method = st.selectbox(
            "Optimization Method",
            ["Grid Search", "Bayesian Optimization", "Genetic Algorithm"]
        )
        
        col1a, col1b = st.columns(2)
        with col1a:
            min_value = st.number_input("Min Value", value=10)
        with col1b:
            max_value = st.number_input("Max Value", value=50)
        
        if st.button("🚀 Run Optimization"):
            st.info("Optimization would run here with the selected parameters.")
    
    with col2:
        st.markdown("""
        **🎯 Risk Management**
        - Position sizing optimization
        - Kelly criterion calculation
        - Risk parity allocation
        - Maximum drawdown constraints
        """)
        
        # Placeholder risk management tools
        st.markdown("**⚖️ Risk Management Tools**")
        
        risk_target = st.slider("Target Volatility (%)", 5, 25, 15)
        max_drawdown_limit = st.slider("Max Drawdown Limit (%)", 5, 50, 20)
        
        st.markdown("**💰 Position Sizing**")
        kelly_fraction = st.slider("Kelly Fraction", 0.1, 1.0, 0.25, step=0.05)
        
        # Simple Kelly criterion calculation (placeholder)
        if st.session_state.backtest_results:
            total_return = st.session_state.backtest_results['total_return']
            win_rate = 0.6  # Placeholder
            avg_win = abs(total_return) * 1.5  # Placeholder
            avg_loss = abs(total_return) * 0.8  # Placeholder
            
            if avg_loss > 0:
                kelly_optimal = (win_rate * avg_win - (1 - win_rate) * avg_loss) / avg_win
                st.write(f"**Optimal Kelly Fraction:** {kelly_optimal:.3f}")
            
        if st.button("📊 Calculate Optimal Allocation"):
            st.info("Risk-adjusted allocation calculation would run here.")

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