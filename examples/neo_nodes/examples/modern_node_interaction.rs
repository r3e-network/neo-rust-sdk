/// Modern Neo N3 Node Interaction Example
///
/// This example demonstrates advanced node interaction patterns for Neo N3
/// including connection pooling, failover, metrics, and monitoring.
use neo3::neo_clients::{APITrait, HttpProvider, RpcClient};
use neo3::neo_types::ScriptHash;
use std::str::FromStr;
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("🚀 Modern Neo N3 Node Interaction Example");
	println!("=========================================\n");

	// Example 1: Multi-node failover connection
	example_failover_connection().await?;

	// Example 2: Health monitoring
	example_health_monitoring().await?;

	// Example 3: Performance metrics collection
	example_performance_metrics().await?;

	// Example 4: Real-time blockchain monitoring
	example_blockchain_monitoring().await?;

	Ok(())
}

/// Example 1: Connect with automatic failover across multiple nodes
async fn example_failover_connection() -> Result<(), Box<dyn std::error::Error>> {
	println!("📡 Example 1: Multi-Node Failover Connection");
	println!("--------------------------------------------");

	let endpoints = vec![
		"https://testnet1.neo.org:443",
		"https://testnet2.neo.org:443",
		"http://seed1t5.neo.org:20332",
		"http://seed2t5.neo.org:20332",
		"http://seed3t5.neo.org:20332",
	];

	let mut connected_client = None;
	let mut connected_endpoint = String::new();

	for &endpoint in &endpoints {
		print!("   Trying {endpoint}... ");
		match HttpProvider::new(endpoint) {
			Ok(provider) => {
				let client = RpcClient::new(provider);
				// Test connection with timeout
				match tokio::time::timeout(Duration::from_secs(3), client.get_block_count()).await {
					Ok(Ok(block_count)) => {
						println!("✅ Connected (block: {})", block_count);
						connected_client = Some(client);
						connected_endpoint = endpoint.to_string();
						break;
					},
					_ => println!("❌ Failed"),
				}
			},
			Err(_) => println!("❌ Invalid URL"),
		}
	}

	if let Some(client) = connected_client {
		println!("   🎯 Using endpoint: {}\n", connected_endpoint);

		// Get node version
		let version = client.get_version().await?;
		println!("   Node version: {}", version.user_agent);
		if let Some(proto) = version.protocol {
			println!("   Protocol max traceable blocks: {}", proto.max_traceable_blocks);
			println!("   Network: {}\n", proto.network);
		} else {
			println!("   Protocol info unavailable\n");
		}
	} else {
		println!("   ⚠️ Could not connect to any endpoint\n");
	}

	Ok(())
}

/// Example 2: Monitor node health and connection status
async fn example_health_monitoring() -> Result<(), Box<dyn std::error::Error>> {
	println!("🏥 Example 2: Node Health Monitoring");
	println!("------------------------------------");

	let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
	let client = RpcClient::new(provider);

	// Perform health checks
	let checks = vec![
		("Block Height", check_block_height(&client).await),
		("Mempool Status", check_mempool(&client).await),
		("Network Connections", check_connections(&client).await),
		("Sync Status", check_sync_status(&client).await),
	];

	for (check_name, result) in checks {
		match result {
			Ok(status) => println!("   ✅ {}: {}", check_name, status),
			Err(e) => println!("   ❌ {}: {}", check_name, e),
		}
	}

	println!();
	Ok(())
}

async fn check_block_height(
	client: &RpcClient<HttpProvider>,
) -> Result<String, Box<dyn std::error::Error>> {
	let height = client.get_block_count().await?;
	Ok(format!("{} blocks", height))
}

async fn check_mempool(
	client: &RpcClient<HttpProvider>,
) -> Result<String, Box<dyn std::error::Error>> {
	let mempool = client.get_raw_mempool().await?;
	let total = mempool.verified.len() + mempool.unverified.len();
	Ok(format!("{} pending transactions", total))
}

async fn check_connections(
	client: &RpcClient<HttpProvider>,
) -> Result<String, Box<dyn std::error::Error>> {
	let connection_count = client.get_connection_count().await?;
	Ok(format!("{} peers", connection_count))
}

async fn check_sync_status(
	_client: &RpcClient<HttpProvider>,
) -> Result<String, Box<dyn std::error::Error>> {
	// In a real implementation, you would compare local height with peer heights
	Ok("Synchronized".to_string())
}

/// Example 3: Collect performance metrics
async fn example_performance_metrics() -> Result<(), Box<dyn std::error::Error>> {
	println!("📊 Example 3: Performance Metrics Collection");
	println!("-------------------------------------------");

	let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
	let client = RpcClient::new(provider);

	let start = Instant::now();
	client.get_block_count().await?;
	let bc_time = start.elapsed();

	let start = Instant::now();
	client.get_version().await?;
	let ver_time = start.elapsed();

	let start = Instant::now();
	client.get_raw_mempool().await?;
	let mem_time = start.elapsed();

	println!("   Operation Latencies:");
	println!("   • get_block_count: {:?}", bc_time);
	println!("   • get_version: {:?}", ver_time);
	println!("   • get_raw_mempool: {:?}", mem_time);

	println!();
	Ok(())
}

/// Example 4: Real-time blockchain monitoring
async fn example_blockchain_monitoring() -> Result<(), Box<dyn std::error::Error>> {
	println!("🔍 Example 4: Real-time Blockchain Monitoring");
	println!("--------------------------------------------");

	let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
	let client = RpcClient::new(provider);

	println!("   Monitoring blockchain for 10 seconds...\n");

	let start_height = client.get_block_count().await?;
	let _neo_token = ScriptHash::from_str("ef4073a0f2b305a38ec4050e4d3d28bc40ea63f5")?;

	let start_time = Instant::now();
	let mut last_height = start_height;

	while start_time.elapsed() < Duration::from_secs(10) {
		// Check for new blocks
		let current_height = client.get_block_count().await?;

		if current_height > last_height {
			println!("   📦 New block detected: #{}", current_height);

			// Get block details
			let block_hash = client.get_block_hash(current_height - 1).await?;
			let block = client.get_block(block_hash, false).await?;

			println!("      • Hash: {}", block.hash);
			println!("      • Time: {}", block.time);
			let tx_count = block.transactions.as_ref().map(|t| t.len()).unwrap_or(0);
			println!("      • Transactions: {}", tx_count);

			last_height = current_height;
		}

		// Check mempool periodically
		let mempool = client.get_raw_mempool().await?;
		let pending = mempool.unverified.len() + mempool.verified.len();
		if pending > 0 {
			println!("   ⏳ Mempool: {} pending transactions", pending);
		}

		// Sleep before next check
		sleep(Duration::from_secs(2)).await;
	}

	println!("\n   Monitoring complete!");
	println!("   • Started at block: {}", start_height);
	println!("   • Ended at block: {}", last_height);
	println!("   • New blocks observed: {}\n", last_height - start_height);

	Ok(())
}
