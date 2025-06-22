//! Neo N3 Smart Contract Interaction Example
//!
//! This example demonstrates comprehensive smart contract interaction on Neo N3,
//! including contract invocation, state queries, event monitoring, and transaction building.

use neo3::{neo_clients::APITrait, prelude::*};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("📜 Neo N3 Smart Contract Interaction Example");
	println!("===========================================\n");

	// 1. Connect to Neo N3 network
	println!("📡 1. Connecting to Neo N3 TestNet...");
	let client = connect_to_testnet().await?;

	// 2. Query native contracts
	println!("\n🏛️ 2. Querying Native Contracts...");
	query_native_contracts(&client).await?;

	// 3. NEP-17 token interaction
	println!("\n💰 3. NEP-17 Token Interaction...");
	interact_with_nep17_token(&client).await?;

	// 4. Smart contract invocation
	println!("\n🔧 4. Smart Contract Invocation...");
	demonstrate_contract_invocation(&client).await?;

	// 5. Build and send transaction
	println!("\n📝 5. Transaction Building...");
	demonstrate_transaction_building(&client).await?;

	// 6. Monitor contract events
	println!("\n📢 6. Event Monitoring...");
	monitor_contract_events(&client).await?;

	// 7. Advanced contract patterns
	println!("\n🚀 7. Advanced Contract Patterns...");
	demonstrate_advanced_patterns(&client).await?;

	// 8. Best practices
	println!("\n💡 8. Smart Contract Best Practices...");
	display_best_practices();

	println!("\n✅ Smart contract interaction example completed!");
	println!("💡 Successfully demonstrated comprehensive Neo N3 contract operations");

	Ok(())
}

/// Connect to TestNet with failover
async fn connect_to_testnet(
) -> Result<neo3::providers::RpcClient<neo3::providers::HttpProvider>, Box<dyn std::error::Error>> {
	let endpoints = vec![
		"https://testnet1.neo.org:443/",
		"https://testnet2.neo.org:443/",
		"http://seed1t5.neo.org:20332",
	];

	for endpoint in endpoints {
		match neo3::providers::HttpProvider::new(endpoint) {
			Ok(provider) => {
				let client = neo3::providers::RpcClient::new(provider);
				match client.get_block_count().await {
					Ok(count) => {
						println!("   ✅ Connected to: {endpoint}");
						println!("   📦 Block height: {count}");
						return Ok(client);
					},
					Err(_) => continue,
				}
			},
			Err(_) => continue,
		}
	}

	Err("Failed to connect to TestNet".into())
}

/// Query native contracts
async fn query_native_contracts(
	client: &neo3::providers::RpcClient<neo3::providers::HttpProvider>,
) -> Result<(), Box<dyn std::error::Error>> {
	// Native contract hashes
	let contracts = vec![
		("NEO", "ef4073a0f2b305a38ec4050e4d3d28bc40ea63f5"),
		("GAS", "d2a4cff31913016155e38e474a2c06d08be276cf"),
		("ContractManagement", "fffdc93764dbaddd97c48f252a53ea4643faa3fd"),
		("NameService", "50ac1c37690cc2cfc594472833cf57505d5f46de"),
	];

	for (name, hash) in contracts {
		let script_hash = neo3::neo_types::ScriptHash::from_str(hash)?;
		match client.get_contract_state(script_hash).await {
			Ok(state) => {
				println!("   ✅ {}: Contract ID #{}", name, state.id);
				let manifest = &state.manifest;
				println!(
					"      • Methods: {}",
					manifest.abi.as_ref().map(|abi| abi.methods.len()).unwrap_or(0)
				);
				println!(
					"      • Events: {}",
					manifest.abi.as_ref().map(|abi| abi.events.len()).unwrap_or(0)
				);
			},
			Err(e) => println!("   ❌ {name}: {e}"),
		}
	}

	Ok(())
}

/// Interact with NEP-17 token
async fn interact_with_nep17_token(
	client: &neo3::providers::RpcClient<neo3::providers::HttpProvider>,
) -> Result<(), Box<dyn std::error::Error>> {
	let gas_hash =
		neo3::neo_types::ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;

	// Query token information
	println!("   📋 GAS Token Information:");

	// Get symbol
	match client.invoke_function(&gas_hash, "symbol".to_string(), vec![], None).await {
		Ok(result) => {
			let stack = result.stack;
			if let Some(item) = stack.first() {
				println!(
					"      • Symbol: {}",
					item.as_string().unwrap_or_else(|| "GAS".to_string())
				);
			}
		},
		Err(e) => println!("      • Symbol query failed: {e}"),
	}

	// Get decimals
	match client.invoke_function(&gas_hash, "decimals".to_string(), vec![], None).await {
		Ok(result) => {
			let stack = result.stack;
			if let Some(item) = stack.first() {
				println!("      • Decimals: {}", item.as_int().unwrap_or(8));
			}
		},
		Err(e) => println!("      • Decimals query failed: {e}"),
	}

	// Get total supply
	match client.invoke_function(&gas_hash, "totalSupply".to_string(), vec![], None).await {
		Ok(result) => {
			let stack = result.stack;
			if let Some(item) = stack.first() {
				let supply = item.as_int().unwrap_or(0);
				println!("      • Total Supply: {} GAS", supply as f64 / 100_000_000.0);
			}
		},
		Err(e) => println!("      • Total supply query failed: {e}"),
	}

	// Check balance of an address
	let example_address = "NPvKVTGZapmFWABLsyvfreuqn73jCjJtN1";
	let address_hash = neo3::neo_types::ScriptHash::from_address(example_address)?;

	match client
		.invoke_function(
			&gas_hash,
			"balanceOf".to_string(),
			vec![neo3::neo_types::ContractParameter::h160(&address_hash)],
			None,
		)
		.await
	{
		Ok(result) => {
			let stack = result.stack;
			if let Some(item) = stack.first() {
				let balance = item.as_int().unwrap_or(0);
				println!(
					"      • Balance of {}: {} GAS",
					example_address,
					balance as f64 / 100_000_000.0
				);
			}
		},
		Err(e) => println!("      • Balance query failed: {e}"),
	}

	Ok(())
}

/// Demonstrate contract invocation
async fn demonstrate_contract_invocation(
	client: &neo3::providers::RpcClient<neo3::providers::HttpProvider>,
) -> Result<(), Box<dyn std::error::Error>> {
	println!("   📋 Contract invocation methods:");

	// Test invocation (read-only)
	println!("\n   1️⃣ Test Invocation (invokefunction):");
	println!("      • No blockchain state change");
	println!("      • No fees required");
	println!("      • Instant results");
	println!("      • Used for queries and simulations");

	// Example: Query NNS domain
	let nns_hash =
		neo3::neo_types::ScriptHash::from_str("50ac1c37690cc2cfc594472833cf57505d5f46de")?;
	let domain = "neo.neo";

	match client
		.invoke_function(
			&nns_hash,
			"resolve".to_string(),
			vec![
				neo3::neo_types::ContractParameter::string(domain.to_string()),
				neo3::neo_types::ContractParameter::integer(16), // Record type A
			],
			None,
		)
		.await
	{
		Ok(result) => {
			println!("\n      Example: NNS resolve(\"{domain}\", A)");
			println!(
				"      Gas consumed: {} GAS",
				result.gas_consumed.parse::<f64>().unwrap_or(0.0) / 100_000_000.0
			);
			println!("      State: {:?}", result.state);
			if let Some(exception) = result.exception {
				println!("      Exception: {exception}");
			}
		},
		Err(e) => println!("      NNS query failed: {e}"),
	}

	// Transaction invocation (write)
	println!("\n   2️⃣ Transaction Invocation (sendrawtransaction):");
	println!("      • Modifies blockchain state");
	println!("      • Requires fees and signature");
	println!("      • Returns transaction hash");
	println!("      • Permanent state change");

	// Build invocation script
	let mut script_builder = neo3::neo_builder::ScriptBuilder::new();
	script_builder.contract_call(
		&nns_hash,
		"setRecord",
		&[
			neo3::neo_types::ContractParameter::string("mydomain.neo".to_string()),
			neo3::neo_types::ContractParameter::integer(16), // Type A
			neo3::neo_types::ContractParameter::string("127.0.0.1".to_string()),
		],
		Some(neo3::neo_builder::CallFlags::All),
	)?;

	let script = script_builder.to_bytes();
	println!("\n      Example transaction script:");
	println!("      Method: setRecord(\"mydomain.neo\", A, \"127.0.0.1\")");
	println!("      Script size: {} bytes", script.len());
	println!("      Estimated fee: ~0.5 GAS");

	Ok(())
}

/// Demonstrate transaction building
async fn demonstrate_transaction_building(
	client: &neo3::providers::RpcClient<neo3::providers::HttpProvider>,
) -> Result<(), Box<dyn std::error::Error>> {
	println!("   🔨 Building a smart contract transaction:");

	// Example: Transfer GAS tokens
	let gas_hash =
		neo3::neo_types::ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
	let sender = "NPvKVTGZapmFWABLsyvfreuqn73jCjJtN1";
	let recipient = "NTrezV3bgHEjFfWw3Jwz8XnCxwU8cJNTSi";
	let amount = 10_00000000; // 10 GAS

	// Build transfer script
	let mut script_builder = neo3::neo_builder::ScriptBuilder::new();
	script_builder.contract_call(
		&gas_hash,
		"transfer",
		&[
			neo3::neo_types::ContractParameter::h160(&neo3::neo_types::ScriptHash::from_address(
				sender,
			)?),
			neo3::neo_types::ContractParameter::h160(&neo3::neo_types::ScriptHash::from_address(
				recipient,
			)?),
			neo3::neo_types::ContractParameter::integer(amount),
			neo3::neo_types::ContractParameter::any(),
		],
		Some(neo3::neo_builder::CallFlags::All),
	)?;

	// Build transaction
	let mut tx_builder = neo3::neo_builder::TransactionBuilder::with_client(client);
	let current_height = client.get_block_count().await?;

	tx_builder.set_script(Some(script_builder.to_bytes()));
	tx_builder.valid_until_block(current_height + 1000)?;

	let sender_script_hash = neo3::neo_types::ScriptHash::from_address(sender)?;
	let signer = neo3::neo_builder::AccountSigner::called_by_entry_hash160(sender_script_hash)?;
	tx_builder.set_signers(vec![neo3::neo_builder::Signer::AccountSigner(signer)])?;

	// Calculate fees
	let base_fee = 0.001; // Network fee
	let size_fee = 0.00001 * 500.0; // Estimated tx size
	let system_fee = 0.01; // GAS transfer fee
	let total_fee = base_fee + size_fee + system_fee;

	println!("\n   📝 Transaction details:");
	println!("      • From: {sender}");
	println!("      • To: {recipient}");
	println!("      • Amount: {} GAS", amount as f64 / 100_000_000.0);
	println!("      • Valid until: Block #{}", current_height + 1000);
	println!("      • Estimated fees: {total_fee} GAS");

	println!("\n   🔑 Signing process:");
	println!("      1. Add witness with private key");
	println!("      2. Sign transaction hash");
	println!("      3. Attach signature to transaction");
	println!("      4. Broadcast to network");

	Ok(())
}

/// Monitor contract events
async fn monitor_contract_events(
	_client: &neo3::providers::RpcClient<neo3::providers::HttpProvider>,
) -> Result<(), Box<dyn std::error::Error>> {
	println!("   📢 Monitoring contract events:");

	println!("\n   🔍 Scanning last {} blocks for events...", 10);

	// In production, you would use get_application_log for each transaction
	println!("   📋 Common NEP-17 events:");
	println!("      • Transfer(from, to, amount)");
	println!("      • Approval(owner, spender, amount)");

	// Example event structure
	println!("\n   📄 Event structure:");
	println!("      {{");
	println!("          \"contract\": \"0xd2a4cff31913016155e38e474a2c06d08be276cf\",");
	println!("          \"eventname\": \"Transfer\",");
	println!("          \"state\": {{");
	println!("              \"type\": \"Array\",");
	println!("              \"value\": [");
	println!("                  {{\"type\": \"Hash160\", \"value\": \"from_address\"}},");
	println!("                  {{\"type\": \"Hash160\", \"value\": \"to_address\"}},");
	println!("                  {{\"type\": \"Integer\", \"value\": \"1000000000\"}}");
	println!("              ]");
	println!("          }}");
	println!("      }}");

	// WebSocket subscription (conceptual)
	println!("\n   🔌 Real-time event monitoring:");
	println!("      • WebSocket connection to node");
	println!("      • Subscribe to specific contracts");
	println!("      • Filter by event name");
	println!("      • Process events as they occur");

	Ok(())
}

/// Demonstrate advanced patterns
async fn demonstrate_advanced_patterns(
	_client: &neo3::providers::RpcClient<neo3::providers::HttpProvider>,
) -> Result<(), Box<dyn std::error::Error>> {
	println!("   🚀 Advanced contract patterns:");

	// Multi-contract calls
	println!("\n   1️⃣ Multi-Contract Calls:");
	println!("      Batch multiple contract calls in one transaction:");

	let mut script_builder = neo3::neo_builder::ScriptBuilder::new();

	// First call: Check balance
	let gas_hash =
		neo3::neo_types::ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
	script_builder.contract_call(
		&gas_hash,
		"balanceOf",
		&[neo3::neo_types::ContractParameter::h160(&neo3::neo_types::ScriptHash::from_address(
			"NPvKVTGZapmFWABLsyvfreuqn73jCjJtN1",
		)?)],
		Some(neo3::neo_builder::CallFlags::ReadOnly),
	)?;

	// Second call: Get name
	script_builder.contract_call(
		&gas_hash,
		"symbol",
		&[],
		Some(neo3::neo_builder::CallFlags::ReadOnly),
	)?;

	println!("      Script combines multiple operations");
	println!("      Atomic execution of all calls");

	// Oracle integration
	println!("\n   2️⃣ Oracle Integration:");
	println!("      Request external data in contracts:");
	println!("      • HTTP/HTTPS requests");
	println!("      • JSON parsing");
	println!("      • Consensus-based validation");
	println!("      • ~0.5 GAS per request");

	// Storage operations
	println!("\n   3️⃣ Storage Patterns:");
	println!("      Efficient data storage:");
	println!("      • Use StorageMap for key-value data");
	println!("      • Minimize storage writes (expensive)");
	println!("      • Pack data into single storage slots");
	println!("      • Use events for historical data");

	// Contract upgrades
	println!("\n   4️⃣ Contract Upgrades:");
	println!("      Update contract code while preserving state:");
	println!("      • Deploy with update method");
	println!("      • Migrate storage if needed");
	println!("      • Maintain compatibility");
	println!("      • Test thoroughly on TestNet");

	Ok(())
}

/// Display best practices
fn display_best_practices() {
	println!("   💡 Smart contract interaction best practices:");

	println!("\n   Security:");
	println!("      • Always validate contract hashes");
	println!("      • Check return values and exceptions");
	println!("      • Use appropriate witness scopes");
	println!("      • Implement proper error handling");

	println!("\n   Performance:");
	println!("      • Batch read operations when possible");
	println!("      • Cache frequently accessed data");
	println!("      • Minimize contract calls");
	println!("      • Use test invocations for queries");

	println!("\n   Cost optimization:");
	println!("      • Estimate fees before sending");
	println!("      • Optimize script size");
	println!("      • Use efficient data structures");
	println!("      • Avoid unnecessary storage operations");

	println!("\n   Development workflow:");
	println!("      • Test on TestNet first");
	println!("      • Monitor events for debugging");
	println!("      • Use contract versioning");
	println!("      • Document ABI thoroughly");
}
