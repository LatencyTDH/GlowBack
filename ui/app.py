"""
GlowBack - Local Strategy Development UI
A Streamlit-based interface for quantitative backtesting and strategy development.
"""

import streamlit as st
import sys
from pathlib import Path
from streamlit_option_menu import option_menu

# Add the parent directory to Python path to import gb-python bindings
parent_dir = Path(__file__).parent.parent
sys.path.insert(0, str(parent_dir))

# Import pages
from pages import data_loader, strategy_editor, backtest_runner, results_dashboard, portfolio_analyzer, advanced_analytics

# Configure Streamlit page
st.set_page_config(
    page_title="GlowBack - Quantitative Backtesting Platform",
    page_icon="🌟",
    layout="wide",
    initial_sidebar_state="expanded"
)

# Dark-mode support — persisted in session state
if "dark_mode" not in st.session_state:
    st.session_state.dark_mode = False

_dark = st.session_state.dark_mode

# Custom CSS — adapts to light / dark preference
_bg = "#1e1e2f" if _dark else "#ffffff"
_fg = "#e0e0e0" if _dark else "#1f1f1f"
_card_bg = "#2a2a3d" if _dark else "#f0f2f6"
_success_bg = "#1b3a2a" if _dark else "#d4edda"
_warning_bg = "#3a351b" if _dark else "#fff3cd"
_sidebar_bg = "#16161f" if _dark else "#f8f9fa"

st.markdown(f"""
<style>
    .main-header {{
        font-size: 2.5rem;
        font-weight: bold;
        color: {_fg};
        margin-bottom: 1rem;
        text-align: center;
    }}
    .metric-card {{
        background-color: {_card_bg};
        padding: 1rem;
        border-radius: 0.5rem;
        border-left: 4px solid #ff6b6b;
    }}
    .success-card {{
        background-color: {_success_bg};
        padding: 1rem;
        border-radius: 0.5rem;
        border-left: 4px solid #28a745;
    }}
    .warning-card {{
        background-color: {_warning_bg};
        padding: 1rem;
        border-radius: 0.5rem;
        border-left: 4px solid #ffc107;
    }}
    .sidebar .sidebar-content {{
        background-color: {_sidebar_bg};
    }}
</style>
""", unsafe_allow_html=True)

def main():
    """Main application entry point"""
    
    # Header
    st.markdown('<h1 class="main-header">🌟 GlowBack Platform</h1>', unsafe_allow_html=True)
    st.markdown("### High-Performance Quantitative Backtesting & Strategy Development")
    
    # Initialize session state
    if 'data_loaded' not in st.session_state:
        st.session_state.data_loaded = False
    if 'strategy_config' not in st.session_state:
        st.session_state.strategy_config = {}
    if 'backtest_results' not in st.session_state:
        st.session_state.backtest_results = None
    if 'portfolio_data' not in st.session_state:
        st.session_state.portfolio_data = None
    if 'saved_runs' not in st.session_state:
        st.session_state.saved_runs = {}
    
    # Main navigation
    with st.sidebar:
        st.image("https://via.placeholder.com/150x50/4CAF50/FFFFFF?text=GlowBack", width=150)

        # Dark mode toggle
        dark_toggle = st.toggle("🌙 Dark Mode", value=st.session_state.dark_mode, key="dark_toggle")
        if dark_toggle != st.session_state.dark_mode:
            st.session_state.dark_mode = dark_toggle
            st.rerun()

        st.markdown("---")
        
        selected = option_menu(
            menu_title="Navigation",
            options=[
                "📊 Data Loader",
                "⚙️ Strategy Editor",
                "🚀 Backtest Runner",
                "📈 Results Dashboard",
                "💼 Portfolio Analyzer",
                "🔬 Advanced Analytics",
            ],
            icons=["database", "code-slash", "play-circle", "graph-up", "briefcase", "bar-chart-line"],
            menu_icon="list",
            default_index=0,
            orientation="vertical",
            styles={
                "container": {"padding": "0!important", "background-color": "#fafafa"},
                "icon": {"color": "#ff6b6b", "font-size": "18px"},
                "nav-link": {
                    "font-size": "16px",
                    "text-align": "left",
                    "margin": "0px",
                    "--hover-color": "#eee",
                },
                "nav-link-selected": {"background-color": "#ff6b6b"},
            }
        )
        
        # Status indicators
        st.markdown("---")
        st.markdown("### 📊 Status")
        
        # Data status
        if st.session_state.data_loaded:
            st.markdown('<div class="success-card">✅ Data Loaded</div>', unsafe_allow_html=True)
        else:
            st.markdown('<div class="warning-card">⚠️ No Data</div>', unsafe_allow_html=True)
        
        # Strategy status
        if st.session_state.strategy_config:
            st.markdown('<div class="success-card">✅ Strategy Configured</div>', unsafe_allow_html=True)
        else:
            st.markdown('<div class="warning-card">⚠️ No Strategy</div>', unsafe_allow_html=True)
        
        # Results status
        if st.session_state.backtest_results:
            st.markdown('<div class="success-card">✅ Results Available</div>', unsafe_allow_html=True)
        else:
            st.markdown('<div class="warning-card">⚠️ No Results</div>', unsafe_allow_html=True)
    
    # Route to appropriate page
    if selected == "📊 Data Loader":
        data_loader.show()
    elif selected == "⚙️ Strategy Editor":
        strategy_editor.show()
    elif selected == "🚀 Backtest Runner":
        backtest_runner.show()
    elif selected == "📈 Results Dashboard":
        results_dashboard.show()
    elif selected == "💼 Portfolio Analyzer":
        portfolio_analyzer.show()
    elif selected == "🔬 Advanced Analytics":
        advanced_analytics.show()
    
    # Footer
    st.markdown("---")
    st.markdown(
        """
        <div style='text-align: center; color: #666; font-size: 0.9rem;'>
            🌟 GlowBack Platform v0.1.0 | Built with Streamlit & Rust | 
            <a href="https://github.com/LatencyTDH/glowback" target="_blank">GitHub</a>
        </div>
        """, 
        unsafe_allow_html=True
    )

if __name__ == "__main__":
    main() 