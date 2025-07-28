"""
GlowBack UI Utilities
Helper functions and error handling for the Streamlit UI
"""

import streamlit as st
import pandas as pd
import traceback
from datetime import datetime
from typing import Optional, Union, List, Dict, Any
import config

def handle_error(func):
    """Decorator for consistent error handling in UI functions"""
    def wrapper(*args, **kwargs):
        try:
            return func(*args, **kwargs)
        except Exception as e:
            st.error(f"❌ Error in {func.__name__}: {str(e)}")
            if st.checkbox("Show detailed error (for debugging)", key=f"debug_{func.__name__}"):
                st.code(traceback.format_exc())
            return None
    return wrapper

def validate_data_frame(df: pd.DataFrame, required_columns: List[str]) -> bool:
    """Validate that a DataFrame has required columns"""
    if df is None or df.empty:
        st.error("❌ DataFrame is empty or None")
        return False
    
    missing_columns = [col for col in required_columns if col not in df.columns]
    if missing_columns:
        st.error(f"❌ Missing required columns: {missing_columns}")
        st.info(f"Available columns: {list(df.columns)}")
        return False
    
    return True

def format_currency(amount: Union[float, int], precision: int = 2) -> str:
    """Format a number as currency"""
    return f"${amount:,.{precision}f}"

def format_percentage(value: Union[float, int], precision: int = 2) -> str:
    """Format a number as percentage"""
    return f"{value:.{precision}f}%"

def safe_divide(numerator: float, denominator: float, default: float = 0.0) -> float:
    """Safely divide two numbers, returning default if denominator is zero"""
    return numerator / denominator if denominator != 0 else default

def validate_symbol(symbol: str) -> bool:
    """Validate a stock symbol format"""
    if not symbol or not isinstance(symbol, str):
        return False
    
    symbol = symbol.strip().upper()
    
    # Basic validation - alphanumeric, 1-10 characters
    if not symbol.isalnum() or len(symbol) < 1 or len(symbol) > 10:
        return False
    
    return True

def validate_date_range(start_date, end_date) -> bool:
    """Validate date range"""
    if start_date is None or end_date is None:
        st.error("❌ Both start and end dates are required")
        return False
    
    if start_date >= end_date:
        st.error("❌ Start date must be before end date")
        return False
    
    # Check if date range is reasonable (not too far in the future)
    if end_date > datetime.now().date():
        st.warning("⚠️ End date is in the future")
    
    return True

def create_download_link(data: Union[pd.DataFrame, str], filename: str, mime_type: str = "text/csv") -> None:
    """Create a download button for data"""
    if isinstance(data, pd.DataFrame):
        data_str = data.to_csv(index=False)
    else:
        data_str = str(data)
    
    st.download_button(
        label=f"📥 Download {filename}",
        data=data_str,
        file_name=filename,
        mime=mime_type
    )

def show_dataframe_info(df: pd.DataFrame, title: str = "Data Info") -> None:
    """Show informative summary of a DataFrame"""
    with st.expander(f"📊 {title}"):
        col1, col2, col3 = st.columns(3)
        
        with col1:
            st.metric("Rows", len(df))
        with col2:
            st.metric("Columns", len(df.columns))
        with col3:
            st.metric("Memory Usage", f"{df.memory_usage(deep=True).sum() / 1024:.1f} KB")
        
        st.write("**Columns:**", ", ".join(df.columns))
        st.write("**Data Types:**")
        st.write(df.dtypes.to_frame("Type"))

def create_metric_card(title: str, value: str, delta: Optional[str] = None, help_text: Optional[str] = None) -> None:
    """Create a styled metric card"""
    st.metric(
        label=title,
        value=value,
        delta=delta,
        help=help_text
    )

def validate_strategy_code(code: str) -> tuple[bool, str]:
    """Validate strategy code and return success status and message"""
    if not code or not code.strip():
        return False, "Strategy code is empty"
    
    try:
        # Basic syntax check
        compile(code, '<string>', 'exec')
        
        # Check for required structure
        if 'class' not in code:
            return False, "Strategy must contain at least one class"
        
        if 'on_bar' not in code:
            return False, "Strategy class must have an 'on_bar' method"
        
        return True, "Strategy code is valid"
        
    except SyntaxError as e:
        return False, f"Syntax error: {str(e)}"
    except Exception as e:
        return False, f"Validation error: {str(e)}"

def get_sample_data_info() -> Dict[str, Any]:
    """Get information about sample data generation"""
    return {
        "default_periods": config.SAMPLE_DATA_PERIODS,
        "default_capital": config.DEFAULT_INITIAL_CAPITAL,
        "supported_resolutions": ["1d", "1h", "5m"],
        "max_data_points": config.MAX_DATA_POINTS
    }

def format_large_number(num: Union[int, float]) -> str:
    """Format large numbers with appropriate suffixes"""
    if abs(num) >= 1_000_000:
        return f"{num/1_000_000:.1f}M"
    elif abs(num) >= 1_000:
        return f"{num/1_000:.1f}K"
    else:
        return f"{num:.0f}"

def create_progress_bar(current: int, total: int, label: str = "Progress") -> None:
    """Create a progress bar with percentage"""
    if total > 0:
        progress = current / total
        st.progress(progress, text=f"{label}: {progress*100:.1f}% ({current}/{total})")
    else:
        st.progress(0, text=f"{label}: 0%")

def session_state_summary() -> Dict[str, Any]:
    """Get a summary of current session state for debugging"""
    return {
        "data_loaded": st.session_state.get('data_loaded', False),
        "strategy_configured": bool(st.session_state.get('strategy_config')),
        "backtest_results": bool(st.session_state.get('backtest_results')),
        "backtest_running": st.session_state.get('backtest_running', False),
        "session_keys": list(st.session_state.keys())
    } 

def validate_alpha_vantage_date_range(start_date, end_date) -> tuple[bool, str]:
    """Validate date range for Alpha Vantage API calls"""
    if not start_date or not end_date:
        return False, "Both start and end dates are required"
    
    if start_date >= end_date:
        return False, "Start date must be before end date"
    
    # Check if date range is reasonable
    date_range_days = (end_date - start_date).days
    
    if date_range_days > 3650:  # More than 10 years
        return False, "Date range cannot exceed 10 years"
    
    if date_range_days < 1:
        return False, "Date range must be at least 1 day"
    
    # Check if end date is not in the future
    if end_date > datetime.now().date():
        return False, "End date cannot be in the future"
    
    return True, f"Valid date range: {date_range_days} days"

def format_alpha_vantage_date_range_info(start_date, end_date) -> dict:
    """Format date range information for Alpha Vantage API"""
    if not start_date or not end_date:
        return {"valid": False, "message": "Invalid date range"}
    
    date_range_days = (end_date - start_date).days
    
    info = {
        "valid": True,
        "days": date_range_days,
        "start": start_date,
        "end": end_date,
        "message": f"Fetching {date_range_days} days of data"
    }
    
    if date_range_days > 100:  # More than 100 days
        info["output_size"] = "full"
        info["note"] = "Will use 'full' output size for complete data"
    elif date_range_days > 730:  # More than 2 years
        info["warning"] = "Large date range may use more API calls"
        info["output_size"] = "full"
    elif date_range_days > 365:  # More than 1 year
        info["info"] = "Fetching over 1 year of data"
        info["output_size"] = "full"
    else:
        info["output_size"] = "compact"
    
    return info

def get_alpha_vantage_rate_limit_info() -> dict:
    """Get Alpha Vantage API rate limit information"""
    return {
        "free_tier": {
            "calls_per_minute": 5,
            "calls_per_day": 500,
            "data_points_per_call": "full"  # or "compact" for last 100 points
        },
        "premium_tier": {
            "calls_per_minute": 1200,
            "calls_per_day": 50000,
            "data_points_per_call": "full"
        }
    } 