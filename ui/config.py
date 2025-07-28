# GlowBack UI Configuration
"""
Configuration settings for the GlowBack Streamlit UI
"""

# UI Settings
DEFAULT_PORT = 8501
DEFAULT_THEME = "light"

# Data Settings
DEFAULT_INITIAL_CAPITAL = 100000
DEFAULT_COMMISSION = 0.001
DEFAULT_SLIPPAGE_BPS = 5
MAX_DATA_POINTS = 10000
SAMPLE_DATA_PERIODS = 252

# Chart Settings
CHART_HEIGHT_MAIN = 800
CHART_HEIGHT_SECONDARY = 400
CHART_HEIGHT_SMALL = 300

# Backtest Settings
DEFAULT_PROGRESS_UPDATE_INTERVAL = 0.01  # seconds
MAX_LOG_LINES = 20
DEFAULT_BATCH_SIZE = 1000

# Risk Settings
DEFAULT_MAX_POSITION_SIZE = 0.95
DEFAULT_KELLY_FRACTION = 0.25
DEFAULT_VAR_CONFIDENCE_LEVELS = [0.95, 0.99, 0.999]

# File Settings
TEMP_FILE_PREFIX = "glowback_"
SUPPORTED_CSV_FORMATS = [".csv", ".CSV"]
SUPPORTED_DATE_FORMATS = ["%Y-%m-%d", "%m/%d/%Y", "%d/%m/%Y", "%Y-%m-%d %H:%M:%S"]

# API Settings
ALPHA_VANTAGE_FREE_RATE_LIMIT = {"calls_per_minute": 5, "calls_per_day": 500}

# Color Scheme
COLORS = {
    "primary": "#ff6b6b",
    "secondary": "#4ecdc4", 
    "success": "#28a745",
    "warning": "#ffc107",
    "danger": "#dc3545",
    "info": "#17a2b8"
} 