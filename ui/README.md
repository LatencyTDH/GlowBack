# 🌟 GlowBack UI - Local Strategy Development Interface

A comprehensive Streamlit-based web interface for quantitative trading strategy development and backtesting.

## ✨ Features

### 📊 **Data Loader**
- **Sample Data Generation**: Create realistic market data for testing
- **CSV File Upload**: Import your own historical data
- **Alpha Vantage API**: Fetch real market data from Alpha Vantage
- **Manual Data Entry**: Create custom datasets point by point
- **Data Validation**: Automatic validation and preview of loaded data

### ⚙️ **Strategy Editor**
- **Code Editor**: Syntax-highlighted Python editor with auto-completion
- **Strategy Templates**: Pre-built strategies (Buy & Hold, MA Crossover, Momentum, Mean Reversion)
- **Real-time Validation**: Instant code validation and error checking
- **Strategy Configuration**: Configurable parameters and risk settings
- **Save/Load**: Save and load your custom strategies

### 🚀 **Backtest Runner**
- **Pre-flight Checks**: Automatic validation of data, strategy, and configuration
- **Real-time Progress**: Live progress tracking and execution logs
- **Multi-threading**: Non-blocking backtest execution
- **Error Handling**: Comprehensive error reporting and debugging
- **Quick Results**: Immediate performance summary

### 📈 **Results Dashboard**
- **Performance Charts**: Interactive equity curve, returns, and drawdown analysis
- **Risk Metrics**: Comprehensive risk analysis including VaR, Sharpe ratio, and drawdown
- **Trade Analysis**: Detailed trade history and execution analysis
- **Monthly Heatmaps**: Visual performance breakdown by time periods
- **Export Options**: Download results as CSV for further analysis

### 💼 **Portfolio Analyzer**
- **Performance Attribution**: Breakdown of strategy performance components
- **Advanced Risk Analysis**: VaR, CVaR, and scenario analysis
- **Monte Carlo Simulation**: Stress testing under different market conditions
- **Optimization Tools**: Parameter optimization and walk-forward analysis (planned)
- **Risk Management**: Position sizing and Kelly criterion calculations

## 🚀 Quick Start

### Prerequisites

- **Python 3.8+**
- **Rust toolchain** (for full functionality with GlowBack core)

### Installation & Launch

1. **Clone the repository:**
   ```bash
   cd glowback/ui
   ```

2. **Run the setup script:**
   ```bash
   python setup.py
   ```
   
   This will:
   - Install all required Python packages
   - Check for Rust bindings
   - Launch the Streamlit UI automatically

3. **Manual installation (alternative):**
   ```bash
   pip install -r requirements.txt
   streamlit run app.py
   ```

4. **Access the UI:**
   Open your browser to `http://localhost:8501`

## 📖 Usage Guide

### 1. Loading Data

**Sample Data (Recommended for testing):**
1. Go to "📊 Data Loader"
2. Select "Sample Data"
3. Configure symbol, time period, and volatility
4. Click "🎲 Generate Sample Data"

**CSV Upload:**
1. Prepare a CSV file with OHLCV data
2. Select "CSV Upload"
3. Upload your file and map columns
4. Configure date format and symbol
5. Click "📊 Load CSV Data"

**Alpha Vantage API:**
1. Get a free API key from [Alpha Vantage](https://www.alphavantage.co/)
2. Select "Alpha Vantage API"
3. Enter your API key and symbol
4. Click "🌐 Fetch Data"

### 2. Creating Strategies

**Using Templates:**
1. Go to "⚙️ Strategy Editor"
2. Select a template from the dropdown
3. Click "📋 Load Template"
4. Customize the code in the editor

**Custom Strategy:**
```python
class MyStrategy:
    def __init__(self):
        self.name = "My Custom Strategy"
        self.position = 0
    
    def on_bar(self, bar, portfolio):
        """Called for each price bar"""
        # Your strategy logic here
        if bar.close > 100 and self.position == 0:
            shares = int(portfolio.cash / bar.close)
            portfolio.buy(bar.symbol, shares, bar.close, bar.timestamp)
            self.position = 1
            return [f"Bought {shares} shares at ${bar.close}"]
        
        return []
```

**Strategy Configuration:**
1. Set strategy name and initial capital
2. Configure commission and slippage
3. Set risk parameters
4. Click "💾 Save Config"

### 3. Running Backtests

1. Ensure data is loaded ✅
2. Ensure strategy is configured ✅
3. Go to "🚀 Backtest Runner"
4. Configure backtest settings (optional)
5. Click "🚀 Run Backtest"
6. Monitor progress in real-time
7. View quick results or go to detailed dashboard

### 4. Analyzing Results

**Quick Metrics:**
- Total Return, Sharpe Ratio, Max Drawdown
- Trade count and win rate
- Final portfolio composition

**Detailed Analysis:**
1. Go to "📈 Results Dashboard"
2. Explore interactive charts
3. Analyze risk metrics
4. Review trade history
5. Export results for further analysis

**Advanced Portfolio Analysis:**
1. Go to "💼 Portfolio Analyzer"
2. View performance attribution
3. Conduct scenario analysis
4. Run Monte Carlo simulations

## 🔧 Advanced Features

### Strategy Templates

**Buy and Hold:**
- Simple buy-and-hold strategy
- Good for benchmarking

**Moving Average Crossover:**
- Configurable short/long periods
- Trend-following strategy

**Momentum:**
- Price momentum-based decisions
- Configurable lookback and thresholds

**Mean Reversion:**
- Statistical arbitrage approach
- Z-score based entry/exit

### Data Sources

**Sample Data Generator:**
- Configurable volatility and trend
- Realistic OHLCV generation
- Perfect for strategy testing

**CSV Upload:**
- Flexible column mapping
- Multiple date formats
- Data validation and preview

**Alpha Vantage:**
- Real market data
- Daily, weekly, monthly frequencies
- Free tier: 5 calls/minute, 500 calls/day

### Export & Integration

**Export Formats:**
- CSV (equity curve, trades)
- JSON (full results)
- PDF reports (planned)

**Integration:**
- Rust core integration
- Python ecosystem compatibility
- Jupyter notebook export (planned)

## 🎯 Strategy Development Tips

### Best Practices

1. **Start Simple**: Begin with buy-and-hold or simple MA strategies
2. **Test with Sample Data**: Use generated data for initial testing
3. **Validate Logic**: Use the code validator before running backtests
4. **Monitor Risk**: Always check drawdown and risk metrics
5. **Compare Strategies**: Run multiple strategies on the same data

### Common Patterns

**Trend Following:**
```python
# Use moving averages or momentum indicators
if short_ma > long_ma:
    # Go long
    pass
```

**Mean Reversion:**
```python
# Use statistical measures
if z_score < -2:
    # Buy oversold
    pass
elif z_score > 2:
    # Sell overbought
    pass
```

**Risk Management:**
```python
# Position sizing
max_position = portfolio.value * 0.1  # 10% max position
shares = min(desired_shares, max_position / bar.close)
```

## 🛠️ Technical Details

### Architecture

- **Frontend**: Streamlit with custom CSS styling
- **Backend**: Python with async processing
- **Charts**: Plotly for interactive visualizations
- **Data**: Pandas for data manipulation
- **Validation**: Real-time Python code execution

### Performance

- **Multi-threading**: Non-blocking backtest execution
- **Memory Efficient**: Streaming data processing
- **Fast Charts**: Plotly WebGL rendering
- **Responsive UI**: Real-time progress updates

### Dependencies

```txt
streamlit>=1.35.0          # Web framework
plotly>=5.17.0            # Interactive charts
pandas>=2.1.0             # Data manipulation
numpy>=1.24.0             # Numerical computing
streamlit-ace>=0.1.1      # Code editor
streamlit-option-menu     # Navigation
```

## 🐛 Troubleshooting

### Common Issues

**Port Already in Use:**
```bash
streamlit run app.py --server.port=8502
```

**Missing Dependencies:**
```bash
pip install -r requirements.txt --upgrade
```

**Rust Bindings Not Found:**
- Ensure Rust toolchain is installed
- Some features will work without Rust (pure Python mode)

**Browser Issues:**
- Try incognito/private mode
- Clear browser cache
- Use Chrome/Firefox for best experience

### Getting Help

1. Check the error logs in the UI
2. Review the strategy validation messages
3. Ensure data format is correct
4. Check Python console for detailed errors

## 🤝 Contributing

We welcome contributions! The UI is designed to be modular and extensible:

- **New Strategy Templates**: Add to `strategy_editor.py`
- **Data Sources**: Extend `data_loader.py`
- **Chart Types**: Add to `results_dashboard.py`
- **Analysis Tools**: Extend `portfolio_analyzer.py`

## 📄 License

MIT License - see the main repository for details.

---

**🌟 Happy Strategy Development with GlowBack!** 