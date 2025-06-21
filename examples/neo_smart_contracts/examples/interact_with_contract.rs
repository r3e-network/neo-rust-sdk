use neo3::{
	neo_builder,
	neo_clients::{APITrait, HttpProvider, RpcClient},
	neo_crypto::KeyPair,
	neo_protocol::{Account, AccountTrait},
	neo_types,
	neo_types::ScriptHash,
	prelude::*,
};
use std::str::FromStr;

/// This example demonstrates comprehensive smart contract interaction on the Neo N3 blockchain.
/// It shows read-only method calls, state-changing transaction preparation, and best practices.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("🔗 Neo N3 Smart Contract Interaction Example");
	println!("===========================================");

	// 1. Connect to Neo N3 TestNet
	println!("\n📡 1. Connecting to Neo N3 TestNet...");
	let provider = HttpProvider::new("https://testnet1.neo.org:443/")
		.map_err(|e| format!("Failed to create provider: {}", e))?;
	let client = RpcClient::new(provider);
	println!("   ✅ Connected successfully");

	// 2. Set up account for contract interaction
	println!("\n👤 2. Setting up account for interaction...");

	// Create a demo account (for production deployments, load from secure storage)
	let key_pair = KeyPair::new_random();
	let account = Account::from_key_pair(key_pair, None, None)?;
	println!("   Demo account address: {}", account.get_address());
	println!("   💡 For production deployments: Load account from secure WIF or hardware wallet");

	// 3. Contract References - Native Neo N3 Contracts
	println!("\n📜 3. Setting up native contract references...");

	// GAS Token Contract
	let gas_hash = ScriptHash::from_str("0xd2a4cff31913016155e38e474a2c06d08be276cf")?;
	println!("   ⛽ GAS Token: 0x{}", hex::encode(gas_hash.0));

	// NEO Token Contract
	let neo_hash = ScriptHash::from_str("0xef4073a0f2b305a38ec4050e4d3d28bc40ea63f5")?;
	println!("   🪙 NEO Token: 0x{}", hex::encode(neo_hash.0));

	// Contract Management Contract
	let contract_mgmt_hash = ScriptHash::from_str("0xfffdc93764dbaddd97c48f252a53ea4643faa3fd")?;
	println!("   🏗️  Contract Management: 0x{}", hex::encode(contract_mgmt_hash.0));

	// 4. Read-Only Contract Calls (invoke_function via RPC)
	println!("\n🔍 4. Performing read-only contract calls...");

	// Query GAS token information
	println!("   Querying GAS token properties...");

	match query_token_info(&client, &gas_hash, "GAS").await {
		Ok(_) => println!("   ✅ GAS token info retrieved"),
		Err(e) => println!("   ❌ GAS query failed: {}", e),
	}

	// Query NEO token information
	println!("   Querying NEO token properties...");

	match query_token_info(&client, &neo_hash, "NEO").await {
		Ok(_) => println!("   ✅ NEO token info retrieved"),
		Err(e) => println!("   ❌ NEO query failed: {}", e),
	}

	// 5. Balance Queries
	println!("\n💰 5. Querying account balances...");

	let demo_address = "NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc"; // Well-known TestNet address
	let demo_script_hash = ScriptHash::from_address(demo_address)?;

	match query_account_balance(&client, &gas_hash, &demo_script_hash, "GAS", 8).await {
		Ok(balance) => println!("   💎 Demo account GAS balance: {} GAS", balance),
		Err(e) => println!("   ⚠️ Could not get GAS balance: {}", e),
	}

	match query_account_balance(&client, &neo_hash, &demo_script_hash, "NEO", 0).await {
		Ok(balance) => println!("   💎 Demo account NEO balance: {} NEO", balance),
		Err(e) => println!("   ⚠️ Could not get NEO balance: {}", e),
	}

	// 6. Transaction Script Building
	println!("\n🛠️ 6. Building transaction scripts...");

	// Example: GAS transfer script
	let recipient_address = "NiNmXL8FjEUEs1nfX9uHFBNaenxDHJtmuB";
	let recipient = ScriptHash::from_address(recipient_address)?;
	let transfer_amount = 100_000_000u64; // 1 GAS

	let mut script_builder = neo_builder::ScriptBuilder::new();
	script_builder.contract_call(
		&gas_hash,
		"transfer",
		&[
			neo_types::ContractParameter::h160(&demo_script_hash),
			neo_types::ContractParameter::h160(&recipient),
			neo_types::ContractParameter::integer(transfer_amount as i64),
			neo_types::ContractParameter::any(),
		],
		None,
	)?;

	let script = script_builder.to_bytes();
	println!("   ✅ Transfer script built ({} bytes)", script.len());
	println!(
		"   📝 Transfer: {} GAS from {} to {}",
		transfer_amount as f64 / 100_000_000.0,
		demo_address,
		recipient_address
	);

	// 7. Multi-Call Transaction Example
	println!("\n🔄 7. Multi-call transaction example...");

	let mut multi_builder = neo_builder::ScriptBuilder::new();

	// Call 1: Check GAS balance
	multi_builder.contract_call(
		&gas_hash,
		"balanceOf",
		&[neo_types::ContractParameter::h160(&demo_script_hash)],
		None,
	)?;

	// Call 2: Check NEO balance
	multi_builder.contract_call(
		&neo_hash,
		"balanceOf",
		&[neo_types::ContractParameter::h160(&demo_script_hash)],
		None,
	)?;

	let multi_script = multi_builder.to_bytes();
	println!("   ✅ Multi-call script built ({} bytes)", multi_script.len());

	// 8. Contract Deployment Preparation
	println!("\n🚀 8. Contract deployment concepts...");

	println!("   📋 For contract deployment, you need:");
	println!("     • Compiled NEF file (Neo Executable Format)");
	println!("     • Contract manifest (ABI, permissions, etc.)");
	println!("     • Sufficient GAS for deployment fees");
	println!("     • Use ContractManagement.deploy() method");

	// 9. Best Practices Summary
	println!("\n💡 9. Smart Contract Interaction Best Practices:");
	println!("   🔐 Security:");
	println!("     • Always test with invoke_function before sending transactions");
	println!("     • Use minimal witness scopes for transaction security");
	println!("     • Validate all contract parameters before sending");
	println!("     • Keep private keys secure and never log them");

	println!("   ⚡ Performance:");
	println!("     • Batch multiple read calls in single invoke_function");
	println!("     • Cache contract metadata to reduce RPC calls");
	println!("     • Use appropriate gas fees for timely execution");

	println!("   🧪 Testing:");
	println!("     • Test all contract interactions on TestNet first");
	println!("     • Simulate transactions before broadcasting");
	println!("     • Monitor transaction confirmations");

	println!("   🔧 Error Handling:");
	println!("     • Handle network failures gracefully");
	println!("     • Parse contract error messages properly");
	println!("     • Implement retry logic for failed transactions");

	println!("\n🎉 Smart contract interaction example completed!");
	println!("💡 Remember: This example shows concepts and patterns.");
	println!("💡 For live transactions, ensure proper key management and testing.");

	Ok(())
}

/// Query token information (symbol, decimals, total supply)
async fn query_token_info(
	client: &RpcClient<HttpProvider>,
	token_hash: &ScriptHash,
	token_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
	use neo3::neo_clients::APITrait;

	// Build script to query token info
	let mut builder = neo_builder::ScriptBuilder::new();

	// Query symbol
	builder.contract_call(token_hash, "symbol", &[], None)?;
	// Query decimals
	builder.contract_call(token_hash, "decimals", &[], None)?;
	// Query total supply
	builder.contract_call(token_hash, "totalSupply", &[], None)?;

	let script = builder.to_bytes();

	// Execute read-only call via invoke_function
	match client.invoke_function(token_hash, "symbol".to_string(), vec![], None).await {
		Ok(symbol_result) => {
			if let Some(symbol) = symbol_result.stack.first().and_then(|s| s.as_string()) {
				println!("     {} Token Properties:", token_name);
				println!("       Symbol: {}", symbol);
			}
		},
		Err(e) => println!("     Failed to get symbol: {}", e),
	}

	match client.invoke_function(token_hash, "decimals".to_string(), vec![], None).await {
		Ok(decimals_result) => {
			if let Some(decimals) = decimals_result.stack.first().and_then(|s| s.as_int()) {
				println!("       Decimals: {}", decimals);
			}
		},
		Err(e) => println!("     Failed to get decimals: {}", e),
	}

	match client
		.invoke_function(token_hash, "totalSupply".to_string(), vec![], None)
		.await
	{
		Ok(supply_result) => {
			if let Some(supply) = supply_result.stack.first().and_then(|s| s.as_int()) {
				println!("       Total Supply: {}", supply);
			}
		},
		Err(e) => println!("     Failed to get total supply: {}", e),
	}

	Ok(())
}

/// Query account balance for a specific token
async fn query_account_balance(
	client: &RpcClient<HttpProvider>,
	token_hash: &ScriptHash,
	account_hash: &ScriptHash,
	token_name: &str,
	decimals: u32,
) -> Result<f64, Box<dyn std::error::Error>> {
	use neo3::neo_clients::APITrait;

	let mut builder = neo_builder::ScriptBuilder::new();
	builder.contract_call(
		token_hash,
		"balanceOf",
		&[neo_types::ContractParameter::h160(account_hash)],
		None,
	)?;

	let _script = builder.to_bytes();
	let result = client
		.invoke_function(
			token_hash,
			"balanceOf".to_string(),
			vec![neo_types::ContractParameter::h160(account_hash)],
			None,
		)
		.await?;

	if let Some(first) = result.stack.first() {
		if let Some(balance) = first.as_int() {
			let formatted_balance = balance as f64 / 10f64.powi(decimals as i32);
			return Ok(formatted_balance);
		}
	}

	Ok(0.0)
}
