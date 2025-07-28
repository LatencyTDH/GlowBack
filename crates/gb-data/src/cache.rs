use std::collections::HashMap;
use chrono::{DateTime, Utc};
use gb_types::{Bar, Symbol, Resolution, GbResult};
use dashmap::DashMap;
use parking_lot::RwLock;

/// Cache key for market data
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    symbol: Symbol,
    resolution: Resolution,
}

/// Cached data entry with metadata
#[derive(Debug, Clone)]
struct CacheEntry {
    bars: Vec<Bar>,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    last_accessed: DateTime<Utc>,
    access_count: u64,
}

impl CacheEntry {
    fn new(bars: Vec<Bar>) -> Self {
        let now = Utc::now();
        let (start_date, end_date) = if bars.is_empty() {
            (now, now)
        } else {
            (
                bars.first().unwrap().timestamp,
                bars.last().unwrap().timestamp,
            )
        };
        
        Self {
            bars,
            start_date,
            end_date,
            last_accessed: now,
            access_count: 0,
        }
    }
    
    fn access(&mut self) {
        self.last_accessed = Utc::now();
        self.access_count += 1;
    }
    
    fn contains_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> bool {
        self.start_date <= start && self.end_date >= end
    }
    
    fn get_bars_in_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<Bar> {
        self.bars
            .iter()
            .filter(|bar| bar.timestamp >= start && bar.timestamp <= end)
            .cloned()
            .collect()
    }
}

/// In-memory cache manager for market data
#[derive(Debug)]
pub struct CacheManager {
    cache: DashMap<CacheKey, RwLock<CacheEntry>>,
    max_entries: usize,
    max_memory_mb: usize,
    stats: RwLock<CacheStats>,
}

impl CacheManager {
    pub fn new() -> GbResult<Self> {
        Ok(Self {
            cache: DashMap::new(),
            max_entries: 1000,  // Maximum number of cached symbol/resolution pairs
            max_memory_mb: 500, // Maximum memory usage in MB
            stats: RwLock::new(CacheStats::default()),
        })
    }
    
    pub fn with_limits(max_entries: usize, max_memory_mb: usize) -> GbResult<Self> {
        Ok(Self {
            cache: DashMap::new(),
            max_entries,
            max_memory_mb,
            stats: RwLock::new(CacheStats::default()),
        })
    }
    
    pub async fn get_bars(
        &self,
        symbol: &Symbol,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        resolution: Resolution,
    ) -> GbResult<Option<Vec<Bar>>> {
        let key = CacheKey {
            symbol: symbol.clone(),
            resolution,
        };
        
        if let Some(entry_lock) = self.cache.get(&key) {
            let mut entry = entry_lock.write();
            
            if entry.contains_range(start_date, end_date) {
                entry.access();
                
                // Update stats
                {
                    let mut stats = self.stats.write();
                    stats.hits += 1;
                }
                
                let bars = entry.get_bars_in_range(start_date, end_date);
                return Ok(Some(bars));
            }
        }
        
        // Cache miss
        {
            let mut stats = self.stats.write();
            stats.misses += 1;
        }
        
        Ok(None)
    }
    
    pub async fn store_bars(
        &self,
        symbol: &Symbol,
        bars: &[Bar],
        resolution: Resolution,
    ) -> GbResult<()> {
        if bars.is_empty() {
            return Ok(());
        }
        
        let key = CacheKey {
            symbol: symbol.clone(),
            resolution,
        };
        
        let entry = CacheEntry::new(bars.to_vec());
        
        // Check if we need to evict entries
        if self.cache.len() >= self.max_entries {
            self.evict_lru().await?;
        }
        
        self.cache.insert(key, RwLock::new(entry));
        
        // Update stats
        {
            let mut stats = self.stats.write();
            stats.stores += 1;
            stats.total_bars_cached += bars.len() as u64;
        }
        
        Ok(())
    }
    
    /// Evict least recently used entries
    async fn evict_lru(&self) -> GbResult<()> {
        let entries_to_remove = self.cache.len() / 10; // Remove 10% of entries
        let mut candidates: Vec<(CacheKey, DateTime<Utc>)> = Vec::new();
        
        // Collect candidates for eviction
        for entry in self.cache.iter() {
            let last_accessed = entry.value().read().last_accessed;
            candidates.push((entry.key().clone(), last_accessed));
        }
        
        // Sort by last accessed time (oldest first)
        candidates.sort_by(|a, b| a.1.cmp(&b.1));
        
        // Remove oldest entries
        for (key, _) in candidates.into_iter().take(entries_to_remove) {
            if let Some((_, entry_lock)) = self.cache.remove(&key) {
                let entry = entry_lock.into_inner();
                
                // Update stats
                {
                    let mut stats = self.stats.write();
                    stats.evictions += 1;
                    stats.total_bars_cached = stats.total_bars_cached.saturating_sub(entry.bars.len() as u64);
                }
            }
        }
        
        Ok(())
    }
    
    pub fn clear(&self) {
        self.cache.clear();
        
        // Reset stats
        {
            let mut stats = self.stats.write();
            *stats = CacheStats::default();
        }
    }
    
    pub fn get_stats(&self) -> CacheStats {
        self.stats.read().clone()
    }
    
    pub fn get_cache_info(&self) -> CacheInfo {
        let mut total_bars = 0u64;
        let mut oldest_access = Utc::now();
        let mut newest_access = DateTime::<Utc>::MIN_UTC;
        
        for entry in self.cache.iter() {
            let guard = entry.value().read();
            total_bars += guard.bars.len() as u64;
            
            if guard.last_accessed < oldest_access {
                oldest_access = guard.last_accessed;
            }
            if guard.last_accessed > newest_access {
                newest_access = guard.last_accessed;
            }
        }
        
        CacheInfo {
            total_entries: self.cache.len(),
            total_bars,
            estimated_memory_mb: self.estimate_memory_usage(),
            oldest_access: if total_bars > 0 { Some(oldest_access) } else { None },
            newest_access: if total_bars > 0 { Some(newest_access) } else { None },
        }
    }
    
    fn estimate_memory_usage(&self) -> f64 {
        // Rough estimation: each bar is approximately 100 bytes
        let total_bars = self.cache.iter()
            .map(|entry| entry.value().read().bars.len())
            .sum::<usize>();
        
        (total_bars * 100) as f64 / (1024.0 * 1024.0) // Convert to MB
    }
}

#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub stores: u64,
    pub evictions: u64,
    pub total_bars_cached: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        if self.hits + self.misses == 0 {
            0.0
        } else {
            self.hits as f64 / (self.hits + self.misses) as f64
        }
    }
    
    pub fn miss_rate(&self) -> f64 {
        1.0 - self.hit_rate()
    }
}

#[derive(Debug, Clone)]
pub struct CacheInfo {
    pub total_entries: usize,
    pub total_bars: u64,
    pub estimated_memory_mb: f64,
    pub oldest_access: Option<DateTime<Utc>>,
    pub newest_access: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use gb_types::{AssetClass, Resolution};
    use rust_decimal::Decimal;
    
    #[tokio::test]
    async fn test_cache_basic_operations() {
        let cache = CacheManager::new().unwrap();
        let symbol = Symbol::new("AAPL", "NASDAQ", AssetClass::Equity);
        
        let now = Utc::now();
        let bars = vec![
            Bar::new(
                symbol.clone(),
                now,
                Decimal::from(100),
                Decimal::from(105),
                Decimal::from(98),
                Decimal::from(102),
                Decimal::from(10000),
                Resolution::Day,
            ),
        ];
        
        // First check should be a cache miss
        let cached_bars = cache.get_bars(&symbol, now, now, Resolution::Day).await.unwrap();
        assert!(cached_bars.is_none());
        
        // Store bars
        cache.store_bars(&symbol, &bars, Resolution::Day).await.unwrap();
        
        // Now retrieve bars should work - request exact timestamp range
        let cached_bars = cache.get_bars(&symbol, now, now, Resolution::Day).await.unwrap();
        assert!(cached_bars.is_some());
        assert_eq!(cached_bars.unwrap().len(), 1);
        
        let stats = cache.get_stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.stores, 1);
    }
} 