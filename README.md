# 🌟 GlowBack - High-Performance Quantitative Backtesting Platform

GlowBack is a modern, high-performance backtesting platform designed for medium-frequency trading strategies. Built with Rust for speed and reliability, with Python bindings for ease of use.

## 🎯 Project Vision

GlowBack aims to provide sophisticated retail traders, small quant hedge funds, academic researchers, and students with:

- **Ultra-realistic market simulation** with microstructure awareness
- **ML-ready interface** compatible with scikit-learn and PyTorch  
- **Built-in statistical robustness checks** for strategy validation
- **High performance** with sub-minute backtests for 10+ years of data

## 🏗️ Architecture Overview

### Core Components

- **gb-types**: Core data structures and types
- **gb-data**: Data ingestion, storage, and management 
- **gb-engine**: High-performance backtesting engine (Rust)
- **gb-python**: Python bindings via PyO3

### Technology Stack

- **Core Engine**: Rust with Arrow/Parquet for data processing
- **Storage**: Columnar Parquet files with DuckDB metadata catalog
- **Caching**: In-memory LRU cache with Redis clustering support
- **Python SDK**: PyO3 bindings with async support
- **UI**: Streamlit for local validation, React for web dashboard

## 🚀 Current Implementation Status

### ✅ Completed (Phase 0 - PoC)

- **Project Structure**: Rust workspace with proper crate organization
- **Core Types**: Complete type system for market data, orders, portfolio, and strategies
- **Error Handling**: Comprehensive error types with proper propagation
- **Data Storage**: Arrow/Parquet integration for efficient data storage
- **Data Providers**: Pluggable provider system (CSV, sample data, API placeholders)
- **Portfolio Management**: Position tracking, P&L calculation, risk metrics
- **Caching System**: High-performance in-memory caching with LRU eviction
- **Data Catalog**: DuckDB-based metadata management
- **Strategy Framework**: Event-driven strategy interface with sample implementations

### 🚧 In Progress

- **Backtesting Engine**: Core simulation engine implementation
- **Execution Simulation**: Realistic order execution with slippage and latency
- **Python Bindings**: PyO3 integration for Python SDK
- **Data Loaders**: CSV and Parquet file processing

### 📋 Next Steps (Phase 1 - Alpha)

- **Complete Engine**: Finish core backtesting simulation
- **Strategy Templates**: Common strategy patterns and indicators
- **Performance Analytics**: Comprehensive metrics and visualization
- **Local UI**: Streamlit-based interface for immediate usability
- **Documentation**: API docs and tutorials

## 🛠️ Development Setup

### Prerequisites

- Rust 1.70+ 
- Python 3.8+
- uv (for Python dependency management)

### Quick Start

```bash
# Clone the repository
git clone <repository-url>
cd glowback

# Check Rust components
cargo check

# Build Python bindings (when ready)
cd crates/gb-python
cargo build --release

# Install Python package (when ready)
uv pip install -e python/
```

## 📊 Key Features

### Market Data
- Multiple data sources (CSV, APIs, databases)
- Efficient columnar storage with Parquet
- High-performance caching layer
- Automatic data validation and cleaning

### Strategy Development
- Event-driven architecture
- Built-in technical indicators
- ML framework integration
- Risk management controls

### Performance Analytics
- Comprehensive risk/return metrics
- Statistical significance testing
- Walk-forward analysis
- Monte Carlo simulation

### Execution Modeling
- Configurable slippage models
- Latency simulation
- Market impact modeling
- Realistic commission structures

## 🔧 Configuration

GlowBack supports multiple configuration methods:

```rust
// Rust API
let config = BacktestConfig::new("My Strategy", strategy_config)
    .with_symbols(vec![Symbol::equity("AAPL")])
    .with_date_range(start_date, end_date)
    .with_capital(Decimal::from(100000));
```

```python
# Python API (planned)
import glowback as gb

config = gb.BacktestConfig("My Strategy")
config.add_symbol("AAPL", "equity")
config.set_date_range("2020-01-01", "2023-12-31")
config.set_capital(100000)
```

## 📈 Performance Goals

- **Speed**: < 60 seconds for 10 years of daily data on 500 equities
- **Memory**: Efficient streaming with configurable memory limits
- **Scale**: Horizontal scaling for parameter optimization
- **Storage**: < 1TB for 10 years of tick data (1000 symbols)

## 🤝 Contributing

GlowBack is open source (MIT License). We welcome contributions!

### Development Workflow

1. **Core Development**: Focus on Rust components for performance-critical code
2. **Integration**: Python bindings for user-facing APIs
3. **Testing**: Comprehensive unit and integration tests
4. **Documentation**: Keep docs updated with implementation

### Code Style

- **Rust**: Follow standard rustfmt conventions
- **Python**: Black formatting, type hints required
- **Commits**: Conventional commit format preferred

## 📚 Documentation

- [Architecture Design](design.md) - Comprehensive system design
- [API Reference](docs/api/) - Detailed API documentation (planned)
- [User Guide](docs/guide/) - Getting started tutorial (planned)
- [Examples](examples/) - Sample strategies and use cases (planned)

## 🔄 Roadmap

### Phase 0: PoC (✅ Current)
- Core types and data structures
- Basic engine framework
- Storage and caching systems

### Phase 1: Alpha (Q1 2024)
- Complete backtesting engine
- Python SDK
- Local Streamlit UI
- Basic strategy library

### Phase 2: Beta (Q2 2024)
- React web dashboard
- Advanced analytics
- Optimization framework
- Docker deployment

### Phase 3: GA (Q3 2024)
- Production deployment
- Community features
- Advanced strategies
- Full documentation

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

## 💬 Community

- **Issues**: Use GitHub Issues for bug reports and feature requests
- **Discussions**: GitHub Discussions for questions and ideas
- **Discord**: Community chat (link TBD)

---

**GlowBack** - Illuminating your trading strategies with precision and performance. 