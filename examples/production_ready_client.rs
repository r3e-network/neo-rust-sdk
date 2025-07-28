use neo3::neo_clients::{
	CacheConfig, CircuitBreakerConfig, PoolConfig, ProductionClientConfig, ProductionRpcClient,
};
use std::time::Duration;

/// Example demonstrating production-ready Neo RPC client with all advanced features
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Initialize logging
	tracing_subscriber::fmt::init();

	println!("🚀 Neo3 Production-Ready RPC Client Example");
	println!("============================================\n");

	// Create production client configuration
	let config = ProductionClientConfig {
		pool_config: PoolConfig {
			max_connections: 10,
			min_idle: 2,
			max_idle_time: Duration::from_secs(300),
			connection_timeout: Duration::from_secs(30),
			request_timeout: Duration::from_secs(60),
			max_retries: 3,
			retry_delay: Duration::from_millis(1000),
		},
		cache_config: CacheConfig {
			max_entries: 1000,
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
	};

	// Create production client
	let client = ProductionRpcClient::new("https://testnet.neo.org:443".to_string(), config);

	println!("✅ Production client created with advanced features:");
	println!("   • Connection pooling (max 10 connections)");
	println!("   • Intelligent caching with TTL");
	println!("   • Circuit breaker for fault tolerance");
	println!("   • Comprehensive metrics and logging\n");

	// Demonstrate basic operations
	println!("📊 Testing basic blockchain operations...");

	// Get block count (cached for 5 seconds)
	match client.get_block_count().await {
		Ok(block_count) => {
			println!("   Current block count: {block_count}");
		},
		Err(e) => {
			println!("   ❌ Failed to get block count: {e}");
		},
	}

	// Get the latest block (cached for 1 hour since blocks are immutable)
	let latest_block_index = if let Ok(count) = client.get_block_count().await {
		count.saturating_sub(1)
	} else {
		1000 // fallback block index
	};

	match client.get_block(serde_json::json!(latest_block_index)).await {
		Ok(block) => {
			if let Some(hash) = block.get("hash").and_then(|h| h.as_str()) {
				println!("   Latest block hash: {hash}");
			}
		},
		Err(e) => {
			println!("   ❌ Failed to get latest block: {e}");
		},
	}

	// Test caching by making the same request again
	println!("\n🔄 Testing cache performance...");
	let start = std::time::Instant::now();
	let _ = client.get_block_count().await;
	let first_call = start.elapsed();

	let start = std::time::Instant::now();
	let _ = client.get_block_count().await;
	let second_call = start.elapsed();

	println!("   First call (no cache): {first_call:?}");
	println!("   Second call (cached): {second_call:?}");
	println!(
		"   Cache speedup: {:.2}x faster",
		first_call.as_nanos() as f64 / second_call.as_nanos() as f64
	);

	// Demonstrate contract interaction
	println!("\n📋 Testing contract operations...");

	// Get NEO token contract state (cached for 1 minute)
	let neo_token_hash = "0xef4073a0f2b305a38ec4050e4d3d28bc40ea63f5"; // NEO token on TestNet
	match client.get_contract_state(neo_token_hash.to_string()).await {
		Ok(contract_state) => {
			if let Some(name) = contract_state
				.get("manifest")
				.and_then(|m| m.get("name"))
				.and_then(|n| n.as_str())
			{
				println!("   Contract name: {name}");
			}
		},
		Err(e) => {
			println!("   ❌ Failed to get contract state: {e}");
		},
	}

	// Test balance checking (cached for 10 seconds)
	let test_address = "NiNmXL8FjEUEs1nfX9uHFBNaenxDHJtmuB"; // TestNet faucet address
	match client.get_nep17_balances(test_address.to_string()).await {
		Ok(balances) => {
			if let Some(balance_array) = balances.get("balance").and_then(|b| b.as_array()) {
				println!("   Address {} has {} token types", test_address, balance_array.len());
			}
		},
		Err(e) => {
			println!("   ❌ Failed to get balances: {e}");
		},
	}

	// Demonstrate health monitoring
	println!("\n🏥 Health monitoring and statistics...");

	let health = client.get_health().await;
	println!("   Health status: {}", health.get("status").unwrap_or(&serde_json::json!("unknown")));

	let stats = client.get_stats().await;
	println!("   Total requests: {}", stats.total_requests);
	println!("   Cache hits: {}", stats.cache_hits);
	println!("   Cache misses: {}", stats.cache_misses);
	println!(
		"   Success rate: {:.2}%",
		if stats.total_requests > 0 {
			(stats.successful_requests as f64 / stats.total_requests as f64) * 100.0
		} else {
			0.0
		}
	);
	println!("   Average response time: {:.2}ms", stats.average_response_time_ms);

	// Demonstrate circuit breaker (simulate failures)
	println!("\n⚡ Testing circuit breaker resilience...");

	// Make requests to a non-existent endpoint to trigger circuit breaker
	let failing_client = ProductionRpcClient::new(
		"https://nonexistent.neo.endpoint:443".to_string(),
		ProductionClientConfig::default(),
	);

	println!("   Making requests to failing endpoint...");
	for i in 1..=7 {
		match failing_client.get_block_count().await {
			Ok(_) => println!("   Request {i}: ✅ Success"),
			Err(_) => println!("   Request {i}: ❌ Failed"),
		}

		if i == 5 {
			println!("   Circuit breaker should be OPEN now (after 5 failures)");
		}
	}

	// Test health check
	println!("\n🔍 Performing health check...");
	match client.health_check().await {
		Ok(true) => println!("   ✅ Client is healthy and responsive"),
		Ok(false) => println!("   ⚠️  Client is not responding properly"),
		Err(e) => println!("   ❌ Health check failed: {e}"),
	}

	// Final statistics
	println!("\n📈 Final performance statistics:");
	let final_stats = client.get_stats().await;
	let final_health = client.get_health().await;

	println!("   Total operations: {}", final_stats.total_requests);
	println!(
		"   Cache efficiency: {:.1}% hit rate",
		if final_stats.cache_hits + final_stats.cache_misses > 0 {
			(final_stats.cache_hits as f64
				/ (final_stats.cache_hits + final_stats.cache_misses) as f64)
				* 100.0
		} else {
			0.0
		}
	);

	if let Some(pool_stats) = final_health.get("stats").and_then(|s| s.get("pool")) {
		if let (Some(active), Some(idle)) = (
			pool_stats.get("active_connections").and_then(serde_json::Value::as_u64),
			pool_stats.get("idle_connections").and_then(serde_json::Value::as_u64),
		) {
			println!("   Connection pool: {active} active, {idle} idle");
		}
	}

	println!("\n🎉 Production client demonstration completed!");
	println!("   The client is ready for production deployment with:");
	println!("   • High availability through connection pooling");
	println!("   • Performance optimization via intelligent caching");
	println!("   • Fault tolerance with circuit breaker pattern");
	println!("   • Comprehensive monitoring and metrics");
	println!("   • Production-grade error handling and logging");

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_production_client_basic_operations() {
		let client = ProductionRpcClient::new(
			"https://testnet.neo.org:443".to_string(),
			ProductionClientConfig::default(),
		);

		// Test that client can be created and basic operations work
		let stats = client.get_stats().await;
		assert_eq!(stats.total_requests, 0);

		// Test health check
		let health_result = client.health_check().await;
		// Don't assert success since network might not be available in CI
		assert!(health_result.is_ok());
	}

	#[tokio::test]
	async fn test_production_client_configuration() {
		let config = ProductionClientConfig {
			pool_config: PoolConfig { max_connections: 5, ..Default::default() },
			cache_config: CacheConfig { max_entries: 100, ..Default::default() },
			..Default::default()
		};

		let client = ProductionRpcClient::new("https://testnet.neo.org:443".to_string(), config);

		let stats = client.get_stats().await;
		assert_eq!(stats.total_requests, 0);
	}
}
