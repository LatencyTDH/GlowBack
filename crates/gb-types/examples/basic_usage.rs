use gb_types::{
    market::{Symbol, AssetClass, Bar, Resolution},
    orders::{Order, Side, Fill},
    portfolio::{Portfolio, Position},
    backtest::{BacktestConfig, BacktestResult, PerformanceMetrics},
    strategy::{StrategyConfig, StrategyMetrics, BuyAndHoldStrategy, MovingAverageCrossoverStrategy, MomentumStrategy, MeanReversionStrategy, Strategy},
    errors::{GbError, DataError},
    GbResult, validation_error, internal_error,
};
use gb_data::*;
use chrono::Utc;
use rust_decimal::Decimal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌟 GlowBack Basic Usage Example");
    
    // Create a symbol
    let symbol = Symbol::equity("AAPL");
    println!("Created symbol: {}", symbol);
    
    // Create some sample market data
    let mut bars = Vec::new();
    let now = Utc::now();
    
    for i in 0..5 {
        let timestamp = now - chrono::Duration::days(i);
        let price = Decimal::from(150) + Decimal::from(i);
        
        let bar = Bar::new(
            symbol.clone(),
            timestamp,
            price,                     // open
            price + Decimal::from(5),  // high  
            price - Decimal::from(2),  // low
            price + Decimal::from(3),  // close
            Decimal::from(10000 + i * 1000), // volume
            Resolution::Day,
        );
        bars.push(bar);
    }
    
    println!("Generated {} bars of sample data", bars.len());
    
    // Test the cache system
    let cache = CacheManager::new()?;
    println!("Created cache manager");
    
    // Store data in cache
    cache.store_bars(&symbol, &bars, Resolution::Day).await?;
    println!("Stored bars in cache");
    
    // Retrieve from cache
    let cached_bars = cache.get_bars(&symbol, now, now, Resolution::Day).await?;
    if let Some(bars) = cached_bars {
        println!("Retrieved {} bars from cache", bars.len());
    }
    
    // Test portfolio functionality
    let mut portfolio = Portfolio::new("demo_account".to_string(), Decimal::from(100000)); // $100k starting capital
    println!("Created portfolio with ${} starting capital", portfolio.cash);
    
    // Create a simple buy order
    let order = Order::market_order(
        symbol.clone(),
        Side::Buy,
        Decimal::from(100), // 100 shares
        "demo_strategy".to_string(),
    );
    println!("Created market buy order for {} shares", order.quantity);
    
    // Simulate order fill
    let fill = Fill::new(
        order.id,
        symbol.clone(),
        order.side,
        order.quantity,
        Decimal::from(150), // fill price
        Decimal::from(0),   // commission
        "demo_strategy".to_string(),
    );
    
    // Apply fill to portfolio
    portfolio.apply_fill(&fill);
    println!("Applied fill to portfolio");
    println!("Cash remaining: ${}", portfolio.cash);
    println!("Positions: {}", portfolio.positions.len());
    
    if let Some(position) = portfolio.positions.get(&symbol) {
        println!("AAPL position: {} shares at ${} avg price", 
                position.quantity, position.average_price);
    }
    
    // Test strategy framework
    let strategy_config = StrategyConfig::new("demo_strategy".to_string(), "Buy and Hold Example".to_string());
    let buy_hold_strategy = BuyAndHoldStrategy::new();
    
    println!("Created strategy: {}", strategy_config.name);
    
    // Test data manager
    let mut data_manager = DataManager::new().await?;
    println!("Created data manager");
    
    // Add sample data provider
    let sample_provider = Box::new(SampleDataProvider::new());
    data_manager.add_provider(sample_provider);
    println!("Added sample data provider");
    
    // Test data catalog
    let catalog_stats = data_manager.catalog.get_catalog_stats().await?;
    println!("Catalog contains {} symbols", catalog_stats.total_symbols);
    
    // Test error handling
    let result: GbResult<()> = Err(DataError::SymbolNotFound {
        symbol: "INVALID".to_string(),
    }.into());
    
    if let Err(e) = result {
        println!("Error handling works: {}", e);
    }
    
    // Create strategy and run backtest
    let _buy_hold_strategy = BuyAndHoldStrategy::new();
    
    println!("✅ All basic functionality working!");
    
    // Demonstrate the new strategy library
    println!("\n🚀 Strategy Library Demonstration:");
    
    // 1. Moving Average Crossover Strategy
    println!("\n📈 Moving Average Crossover Strategy:");
    let mut ma_strategy = MovingAverageCrossoverStrategy::new(5, 20); // 5-day vs 20-day MA
    let mut ma_config = StrategyConfig::new("ma_crossover".to_string(), "MA Crossover".to_string());
    ma_config.add_symbol(symbol.clone());
    ma_config.set_parameter("position_size", 0.90f64);
    
    if let Ok(()) = ma_strategy.initialize(&ma_config) {
        println!("  • Initialized: 5-day vs 20-day moving average crossover");
        println!("  • Position size: 90% of capital");
        println!("  • Strategy ID: {}", ma_strategy.get_config().strategy_id);
    }
    
    // 2. Momentum Strategy
    println!("\n🎯 Momentum Strategy:");
    let mut momentum_strategy = MomentumStrategy::new(10, 0.05); // 10-day lookback, 5% threshold
    let mut momentum_config = StrategyConfig::new("momentum".to_string(), "Momentum".to_string());
    momentum_config.add_symbol(symbol.clone());
    momentum_config.set_parameter("rebalance_frequency", 3); // Rebalance every 3 days
    
    if let Ok(()) = momentum_strategy.initialize(&momentum_config) {
        println!("  • Initialized: 10-day momentum with 5% threshold");
        println!("  • Rebalance frequency: 3 days");
        println!("  • Strategy ID: {}", momentum_strategy.get_config().strategy_id);
    }
    
    // 3. Mean Reversion Strategy
    println!("\n🔄 Mean Reversion Strategy:");
    let mut mean_rev_strategy = MeanReversionStrategy::new(15, 2.0, 1.0); // 15-day lookback, 2.0σ entry, 1.0σ exit
    let mut mean_rev_config = StrategyConfig::new("mean_reversion".to_string(), "Mean Reversion".to_string());
    mean_rev_config.add_symbol(symbol.clone());
    mean_rev_config.set_parameter("position_size", 0.25f64); // Smaller positions
    mean_rev_config.set_parameter("max_position_size", 0.75f64);
    
    if let Ok(()) = mean_rev_strategy.initialize(&mean_rev_config) {
        println!("  • Initialized: 15-day mean reversion with 2.0σ entry threshold");
        println!("  • Position size: 25% increments, max 75%");
        println!("  • Strategy ID: {}", mean_rev_strategy.get_config().strategy_id);
    }
    
    // Display strategy features
    println!("\n✨ Strategy Library Features:");
    println!("  📊 Moving Average Crossover: Trend following with customizable periods");
    println!("  🚀 Momentum: Rides trends with configurable lookback and thresholds");  
    println!("  ⚖️  Mean Reversion: Statistical arbitrage with z-score analysis");
    println!("  ⚙️  Configurable Parameters: All strategies support custom parameters");
    println!("  🎮 Event-Driven: Real-time market data processing");
    println!("  💰 Risk Management: Position sizing and portfolio constraints");
    
    println!("\n🎊 Strategy library complete with {} different strategies!", 4);
    
    Ok(())
} 