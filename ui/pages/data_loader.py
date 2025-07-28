"""
Data Loader Page - Load market data from various sources
"""

import streamlit as st
import pandas as pd
import plotly.graph_objects as go
from datetime import datetime, timedelta
import tempfile
import os
from pathlib import Path

def show():
    """Main data loader page"""
    
    st.title("📊 Data Loader")
    st.markdown("Load market data from various sources to fuel your backtesting strategies.")
    
    # Data source selection
    col1, col2 = st.columns([1, 2])
    
    with col1:
        st.subheader("🔌 Data Sources")
        data_source = st.selectbox(
            "Select Data Source",
            ["Sample Data", "CSV Upload", "Alpha Vantage API", "Manual Entry"],
            help="Choose how to load market data"
        )
    
    with col2:
        st.subheader("📈 Data Preview")
        if st.session_state.data_loaded and 'market_data' in st.session_state:
            df = st.session_state.market_data
            st.write(f"**Loaded:** {len(df)} bars from {df['timestamp'].min()} to {df['timestamp'].max()}")
            with st.expander("View Raw Data"):
                st.dataframe(df)
        else:
            st.info("No data loaded yet. Select a data source and configure it below.")
    
    st.markdown("---")
    
    # Data source specific configurations
    if data_source == "Sample Data":
        load_sample_data()
    elif data_source == "CSV Upload":
        load_csv_data()
    elif data_source == "Alpha Vantage API":
        load_alpha_vantage_data()
    elif data_source == "Manual Entry":
        load_manual_data()
    
    # Data validation and preview
    if st.session_state.data_loaded and 'market_data' in st.session_state:
        st.markdown("---")
        show_data_analysis()

def load_sample_data():
    """Generate sample data for testing"""
    st.subheader("🎲 Sample Data Generator")
    
    col1, col2, col3 = st.columns(3)
    
    with col1:
        symbol = st.text_input("Symbol", value="AAPL", help="Stock symbol")
        start_price = st.number_input("Starting Price", value=100.0, min_value=1.0)
    
    with col2:
        days = st.number_input("Number of Days", value=252, min_value=1, max_value=2000)
        volatility = st.slider("Volatility", 0.1, 1.0, 0.2, help="Daily price volatility")
    
    with col3:
        trend = st.slider("Trend", -0.1, 0.1, 0.0005, help="Daily trend (positive = upward)")
        resolution = st.selectbox("Resolution", ["1d", "1h", "5m"], index=0)
    
    if st.button("🎲 Generate Sample Data", type="primary"):
        with st.spinner("Generating sample data..."):
            try:
                # Generate sample data using Python
                import numpy as np
                
                dates = pd.date_range(start=datetime.now() - timedelta(days=days), periods=days, freq='D')
                
                # Generate price series with random walk
                np.random.seed(42)  # For reproducibility
                returns = np.random.normal(trend, volatility, days)
                prices = [start_price]
                
                for ret in returns[1:]:
                    new_price = prices[-1] * (1 + ret)
                    prices.append(max(new_price, 0.01))  # Prevent negative prices
                
                # Create OHLCV data
                data = []
                for i, (date, close) in enumerate(zip(dates, prices)):
                    open_price = close * np.random.uniform(0.995, 1.005)
                    high = max(open_price, close) * np.random.uniform(1.0, 1.02)
                    low = min(open_price, close) * np.random.uniform(0.98, 1.0)
                    volume = np.random.randint(10000, 1000000)
                    
                    data.append({
                        'timestamp': date,
                        'symbol': symbol,
                        'open': round(open_price, 2),
                        'high': round(high, 2),
                        'low': round(low, 2),
                        'close': round(close, 2),
                        'volume': volume,
                        'resolution': resolution
                    })
                
                df = pd.DataFrame(data)
                st.session_state.market_data = df
                st.session_state.data_loaded = True
                st.session_state.data_source = "Sample Data"
                st.session_state.symbol = symbol
                
                st.success(f"✅ Generated {len(df)} bars of sample data for {symbol}")
                st.rerun()
                
            except Exception as e:
                st.error(f"❌ Error generating sample data: {str(e)}")

def load_csv_data():
    """Load data from CSV file"""
    st.subheader("📄 CSV File Upload")
    
    uploaded_file = st.file_uploader(
        "Upload CSV File",
        type=['csv'],
        help="Upload a CSV file with OHLCV data"
    )
    
    if uploaded_file is not None:
        col1, col2 = st.columns(2)
        
        with col1:
            symbol = st.text_input("Symbol", value="UPLOADED", help="Symbol for the uploaded data")
            has_header = st.checkbox("File has header row", value=True)
        
        with col2:
            date_format = st.selectbox(
                "Date Format",
                ["%Y-%m-%d", "%m/%d/%Y", "%d/%m/%Y", "%Y-%m-%d %H:%M:%S"],
                help="Format of date/timestamp column"
            )
            resolution = st.selectbox("Resolution", ["1d", "1h", "5m"], index=0)
        
        # Column mapping
        st.markdown("**Column Mapping**")
        col_cols = st.columns(6)
        with col_cols[0]:
            date_col = st.selectbox("Date/Timestamp", ["timestamp", "date", "Date", "Timestamp"], index=0)
        with col_cols[1]:
            open_col = st.selectbox("Open", ["open", "Open", "OPEN"], index=0)
        with col_cols[2]:
            high_col = st.selectbox("High", ["high", "High", "HIGH"], index=0)
        with col_cols[3]:
            low_col = st.selectbox("Low", ["low", "Low", "LOW"], index=0)
        with col_cols[4]:
            close_col = st.selectbox("Close", ["close", "Close", "CLOSE"], index=0)
        with col_cols[5]:
            volume_col = st.selectbox("Volume", ["volume", "Volume", "VOLUME"], index=0)
        
        if st.button("📊 Load CSV Data", type="primary"):
            try:
                # Read the CSV file
                df = pd.read_csv(uploaded_file, header=0 if has_header else None)
                
                # Preview first few rows
                st.write("**CSV Preview:**")
                st.dataframe(df.head())
                
                # Process the data
                processed_data = []
                for _, row in df.iterrows():
                    try:
                        timestamp = pd.to_datetime(row[date_col], format=date_format)
                        processed_data.append({
                            'timestamp': timestamp,
                            'symbol': symbol,
                            'open': float(row[open_col]),
                            'high': float(row[high_col]),
                            'low': float(row[low_col]),
                            'close': float(row[close_col]),
                            'volume': int(row[volume_col]) if volume_col in row else 0,
                            'resolution': resolution
                        })
                    except Exception as e:
                        st.warning(f"Skipping row due to error: {e}")
                        continue
                
                if processed_data:
                    market_df = pd.DataFrame(processed_data)
                    market_df = market_df.sort_values('timestamp')
                    
                    st.session_state.market_data = market_df
                    st.session_state.data_loaded = True
                    st.session_state.data_source = "CSV Upload"
                    st.session_state.symbol = symbol
                    
                    st.success(f"✅ Loaded {len(market_df)} bars from CSV file")
                    st.rerun()
                else:
                    st.error("❌ No valid data found in CSV file")
                    
            except Exception as e:
                st.error(f"❌ Error loading CSV file: {str(e)}")

def load_alpha_vantage_data():
    """Load data from Alpha Vantage API"""
    st.subheader("🌐 Alpha Vantage API")
    
    col1, col2 = st.columns(2)
    
    with col1:
        api_key = st.text_input("API Key", type="password", help="Get your free API key from Alpha Vantage")
        symbol = st.text_input("Symbol", value="AAPL", help="Stock symbol (e.g., AAPL, MSFT)")
    
    with col2:
        function = st.selectbox(
            "Data Function",
            ["TIME_SERIES_DAILY", "TIME_SERIES_WEEKLY", "TIME_SERIES_MONTHLY"],
            help="Type of time series data to fetch"
        )
        output_size = st.selectbox("Output Size", ["compact", "full"], help="Compact = last 100 points, Full = all available")
    
    if st.button("🌐 Fetch Data", type="primary", disabled=not api_key):
        if not api_key:
            st.error("❌ Please provide an Alpha Vantage API key")
            return
            
        with st.spinner(f"Fetching data for {symbol}..."):
            try:
                import requests
                
                # Alpha Vantage API call
                url = f"https://www.alphavantage.co/query"
                params = {
                    'function': function,
                    'symbol': symbol,
                    'apikey': api_key,
                    'outputsize': output_size,
                    'datatype': 'json'
                }
                
                response = requests.get(url, params=params)
                data = response.json()
                
                # Check for API errors
                if "Error Message" in data:
                    st.error(f"❌ API Error: {data['Error Message']}")
                    return
                if "Note" in data:
                    st.warning(f"⚠️ API Limit: {data['Note']}")
                    return
                
                # Extract time series data
                time_series_key = list(data.keys())[1]  # Usually the second key
                time_series = data[time_series_key]
                
                # Process the data
                processed_data = []
                for date_str, values in time_series.items():
                    timestamp = pd.to_datetime(date_str)
                    processed_data.append({
                        'timestamp': timestamp,
                        'symbol': symbol,
                        'open': float(values['1. open']),
                        'high': float(values['2. high']),
                        'low': float(values['3. low']),
                        'close': float(values['4. close']),
                        'volume': int(values['5. volume']),
                        'resolution': '1d'  # Alpha Vantage daily data
                    })
                
                df = pd.DataFrame(processed_data)
                df = df.sort_values('timestamp')
                
                st.session_state.market_data = df
                st.session_state.data_loaded = True
                st.session_state.data_source = "Alpha Vantage API"
                st.session_state.symbol = symbol
                
                st.success(f"✅ Fetched {len(df)} bars for {symbol}")
                st.rerun()
                
            except Exception as e:
                st.error(f"❌ Error fetching data: {str(e)}")

def load_manual_data():
    """Manually enter data points"""
    st.subheader("✍️ Manual Data Entry")
    
    col1, col2 = st.columns(2)
    with col1:
        symbol = st.text_input("Symbol", value="MANUAL")
    with col2:
        resolution = st.selectbox("Resolution", ["1d", "1h", "5m"])
    
    # Initialize manual data in session state
    if 'manual_data' not in st.session_state:
        st.session_state.manual_data = []
    
    # Data entry form
    with st.form("manual_entry"):
        st.markdown("**Add Data Point**")
        col1, col2, col3, col4, col5, col6 = st.columns(6)
        
        with col1:
            date = st.date_input("Date", value=datetime.now().date())
        with col2:
            open_price = st.number_input("Open", value=100.0, min_value=0.01)
        with col3:
            high_price = st.number_input("High", value=105.0, min_value=0.01)
        with col4:
            low_price = st.number_input("Low", value=95.0, min_value=0.01)
        with col5:
            close_price = st.number_input("Close", value=102.0, min_value=0.01)
        with col6:
            volume = st.number_input("Volume", value=100000, min_value=0)
        
        submitted = st.form_submit_button("➕ Add Data Point")
        
        if submitted:
            # Validate data
            if high_price < max(open_price, close_price) or low_price > min(open_price, close_price):
                st.error("❌ Invalid OHLC data: High/Low prices don't match Open/Close")
            else:
                new_point = {
                    'timestamp': pd.to_datetime(date),
                    'symbol': symbol,
                    'open': open_price,
                    'high': high_price,
                    'low': low_price,
                    'close': close_price,
                    'volume': volume,
                    'resolution': resolution
                }
                st.session_state.manual_data.append(new_point)
                st.success("✅ Data point added!")
    
    # Show current manual data
    if st.session_state.manual_data:
        st.markdown("**Current Manual Data**")
        df = pd.DataFrame(st.session_state.manual_data)
        st.dataframe(df)
        
        col1, col2 = st.columns(2)
        with col1:
            if st.button("💾 Save Manual Data"):
                df = df.sort_values('timestamp')
                st.session_state.market_data = df
                st.session_state.data_loaded = True
                st.session_state.data_source = "Manual Entry"
                st.session_state.symbol = symbol
                st.success(f"✅ Saved {len(df)} manual data points")
                st.rerun()
        
        with col2:
            if st.button("🗑️ Clear Manual Data"):
                st.session_state.manual_data = []
                st.success("✅ Manual data cleared")
                st.rerun()

def show_data_analysis():
    """Show analysis of loaded data"""
    st.subheader("📊 Data Analysis")
    
    df = st.session_state.market_data
    
    # Basic statistics
    col1, col2, col3, col4 = st.columns(4)
    
    with col1:
        st.metric("Total Bars", len(df))
    with col2:
        st.metric("Date Range", f"{(df['timestamp'].max() - df['timestamp'].min()).days} days")
    with col3:
        price_change = ((df['close'].iloc[-1] - df['close'].iloc[0]) / df['close'].iloc[0] * 100)
        st.metric("Total Return", f"{price_change:.2f}%")
    with col4:
        daily_returns = df['close'].pct_change().dropna()
        volatility = daily_returns.std() * (252 ** 0.5) * 100  # Annualized
        st.metric("Volatility (Ann.)", f"{volatility:.2f}%")
    
    # Price chart
    fig = go.Figure()
    
    # Candlestick chart
    fig.add_trace(go.Candlestick(
        x=df['timestamp'],
        open=df['open'],
        high=df['high'],
        low=df['low'],
        close=df['close'],
        name="Price"
    ))
    
    fig.update_layout(
        title=f"{st.session_state.symbol} Price Chart",
        xaxis_title="Date",
        yaxis_title="Price",
        height=400,
        showlegend=False
    )
    
    st.plotly_chart(fig, use_container_width=True)
    
    # Volume chart
    vol_fig = go.Figure()
    vol_fig.add_trace(go.Bar(
        x=df['timestamp'],
        y=df['volume'],
        name="Volume",
        marker_color='rgba(55, 128, 191, 0.7)'
    ))
    
    vol_fig.update_layout(
        title="Volume",
        xaxis_title="Date",
        yaxis_title="Volume",
        height=200
    )
    
    st.plotly_chart(vol_fig, use_container_width=True) 