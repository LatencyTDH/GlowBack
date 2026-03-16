"""
Backtest Runner Page - Execute backtests and monitor progress
"""

import queue
import threading
import time

import pandas as pd
import streamlit as st

from backtest_core import run_backtest

def show():
    """Main backtest runner page"""
    
    st.title("🚀 Backtest Runner")
    st.markdown("Execute your trading strategies against historical data.")
    
    # Pre-flight checks
    col1, col2 = st.columns(2)
    
    with col1:
        st.subheader("🔍 Pre-flight Checks")
        
        # Check data
        data_status = "✅ Ready" if st.session_state.data_loaded else "❌ No Data"
        st.write(f"**Market Data:** {data_status}")
        
        if st.session_state.data_loaded:
            df = st.session_state.market_data
            st.write(f"  • Symbol: {st.session_state.symbol}")
            st.write(f"  • Bars: {len(df)}")
            st.write(f"  • Period: {df['timestamp'].min().date()} to {df['timestamp'].max().date()}")
        
        # Check strategy
        strategy_status = "✅ Ready" if 'strategy_code' in st.session_state else "❌ No Strategy"
        st.write(f"**Strategy Code:** {strategy_status}")
        
        # Check config
        config_status = "✅ Ready" if st.session_state.strategy_config else "❌ No Config"
        st.write(f"**Configuration:** {config_status}")
        
        if st.session_state.strategy_config:
            config = st.session_state.strategy_config
            st.write(f"  • Capital: ${config.get('initial_capital', 0):,.2f}")
            st.write(f"  • Commission: {config.get('commission', 0):.4f}")
    
    with col2:
        st.subheader("⚙️ Backtest Settings")
        
        # Advanced backtest settings
        with st.form("backtest_settings"):
            st.markdown("**Execution Settings**")
            
            col1, col2 = st.columns(2)
            with col1:
                start_date = st.date_input("Start Date", value=None)
                commission_override = st.number_input("Commission Override", value=0.001, format="%.4f")
            
            with col2:
                end_date = st.date_input("End Date", value=None)
                slippage_override = st.number_input("Slippage Override (bps)", value=5)
            
            benchmark_symbol = st.text_input("Benchmark Symbol", value="SPY", help="Symbol to compare against")
            
            submitted = st.form_submit_button("🚀 Run Backtest", type="primary")
    
    # Backtest execution
    if submitted:
        run_backtest_execution()
    
    # Show running backtest status
    if 'backtest_running' in st.session_state and st.session_state.backtest_running:
        show_backtest_progress()
    
    # Show completed results
    if st.session_state.backtest_results:
        st.markdown("---")
        show_quick_results()

def run_backtest_execution():
    """Execute the backtest"""
    
    # Validation
    if not st.session_state.data_loaded:
        st.error("❌ No market data loaded. Please go to Data Loader first.")
        return
    
    if 'strategy_code' not in st.session_state:
        st.error("❌ No strategy code found. Please go to Strategy Editor first.")
        return
    
    if not st.session_state.strategy_config:
        st.error("❌ No strategy configuration found. Please configure in Strategy Editor.")
        return
    
    try:
        # Initialize progress tracking
        st.session_state.backtest_running = True
        st.session_state.backtest_progress = 0
        st.session_state.backtest_logs = []
        
        # Create progress and log containers
        progress_container = st.container()
        log_container = st.container()
        
        with progress_container:
            st.info("🚀 Starting backtest...")
            progress_bar = st.progress(0)
            status_text = st.empty()
        
        with log_container:
            st.subheader("📋 Execution Log")
            log_placeholder = st.empty()
        
        # Prepare data and run backtest
        market_data = st.session_state.market_data.copy()
        strategy_code = st.session_state.strategy_code
        config = st.session_state.strategy_config
        
        # Create queues for communication
        progress_queue = queue.Queue()
        log_queue = queue.Queue()
        
        # Start backtest in thread
        result_container = [None]  # Mutable container for result
        
        def backtest_thread():
            result = run_backtest(strategy_code, market_data, config, progress_queue, log_queue)
            result_container[0] = result
        
        thread = threading.Thread(target=backtest_thread)
        thread.start()
        
        # Update progress and logs
        logs = []
        while thread.is_alive():
            # Update progress
            try:
                while True:
                    progress = progress_queue.get_nowait()
                    progress_bar.progress(progress)
                    status_text.text(f"Processing... {progress*100:.1f}%")
            except queue.Empty:
                pass
            
            # Update logs
            try:
                while True:
                    log = log_queue.get_nowait()
                    logs.append(log)
                    log_text = "\n".join(logs[-20:])  # Show last 20 logs
                    log_placeholder.text(log_text)
            except queue.Empty:
                pass
            
            time.sleep(0.1)
        
        # Wait for thread to complete
        thread.join()
        
        # Get final logs
        try:
            while True:
                log = log_queue.get_nowait()
                logs.append(log)
        except queue.Empty:
            pass
        
        # Process results
        result = result_container[0]
        if result:
            st.session_state.backtest_results = result
            st.session_state.backtest_logs = logs
            progress_bar.progress(1.0)
            status_text.text("✅ Backtest completed!")
            
            # Show success message
            st.success(f"🎉 Backtest completed! Final value: ${result['final_value']:,.2f}")
            
        else:
            st.error("❌ Backtest failed. Check the logs for details.")
        
        st.session_state.backtest_running = False
        
    except Exception as e:
        st.error(f"❌ Error running backtest: {str(e)}")
        st.session_state.backtest_running = False

def show_backtest_progress():
    """Show backtest progress"""
    st.subheader("⏳ Backtest in Progress")
    
    progress = st.session_state.get('backtest_progress', 0)
    st.progress(progress)
    st.write(f"Progress: {progress*100:.1f}%")
    
    if st.session_state.get('backtest_logs'):
        with st.expander("📋 Live Log", expanded=True):
            log_text = "\n".join(st.session_state.backtest_logs[-10:])
            st.text(log_text)

def show_quick_results():
    """Show quick results summary"""
    results = st.session_state.backtest_results
    
    st.subheader("📊 Quick Results")
    
    # Key metrics
    col1, col2, col3, col4 = st.columns(4)
    
    with col1:
        st.metric("Total Return", f"{results['total_return']:.2f}%")
    
    with col2:
        st.metric("Sharpe Ratio", f"{results['sharpe_ratio']:.2f}")
    
    with col3:
        st.metric("Max Drawdown", f"{results['max_drawdown']:.2f}%")
    
    with col4:
        st.metric("Total Trades", results['total_trades'])
    
    # Final portfolio
    col1, col2 = st.columns(2)
    
    with col1:
        st.write("**Final Portfolio:**")
        st.write(f"Cash: ${results['final_cash']:,.2f}")
        if results['final_positions']:
            for symbol, shares in results['final_positions'].items():
                st.write(f"{symbol}: {shares} shares")
    
    with col2:
        st.write("**Trade Summary:**")
        if results['trades']:
            trades_df = pd.DataFrame(results['trades'])
            buy_trades = len(trades_df[trades_df['action'] == 'BUY'])
            sell_trades = len(trades_df[trades_df['action'] == 'SELL'])
            st.write(f"Buy orders: {buy_trades}")
            st.write(f"Sell orders: {sell_trades}")
    
    # Quick actions
    col1, col2, col3 = st.columns(3)
    
    with col1:
        if st.button("📈 View Detailed Results"):
            st.info("💡 Please use the sidebar navigation to go to 'Results Dashboard'")
    
    with col2:
        if st.button("🔄 Run Another Backtest"):
            st.session_state.backtest_results = None
            st.rerun()
    
    with col3:
        if st.button("💾 Export Results"):
            export_results()

def export_results():
    """Export backtest results"""
    results = st.session_state.backtest_results
    
    # Convert to DataFrames
    equity_df = pd.DataFrame(results['equity_curve'])
    trades_df = pd.DataFrame(results['trades']) if results['trades'] else pd.DataFrame()
    
    # Create downloadable files
    col1, col2 = st.columns(2)
    
    with col1:
        if not equity_df.empty:
            csv_equity = equity_df.to_csv(index=False)
            st.download_button(
                label="📊 Download Equity Curve",
                data=csv_equity,
                file_name="backtest_equity_curve.csv",
                mime="text/csv"
            )
    
    with col2:
        if not trades_df.empty:
            csv_trades = trades_df.to_csv(index=False)
            st.download_button(
                label="📋 Download Trade History",
                data=csv_trades,
                file_name="backtest_trades.csv",
                mime="text/csv"
            ) 