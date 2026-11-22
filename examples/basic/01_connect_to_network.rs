/*!
# Basic Network Connection Example

This example demonstrates how to connect to Neo N3 networks and perform basic queries.

## What you'll learn:
- Connecting to TestNet and MainNet
- Checking node health and network status
- Querying basic blockchain information
- Handling connection errors gracefully

## Security Notes:
- Always verify TLS certificates in production
- Use multiple RPC endpoints for redundancy
- Implement proper error handling and retries
*/

use colored::*;
use neo3::neo_clients::{APITrait, HttpProvider, RpcClient};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("{}", "🚀 NeoRust Basic Network Connection Example".cyan().bold());
	println!("{}", "=".repeat(50));

	// Define network endpoints
	let testnet_endpoints = vec!["https://testnet1.neo.org:443", "https://testnet2.neo.org:443"];

	let mainnet_endpoints = vec!["https://mainnet1.neo.org:443", "https://mainnet2.neo.org:443"];

	// Test TestNet connections
	println!("\n{}", "📡 Testing TestNet Connections...".yellow().bold());
	for endpoint in testnet_endpoints {
		test_connection("TestNet", endpoint).await;
	}

	// Test MainNet connections
	println!("\n{}", "📡 Testing MainNet Connections...".yellow().bold());
	for endpoint in mainnet_endpoints {
		test_connection("MainNet", endpoint).await;
	}

	// Demonstrate detailed network information
	println!("\n{}", "📊 Detailed Network Information".green().bold());
	if let Ok(provider) = HttpProvider::new("https://testnet1.neo.org:443") {
		let client = RpcClient::new(provider);
		get_detailed_info(&client, "TestNet").await;
	}

	println!("\n{}", "✅ Network connection examples completed!".green().bold());
	Ok(())
}

async fn test_connection(network: &str, endpoint: &str) {
	print!("  {network} {endpoint}: ");

	match HttpProvider::new(endpoint) {
		Ok(provider) => {
			let client = RpcClient::new(provider);

			// Test with timeout
			match tokio::time::timeout(Duration::from_secs(10), client.get_block_count()).await {
				Ok(Ok(block_count)) => {
					println!(
						"{} (Block: {})",
						"✅ Connected".green(),
						block_count.to_string().cyan()
					);
				},
				Ok(Err(e)) => {
					println!("{} ({})", "❌ RPC Error".red(), e.to_string().dimmed());
				},
				Err(_) => {
					println!("{}", "⏰ Timeout".yellow());
				},
			}
		},
		Err(e) => {
			println!("{} ({})", "❌ Connection Error".red(), e.to_string().dimmed());
		},
	}
}

async fn get_detailed_info(client: &RpcClient<HttpProvider>, network: &str) {
	println!("\n{} Network Details:", network.cyan().bold());

	// Get version information
	if let Ok(version) = client.get_version().await {
		println!("  🔢 Node Version: {}", format!("{version:?}").cyan());
	}

	// Get current block count
	if let Ok(block_count) = client.get_block_count().await {
		println!("  📦 Current Block: {}", block_count.to_string().cyan());
	}

	// Get latest block information
	if let Ok(block_count) = client.get_block_count().await
		&& let Ok(block) = client.get_block_by_index(block_count - 1, true).await
	{
		println!("  ⏰ Latest Block Time: {}", format_timestamp(block.time));
		println!("  🔗 Block Hash: {}", block.hash.to_string().dimmed());
		if let Some(transactions) = &block.transactions {
			println!("  📋 Transactions: {}", transactions.len().to_string().cyan());
		} else {
			println!("  📋 Transactions: {}", "0".dimmed());
		}
	}

	// Get connection count
	if let Ok(connections) = client.get_connection_count().await {
		println!("  🌐 Connected Peers: {}", connections.to_string().cyan());
	}
}

fn format_timestamp(timestamp: u64) -> String {
	use std::time::{Duration, UNIX_EPOCH};

	let datetime = UNIX_EPOCH + Duration::from_millis(timestamp);
	format!("{datetime:?}").cyan().to_string()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	#[ignore = "requires a live Neo RPC endpoint"]
	async fn test_network_connection() {
		// Test that we can connect to at least one TestNet endpoint
		let provider = HttpProvider::new("https://testnet1.neo.org:443").unwrap();
		let client = RpcClient::new(provider);

		let block_count = tokio::time::timeout(Duration::from_secs(10), client.get_block_count())
			.await
			.expect("RPC request should not time out");
		let block_count = block_count.expect("Should be able to get block count from the network");
		assert!(block_count > 0, "Block count should be greater than 0");
	}
}
