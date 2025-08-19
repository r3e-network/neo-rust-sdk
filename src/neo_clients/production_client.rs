use crate::{
	neo_clients::{
		Cache, CacheConfig, CircuitBreaker, CircuitBreakerConfig, ConnectionPool, PoolConfig, RpcCache,
	},
	neo_error::{Neo3Error, Neo3Result},
};
use serde_json::Value;
use std::{sync::Arc, time::Duration};
use tokio::sync::RwLock;

/// Production-ready RPC client with connection pooling, caching, and circuit breaker
pub struct ProductionRpcClient {
	pool: ConnectionPool,
	cache: RpcCache,
	circuit_breaker: CircuitBreaker,
	config: ProductionClientConfig,
	stats: Arc<RwLock<ProductionClientStats>>,
}

/// Configuration for production RPC client
#[derive(Debug, Clone)]
pub struct ProductionClientConfig {
	/// Connection pool configuration
	pub pool_config: PoolConfig,
	/// Cache configuration
	pub cache_config: CacheConfig,
	/// Circuit breaker configuration
	pub circuit_breaker_config: CircuitBreakerConfig,
	/// Enable request/response logging
	pub enable_logging: bool,
	/// Enable metrics collection
	pub enable_metrics: bool,
}

impl Default for ProductionClientConfig {
	fn default() -> Self {
		Self {
			pool_config: PoolConfig {
				max_connections: 20,
				min_idle: 5,
				max_idle_time: Duration::from_secs(300),
				connection_timeout: Duration::from_secs(30),
				request_timeout: Duration::from_secs(60),
				max_retries: 3,
				retry_delay: Duration::from_millis(1000),
			},
			cache_config: CacheConfig {
				max_entries: 10000,
				default_ttl: Duration::from_secs(30),
				cleanup_interval: Duration::from_secs(60),
				enable_lru: true,
			},
			circuit_breaker_config: CircuitBreakerConfig {
				failure_threshold: 5,
				timeout: Duration::from_secs(60),
				success_threshold: 3,
				failure_window: Duration::from_secs(60),
				half_open_max_requests: 3,
			},
			enable_logging: true,
			enable_metrics: true,
		}
	}
}

/// Production client statistics
#[derive(Debug, Default)]
pub struct ProductionClientStats {
	pub total_requests: u64,
	pub cache_hits: u64,
	pub cache_misses: u64,
	pub circuit_breaker_rejections: u64,
	pub successful_requests: u64,
	pub failed_requests: u64,
	pub average_response_time_ms: f64,
}

impl ProductionRpcClient {
	/// Create a new production RPC client
	pub fn new(endpoint: String, config: ProductionClientConfig) -> Self {
		let pool = ConnectionPool::new(endpoint, config.pool_config.clone());
		let cache = Cache::new(config.cache_config.clone());
		let circuit_breaker = CircuitBreaker::new(config.circuit_breaker_config.clone());

		Self {
			pool,
			cache,
			circuit_breaker,
			config,
			stats: Arc::new(RwLock::new(ProductionClientStats::default())),
		}
	}

	/// Execute an RPC call with full production features
	pub async fn call(&self, method: &str, params: Vec<Value>) -> Neo3Result<Value> {
		let start_time = std::time::Instant::now();

		// Update total requests
		{
			let mut stats = self.stats.write().await;
			stats.total_requests += 1;
		}

		// Create cache key
		let cache_key = self.create_cache_key(method, &params);

		// Check cache first for idempotent operations
		if self.is_cacheable_method(method) {
			if let Some(cached_result) = self.cache.get(&cache_key).await {
				let mut stats = self.stats.write().await;
				stats.cache_hits += 1;
				return Ok(cached_result);
			} else {
				let mut stats = self.stats.write().await;
				stats.cache_misses += 1;
			}
		}

		// Clone params for the closure
		let params_clone = params.clone();
		let method_clone = method.to_string();

		// Execute through circuit breaker
		let result: Neo3Result<Value> = self
			.circuit_breaker
			.call(async move {
				self.pool
					.execute(move |client| {
						let params_inner = params_clone.clone();
						let method_inner = method_clone.clone();
						Box::pin(async move {
							client.request(&method_inner, params_inner).await.map_err(|e| {
								Neo3Error::Network(crate::neo_error::NetworkError::RpcError {
									code: -1,
									message: e.to_string(),
								})
							})
						})
					})
					.await
			})
			.await;

		// Update statistics
		let elapsed = start_time.elapsed();
		let mut stats = self.stats.write().await;

		match &result {
			Ok(value) => {
				stats.successful_requests += 1;

				// Cache successful results for cacheable methods
				if self.is_cacheable_method(method) {
					let ttl = self.get_cache_ttl(method);
					drop(stats);
					self.cache.insert_with_ttl(cache_key, value.clone(), ttl).await;
					stats = self.stats.write().await;
				}
			},
			Err(_) => {
				stats.failed_requests += 1;
			},
		}

		// Update average response time
		let total_requests = stats.successful_requests + stats.failed_requests;
		if total_requests > 0 {
			stats.average_response_time_ms = (stats.average_response_time_ms
				* (total_requests - 1) as f64
				+ elapsed.as_millis() as f64)
				/ total_requests as f64;
		}

		if self.config.enable_logging {
			match &result {
				Ok(_) => {
					tracing::info!(
						method = method,
						duration_ms = elapsed.as_millis(),
						"RPC call successful"
					);
				},
				Err(e) => {
					tracing::error!(
						method = method,
						duration_ms = elapsed.as_millis(),
						error = %e,
						"RPC call failed"
					);
				},
			}
		}

		result
	}

	/// Get current block count with caching
	pub async fn get_block_count(&self) -> Neo3Result<u64> {
		let result = self.call("getblockcount", vec![]).await?;
		result.as_u64().ok_or_else(|| {
			Neo3Error::Serialization(crate::neo_error::SerializationError::InvalidFormat(
				"Invalid block count format".to_string(),
			))
		})
	}

	/// Get block by hash or index with long-term caching
	pub async fn get_block(&self, identifier: Value) -> Neo3Result<Value> {
		let result = self.call("getblock", vec![identifier.clone(), Value::Bool(true)]).await?;

		// Cache blocks for longer since they're immutable
		let cache_key = format!("block:{}", identifier);
		self.cache
			.insert_with_ttl(cache_key, result.clone(), Duration::from_secs(3600))
			.await;

		Ok(result)
	}

	/// Get transaction with long-term caching
	pub async fn get_transaction(&self, tx_hash: String) -> Neo3Result<Value> {
		let result = self
			.call("getrawtransaction", vec![Value::String(tx_hash.clone()), Value::Bool(true)])
			.await?;

		// Cache transactions for longer since they're immutable
		let cache_key = format!("tx:{}", tx_hash);
		self.cache
			.insert_with_ttl(cache_key, result.clone(), Duration::from_secs(3600))
			.await;

		Ok(result)
	}

	/// Get contract state with short-term caching
	pub async fn get_contract_state(&self, contract_hash: String) -> Neo3Result<Value> {
		let result = self
			.call("getcontractstate", vec![Value::String(contract_hash.clone())])
			.await?;

		// Cache contract state for shorter time since it can change
		let cache_key = format!("contract:{}", contract_hash);
		self.cache
			.insert_with_ttl(cache_key, result.clone(), Duration::from_secs(60))
			.await;

		Ok(result)
	}

	/// Get balance with very short-term caching
	pub async fn get_nep17_balances(&self, address: String) -> Neo3Result<Value> {
		let result = self.call("getnep17balances", vec![Value::String(address.clone())]).await?;

		// Cache balances for very short time since they change frequently
		let cache_key = format!("balance:{}", address);
		self.cache
			.insert_with_ttl(cache_key, result.clone(), Duration::from_secs(10))
			.await;

		Ok(result)
	}

	/// Send raw transaction (not cached)
	pub async fn send_raw_transaction(&self, transaction_hex: String) -> Neo3Result<Value> {
		self.call("sendrawtransaction", vec![Value::String(transaction_hex)]).await
	}

	/// Get production client statistics
	pub async fn get_stats(&self) -> ProductionClientStats {
		let stats = self.stats.read().await;
		ProductionClientStats {
			total_requests: stats.total_requests,
			cache_hits: stats.cache_hits,
			cache_misses: stats.cache_misses,
			circuit_breaker_rejections: stats.circuit_breaker_rejections,
			successful_requests: stats.successful_requests,
			failed_requests: stats.failed_requests,
			average_response_time_ms: stats.average_response_time_ms,
		}
	}

	/// Get detailed health information
	pub async fn get_health(&self) -> serde_json::Value {
		let stats = self.get_stats().await;
		let pool_stats = self.pool.get_stats().await;
		let cache_stats = self.cache.stats().await;
		let cb_stats = self.circuit_breaker.get_stats().await;

		serde_json::json!({
			"status": if cb_stats.current_state == crate::neo_clients::CircuitState::Open { "unhealthy" } else { "healthy" },
			"timestamp": chrono::Utc::now().to_rfc3339(),
			"stats": {
				"total_requests": stats.total_requests,
				"success_rate": if stats.total_requests > 0 {
					stats.successful_requests as f64 / stats.total_requests as f64
				} else { 0.0 },
				"cache_hit_rate": cache_stats.hit_rate(),
				"average_response_time_ms": stats.average_response_time_ms,
				"circuit_breaker_state": format!("{:?}", cb_stats.current_state),
				"pool": {
					"active_connections": pool_stats.current_active_connections,
					"idle_connections": pool_stats.current_idle_connections,
					"total_created": pool_stats.total_connections_created
				}
			}
		})
	}

	/// Perform health check by calling a simple RPC method
	pub async fn health_check(&self) -> Neo3Result<bool> {
		match self.call("getversion", vec![]).await {
			Ok(_) => Ok(true),
			Err(_) => Ok(false),
		}
	}

	/// Create cache key for method and parameters
	fn create_cache_key(&self, method: &str, params: &[Value]) -> String {
		let params_str = serde_json::to_string(params).unwrap_or_default();
		format!("{}:{}", method, params_str)
	}

	/// Check if a method should be cached
	fn is_cacheable_method(&self, method: &str) -> bool {
		matches!(
			method,
			"getblock"
				| "getrawtransaction"
				| "getcontractstate"
				| "getnep17balances"
				| "getblockcount"
				| "getversion"
				| "getpeers" | "getconnectioncount"
		)
	}

	/// Get appropriate cache TTL for different methods
	fn get_cache_ttl(&self, method: &str) -> Duration {
		match method {
			"getblock" | "getrawtransaction" => Duration::from_secs(3600), // 1 hour - immutable
			"getcontractstate" => Duration::from_secs(60),                 // 1 minute - can change
			"getnep17balances" => Duration::from_secs(10),                 // 10 seconds - changes frequently
			"getblockcount" => Duration::from_secs(5), // 5 seconds - changes every ~15 seconds
			_ => self.config.cache_config.default_ttl,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_production_client_creation() {
		let config = ProductionClientConfig::default();
		let client = ProductionRpcClient::new("https://testnet.neo.org:443".to_string(), config);

		let stats = client.get_stats().await;
		assert_eq!(stats.total_requests, 0);
	}

	#[tokio::test]
	async fn test_cache_key_generation() {
		let config = ProductionClientConfig::default();
		let client = ProductionRpcClient::new("https://testnet.neo.org:443".to_string(), config);

		let key1 = client.create_cache_key("getblock", &[Value::String("hash1".to_string())]);
		let key2 = client.create_cache_key("getblock", &[Value::String("hash2".to_string())]);

		assert_ne!(key1, key2);
		assert!(key1.contains("getblock"));
	}

	#[tokio::test]
	async fn test_cacheable_methods() {
		let config = ProductionClientConfig::default();
		let client = ProductionRpcClient::new("https://testnet.neo.org:443".to_string(), config);

		assert!(client.is_cacheable_method("getblock"));
		assert!(client.is_cacheable_method("getrawtransaction"));
		assert!(!client.is_cacheable_method("sendrawtransaction"));
	}
}
