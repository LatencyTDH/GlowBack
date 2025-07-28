// Market simulator - comprehensive implementation for realistic backtesting
use std::collections::{HashMap, BTreeMap, VecDeque};
use chrono::{DateTime, Utc, Datelike, Timelike};
use gb_types::{Bar, Symbol, Resolution, GbResult, MarketEvent, DataError};
use tracing::{info, debug};

/// Market data event with timestamp for chronological ordering
#[derive(Debug, Clone)]
pub struct TimestampedEvent {
    pub timestamp: DateTime<Utc>,
    pub symbol: Symbol,
    pub event: MarketEvent,
}

impl PartialEq for TimestampedEvent {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp && self.symbol == other.symbol
    }
}

impl Eq for TimestampedEvent {}

impl PartialOrd for TimestampedEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimestampedEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.timestamp.cmp(&other.timestamp)
            .then_with(|| self.symbol.symbol.cmp(&other.symbol.symbol))
    }
}

/// Comprehensive market simulator for realistic backtesting
#[derive(Debug)]
pub struct MarketSimulator {
    /// All market events ordered by timestamp
    events: BTreeMap<DateTime<Utc>, Vec<TimestampedEvent>>,
    /// Current market data state for each symbol
    current_data: HashMap<Symbol, Bar>,
    /// Event queue for the current simulation time
    current_events: VecDeque<TimestampedEvent>,
    /// Current simulation time
    current_time: Option<DateTime<Utc>>,
    /// Simulation start time
    start_time: Option<DateTime<Utc>>,
    /// Simulation end time  
    end_time: Option<DateTime<Utc>>,
    /// Symbols being simulated
    symbols: Vec<Symbol>,
    /// Resolution for time advancement
    resolution: Resolution,
    /// Market hours configuration
    market_hours: MarketHours,
}

/// Market hours configuration for realistic simulation
#[derive(Debug, Clone)]
pub struct MarketHours {
    /// Market open time (UTC hour)
    pub open_hour: u32,
    /// Market close time (UTC hour)
    pub close_hour: u32,
    /// Weekend trading enabled
    pub weekend_trading: bool,
}

impl Default for MarketHours {
    fn default() -> Self {
        Self {
            open_hour: 14, // 9:30 AM EST = 14:30 UTC
            close_hour: 21, // 4:00 PM EST = 21:00 UTC
            weekend_trading: false,
        }
    }
}

impl MarketSimulator {
    /// Create a new market simulator
    pub fn new() -> Self {
        Self {
            events: BTreeMap::new(),
            current_data: HashMap::new(),
            current_events: VecDeque::new(),
            current_time: None,
            start_time: None,
            end_time: None,
            symbols: Vec::new(),
            resolution: Resolution::Day,
            market_hours: MarketHours::default(),
        }
    }

    /// Configure market hours
    pub fn with_market_hours(mut self, market_hours: MarketHours) -> Self {
        self.market_hours = market_hours;
        self
    }

    /// Set simulation resolution
    pub fn with_resolution(mut self, resolution: Resolution) -> Self {
        self.resolution = resolution;
        self
    }

    /// Add market data feed for a symbol
    pub fn add_data_feed(&mut self, symbol: Symbol, bars: Vec<Bar>) -> GbResult<()> {
        if bars.is_empty() {
            return Err(DataError::InsufficientData {
                message: format!("No data provided for symbol {}", symbol)
            }.into());
        }

        info!("Adding data feed for {} with {} bars", symbol, bars.len());

        // Add symbol to simulation
        if !self.symbols.contains(&symbol) {
            self.symbols.push(symbol.clone());
        }

        // Convert bars to market events and add to timeline
        for bar in bars {
            let event = TimestampedEvent {
                timestamp: bar.timestamp,
                symbol: symbol.clone(),
                event: MarketEvent::Bar(bar.clone()),
            };

            self.events.entry(bar.timestamp)
                .or_insert_with(Vec::new)
                .push(event);

            // Update simulation time bounds
            if self.start_time.is_none() || bar.timestamp < self.start_time.unwrap() {
                self.start_time = Some(bar.timestamp);
            }
            if self.end_time.is_none() || bar.timestamp > self.end_time.unwrap() {
                self.end_time = Some(bar.timestamp);
            }
        }

        debug!("Data feed added: {} events between {:?} and {:?}", 
               self.events.len(), self.start_time, self.end_time);
        
        Ok(())
    }

    /// Initialize simulation
    pub fn initialize(&mut self) -> GbResult<()> {
        if self.events.is_empty() {
            return Err(DataError::InsufficientData {
                message: "No market data available for simulation".to_string()
            }.into());
        }

        // Set current time to just before start time so we can capture the first events
        self.current_time = self.start_time.map(|start| start - chrono::Duration::nanoseconds(1));
        
        info!("Market simulator initialized: {} symbols, {} time points", 
              self.symbols.len(), self.events.len());
        info!("Simulation period: {:?} to {:?}", self.start_time, self.end_time);

        Ok(())
    }

    /// Advance simulation to next time step and return market events
    pub fn next_events(&mut self) -> GbResult<Vec<TimestampedEvent>> {
        // If we have events queued for current time, return them
        if !self.current_events.is_empty() {
            let events: Vec<_> = self.current_events.drain(..).collect();
            debug!("Returning {} queued events for {:?}", events.len(), self.current_time);
            return Ok(events);
        }

        // Find next time with events
        let current_time = self.current_time.ok_or_else(|| DataError::LoadingFailed {
            message: "Simulation not initialized".to_string()
        })?;

        // Find next timestamp with events (use Excluded to find events after current time)
        let next_time = self.events.range((std::ops::Bound::Excluded(current_time), std::ops::Bound::Unbounded))
            .next()
            .map(|(time, _)| *time);

        if let Some(next_time) = next_time {
            // Check if we've reached the end
            if let Some(end_time) = self.end_time {
                if next_time > end_time {
                    debug!("Simulation reached end time: {:?}", end_time);
                    return Ok(Vec::new());
                }
            }

            // Advance to next time
            self.current_time = Some(next_time);

            // Get events for this time
            if let Some(events) = self.events.get(&next_time) {
                let events = events.clone();
                
                // Update current market data state
                for event in &events {
                    if let MarketEvent::Bar(bar) = &event.event {
                        self.current_data.insert(event.symbol.clone(), bar.clone());
                    }
                }

                debug!("Advanced to {:?}, returning {} events", next_time, events.len());
                Ok(events)
            } else {
                Ok(Vec::new())
            }
        } else {
            // No more events
            debug!("No more market events available");
            Ok(Vec::new())
        }
    }

    /// Get current market data for a symbol
    pub fn get_current_data(&self, symbol: &Symbol) -> Option<&Bar> {
        self.current_data.get(symbol)
    }

    /// Get current market data for all symbols
    pub fn get_all_current_data(&self) -> &HashMap<Symbol, Bar> {
        &self.current_data
    }

    /// Get current simulation time
    pub fn current_time(&self) -> Option<DateTime<Utc>> {
        self.current_time
    }

    /// Check if simulation is complete
    pub fn is_complete(&self) -> bool {
        if let (Some(current), Some(end)) = (self.current_time, self.end_time) {
            current >= end
        } else {
            false
        }
    }

    /// Get simulation progress (0.0 to 1.0)
    pub fn progress(&self) -> f64 {
        if let (Some(start), Some(current), Some(end)) = (self.start_time, self.current_time, self.end_time) {
            let total_duration = end.signed_duration_since(start);
            let current_duration = current.signed_duration_since(start);
            
            if total_duration.num_milliseconds() > 0 {
                current_duration.num_milliseconds() as f64 / total_duration.num_milliseconds() as f64
            } else {
                1.0
            }
        } else {
            0.0
        }
    }

    /// Reset simulation to start
    pub fn reset(&mut self) {
        self.current_time = self.start_time;
        self.current_events.clear();
        self.current_data.clear();
        info!("Market simulator reset to start time: {:?}", self.start_time);
    }

    /// Get simulation statistics
    pub fn get_stats(&self) -> SimulationStats {
        SimulationStats {
            total_symbols: self.symbols.len(),
            total_events: self.events.values().map(|v| v.len()).sum(),
            time_span_days: self.start_time.zip(self.end_time)
                .map(|(start, end)| end.signed_duration_since(start).num_days())
                .unwrap_or(0),
            current_progress: self.progress(),
            is_complete: self.is_complete(),
        }
    }

    /// Run full simulation with callback for each time step
    pub async fn run_with_callback<F, Fut>(&mut self, mut callback: F) -> GbResult<()>
    where
        F: FnMut(DateTime<Utc>, Vec<TimestampedEvent>) -> Fut,
        Fut: std::future::Future<Output = GbResult<()>>,
    {
        self.initialize()?;
        
        info!("Starting market simulation");
        let start_time = std::time::Instant::now();
        let mut event_count = 0;

        while !self.is_complete() {
            let events = self.next_events()?;
            
            if events.is_empty() {
                break;
            }

            let current_time = self.current_time().unwrap();
            event_count += events.len();
            
            // Call the provided callback
            callback(current_time, events).await?;

            // Log progress periodically
            if event_count % 10000 == 0 {
                let progress = self.progress();
                debug!("Simulation progress: {:.1}% ({} events processed)", 
                       progress * 100.0, event_count);
            }
        }

        let duration = start_time.elapsed();
        info!("Market simulation completed: {} events in {:?} ({:.0} events/sec)", 
              event_count, duration, event_count as f64 / duration.as_secs_f64());

        Ok(())
    }

    /// Check if market is open at given time (simplified)
    pub fn is_market_open(&self, time: DateTime<Utc>) -> bool {
        // Weekend check
        if !self.market_hours.weekend_trading {
            let weekday = time.weekday();
            if weekday == chrono::Weekday::Sat || weekday == chrono::Weekday::Sun {
                return false;
            }
        }

        // Market hours check
        let hour = time.hour();
        hour >= self.market_hours.open_hour && hour < self.market_hours.close_hour
    }
}

/// Simulation statistics
#[derive(Debug, Clone)]
pub struct SimulationStats {
    pub total_symbols: usize,
    pub total_events: usize,
    pub time_span_days: i64,
    pub current_progress: f64,
    pub is_complete: bool,
}

impl Default for MarketSimulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gb_types::AssetClass;
    use rust_decimal::Decimal;

    #[tokio::test]
    async fn test_market_simulator_basic() {
        let mut simulator = MarketSimulator::new();
        
        let symbol = Symbol::new("AAPL", "NASDAQ", AssetClass::Equity);
        let bars = vec![
            Bar::new(
                symbol.clone(),
                Utc::now(),
                Decimal::from(100),
                Decimal::from(105),
                Decimal::from(99),
                Decimal::from(102),
                Decimal::from(1000),
                Resolution::Day,
            ),
        ];

        simulator.add_data_feed(symbol.clone(), bars).unwrap();
        simulator.initialize().unwrap();

        let events = simulator.next_events().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].symbol, symbol);
    }

    #[tokio::test] 
    async fn test_market_simulator_multi_symbol() {
        let mut simulator = MarketSimulator::new();
        
        let symbol1 = Symbol::new("AAPL", "NASDAQ", AssetClass::Equity);
        let symbol2 = Symbol::new("GOOGL", "NASDAQ", AssetClass::Equity);
        
        let time = Utc::now();
        let bars1 = vec![Bar::new(symbol1.clone(), time, Decimal::from(100), Decimal::from(105), Decimal::from(99), Decimal::from(102), Decimal::from(1000), Resolution::Day)];
        let bars2 = vec![Bar::new(symbol2.clone(), time, Decimal::from(200), Decimal::from(205), Decimal::from(199), Decimal::from(202), Decimal::from(2000), Resolution::Day)];

        simulator.add_data_feed(symbol1.clone(), bars1).unwrap();
        simulator.add_data_feed(symbol2.clone(), bars2).unwrap();
        simulator.initialize().unwrap();

        let events = simulator.next_events().unwrap();
        assert_eq!(events.len(), 2);
        
        let stats = simulator.get_stats();
        assert_eq!(stats.total_symbols, 2);
        assert_eq!(stats.total_events, 2);
    }
} 