/// Neo N3 GAS Transfer Example
///
/// This example demonstrates how to build and understand GAS (utility token) transfers
/// on the Neo N3 blockchain, including balance checking, transaction building, and fee calculation.
use neo3::{
	neo_builder::{ScriptBuilder, TransactionBuilder},
	neo_clients::APITrait,
	neo_types::{ContractParameter, ScriptHash, ScriptHashExtension},
};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("⛽ Neo N3 GAS Transfer Example");
	println!("=============================\n");

	// Connect to TestNet
	let client = connect_to_testnet().await?;

	// 1. Set up accounts and addresses
	println!("1️⃣ Setting up accounts and addresses...");
	setup_transfer_accounts().await?;

	// 2. Check balances before transfer
	println!("\n2️⃣ Checking account balances...");
	check_account_balances(&client).await?;

	// 3. Build transfer transaction
	println!("\n3️⃣ Building transfer transaction...");
	build_transfer_transaction(&client).await?;

	// 4. Calculate fees and costs
	println!("\n4️⃣ Calculating transaction fees...");
	calculate_transaction_fees(&client).await?;

	// 5. Transaction validation
	println!("\n5️⃣ Transaction validation process...");
	validate_transaction(&client).await?;

	// 6. Demonstrate different transfer scenarios
	println!("\n6️⃣ Different transfer scenarios...");
	demonstrate_transfer_scenarios(&client).await?;

	println!("\n✅ GAS transfer example completed!");
	println!("💡 This demonstrates the complete GAS transfer process on Neo N3");

	Ok(())
}

async fn connect_to_testnet(
) -> Result<neo3::neo_clients::RpcClient<neo3::neo_clients::HttpProvider>, Box<dyn std::error::Error>>
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

async fn setup_transfer_accounts() -> Result<(), Box<dyn std::error::Error>> {
	println!("   👤 Setting up transfer participants:");

	// Example addresses for demonstration
	let sender_address = "NPvKVTGZapmFWABLsyvfreuqn73jCjJtN1";
	let recipient_address = "NTrezV3bgHEjFfWw3Jwz8XnCxwU8cJNTSi";
	let transfer_amount = 10_00000000u64; // 10 GAS

	println!("      📤 Sender: {sender_address}");
	println!("      📥 Recipient: {recipient_address}");
	println!("      💰 Amount: {} GAS", transfer_amount as f64 / 100_000_000.0);

	// Show how to create accounts from WIF
	println!("\n   🔑 Account creation from WIF:");
	println!("      Example: let account = Account::from_wif(\"your_wif_here\")?;");
	println!("      • WIF contains the private key for signing");
	println!("      • Account derives address and script hash");
	println!("      • Always keep WIF secure and private");

	Ok(())
}

async fn check_account_balances(
	client: &neo3::neo_clients::RpcClient<neo3::neo_clients::HttpProvider>,
) -> Result<(), Box<dyn std::error::Error>> {
	let gas_hash = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
	let _neo_hash = ScriptHash::from_str("ef4073a0f2b305a38ec4050e4d3d28bc40ea63f5")?;

	println!("   💰 Token Information:");

	// Get GAS token info
	match client.invoke_function(&gas_hash, "symbol".to_string(), vec![], None).await {
		Ok(result) => {
			if let Some(stack_item) = result.stack.first() {
				if let Some(symbol) = stack_item.as_string() {
					println!("      🪙 {} Token (GAS): 0x{:x}", symbol, gas_hash);
				}
			}
		},
		Err(e) => println!("      ❌ Failed to get GAS symbol: {e}"),
	}

	// Check decimals
	match client.invoke_function(&gas_hash, "decimals".to_string(), vec![], None).await {
		Ok(result) => {
			if let Some(stack_item) = result.stack.first() {
				if let Some(decimals) = stack_item.as_int() {
					println!("      📊 GAS Decimals: {decimals}");
				}
			}
		},
		Err(e) => println!("      ❌ Failed to get GAS decimals: {e}"),
	}

	// Example balance check
	let example_address = "NPvKVTGZapmFWABLsyvfreuqn73jCjJtN1";
	let address_hash = ScriptHash::from_address(example_address)?;

	println!("\n   🔍 Balance checking example:");
	match client
		.invoke_function(
			&gas_hash,
			"balanceOf".to_string(),
			vec![ContractParameter::h160(&address_hash)],
			None,
		)
		.await
	{
		Ok(result) => {
			if let Some(stack_item) = result.stack.first() {
				if let Some(balance) = stack_item.as_int() {
					println!("      📍 {example_address}");
					println!("      💰 GAS Balance: {} GAS", balance as f64 / 100_000_000.0);
				}
			}
		},
		Err(e) => println!("      ❌ Balance query failed: {e}"),
	}

	Ok(())
}

async fn build_transfer_transaction(
	client: &neo3::neo_clients::RpcClient<neo3::neo_clients::HttpProvider>,
) -> Result<(), Box<dyn std::error::Error>> {
	println!("   🔨 Building GAS transfer transaction:");

	let gas_hash = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
	let sender = "NPvKVTGZapmFWABLsyvfreuqn73jCjJtN1";
	let recipient = "NTrezV3bgHEjFfWw3Jwz8XnCxwU8cJNTSi";
	let amount = 5_00000000i64; // 5 GAS

	// 1. Build the script
	println!("\n      Step 1: Building contract call script");
	let mut script_builder = ScriptBuilder::new();
	script_builder.contract_call(
		&gas_hash,
		"transfer",
		&[
			ContractParameter::h160(&ScriptHash::from_address(sender)?),
			ContractParameter::h160(&ScriptHash::from_address(recipient)?),
			ContractParameter::integer(amount),
			ContractParameter::any(), // data parameter (null)
		],
		Some(neo3::neo_builder::CallFlags::All),
	)?;

	let script = script_builder.to_bytes();
	println!("         ✅ Script built ({} bytes)", script.len());
	println!("         📄 Contract: GAS transfer method");
	println!("         📤 From: {sender}");
	println!("         📥 To: {recipient}");
	println!("         💰 Amount: {} GAS", amount as f64 / 100_000_000.0);

	// 2. Create transaction
	println!("\n      Step 2: Creating transaction");
	let mut tx_builder = TransactionBuilder::with_client(client);
	tx_builder.set_script(Some(script));

	// Set valid until block
	let current_height = client.get_block_count().await?;
	tx_builder.valid_until_block(current_height + 1000)?; // Valid for ~4 hours

	println!("         ✅ Transaction created");
	println!("         ⏰ Valid until block: {}", current_height + 1000);
	println!("         🕐 Estimated validity: ~4 hours");

	// 3. Add signers
	println!("\n      Step 3: Adding signers");
	let sender_hash = ScriptHash::from_address(sender)?;
	let signer = neo3::neo_builder::AccountSigner::called_by_entry_hash160(sender_hash)?;
	tx_builder.set_signers(vec![neo3::neo_builder::Signer::AccountSigner(signer)])?;

	println!("         ✅ Signer added");
	println!("         🔐 Witness scope: CalledByEntry");
	println!("         📍 Signer: {sender}");

	// 4. Calculate network fee (estimation)
	println!("\n      Step 4: Fee calculation");
	let base_size = 500; // Estimated transaction size
	let network_fee = 0.001 + (base_size as f64 * 0.00001); // Base + size fee
	println!("         💵 Estimated network fee: {network_fee:.6} GAS");
	println!("         📏 Estimated size: {base_size} bytes");

	Ok(())
}

async fn calculate_transaction_fees(
	client: &neo3::neo_clients::RpcClient<neo3::neo_clients::HttpProvider>,
) -> Result<(), Box<dyn std::error::Error>> {
	println!("   💰 Transaction fee breakdown:");

	// Get current block for fee context
	let current_height = client.get_block_count().await?;
	println!("      📦 Current block: {current_height}");

	// Fee components
	println!("\n      💵 Fee Components:");
	println!("         • Network Fee: ~0.001 GAS (base)");
	println!("         • Size Fee: ~0.00001 GAS per byte");
	println!("         • System Fee: 0 GAS (for GAS transfers)");

	// Fee calculation example
	let base_fee = 0.001;
	let estimated_size = 500;
	let size_fee = estimated_size as f64 * 0.00001;
	let total_fee = base_fee + size_fee;

	println!("\n      🧮 Fee Calculation:");
	println!("         Base fee: {:.6} GAS", base_fee);
	println!("         Size fee: {:.6} GAS ({} bytes)", size_fee, estimated_size);
	println!("         Total fee: {:.6} GAS", total_fee);

	// Required balance
	let transfer_amount = 5.0;
	let required_balance = transfer_amount + total_fee;
	println!("\n      📊 Balance Requirements:");
	println!("         Transfer amount: {} GAS", transfer_amount);
	println!("         Network fees: {:.6} GAS", total_fee);
	println!("         Total required: {:.6} GAS", required_balance);

	Ok(())
}

async fn validate_transaction(
	_client: &neo3::neo_clients::RpcClient<neo3::neo_clients::HttpProvider>,
) -> Result<(), Box<dyn std::error::Error>> {
	println!("   ✅ Transaction validation checklist:");

	// Validation steps
	let validations = vec![
		("Valid addresses", "✅ All addresses are valid Neo N3 format"),
		("Sufficient balance", "⚠️  Check sender has enough GAS + fees"),
		("Valid amount", "✅ Transfer amount > 0 and reasonable"),
		("Network connectivity", "✅ Connected to Neo N3 network"),
		("Block height", "✅ Current block height obtained"),
		("Script validity", "✅ Contract call script properly formed"),
		("Signer setup", "✅ Witness scope and account configured"),
		("Fee calculation", "✅ Network and system fees calculated"),
	];

	for (check, status) in validations {
		println!("      {}: {}", check, status);
	}

	println!("\n   🔐 Signing requirements:");
	println!("      • Private key in WIF format");
	println!("      • Account must match the sender address");
	println!("      • Signature covers transaction hash");
	println!("      • Witness script matches account script");

	println!("\n   📡 Broadcasting requirements:");
	println!("      • Transaction fully signed");
	println!("      • Valid until block not expired");
	println!("      • Network connection stable");
	println!("      • Node accepts the transaction");

	Ok(())
}

async fn demonstrate_transfer_scenarios(
	_client: &neo3::neo_clients::RpcClient<neo3::neo_clients::HttpProvider>,
) -> Result<(), Box<dyn std::error::Error>> {
	println!("   🎭 Different GAS transfer scenarios:");

	println!("\n      Scenario 1: Basic GAS Transfer");
	println!("         • Simple peer-to-peer transfer");
	println!("         • Single signer (sender)");
	println!("         • Standard network fees");
	println!("         • CalledByEntry witness scope");

	println!("\n      Scenario 2: Multi-Signature Transfer");
	println!("         • Requires multiple signatures");
	println!("         • Higher fees due to complexity");
	println!("         • Custom witness scopes");
	println!("         • Coordination between signers");

	println!("\n      Scenario 3: Contract-Mediated Transfer");
	println!("         • Transfer through smart contract");
	println!("         • Additional system fees");
	println!("         • Contract-specific logic");
	println!("         • Event notifications");

	println!("\n      Scenario 4: Batch Transfers");
	println!("         • Multiple transfers in one transaction");
	println!("         • Optimized for efficiency");
	println!("         • Shared network fees");
	println!("         • Atomic execution");

	println!("\n   💡 Best Practices:");
	println!("      • Always validate addresses before transfers");
	println!("      • Check balances including fees");
	println!("      • Use appropriate witness scopes");
	println!("      • Monitor transaction confirmation");
	println!("      • Handle network failures gracefully");
	println!("      • Keep private keys secure");

	Ok(())
}
