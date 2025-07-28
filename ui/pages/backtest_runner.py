"""
Backtest Runner Page - Execute backtests and monitor progress
"""

import streamlit as st
import pandas as pd
import time
import threading
from datetime import datetime
import queue

class SimplePortfolio:
    """Simple portfolio implementation for backtesting"""
    
    def __init__(self, initial_cash=100000):
        self.initial_cash = initial_cash
        self.cash = initial_cash
        self.positions = {}
        self.trades = []
        self.equity_curve = []
        
    def buy(self, symbol, shares, price):
        """Buy shares"""
        cost = shares * price
        if cost <= self.cash:
            self.cash -= cost
            self.positions[symbol] = self.positions.get(symbol, 0) + shares
            self.trades.append({
                'timestamp': datetime.now(),
                'symbol': symbol,
                'action': 'BUY',
                'shares': shares,
                'price': price,
                'cost': cost
            })
            return True
        return False
    
    def sell(self, symbol, shares, price):
        """Sell shares"""
        if self.positions.get(symbol, 0) >= shares:
            proceeds = shares * price
            self.cash += proceeds
            self.positions[symbol] -= shares
            if self.positions[symbol] == 0:
                del self.positions[symbol]
            self.trades.append({
                'timestamp': datetime.now(),
                'symbol': symbol,
                'action': 'SELL',
                'shares': shares,
                'price': price,
                'proceeds': proceeds
            })
            return True
        return False
    
    def get_position(self, symbol):
        """Get position size for symbol"""
        return self.positions.get(symbol, 0)
    
    def get_positions(self):
        """Get all positions"""
        return self.positions.copy()
    
    def calculate_value(self, current_prices):
        """Calculate total portfolio value"""
        position_value = sum(shares * current_prices.get(symbol, 0) 
                           for symbol, shares in self.positions.items())
        return self.cash + position_value
    
    @property
    def value(self):
        """Total portfolio value (simplified)"""
        return self.cash + sum(self.positions.values()) * 100  # Simplified
    
    @property
    def total_equity(self):
        """Total equity"""
        return self.value
    
    @property
    def unrealized_pnl(self):
        """Unrealized P&L (simplified)"""
        return self.value - self.initial_cash
    
    @property
    def realized_pnl(self):
        """Realized P&L"""
        total_bought = sum(t['cost'] for t in self.trades if t['action'] == 'BUY')
        total_sold = sum(t.get('proceeds', 0) for t in self.trades if t['action'] == 'SELL')
        return total_sold - total_bought

class SimpleBar:
    """Simple bar data structure"""
    
    def __init__(self, timestamp, symbol, open_price, high, low, close, volume, resolution):
        self.timestamp = timestamp
        self.symbol = symbol
        self.open = open_price
        self.high = high
        self.low = low
        self.close = close
        self.volume = volume
        self.resolution = resolution

def run_backtest(strategy_code, market_data, config, progress_queue, log_queue):
    """Run backtest in separate thread"""
    try:
        # Execute strategy code
        namespace = {}
        exec(strategy_code, namespace)
        
        # Find strategy class
        strategy_classes = [obj for obj in namespace.values() 
                          if isinstance(obj, type) and hasattr(obj, 'on_bar')]
        
        if not strategy_classes:
            log_queue.put("ERROR: No strategy class found")
            return None
        
        # Initialize strategy and portfolio
        strategy_class = strategy_classes[0]
        strategy = strategy_class()
        portfolio = SimplePortfolio(config.get('initial_capital', 100000))
        
        log_queue.put(f"Started backtest: {getattr(strategy, 'name', 'Unknown Strategy')}")
        log_queue.put(f"Initial capital: ${portfolio.initial_cash:,.2f}")
        
        # Run backtest
        total_bars = len(market_data)
        equity_curve = []
        all_logs = []
        
        for i, row in market_data.iterrows():
            # Create bar object
            bar = SimpleBar(
                timestamp=row['timestamp'],
                symbol=row['symbol'],
                open_price=row['open'],
                high=row['high'],
                low=row['low'],
                close=row['close'],
                volume=row['volume'],
                resolution=row['resolution']
            )
            
            # Execute strategy
            try:
                logs = strategy.on_bar(bar, portfolio)
                if logs:
                    for log in logs:
                        log_queue.put(f"{bar.timestamp.strftime('%Y-%m-%d')}: {log}")
                        all_logs.append(f"{bar.timestamp.strftime('%Y-%m-%d')}: {log}")
            except Exception as e:
                log_queue.put(f"ERROR on {bar.timestamp}: {str(e)}")
            
            # Calculate portfolio value
            current_prices = {row['symbol']: row['close']}
            portfolio_value = portfolio.calculate_value(current_prices)
            
            equity_curve.append({
                'timestamp': row['timestamp'],
                'value': portfolio_value,
                'cash': portfolio.cash,
                'positions': sum(portfolio.positions.values()),
                'returns': (portfolio_value - portfolio.initial_cash) / portfolio.initial_cash * 100
            })
            
            # Update progress
            progress = (i + 1) / total_bars
            progress_queue.put(progress)
            
            # Small delay to show progress
            time.sleep(0.01)
        
        log_queue.put("Backtest completed!")
        
        # Calculate final metrics
        final_value = equity_curve[-1]['value']
        total_return = (final_value - portfolio.initial_cash) / portfolio.initial_cash * 100
        
        # Simple Sharpe ratio calculation
        returns = [point['returns'] for point in equity_curve[1:]]
        returns_series = pd.Series(returns)
        daily_returns = returns_series.pct_change().dropna()
        
        if len(daily_returns) > 0 and daily_returns.std() != 0:
            sharpe_ratio = daily_returns.mean() / daily_returns.std() * (252 ** 0.5)
        else:
            sharpe_ratio = 0
        
        # Max drawdown
        equity_values = [point['value'] for point in equity_curve]
        peak = equity_values[0]
        max_drawdown = 0
        
        for value in equity_values:
            if value > peak:
                peak = value
            drawdown = (peak - value) / peak
            if drawdown > max_drawdown:
                max_drawdown = drawdown
        
        results = {
            'equity_curve': equity_curve,
            'trades': portfolio.trades,
            'final_value': final_value,
            'total_return': total_return,
            'sharpe_ratio': sharpe_ratio,
            'max_drawdown': max_drawdown * 100,
            'total_trades': len(portfolio.trades),
            'final_cash': portfolio.cash,
            'final_positions': portfolio.positions,
            'logs': all_logs
        }
        
        return results
        
    except Exception as e:
        log_queue.put(f"FATAL ERROR: {str(e)}")
        return None

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