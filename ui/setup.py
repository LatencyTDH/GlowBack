#!/usr/bin/env python3
"""
GlowBack UI Setup Script
Automatically sets up and launches the Streamlit UI for local strategy development.
"""

import subprocess
import sys
import os
from pathlib import Path

def install_requirements():
    """Install required packages"""
    print("📦 Installing required packages...")
    
    try:
        subprocess.run([sys.executable, "-m", "pip", "install", "-r", "requirements.txt"], 
                      check=True, capture_output=True, text=True)
        print("✅ All packages installed successfully!")
    except subprocess.CalledProcessError as e:
        print(f"❌ Error installing packages: {e}")
        print("Output:", e.stdout)
        print("Error:", e.stderr)
        return False
    return True

def check_rust_bindings():
    """Check if Rust Python bindings are available"""
    print("🔍 Checking Rust Python bindings...")
    
    # Check if we can import the Rust bindings
    parent_dir = Path(__file__).parent.parent
    crates_dir = parent_dir / "crates"
    
    if not crates_dir.exists():
        print("⚠️  Rust crates not found. Some features may not work.")
        return False
    
    print("✅ Rust crates found!")
    return True

def launch_streamlit():
    """Launch the Streamlit app"""
    print("🚀 Launching GlowBack UI...")
    
    try:
        # Change to UI directory
        os.chdir(Path(__file__).parent)
        
        # Launch Streamlit
        subprocess.run([sys.executable, "-m", "streamlit", "run", "app.py", 
                       "--server.port=8501", "--server.headless=false"], 
                      check=True)
    except subprocess.CalledProcessError as e:
        print(f"❌ Error launching Streamlit: {e}")
        return False
    except KeyboardInterrupt:
        print("\n👋 GlowBack UI stopped by user.")
    
    return True

def main():
    """Main setup and launch function"""
    print("🌟 GlowBack UI Setup")
    print("=" * 50)
    
    # Check Python version
    if sys.version_info < (3, 8):
        print("❌ Python 3.8 or higher is required.")
        sys.exit(1)
    
    print(f"✅ Python {sys.version_info.major}.{sys.version_info.minor} detected")
    
    # Install requirements
    if not install_requirements():
        print("❌ Failed to install requirements. Please install manually:")
        print("   pip install -r requirements.txt")
        sys.exit(1)
    
    # Check Rust bindings
    check_rust_bindings()
    
    print("\n🎉 Setup complete!")
    print("\n📋 Quick Start Guide:")
    print("1. 📊 Load market data (CSV, API, or sample data)")
    print("2. ⚙️  Create or edit your trading strategy")
    print("3. 🚀 Run backtest with your strategy")
    print("4. 📈 Analyze results in the dashboard")
    
    print("\n🌐 Starting GlowBack UI...")
    print("   URL: http://localhost:8501")
    print("   Press Ctrl+C to stop")
    
    # Launch the app
    launch_streamlit()

if __name__ == "__main__":
    main() 