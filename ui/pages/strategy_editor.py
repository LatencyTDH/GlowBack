"""
Strategy Editor Page - Create and edit trading strategies
"""

import streamlit as st
from streamlit_ace import st_ace

# Strategy templates
STRATEGY_TEMPLATES = {
    "Buy and Hold": '''
# Buy and Hold Strategy
# Simply buys and holds the asset

class BuyAndHoldStrategy:
    def __init__(self):
        self.position_opened = False
        self.name = "Buy and Hold"
        
    def on_bar(self, bar, portfolio):
        """Called on each new bar"""
        if not self.position_opened and portfolio.cash > 0:
            # Buy with 95% of available cash
            shares = int(portfolio.cash * 0.95 / bar.close)
            if shares > 0:
                portfolio.buy(bar.symbol, shares, bar.close, bar.timestamp)
                self.position_opened = True
        
        return []  # No specific actions
    ''',
    
    "Moving Average Crossover": '''
# Moving Average Crossover Strategy
# Buys when short MA crosses above long MA, sells when it crosses below

class MovingAverageCrossover:
    def __init__(self, short_window=10, long_window=20):
        self.short_window = short_window
        self.long_window = long_window
        self.prices = []
        self.position = 0
        self.name = f"MA Crossover ({short_window}/{long_window})"
        
    def calculate_ma(self, window):
        """Calculate moving average"""
        if len(self.prices) < window:
            return None
        return sum(self.prices[-window:]) / window
    
    def on_bar(self, bar, portfolio):
        """Called on each new bar"""
        self.prices.append(bar.close)
        
        # Only calculate when we have enough data
        if len(self.prices) < self.long_window:
            return []
        
        short_ma = self.calculate_ma(self.short_window)
        long_ma = self.calculate_ma(self.long_window)
        
        if short_ma and long_ma:
            # Buy signal: short MA crosses above long MA
            if short_ma > long_ma and self.position <= 0:
                shares = int(portfolio.cash * 0.95 / bar.close)
                if shares > 0:
                    portfolio.buy(bar.symbol, shares, bar.close, bar.timestamp)
                    self.position = 1
                    return [f"BUY: {shares} shares at ${bar.close}"]
            
            # Sell signal: short MA crosses below long MA
            elif short_ma < long_ma and self.position > 0:
                current_position = portfolio.get_position(bar.symbol)
                if current_position > 0:
                    portfolio.sell(bar.symbol, current_position, bar.close, bar.timestamp)
                    self.position = -1
                    return [f"SELL: {current_position} shares at ${bar.close}"]
        
        return []
    ''',
    
    "Mean Reversion": '''
# Mean Reversion Strategy
# Buys when price is below average, sells when above

import statistics

class MeanReversionStrategy:
    def __init__(self, lookback=20, threshold=0.02):
        self.lookback = lookback
        self.threshold = threshold  # 2% threshold
        self.prices = []
        self.name = f"Mean Reversion (lookback={lookback})"
        
    def on_bar(self, bar, portfolio):
        """Called on each new bar"""
        self.prices.append(bar.close)
        
        # Keep only the lookback window
        if len(self.prices) > self.lookback:
            self.prices = self.prices[-self.lookback:]
        
        # Need enough data for calculation
        if len(self.prices) < self.lookback:
            return []
        
        # Calculate mean and standard deviation
        mean_price = statistics.mean(self.prices)
        std_dev = statistics.stdev(self.prices)
        
        if std_dev == 0:
            return []
        
        # Calculate z-score
        z_score = (bar.close - mean_price) / std_dev
        
        current_position = portfolio.get_position(bar.symbol)
        
        # Buy when price is significantly below mean (z-score < -threshold)
        if z_score < -self.threshold and current_position == 0:
            shares = int(portfolio.cash * 0.3 / bar.close)  # Use 30% of cash
            if shares > 0:
                portfolio.buy(bar.symbol, shares, bar.close, bar.timestamp)
                return [f"BUY: {shares} shares at ${bar.close} (z-score: {z_score:.2f})"]
        
        # Sell when price is significantly above mean (z-score > threshold)
        elif z_score > self.threshold and current_position > 0:
            portfolio.sell(bar.symbol, current_position, bar.close, bar.timestamp)
            return [f"SELL: {current_position} shares at ${bar.close} (z-score: {z_score:.2f})"]
        
        return []
    ''',
    
    "Momentum": '''
# Momentum Strategy
# Follows price momentum and trends

class MomentumStrategy:
    def __init__(self, lookback=10, momentum_threshold=0.05):
        self.lookback = lookback
        self.momentum_threshold = momentum_threshold  # 5% momentum threshold
        self.prices = []
        self.position = 0
        self.name = f"Momentum (lookback={lookback})"
        
    def calculate_momentum(self):
        """Calculate price momentum over lookback period"""
        if len(self.prices) < self.lookback:
            return 0
        
        start_price = self.prices[-self.lookback]
        end_price = self.prices[-1]
        
        if start_price == 0:
            return 0
            
        return (end_price - start_price) / start_price
    
    def on_bar(self, bar, portfolio):
        """Called on each new bar"""
        self.prices.append(bar.close)
        
        # Need enough data for calculation
        if len(self.prices) < self.lookback:
            return []
        
        momentum = self.calculate_momentum()
        current_position = portfolio.get_position(bar.symbol)
        
        # Strong positive momentum - go long
        if momentum > self.momentum_threshold and self.position <= 0:
            shares = int(portfolio.cash * 0.8 / bar.close)  # Use 80% of cash
            if shares > 0:
                portfolio.buy(bar.symbol, shares, bar.close, bar.timestamp)
                self.position = 1
                return [f"BUY: {shares} shares at ${bar.close} (momentum: {momentum*100:.1f}%)"]
        
        # Strong negative momentum - close position
        elif momentum < -self.momentum_threshold and current_position > 0:
            portfolio.sell(bar.symbol, current_position, bar.close, bar.timestamp)
            self.position = 0
            return [f"SELL: {current_position} shares at ${bar.close} (momentum: {momentum*100:.1f}%)"]
        
        return []
    '''
}

def show():
    """Main strategy editor page"""
    
    st.title("⚙️ Strategy Editor")
    st.markdown("Create and customize your trading strategies with Python code.")
    
    # Strategy selection and templates
    col1, col2 = st.columns([1, 2])
    
    with col1:
        st.subheader("🎯 Strategy Templates")
        
        selected_template = st.selectbox(
            "Choose Template",
            list(STRATEGY_TEMPLATES.keys()) + ["Custom Strategy"],
            help="Select a pre-built strategy template or start from scratch"
        )
        
        if st.button("📋 Load Template"):
            if selected_template in STRATEGY_TEMPLATES:
                st.session_state.strategy_code = STRATEGY_TEMPLATES[selected_template]
                st.success(f"✅ Loaded {selected_template} template")
                st.rerun()
            else:
                st.session_state.strategy_code = "# Your custom strategy here\n\nclass CustomStrategy:\n    def __init__(self):\n        self.name = 'Custom Strategy'\n    \n    def on_bar(self, bar, portfolio):\n        # Implement your strategy logic here\n        return []\n"
                st.success("✅ Loaded custom strategy template")
                st.rerun()
        
        # Strategy configuration
        st.markdown("---")
        st.subheader("⚙️ Configuration")
        
        strategy_name = st.text_input("Strategy Name", value="My Strategy")
        initial_capital = st.number_input("Initial Capital", value=100000, min_value=1000, step=1000)
        
        # Advanced settings
        with st.expander("Advanced Settings"):
            commission = st.number_input("Commission per Trade", value=0.001, min_value=0.0, format="%.4f")
            slippage = st.number_input("Slippage (bps)", value=5, min_value=0)
            max_position_size = st.slider("Max Position Size (%)", 1, 100, 95)
        
        # Save configuration
        if st.button("💾 Save Config"):
            st.session_state.strategy_config = {
                "name": strategy_name,
                "initial_capital": initial_capital,
                "commission": commission,
                "slippage": slippage,
                "max_position_size": max_position_size / 100
            }
            st.success("✅ Configuration saved!")
    
    with col2:
        st.subheader("💻 Code Editor")
        
        # Initialize strategy code if not exists
        if 'strategy_code' not in st.session_state:
            st.session_state.strategy_code = STRATEGY_TEMPLATES["Buy and Hold"]
        
        # Code editor with syntax highlighting
        strategy_code = st_ace(
            value=st.session_state.strategy_code,
            language='python',
            theme='github',
            key="strategy_editor",
            height=400,
            auto_update=True,
            font_size=14,
            tab_size=4,
            wrap=False,
            annotations=None,
            markers=None,
        )
        
        # Update session state when code changes
        if strategy_code != st.session_state.strategy_code:
            st.session_state.strategy_code = strategy_code
        
        # Code validation
        col1, col2, col3 = st.columns(3)
        
        with col1:
            if st.button("✅ Validate Code"):
                validate_strategy_code(strategy_code)
        
        with col2:
            if st.button("💾 Save Strategy"):
                save_strategy_code(strategy_code, strategy_name)
        
        with col3:
            if st.button("📂 Load Strategy"):
                load_saved_strategy()
    
    st.markdown("---")
    
    # Documentation and help
    show_strategy_documentation()

def validate_strategy_code(code):
    """Validate the strategy code"""
    try:
        # Basic syntax check
        compile(code, '<string>', 'exec')
        
        # Check for required methods and structure
        namespace = {}
        exec(code, namespace)
        
        # Find strategy classes
        strategy_classes = [obj for obj in namespace.values() 
                          if isinstance(obj, type) and hasattr(obj, 'on_bar')]
        
        if not strategy_classes:
            st.error("❌ No strategy class found. Your strategy must have an 'on_bar' method.")
            return False
        
        strategy_class = strategy_classes[0]
        
        # Check required methods
        if not hasattr(strategy_class, 'on_bar'):
            st.error("❌ Strategy class must have an 'on_bar' method.")
            return False
        
        # Try to instantiate
        try:
            strategy_instance = strategy_class()
            st.success("✅ Strategy code is valid!")
            
            # Show strategy info
            if hasattr(strategy_instance, 'name'):
                st.info(f"📊 Strategy: {strategy_instance.name}")
            
            return True
            
        except Exception as e:
            st.error(f"❌ Error creating strategy instance: {str(e)}")
            return False
            
    except SyntaxError as e:
        st.error(f"❌ Syntax Error: {str(e)}")
        return False
    except Exception as e:
        st.error(f"❌ Validation Error: {str(e)}")
        return False

def save_strategy_code(code, name):
    """Save strategy code to session state"""
    if 'saved_strategies' not in st.session_state:
        st.session_state.saved_strategies = {}
    
    st.session_state.saved_strategies[name] = code
    st.success(f"✅ Strategy '{name}' saved!")

def load_saved_strategy():
    """Load a saved strategy"""
    if 'saved_strategies' not in st.session_state or not st.session_state.saved_strategies:
        st.warning("⚠️ No saved strategies found.")
        return
    
    strategy_name = st.selectbox("Select Saved Strategy", list(st.session_state.saved_strategies.keys()))
    
    if st.button("📂 Load Selected Strategy"):
        st.session_state.strategy_code = st.session_state.saved_strategies[strategy_name]
        st.success(f"✅ Loaded strategy '{strategy_name}'")
        st.rerun()

def show_strategy_documentation():
    """Show strategy development documentation"""
    st.subheader("📚 Strategy Development Guide")
    
    tab1, tab2, tab3 = st.tabs(["📖 Basics", "🔧 API Reference", "💡 Examples"])
    
    with tab1:
        st.markdown("""
        ### Strategy Structure
        
        Every strategy must be a Python class with an `on_bar` method:
        
        ```python
        class MyStrategy:
            def __init__(self):
                self.name = "My Strategy"
                # Initialize your strategy variables here
            
            def on_bar(self, bar, portfolio):
                # Your strategy logic here
                # Return list of log messages (optional)
                return []
        ```
        
        ### Available Data
        
        **Bar Object:**
        - `bar.timestamp` - Date/time of the bar
        - `bar.symbol` - Symbol (e.g., "AAPL")
        - `bar.open`, `bar.high`, `bar.low`, `bar.close` - OHLC prices
        - `bar.volume` - Trading volume
        
        **Portfolio Object:**
        - `portfolio.cash` - Available cash
        - `portfolio.value` - Total portfolio value
        - `portfolio.buy(symbol, shares, price, timestamp)` - Place buy order
        - `portfolio.sell(symbol, shares, price, timestamp)` - Place sell order
        - `portfolio.get_position(symbol)` - Get current position size
        """)
    
    with tab2:
        st.markdown("""
        ### Portfolio API Reference
        
        ```python
        # Trading Methods
        portfolio.buy(symbol, shares, price, timestamp)     # Buy shares
        portfolio.sell(symbol, shares, price, timestamp)    # Sell shares
        
        # Information Methods
        portfolio.cash                           # Available cash
        portfolio.value                          # Total portfolio value
        portfolio.get_position(symbol)          # Position size for symbol
        portfolio.get_positions()               # All positions dict
        
        # Risk Management
        portfolio.total_equity                   # Total equity value
        portfolio.unrealized_pnl                # Unrealized P&L
        portfolio.realized_pnl                  # Realized P&L
        ```
        
        ### Bar Data Structure
        
        ```python
        bar.timestamp    # datetime object
        bar.symbol       # string (e.g., "AAPL")
        bar.open         # float
        bar.high         # float
        bar.low          # float
        bar.close        # float
        bar.volume       # int
        bar.resolution   # string (e.g., "1d")
        ```
        """)
    
    with tab3:
        st.markdown("""
        ### Example Strategies
        
        **Simple Moving Average:**
        ```python
        class SimpleMA:
            def __init__(self, period=20):
                self.period = period
                self.prices = []
                self.name = f"Simple MA {period}"
            
            def on_bar(self, bar, portfolio):
                self.prices.append(bar.close)
                if len(self.prices) > self.period:
                    self.prices.pop(0)
                
                if len(self.prices) == self.period:
                    ma = sum(self.prices) / self.period
                    if bar.close > ma and portfolio.get_position(bar.symbol) == 0:
                        shares = int(portfolio.cash / bar.close)
                        portfolio.buy(bar.symbol, shares, bar.close, bar.timestamp)
                        return [f"Bought {shares} shares at ${bar.close}"]
                return []
        ```
        
        **RSI Strategy:**
        ```python
        class RSIStrategy:
            def __init__(self, period=14, oversold=30, overbought=70):
                self.period = period
                self.oversold = oversold
                self.overbought = overbought
                self.prices = []
                self.name = f"RSI ({period})"
            
            def calculate_rsi(self):
                if len(self.prices) < self.period + 1:
                    return 50  # Neutral RSI
                
                gains = []
                losses = []
                
                for i in range(1, len(self.prices)):
                    change = self.prices[i] - self.prices[i-1]
                    if change > 0:
                        gains.append(change)
                        losses.append(0)
                    else:
                        gains.append(0)
                        losses.append(-change)
                
                avg_gain = sum(gains[-self.period:]) / self.period
                avg_loss = sum(losses[-self.period:]) / self.period
                
                if avg_loss == 0:
                    return 100
                
                rs = avg_gain / avg_loss
                rsi = 100 - (100 / (1 + rs))
                return rsi
            
            def on_bar(self, bar, portfolio):
                self.prices.append(bar.close)
                rsi = self.calculate_rsi()
                
                position = portfolio.get_position(bar.symbol)
                
                if rsi < self.oversold and position == 0:
                    shares = int(portfolio.cash * 0.5 / bar.close)
                    portfolio.buy(bar.symbol, shares, bar.close, bar.timestamp)
                    return [f"RSI Oversold: Bought {shares} shares (RSI: {rsi:.1f})"]
                
                elif rsi > self.overbought and position > 0:
                    portfolio.sell(bar.symbol, position, bar.close, bar.timestamp)
                    return [f"RSI Overbought: Sold {position} shares (RSI: {rsi:.1f})"]
                
                return []
        ```
        """) 