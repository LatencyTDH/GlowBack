use gb_types::*;
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
    
    println!("✅ All basic functionality working!");
    println!("\n🚀 GlowBack Phase 0 (PoC) Complete!");
    println!("Ready for Phase 1 development:");
    println!("  - Complete backtesting engine");
    println!("  - Strategy library expansion"); 
    println!("  - Streamlit UI");
    println!("  - Performance analytics");
    
    Ok(())
} 