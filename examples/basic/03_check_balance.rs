/*!
# Basic Balance Checking Example

This example demonstrates how to check NEO and GAS balances for addresses.

## What you'll learn:
- Checking NEO and GAS balances
- Querying NEP-17 token balances
- Understanding decimal precision
- Handling balance formatting

## Network Requirements:
- Requires connection to Neo N3 TestNet or MainNet
- Some test addresses may have zero balances
*/

use colored::*;
use neo3::{
	neo_clients::{APITrait, HttpProvider, RpcClient},
	neo_types::{ScriptHash, ScriptHashExtension},
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("{}", "💰 NeoRust Basic Balance Checking Example".cyan().bold());
	println!("{}", "=".repeat(50));

	// Connect to TestNet
	println!("\n{}", "📡 Connecting to TestNet...".yellow().bold());
	let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
	let client = RpcClient::new(provider);

	// Test connection
	match tokio::time::timeout(Duration::from_secs(10), client.get_block_count()).await {
		Ok(Ok(block_count)) => {
			println!("  ✅ Connected to TestNet (Block: {})", block_count.to_string().cyan());
		},
		Ok(Err(e)) => {
			println!("  ❌ RPC Error: {}", e.to_string().red());
			return Ok(());
		},
		Err(_) => {
			println!("  ⏰ Connection timeout");
			return Ok(());
		},
	}

	// Example addresses (these may have zero balances)
	let test_addresses = vec![
		"NM7Aky765FG8NhhwtxjXRx7jEL1cnw7PBP", // Common test address
		"NXV6f9rNvR7hqP8PD5nTi6fQjZDmb7TaX5", // Another test address
		"NfNkevdh2MZ7uutXM6W8s5uD7XhP4AkrFs", // Genesis address
	];

	// Check balances for each address
	for address_str in test_addresses {
		println!("\n{}", format!("💳 Checking balances for: {address_str}").green().bold());
		check_address_balances(&client, address_str).await;
	}

	// Demonstrate balance formatting
	println!("\n{}", "📊 Balance Formatting Examples".green().bold());
	demonstrate_balance_formatting();

	println!("\n{}", "✅ Balance checking examples completed!".green().bold());
	println!("\n{}", "💡 Tips:".yellow().bold());
	println!("  • NEO is indivisible (no decimal places)");
	println!("  • GAS has 8 decimal places");
	println!("  • Use TestNet faucets to get test tokens");
	println!("  • Always handle network timeouts gracefully");

	Ok(())
}

async fn check_address_balances(client: &RpcClient<HttpProvider>, address_str: &str) {
	// Convert address to script hash
	let script_hash = match ScriptHash::from_address(address_str) {
		Ok(hash) => hash,
		Err(_) => {
			println!("  ❌ Invalid address format");
			return;
		},
	};

	// Check NEP-17 token balances
	match tokio::time::timeout(Duration::from_secs(10), client.get_nep17_balances(script_hash))
		.await
	{
		Ok(Ok(balances)) => {
			// Look for NEO and GAS in the balances
			let mut neo_found = false;
			let mut gas_found = false;

			for balance in &balances.balances {
				match balance.asset_hash.to_string().as_str() {
					// NEO token hash
					"0xef4073a0f2b305a38ec4050e4d3d28bc40ea63f5" => {
						let amount = balance.amount.parse::<u64>().unwrap_or(0);
						print!("  🔶 NEO Balance: ");
						if amount > 0 {
							println!("{}", format!("{amount} NEO").green());
						} else {
							println!("{}", "0 NEO".dimmed());
						}
						neo_found = true;
					},
					// GAS token hash
					"0xd2a4cff31913016155e38e474a2c06d08be276cf" => {
						let amount = balance.amount.parse::<u64>().unwrap_or(0);
						print!("  ⛽ GAS Balance: ");
						if amount > 0 {
							println!(
								"{}",
								format!("{:.8} GAS", amount as f64 / 100_000_000.0).green()
							);
						} else {
							println!("{}", "0.00000000 GAS".dimmed());
						}
						gas_found = true;
					},
					_ => {},
				}
			}

			// Show zero balances if tokens not found
			if !neo_found {
				println!("  🔶 NEO Balance: {}", "0 NEO".dimmed());
			}
			if !gas_found {
				println!("  ⛽ GAS Balance: {}", "0.00000000 GAS".dimmed());
			}
		},
		Ok(Err(e)) => {
			println!("  ❌ RPC Error: {}", e.to_string().red());
		},
		Err(_) => {
			println!("  ⏰ Request timeout");
		},
	}
}

// Removed - balance checking is now done directly in check_address_balances

fn demonstrate_balance_formatting() {
	let neo_amounts = vec![1, 50, 1000, 10000];
	let gas_amounts = vec![100000000u64, 250000000, 1000000000, 10000000000]; // In base units

	println!("\n  🔶 NEO Formatting:");
	for amount in neo_amounts {
		println!("    {} NEO", amount.to_string().cyan());
	}

	println!("\n  ⛽ GAS Formatting:");
	for amount in gas_amounts {
		let gas_value = amount as f64 / 100_000_000.0;
		println!("    {:.8} GAS", gas_value.to_string().cyan());
	}

	println!("\n  📏 Precision Examples:");
	println!("    1 NEO = {} base units", "1".cyan());
	println!("    1 GAS = {} base units", "100000000".cyan());
	println!("    0.00000001 GAS = {} base unit", "1".cyan());
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_gas_formatting() {
		let base_units = 100_000_000u64; // 1 GAS in base units
		let gas_value = base_units as f64 / 100_000_000.0;
		assert_eq!(gas_value, 1.0);

		let small_amount = 1u64; // Smallest GAS unit
		let small_gas = small_amount as f64 / 100_000_000.0;
		assert_eq!(small_gas, 0.00000001);
	}

	#[test]
	fn test_address_validation() {
		// Valid Neo N3 addresses should pass validation
		assert!(ScriptHash::from_address("NM7Aky765FG8NhhwtxjXRx7jEL1cnw7PBP").is_ok());

		// Invalid addresses should fail
		assert!(ScriptHash::from_address("InvalidAddress").is_err());
		assert!(ScriptHash::from_address("").is_err());
	}

	#[tokio::test]
	async fn test_connection_timeout() {
		// This test verifies timeout handling works
		let provider = HttpProvider::new("https://invalid-endpoint.example.com").unwrap();
		let client = RpcClient::new(provider);

		let result =
			tokio::time::timeout(Duration::from_millis(100), client.get_block_count()).await;

		// Should timeout or error, not hang indefinitely
		assert!(result.is_err() || result.unwrap().is_err());
	}
}
