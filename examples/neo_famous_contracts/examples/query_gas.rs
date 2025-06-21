use neo3::{
	neo_clients::{APITrait, HttpProvider, RpcClient},
	neo_crypto::KeyPair,
	neo_protocol::{Account, AccountTrait},
	neo_types::ScriptHash,
	prelude::*,
};
use std::str::FromStr;

/// This example demonstrates how to query GAS token information and balances on the Neo N3 blockchain.
/// GAS is the utility token of Neo used for transaction fees and smart contract execution.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("⛽ Neo N3 GAS Token Query Example");
	println!("================================");

	// Connect to Neo N3 TestNet
	println!("\n📡 Connecting to Neo N3 TestNet...");
	let provider = HttpProvider::new("https://testnet1.neo.org:443/")
		.map_err(|e| format!("Failed to create provider: {}", e))?;
	let client = RpcClient::new(provider);
	println!("   ✅ Connected successfully");

	// GAS token contract hash on Neo N3
	let gas_hash = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
	println!("\n📝 GAS Contract Hash: 0x{}", hex::encode(&gas_hash.0));

	// 1. Query GAS token information
	println!("\n1️⃣ Querying GAS Token Information...");
	query_gas_info(&client, &gas_hash).await?;

	// 2. Create a test account and check its balance
	println!("\n2️⃣ Creating test account...");
	let key_pair = KeyPair::new_random();
	let account = Account::from_key_pair(key_pair, None, None)?;
	println!("   🆔 Account Address: {}", account.get_address());
	println!("   #️⃣  Script Hash: 0x{}", hex::encode(account.get_script_hash().0));

	// 3. Check GAS balance
	println!("\n3️⃣ Checking GAS Balance...");
	check_gas_balance(&client, &gas_hash, &account).await?;

	// 4. Query some well-known addresses (optional)
	println!("\n4️⃣ Checking Well-Known Addresses...");
	let known_addresses = vec![
		("Neo Foundation", "NUuJw4C4XJFzxAvSZnFTfsNoWZytmQKXQP"),
		("Example Address", "NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc"),
	];

	for (name, address) in known_addresses {
		match ScriptHash::from_address(address) {
			Ok(script_hash) => {
				println!("\n   🏗️ {} ({})", name, address);
				check_balance_by_script_hash(&client, &gas_hash, &script_hash).await?;
			},
			Err(e) => {
				println!("   ⚠️ Failed to parse address {}: {}", address, e);
			},
		}
	}

	// 5. Demonstrate GAS economics
	println!("\n5️⃣ GAS Token Economics:");
	println!("   💰 Total Supply: 52,910,000 GAS (at genesis)");
	println!("   🔄 Generation: 5 GAS per block (initial rate)");
	println!("   📉 Decay: Generation rate decreases over time");
	println!("   🎯 Target: ~200 million GAS over ~22 years");
	println!("   ⛽ Usage: Transaction fees, smart contract deployment/execution");

	// 6. Best practices
	println!("\n💡 Best Practices:");
	println!("   • Always check GAS balance before transactions");
	println!("   • Monitor network fees during high congestion");
	println!("   • Use efficient contract patterns to minimize GAS usage");
	println!("   • Consider GAS price volatility in dApp economics");

	println!("\n✅ GAS query example completed!");

	Ok(())
}

/// Query comprehensive GAS token information
async fn query_gas_info(
	client: &RpcClient<HttpProvider>,
	gas_hash: &ScriptHash,
) -> Result<(), Box<dyn std::error::Error>> {
	// Get token symbol
	match client.invoke_function(gas_hash, "symbol".to_string(), vec![], None).await {
		Ok(result) =>
			if let Some(stack_item) = result.stack.first() {
				if let Some(symbol) = stack_item.as_string() {
					println!("   🏷️ Symbol: {}", symbol);
				}
			},
		Err(e) => println!("   ⚠️ Failed to get symbol: {}", e),
	}

	// Get token decimals
	match client.invoke_function(gas_hash, "decimals".to_string(), vec![], None).await {
		Ok(result) =>
			if let Some(stack_item) = result.stack.first() {
				if let Some(decimals) = stack_item.as_int() {
					println!("   🔢 Decimals: {}", decimals);
				}
			},
		Err(e) => println!("   ⚠️ Failed to get decimals: {}", e),
	}

	// Get total supply
	match client.invoke_function(gas_hash, "totalSupply".to_string(), vec![], None).await {
		Ok(result) => {
			if let Some(stack_item) = result.stack.first() {
				if let Some(supply) = stack_item.as_int() {
					let gas_decimal = supply as f64 / 100_000_000.0; // GAS has 8 decimals
					println!("   📊 Total Supply: {:.8} GAS", gas_decimal);
					println!("   🔍 Raw Value: {} (in smallest unit)", supply);
				}
			}
		},
		Err(e) => println!("   ⚠️ Failed to get total supply: {}", e),
	}

	Ok(())
}

/// Check GAS balance for an account
async fn check_gas_balance(
	client: &RpcClient<HttpProvider>,
	gas_hash: &ScriptHash,
	account: &Account,
) -> Result<(), Box<dyn std::error::Error>> {
	let script_hash = account.get_script_hash();

	match client
		.invoke_function(
			gas_hash,
			"balanceOf".to_string(),
			vec![neo3::neo_types::ContractParameter::h160(&script_hash)],
			None,
		)
		.await
	{
		Ok(result) =>
			if let Some(balance_item) = result.stack.first() {
				let balance = balance_item.as_int().unwrap_or(0);
				let gas_balance = balance as f64 / 100_000_000.0;
				println!("   💰 GAS Balance: {} GAS", gas_balance);
				println!("   🔍 Raw Balance: {} (in smallest unit)", balance);

				if balance == 0 {
					println!("   💭 This is a new account with no GAS");
					println!("   💡 To get TestNet GAS, visit: https://neowish.ngd.network/");
				}
			},
		Err(e) => {
			println!("   ⚠️ Unable to fetch balance: {}", e);
			println!("   💭 This might be a new account with no transaction history");
		},
	}

	Ok(())
}

/// Check balance by script hash
async fn check_balance_by_script_hash(
	client: &RpcClient<HttpProvider>,
	gas_hash: &ScriptHash,
	script_hash: &ScriptHash,
) -> Result<(), Box<dyn std::error::Error>> {
	match client
		.invoke_function(
			gas_hash,
			"balanceOf".to_string(),
			vec![neo3::neo_types::ContractParameter::h160(script_hash)],
			None,
		)
		.await
	{
		Ok(result) =>
			if let Some(balance_item) = result.stack.first() {
				let balance = balance_item.as_int().unwrap_or(0);
				let gas_balance = balance as f64 / 100_000_000.0;
				println!("      💰 Balance: {} GAS", gas_balance);
			},
		Err(_) => {
			println!("      💰 Balance: 0 GAS (or unable to fetch)");
		},
	}

	Ok(())
}
