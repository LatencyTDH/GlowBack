"""
Data Loader Page - Load market data from various sources
"""

import streamlit as st
import pandas as pd
import plotly.graph_objects as go
from datetime import datetime, timedelta

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
                ["ISO8601", "%Y-%m-%d", "%m/%d/%Y", "%d/%m/%Y", "%Y-%m-%d %H:%M:%S", "Auto Detect"],
                help="Format of date/timestamp column. Use 'Auto Detect' for automatic parsing."
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
                
                # Auto-detect timestamp format if requested
                if date_format == "Auto Detect":
                    from utils import detect_timestamp_format
                    sample_timestamps = df[date_col].head(5).tolist()
                    detected_format = detect_timestamp_format(sample_timestamps)
                    st.info(f"🔍 Auto-detected format: {detected_format}")
                    date_format = detected_format
                
                # Process the data
                processed_data = []
                error_count = 0
                total_rows = len(df)
                
                for idx, row in df.iterrows():
                    try:
                        # Handle different date format options
                        if date_format == "ISO8601":
                            timestamp = pd.to_datetime(row[date_col], format='ISO8601')
                        elif date_format == "Auto Detect":
                            timestamp = pd.to_datetime(row[date_col])
                        else:
                            timestamp = pd.to_datetime(row[date_col], format=date_format)
                        
                        # Validate OHLC data
                        open_price = float(row[open_col])
                        high_price = float(row[high_col])
                        low_price = float(row[low_col])
                        close_price = float(row[close_col])
                        
                        # Basic OHLC validation
                        if not (low_price <= open_price <= high_price and 
                               low_price <= close_price <= high_price):
                            st.warning(f"Row {idx+1}: Invalid OHLC data (low: {low_price}, open: {open_price}, high: {high_price}, close: {close_price})")
                            error_count += 1
                            continue
                        
                        processed_data.append({
                            'timestamp': timestamp,
                            'symbol': symbol,
                            'open': open_price,
                            'high': high_price,
                            'low': low_price,
                            'close': close_price,
                            'volume': int(row[volume_col]) if volume_col in row else 0,
                            'resolution': resolution
                        })
                    except Exception as e:
                        st.warning(f"Row {idx+1}: Error parsing data - {e}")
                        error_count += 1
                        continue
                
                # Show processing summary
                if error_count > 0:
                    st.warning(f"⚠️ Skipped {error_count} rows due to errors out of {total_rows} total rows")
                
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
    
    # Show rate limit information
    with st.expander("ℹ️ API Rate Limits", expanded=False):
        from utils import get_alpha_vantage_rate_limit_info
        rate_info = get_alpha_vantage_rate_limit_info()
        
        st.markdown("**Free Tier Limits:**")
        st.markdown(f"- {rate_info['free_tier']['calls_per_minute']} calls per minute")
        st.markdown(f"- {rate_info['free_tier']['calls_per_day']} calls per day")
        st.markdown(f"- Data: {rate_info['free_tier']['data_points_per_call']} history")
        
        st.markdown("**Premium Tier Limits:**")
        st.markdown(f"- {rate_info['premium_tier']['calls_per_minute']} calls per minute")
        st.markdown(f"- {rate_info['premium_tier']['calls_per_day']} calls per day")
        st.markdown(f"- Data: {rate_info['premium_tier']['data_points_per_call']} history")
        
        st.markdown("**💡 Tips:**")
        st.markdown("- Use 'full' output size for date ranges > 100 days")
        st.markdown("- Alpha Vantage free tier has limited historical data")
        st.markdown("- Consider premium tier for extensive historical data")
    
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
    
    # Date range selection
    st.subheader("📅 Date Range")
    col1, col2 = st.columns(2)
    
    with col1:
        # Default to last 2 years for start date
        default_start = datetime.now() - timedelta(days=730)
        start_date = st.date_input(
            "Start Date",
            value=default_start,
            max_value=datetime.now().date(),
            help="Start date for data fetching"
        )
    
    with col2:
        end_date = st.date_input(
            "End Date",
            value=datetime.now().date(),
            max_value=datetime.now().date(),
            help="End date for data fetching"
        )
    
    # Output size selection based on date range
    if start_date and end_date:
        date_range_days = (end_date - start_date).days
        if date_range_days > 100:
            st.info("📊 Large date range detected - will use 'full' output size")
            output_size = "full"
        else:
            output_size = st.selectbox("Output Size", ["compact", "full"], 
                help="Compact = last 100 points, Full = all available")
    else:
        output_size = st.selectbox("Output Size", ["compact", "full"], 
            help="Compact = last 100 points, Full = all available")
    
    # Validate date range using utility functions
    if start_date and end_date:
        from utils import validate_alpha_vantage_date_range, format_alpha_vantage_date_range_info
        
        is_valid, message = validate_alpha_vantage_date_range(start_date, end_date)
        if not is_valid:
            st.error(f"❌ {message}")
            return
        
        # Get formatted date range info
        range_info = format_alpha_vantage_date_range_info(start_date, end_date)
        
        if "warning" in range_info:
            st.warning(f"⚠️ {range_info['warning']}")
        elif "info" in range_info:
            st.info(f"📊 {range_info['info']}")
        else:
            st.success(f"✅ {range_info['message']}")
        
        # Show output size recommendation
        if "note" in range_info:
            st.info(f"💡 {range_info['note']}")
    
    if st.button("🌐 Fetch Data", type="primary", disabled=not api_key):
        if not api_key:
            st.error("❌ Please provide an Alpha Vantage API key")
            return
        
        if not start_date or not end_date:
            st.error("❌ Please select both start and end dates")
            return
            
        with st.spinner(f"Fetching data for {symbol} from {start_date} to {end_date}..."):
            try:
                import requests
                from datetime import datetime as dt
                
                # Convert dates to datetime objects
                start_dt = dt.combine(start_date, dt.min.time())
                end_dt = dt.combine(end_date, dt.max.time())
                
                # Alpha Vantage API call
                url = "https://www.alphavantage.co/query"
                params = {
                    'function': function,
                    'symbol': symbol,
                    'apikey': api_key,
                    'outputsize': output_size,
                    'datatype': 'json'
                }
                
                # Log the API request details
                st.info(f"🌐 Requesting {output_size} data for {symbol} ({function})")
                
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
                
                # Show information about raw data received
                total_data_points = len(time_series)
                st.info(f"📊 Received {total_data_points} total data points from API")
                
                # Process the data with date filtering
                processed_data = []
                for date_str, values in time_series.items():
                    timestamp = pd.to_datetime(date_str)
                    
                    # Filter by date range
                    if start_dt <= timestamp <= end_dt:
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
                
                if df.empty:
                    st.warning(f"⚠️ No data found for {symbol} in the specified date range ({start_date} to {end_date})")
                    st.info("💡 Try using 'full' output size or check if the symbol has data for this period")
                    return
                
                # Show data summary
                if len(df) < total_data_points:
                    st.warning(f"⚠️ Only {len(df)} out of {total_data_points} data points fall within your date range")
                    st.info("💡 Consider using a larger date range or 'full' output size for more data")
                
                st.session_state.market_data = df
                st.session_state.data_loaded = True
                st.session_state.data_source = "Alpha Vantage API"
                st.session_state.symbol = symbol
                
                st.success(f"✅ Fetched {len(df)} bars for {symbol} from {start_date} to {end_date}")
                st.rerun()
                
            except Exception as e:
                st.error(f"❌ Error fetching data: {str(e)}")
                st.info("💡 Tip: Check your API key and symbol. Alpha Vantage free tier has rate limits.")

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