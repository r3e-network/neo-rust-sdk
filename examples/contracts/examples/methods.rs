//! Neo N3 Contract Methods Example
//!
//! This example demonstrates how to interact with contract methods on Neo N3,
//! including reading contract state, invoking functions, and handling different
//! parameter types and return values.

use neo3::{neo_clients::APITrait, neo_types::ContractParameter, prelude::*};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("🔧 Neo N3 Contract Methods Example");
	println!("==================================\n");

	// Connect to TestNet
	let client = connect_to_testnet().await?;

	// 1. Query contract information
	println!("1️⃣ Contract Information Query...");
	query_contract_info(&client).await?;

	// 2. Read-only method calls
	println!("\n2️⃣ Read-Only Method Calls...");
	test_readonly_methods(&client).await?;

	// 3. Method parameter types
	println!("\n3️⃣ Method Parameter Types...");
	demonstrate_parameter_types(&client).await?;

	// 4. Return value handling
	println!("\n4️⃣ Return Value Handling...");
	handle_return_values(&client).await?;

	// 5. Contract state queries
	println!("\n5️⃣ Contract State Queries...");
	query_contract_state(&client).await?;

	println!("\n✅ Contract methods example completed!");
	Ok(())
}

async fn connect_to_testnet()
-> Result<neo3::neo_clients::RpcClient<neo3::neo_clients::HttpProvider>, Box<dyn std::error::Error>>
{
	let endpoints = vec![
		"https://testnet1.neo.org:443/",
		"https://testnet2.neo.org:443/",
		"http://seed1t5.neo.org:20332",
	];

	for endpoint in endpoints {
		if let Ok(provider) = neo3::neo_clients::HttpProvider::new(endpoint) {
			let client = neo3::neo_clients::RpcClient::new(provider);
			if let Ok(height) = client.get_block_count().await {
				println!("   ✅ Connected to: {endpoint}");
				println!("   📦 Block height: {height}\n");
				return Ok(client);
			}
		}
	}

	Err("Failed to connect to TestNet".into())
}

async fn query_contract_info(
	client: &neo3::neo_clients::RpcClient<neo3::neo_clients::HttpProvider>,
) -> Result<(), Box<dyn std::error::Error>> {
	let gas_hash =
		neo3::neo_types::ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;

	println!("   📋 GAS Contract Information:");

	// Get contract state
	match client.get_contract_state(gas_hash).await {
		Ok(state) => {
			println!("      Contract ID: #{}", state.id);
			println!("      NEF checksum: 0x{:x}", state.nef.checksum);

			if let Some(abi) = &state.manifest.abi {
				println!("      Methods: {}", abi.methods.len());
				println!("      Events: {}", abi.events.len());

				// List some key methods
				println!("      Key methods:");
				for method in &abi.methods {
					if ["symbol", "decimals", "totalSupply", "balanceOf", "transfer"]
						.contains(&method.name.as_str())
					{
						println!("        • {} (offset: {})", method.name, method.offset);
					}
				}
			}
		},
		Err(e) => println!("      ❌ Failed to get contract state: {e}"),
	}

	Ok(())
}

async fn test_readonly_methods(
	client: &neo3::neo_clients::RpcClient<neo3::neo_clients::HttpProvider>,
) -> Result<(), Box<dyn std::error::Error>> {
	let gas_hash =
		neo3::neo_types::ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;

	println!("   📖 Testing read-only methods:");

	// Test symbol() method
	match client.invoke_function(&gas_hash, "symbol".to_string(), vec![], None).await {
		Ok(result) => {
			println!("      symbol() → State: {:?}", result.state);
			if let Some(stack_item) = result.stack.first()
				&& let Some(symbol) = stack_item.as_string()
			{
				println!("      symbol() → Value: \"{symbol}\"");
			}
			println!(
				"      symbol() → Gas: {} GAS",
				result.gas_consumed.parse::<f64>().unwrap_or(0.0) / 100_000_000.0
			);
		},
		Err(e) => println!("      ❌ symbol() failed: {e}"),
	}

	// Test decimals() method
	match client.invoke_function(&gas_hash, "decimals".to_string(), vec![], None).await {
		Ok(result) => {
			if let Some(stack_item) = result.stack.first()
				&& let Some(decimals) = stack_item.as_int()
			{
				println!("      decimals() → Value: {decimals}");
			}
		},
		Err(e) => println!("      ❌ decimals() failed: {e}"),
	}

	// Test totalSupply() method
	match client.invoke_function(&gas_hash, "totalSupply".to_string(), vec![], None).await {
		Ok(result) => {
			if let Some(stack_item) = result.stack.first()
				&& let Some(supply) = stack_item.as_int()
			{
				println!("      totalSupply() → Value: {} GAS", supply as f64 / 100_000_000.0);
			}
		},
		Err(e) => println!("      ❌ totalSupply() failed: {e}"),
	}

	Ok(())
}

async fn demonstrate_parameter_types(
	client: &neo3::neo_clients::RpcClient<neo3::neo_clients::HttpProvider>,
) -> Result<(), Box<dyn std::error::Error>> {
	let gas_hash =
		neo3::neo_types::ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
	let example_address = "NPvKVTGZapmFWABLsyvfreuqn73jCjJtN1";

	println!("   🔢 Parameter type demonstrations:");

	// Hash160 parameter (address)
	let address_hash = neo3::neo_types::ScriptHash::from_address(example_address)?;
	let hash160_param = ContractParameter::h160(&address_hash);
	println!("      Hash160: {} → 0x{}", example_address, hex::encode(address_hash.0));

	// Test balanceOf with Hash160 parameter
	match client
		.invoke_function(&gas_hash, "balanceOf".to_string(), vec![hash160_param], None)
		.await
	{
		Ok(result) => {
			if let Some(stack_item) = result.stack.first()
				&& let Some(balance) = stack_item.as_int()
			{
				println!(
					"      balanceOf({example_address}) → {} GAS",
					balance as f64 / 100_000_000.0
				);
			}
		},
		Err(e) => println!("      ❌ balanceOf failed: {e}"),
	}

	// Integer parameters
	println!("\n   🔢 Other parameter types:");
	println!("      • Integer: ContractParameter::integer(42)");
	println!("      • String: ContractParameter::string(\"hello\")");
	println!("      • Boolean: ContractParameter::boolean(true)");
	println!("      • ByteArray: ContractParameter::byte_array(vec![1,2,3])");
	println!("      • Array: ContractParameter::array(vec![...])");
	println!("      • Map: ContractParameter::map(HashMap::new())");

	Ok(())
}

async fn handle_return_values(
	client: &neo3::neo_clients::RpcClient<neo3::neo_clients::HttpProvider>,
) -> Result<(), Box<dyn std::error::Error>> {
	let gas_hash =
		neo3::neo_types::ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;

	println!("   📤 Return value handling:");

	match client.invoke_function(&gas_hash, "symbol".to_string(), vec![], None).await {
		Ok(result) => {
			println!("      Raw result: {:?}", result.state);

			if let Some(stack_item) = result.stack.first() {
				println!("      Stack item type: {stack_item:?}");

				// Try different type conversions
				if let Some(string_val) = stack_item.as_string() {
					println!("      As string: \"{string_val}\"");
				}
				if let Some(bytes_val) = stack_item.as_bytes() {
					println!("      As bytes: {} bytes", bytes_val.len());
				}
				if let Some(int_val) = stack_item.as_int() {
					println!("      As integer: {int_val}");
				}
			}
		},
		Err(e) => println!("      ❌ Failed: {e}"),
	}

	println!("\n   📋 Common return types:");
	println!("      • Void: Method executed successfully, no return value");
	println!("      • Integer: Numeric values, balances, counts");
	println!("      • String: Text data, symbols, names");
	println!("      • ByteArray: Raw binary data, hashes");
	println!("      • Array: Multiple values, lists");
	println!("      • Boolean: True/false values");

	Ok(())
}

async fn query_contract_state(
	_client: &neo3::neo_clients::RpcClient<neo3::neo_clients::HttpProvider>,
) -> Result<(), Box<dyn std::error::Error>> {
	let _gas_hash =
		neo3::neo_types::ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;

	println!("   🗂️ Contract storage queries:");

	// Find storage items (if accessible)
	// Note: find_states requires a root hash - we'll demonstrate the API structure
	println!(
		"      💡 Storage query would use find_states(root_hash, contract_hash, key_prefix, start_key, count)"
	);
	println!("      • root_hash: Block state root hash");
	println!("      • contract_hash: Target contract script hash");
	println!("      • key_prefix: Storage key prefix to filter");
	println!("      • start_key: Starting key for pagination");
	println!("      • count: Maximum items to return");
	println!("      ⚠️ This requires a specific block's state root hash");

	println!("\n   💡 Storage query patterns:");
	println!("      • find_states(): Get all storage items");
	println!("      • get_state(): Get specific storage value");
	println!("      • Use prefixes to filter storage keys");
	println!("      • Storage reads are free (test invocations)");

	Ok(())
}
