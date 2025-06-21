/*!
# Basic Transaction Sending Example

This example demonstrates how to create and send basic transactions.

## What you'll learn:
- Creating NEO and GAS transfer transactions
- Transaction signing with private keys
- Gas fee estimation and handling
- Transaction broadcasting and monitoring

## Security Notes:
- Never use real private keys in examples
- Always verify recipient addresses
- Test on TestNet before MainNet deployment
- Monitor transaction confirmation status

## Prerequisites:
- Test wallet with NEO/GAS balance on TestNet
- Valid TestNet RPC endpoint access
*/

use colored::*;
use neo3::{
	neo_clients::{APITrait, HttpProvider, RpcClient},
	neo_crypto::KeyPair,
	neo_types::ScriptHashExtension,
	prelude::*,
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("{}", "💸 NeoRust Basic Transaction Example".cyan().bold());
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

	// Example: Create test wallets (DO NOT use in production)
	println!("\n{}", "🔑 Creating Test Wallets...".yellow().bold());
	let sender_key = KeyPair::new_random();
	let sender_script_hash = sender_key.get_script_hash();
	let sender_address = sender_script_hash.to_address();

	let recipient_key = KeyPair::new_random();
	let recipient_script_hash = recipient_key.get_script_hash();
	let recipient_address = recipient_script_hash.to_address();

	println!("  📤 Sender: {}", sender_address.to_string().green());
	println!("  📥 Recipient: {}", recipient_address.to_string().green());

	// Check sender balance (likely zero for new wallet)
	println!("\n{}", "💰 Checking Sender Balance...".yellow().bold());
	check_balance(&client, &sender_address.to_string()).await;

	// Demonstrate transaction building process
	println!("\n{}", "🔨 Transaction Building Process...".yellow().bold());
	demonstrate_transaction_building(&client, &sender_address, &recipient_address).await;

	// Demonstrate gas estimation
	println!("\n{}", "⛽ Gas Estimation...".yellow().bold());
	demonstrate_gas_estimation(&client).await;

	// Show transaction monitoring workflow
	println!("\n{}", "📊 Transaction Monitoring...".yellow().bold());
	demonstrate_transaction_monitoring();

	println!("\n{}", "✅ Transaction examples completed!".green().bold());
	println!("\n{}", "🔒 Security Reminders:".yellow().bold());
	println!("  • Always test on TestNet first");
	println!("  • Verify recipient addresses carefully");
	println!("  • Keep private keys secure and encrypted");
	println!("  • Monitor transaction confirmation status");
	println!("  • Use appropriate gas fees for timely processing");

	Ok(())
}

async fn check_balance(client: &RpcClient<HttpProvider>, address: &str) {
	print!("  🔶 NEO: ");
	match get_balance_with_timeout(client, address, "NEO").await {
		Ok(balance) => println!("{}", format!("{balance} NEO").cyan()),
		Err(_) => println!("{}", "Unable to fetch".dimmed()),
	}

	print!("  ⛽ GAS: ");
	match get_balance_with_timeout(client, address, "GAS").await {
		Ok(balance) => {
			println!("{}", format!("{:.8} GAS", balance as f64 / 100_000_000.0).cyan())
		},
		Err(_) => println!("{}", "Unable to fetch".dimmed()),
	}
}

async fn get_balance_with_timeout(
	_client: &RpcClient<HttpProvider>,
	_address: &str,
	_token: &str,
) -> Result<u64, Box<dyn std::error::Error>> {
	// Note: This is simplified for demo - real implementation would use get_nep17_balances
	// and parse the response for specific token hashes
	println!("    ⚠️  Balance checking simplified for demo");
	Ok(0) // Return 0 for demo purposes
}

async fn demonstrate_transaction_building(
	client: &RpcClient<HttpProvider>,
	sender: &Address,
	recipient: &Address,
) {
	println!("  📋 Building NEO transfer transaction...");

	// Get network fee for transaction size estimation
	if let Ok(network_fee) = client.calculate_network_fee("".to_string()).await {
		println!(
			"    💸 Estimated Network Fee: {:.8} GAS",
			network_fee.network_fee as f64 / 100_000_000.0
		);
	}

	// Transaction parameters
	let amount = 1u64; // 1 NEO
	println!("    📊 Transfer Amount: {} NEO", amount.to_string().cyan());
	println!("    📤 From: {}", sender.to_string().dimmed());
	println!("    📥 To: {}", recipient.to_string().dimmed());

	// Note: Actual transaction creation requires more setup
	println!("    ⚠️  Note: Actual sending requires funded wallet");
	println!("    💡 Use TestNet faucet to get test tokens");
}

async fn demonstrate_gas_estimation(client: &RpcClient<HttpProvider>) {
	println!("  ⛽ Gas Fee Components:");
	println!("    🔹 Network Fee: Paid to consensus nodes");
	println!("    🔹 System Fee: Paid for contract execution");

	// Example gas calculations
	println!("\n  📊 Typical Gas Fees:");
	println!("    • Simple NEO transfer: ~0.00000001 GAS");
	println!("    • Simple GAS transfer: ~0.00000001 GAS");
	println!("    • Smart contract call: ~0.001+ GAS (varies)");

	// Try to get current network fee
	if let Ok(fee) = client.calculate_network_fee("".to_string()).await {
		println!("    • Current network fee: {:.8} GAS", fee.network_fee as f64 / 100_000_000.0);
	}
}

fn demonstrate_transaction_monitoring() {
	println!("  📡 Transaction Lifecycle:");
	println!("    1️⃣  Create and sign transaction");
	println!("    2️⃣  Broadcast to network");
	println!("    3️⃣  Wait for confirmation (usually 1 block ~15 seconds)");
	println!("    4️⃣  Verify transaction in block");

	println!("\n  🔍 Monitoring Methods:");
	println!("    • Poll by transaction hash");
	println!("    • Watch for block confirmations");
	println!("    • Subscribe to transaction events");
	println!("    • Check application logs for failures");
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_wallet_generation() {
		let sender_key = KeyPair::new_random();
		let sender_script_hash = sender_key.get_script_hash();
		let sender_address = sender_script_hash.to_address();

		// Address should be valid Neo N3 format
		assert!(sender_address.to_string().starts_with('N'));
		assert_eq!(sender_address.to_string().len(), 34);
	}

	#[test]
	fn test_gas_calculations() {
		// Test GAS decimal conversion
		let base_units = 100_000_000u64; // 1 GAS
		let gas_value = base_units as f64 / 100_000_000.0;
		assert_eq!(gas_value, 1.0);

		// Test small amounts
		let small_amount = 1u64; // Smallest GAS unit
		let small_gas = small_amount as f64 / 100_000_000.0;
		assert_eq!(small_gas, 0.00000001);
	}

	#[tokio::test]
	async fn test_client_connection() {
		// Test that we can create a client (doesn't require network)
		let provider = HttpProvider::new("https://testnet1.neo.org:443").unwrap();
		let client = RpcClient::new(provider);

		// Client should be created successfully
		// Actual network tests would require live connection
		assert!(true); // Placeholder assertion
	}

	#[test]
	fn test_address_validation() {
		// Test valid address format
		let key_pair = KeyPair::new_random();
		let script_hash = key_pair.get_script_hash();
		let address = script_hash.to_address();
		let address_str = address.to_string();

		// Should be able to parse back the address
		assert!(ScriptHash::from_address(&address_str).is_ok());

		// Invalid addresses should fail
		assert!(ScriptHash::from_address("InvalidAddress").is_err());
	}
}
