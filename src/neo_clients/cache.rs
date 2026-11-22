use serde::{Deserialize, Serialize};
use std::{
	collections::HashMap,
	hash::Hash,
	sync::Arc,
	time::{Duration, Instant},
};
use tokio::sync::RwLock;

/// Cache configuration
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct CacheConfig {
	/// Maximum number of entries in the cache
	pub max_entries: usize,
	/// Default TTL for cache entries
	pub default_ttl: Duration,
	/// Cleanup interval for expired entries
	pub cleanup_interval: Duration,
	/// Enable LRU eviction when cache is full
	pub enable_lru: bool,
}

impl CacheConfig {
	/// Creates a new builder for the configuration
	#[must_use]
	pub fn builder() -> CacheConfigBuilder {
		CacheConfigBuilder::default()
	}
}

/// Builder for `CacheConfig`
#[derive(Debug, Default, Clone)]
pub struct CacheConfigBuilder {
	max_entries: Option<usize>,
	default_ttl: Option<Duration>,
	cleanup_interval: Option<Duration>,
	enable_lru: Option<bool>,
}

impl CacheConfigBuilder {
	/// Sets the maximum number of entries
	#[must_use]
	pub fn max_entries(mut self, val: usize) -> Self {
		self.max_entries = Some(val);
		self
	}

	/// Sets the default TTL
	#[must_use]
	pub fn default_ttl(mut self, val: Duration) -> Self {
		self.default_ttl = Some(val);
		self
	}

	/// Sets the cleanup interval
	#[must_use]
	pub fn cleanup_interval(mut self, val: Duration) -> Self {
		self.cleanup_interval = Some(val);
		self
	}

	/// Enables or disables LRU eviction
	#[must_use]
	pub fn enable_lru(mut self, val: bool) -> Self {
		self.enable_lru = Some(val);
		self
	}

	/// Builds the `CacheConfig`
	pub fn build(self) -> CacheConfig {
		let default = CacheConfig::default();
		CacheConfig {
			max_entries: self.max_entries.unwrap_or(default.max_entries),
			default_ttl: self.default_ttl.unwrap_or(default.default_ttl),
			cleanup_interval: self.cleanup_interval.unwrap_or(default.cleanup_interval),
			enable_lru: self.enable_lru.unwrap_or(default.enable_lru),
		}
	}
}

impl Default for CacheConfig {
	fn default() -> Self {
		Self {
			max_entries: 1000,
			default_ttl: Duration::from_secs(300),     // 5 minutes
			cleanup_interval: Duration::from_secs(60), // 1 minute
			enable_lru: true,
		}
	}
}

/// Cache entry with expiration and access tracking
#[derive(Debug, Clone)]
struct CacheEntry<V> {
	value: V,
	expires_at: Instant,
	last_accessed: Instant,
	/// Access count for frequency-based eviction (LFU).
	///
	/// # Note
	/// This field is reserved for future LFU (Least Frequently Used) eviction
	/// strategy enhancements. It is not currently used by the active LRU
	/// eviction implementation but is maintained to preserve API compatibility
	/// for future caching enhancements.
	#[doc(hidden)]
	#[allow(dead_code)]
	access_count: u64,
}

impl<V> CacheEntry<V> {
	fn new(value: V, ttl: Duration) -> Self {
		let now = Instant::now();
		Self { value, expires_at: now + ttl, last_accessed: now, access_count: 1 }
	}

	fn is_expired(&self) -> bool {
		Instant::now() > self.expires_at
	}

	/// Access the cached value and update tracking metadata.
	///
	/// # Note
	/// This method is reserved for future eviction strategy enhancements that
	/// require write-lock-based access patterns (e.g., LFU). The current
	/// implementation uses read-optimized access patterns via `get()`.
	///
	/// # Returns
	/// A reference to the cached value.
	#[doc(hidden)]
	#[allow(dead_code)]
	fn access(&mut self) -> &V {
		self.last_accessed = Instant::now();
		self.access_count += 1;
		&self.value
	}
}

/// High-performance cache with TTL and LRU eviction
#[derive(Debug)]
pub struct Cache<K, V> {
	config: CacheConfig,
	entries: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
	stats: Arc<RwLock<CacheStats>>,
}

/// Cache statistics
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CacheStats {
	pub hits: u64,
	pub misses: u64,
	pub evictions: u64,
	pub expired_removals: u64,
	pub current_size: usize,
	pub max_size_reached: u64,
}

impl CacheStats {
	pub fn hit_rate(&self) -> f64 {
		if self.hits + self.misses == 0 {
			0.0
		} else {
			self.hits as f64 / (self.hits + self.misses) as f64
		}
	}
}

impl<K, V> Cache<K, V>
where
	K: Hash + Eq + Clone + Send + Sync + 'static,
	V: Clone + Send + Sync + 'static,
{
	/// Create a new cache with the given configuration
	pub fn new(config: CacheConfig) -> Self {
		Self {
			config,
			entries: Arc::new(RwLock::new(HashMap::new())),
			stats: Arc::new(RwLock::new(CacheStats::default())),
		}
	}

	/// Get a value from the cache
	///
	/// PERFORMANCE: Uses a two-phase approach - first tries with read lock,
	/// only acquires write lock if the entry is expired and needs removal.
	pub async fn get(&self, key: &K) -> Option<V> {
		// Phase 1: Try to get with read lock first (fast path)
		{
			let entries = self.entries.read().await;
			if let Some(entry) = entries.get(key) {
				if !entry.is_expired() {
					// Entry exists and is valid - update stats and return
					let mut stats = self.stats.write().await;
					stats.hits += 1;
					return Some(entry.value.clone());
				}
				// Entry is expired - fall through to write lock path
			} else {
				// Entry doesn't exist
				let mut stats = self.stats.write().await;
				stats.misses += 1;
				return None;
			}
		}

		// Phase 2: Entry was expired, need write lock to remove it
		let mut entries = self.entries.write().await;
		let mut stats = self.stats.write().await;

		// Double-check the entry (another thread may have already removed it)
		if let Some(entry) = entries.get(key) {
			if entry.is_expired() {
				entries.remove(key);
				stats.expired_removals += 1;
				stats.misses += 1;
				stats.current_size = entries.len();
				return None;
			}
			// Entry was refreshed by another thread
			stats.hits += 1;
			Some(entry.value.clone())
		} else {
			stats.misses += 1;
			None
		}
	}

	/// Insert a value into the cache with default TTL
	pub async fn insert(&self, key: K, value: V) {
		self.insert_with_ttl(key, value, self.config.default_ttl).await;
	}

	/// Insert a value into the cache with custom TTL
	pub async fn insert_with_ttl(&self, key: K, value: V, ttl: Duration) {
		let mut entries = self.entries.write().await;
		let mut stats = self.stats.write().await;

		// Check if we need to evict entries
		if entries.len() >= self.config.max_entries && !entries.contains_key(&key) {
			if self.config.enable_lru {
				self.evict_lru(&mut entries, &mut stats);
			} else {
				// Simple eviction - remove first entry
				if let Some(first_key) = entries.keys().next().cloned() {
					entries.remove(&first_key);
					stats.evictions += 1;
				}
			}
			stats.max_size_reached += 1;
		}

		let entry = CacheEntry::new(value, ttl);
		entries.insert(key, entry);
		stats.current_size = entries.len();
	}

	/// Remove a value from the cache
	pub async fn remove(&self, key: &K) -> Option<V> {
		let mut entries = self.entries.write().await;
		let mut stats = self.stats.write().await;

		let result = entries.remove(key).map(|entry| entry.value);
		stats.current_size = entries.len();
		result
	}

	/// Clear all entries from the cache
	pub async fn clear(&self) {
		let mut entries = self.entries.write().await;
		let mut stats = self.stats.write().await;

		entries.clear();
		stats.current_size = 0;
	}

	/// Get cache statistics
	pub async fn stats(&self) -> CacheStats {
		let stats = self.stats.read().await;
		CacheStats {
			hits: stats.hits,
			misses: stats.misses,
			evictions: stats.evictions,
			expired_removals: stats.expired_removals,
			current_size: stats.current_size,
			max_size_reached: stats.max_size_reached,
		}
	}

	/// Clean up expired entries
	pub async fn cleanup_expired(&self) {
		let mut entries = self.entries.write().await;
		let mut stats = self.stats.write().await;

		let initial_size = entries.len();
		entries.retain(|_, entry| !entry.is_expired());
		let removed = initial_size - entries.len();

		stats.expired_removals += removed as u64;
		stats.current_size = entries.len();
	}

	/// Get current cache size
	pub async fn size(&self) -> usize {
		let entries = self.entries.read().await;
		entries.len()
	}

	/// Check if cache contains a key
	pub async fn contains_key(&self, key: &K) -> bool {
		let entries = self.entries.read().await;
		entries.contains_key(key)
	}

	/// Evict least recently used entry
	fn evict_lru(&self, entries: &mut HashMap<K, CacheEntry<V>>, stats: &mut CacheStats) {
		if let Some((lru_key, _)) = entries
			.iter()
			.min_by_key(|(_, entry)| entry.last_accessed)
			.map(|(k, v)| (k.clone(), v.last_accessed))
		{
			entries.remove(&lru_key);
			stats.evictions += 1;
		}
	}

	/// Invalidate all entries matching a predicate.
	///
	/// Returns the number of entries removed.
	pub async fn invalidate_where<F>(&self, predicate: F) -> usize
	where
		F: Fn(&K) -> bool,
	{
		let mut entries = self.entries.write().await;
		let mut stats = self.stats.write().await;

		let initial_size = entries.len();
		entries.retain(|key, _| !predicate(key));
		let removed = initial_size - entries.len();

		stats.evictions += removed as u64;
		stats.current_size = entries.len();
		removed
	}

	/// Start background cleanup task
	pub fn start_cleanup_task(&self) -> tokio::task::JoinHandle<()> {
		let cache = Cache {
			config: self.config,
			entries: Arc::clone(&self.entries),
			stats: Arc::clone(&self.stats),
		};

		tokio::spawn(async move {
			let mut interval = tokio::time::interval(cache.config.cleanup_interval);
			loop {
				interval.tick().await;
				cache.cleanup_expired().await;
			}
		})
	}
}

/// Specialized cache for RPC responses
pub type RpcCache = Cache<String, serde_json::Value>;

impl RpcCache {
	/// Create a new RPC cache with optimized settings
	pub fn new_rpc_cache() -> Self {
		let config = CacheConfig {
			max_entries: 5000,
			default_ttl: Duration::from_secs(30), // 30 seconds for RPC responses
			cleanup_interval: Duration::from_secs(60),
			enable_lru: true,
		};
		Self::new(config)
	}

	/// Cache a block by hash or index
	pub async fn cache_block(&self, identifier: String, block: serde_json::Value) {
		// Blocks are immutable, so cache them for longer
		self.insert_with_ttl(
			format!("block:{}", identifier),
			block,
			Duration::from_secs(3600), // 1 hour
		)
		.await;
	}

	/// Cache a transaction by hash
	pub async fn cache_transaction(&self, tx_hash: String, transaction: serde_json::Value) {
		// Transactions are immutable, so cache them for longer
		self.insert_with_ttl(
			format!("tx:{}", tx_hash),
			transaction,
			Duration::from_secs(3600), // 1 hour
		)
		.await;
	}

	/// Cache contract state
	pub async fn cache_contract_state(&self, contract_hash: String, state: serde_json::Value) {
		// Contract state can change, so shorter TTL
		self.insert_with_ttl(
			format!("contract:{}", contract_hash),
			state,
			Duration::from_secs(60), // 1 minute
		)
		.await;
	}

	/// Cache balance information
	pub async fn cache_balance(&self, address: String, balance: serde_json::Value) {
		// Balances change frequently, so very short TTL
		self.insert_with_ttl(
			format!("balance:{}", address),
			balance,
			Duration::from_secs(10), // 10 seconds
		)
		.await;
	}

	/// Invalidate all cached entries with keys matching the given prefix.
	///
	/// Useful for clearing stale data after state changes, e.g.:
	/// - `invalidate_by_prefix("balance:")` after sending a transaction
	/// - `invalidate_by_prefix("contract:0xabc...")` after a contract update
	pub async fn invalidate_by_prefix(&self, prefix: &str) -> usize {
		let prefix = prefix.to_string();
		self.invalidate_where(|key| key.starts_with(&prefix)).await
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use tokio::time::{Duration, sleep};

	#[tokio::test]
	async fn test_cache_basic_operations() {
		let cache = Cache::new(CacheConfig::default());

		// Test insert and get
		cache.insert("key1".to_string(), "value1".to_string()).await;
		assert_eq!(cache.get(&"key1".to_string()).await, Some("value1".to_string()));

		// Test miss
		assert_eq!(cache.get(&"nonexistent".to_string()).await, None);

		// Test remove
		assert_eq!(cache.remove(&"key1".to_string()).await, Some("value1".to_string()));
		assert_eq!(cache.get(&"key1".to_string()).await, None);
	}

	#[tokio::test]
	async fn test_cache_expiration() {
		let config = CacheConfig { default_ttl: Duration::from_millis(100), ..Default::default() };
		let cache = Cache::new(config);

		cache.insert("key1".to_string(), "value1".to_string()).await;
		assert_eq!(cache.get(&"key1".to_string()).await, Some("value1".to_string()));

		// Wait for expiration
		sleep(Duration::from_millis(150)).await;
		assert_eq!(cache.get(&"key1".to_string()).await, None);
	}

	#[tokio::test]
	async fn test_cache_stats() {
		let cache = Cache::new(CacheConfig::default());

		// Test hits and misses
		cache.insert("key1".to_string(), "value1".to_string()).await;
		cache.get(&"key1".to_string()).await; // hit
		cache.get(&"nonexistent".to_string()).await; // miss

		let stats = cache.stats().await;
		assert_eq!(stats.hits, 1);
		assert_eq!(stats.misses, 1);
		assert_eq!(stats.hit_rate(), 0.5);
	}

	#[tokio::test]
	async fn test_rpc_cache() {
		let cache = RpcCache::new_rpc_cache();

		let block_data = serde_json::json!({
			"hash": "0x1234",
			"index": 100
		});

		cache.cache_block("100".to_string(), block_data.clone()).await;
		assert_eq!(cache.get(&"block:100".to_string()).await, Some(block_data));
	}
}
