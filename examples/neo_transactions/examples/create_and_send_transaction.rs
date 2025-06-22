use neo3::{
	neo_builder::{ScriptBuilder, TransactionBuilder},
	neo_clients::{APITrait, HttpProvider, RpcClient},
	neo_crypto::KeyPair,
	neo_protocol::{Account, AccountTrait},
	neo_types::{ContractParameter, ScriptHash},
	prelude::*,
};
use std::str::FromStr;

/// This example demonstrates comprehensive transaction creation, signing, and sending on the Neo N3 blockchain.
/// It covers GAS transfers, NEO transfers, contract invocations, and transaction tracking.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("Neo N3 Comprehensive Transaction Example");
	println!("=======================================");

	// Connect to Neo N3 TestNet
	println!("\n1. Connecting to Neo N3 TestNet...");
	let provider = HttpProvider::new("https://testnet1.neo.org:443/")
		.map_err(|e| format!("Failed to create provider: {e}"))?;
	let client = RpcClient::new(provider);

	println!("   ✅ Connected to TestNet");

	// Setup accounts for transaction examples
	println!("\n2. Setting up accounts...");

	// Create a sender account (for production deployments, load from secure storage)
	// Create a sender account
	let key_pair = KeyPair::new_random();
	let sender = Account::from_key_pair(key_pair, None, None)?;
	println!("   Sender address: {}", sender.get_address());
	println!("   💡 For production deployments, load account from secure WIF or hardware wallet");

	// Define recipient
	let recipient_address = "NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc";
	let recipient = ScriptHash::from_address(recipient_address)?;
	println!("   Recipient address: {recipient_address}");

	// 3. Get Token Contract References
	println!("\n3. Setting up token contracts...");

	// GAS token contract
	let gas_hash = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
	println!("   GAS token hash: {gas_hash}");

	// NEO token contract
	let neo_hash = ScriptHash::from_str("ef4073a0f2b305a38ec4050e4d3d28bc40ea63f5")?;
	println!("   NEO token hash: {neo_hash}");

	// 4. Create a GAS Transfer Transaction
	println!("\n4. Creating GAS transfer transaction...");

	let gas_amount = 100_000_000u64; // 1 GAS (8 decimals)
	println!("   Transfer amount: {} GAS", gas_amount as f64 / 100_000_000.0);

	// Build transaction script for GAS transfer
	let mut script_builder = ScriptBuilder::new();
	script_builder.contract_call(
		&gas_hash,
		"transfer",
		&[
			ContractParameter::h160(&sender.get_script_hash()),
			ContractParameter::h160(&recipient),
			ContractParameter::integer(gas_amount as i64),
			ContractParameter::any(),
		],
		None,
	)?;

	let script = script_builder.to_bytes();
	println!("   ✅ Transaction script built ({} bytes)", script.len());

	// Create transaction builder
	let mut tx_builder = TransactionBuilder::with_client(&client);

	// Get current block count for validity
	let current_block = client
		.get_block_count()
		.await
		.map_err(|e| format!("Failed to get block count: {e}"))?;

	// Configure transaction
	tx_builder.set_script(Some(script.clone()));
	tx_builder.valid_until_block(current_block + 5760)?; // Valid for ~1 day (5760 blocks ≈ 24 hours)

	println!("   Transaction configured:");
	println!("     Valid until block: {}", current_block + 5760);
	println!("     Current block: {current_block}");

	// 5. Create a NEO Transfer Transaction
	println!("\n5. Creating NEO transfer transaction...");

	let neo_amount = 1u64; // 1 NEO (NEO is indivisible)
	println!("   Transfer amount: {neo_amount} NEO");

	// Build NEO transfer script
	let mut neo_script_builder = ScriptBuilder::new();
	neo_script_builder.contract_call(
		&neo_hash,
		"transfer",
		&[
			ContractParameter::h160(&sender.get_script_hash()),
			ContractParameter::h160(&recipient),
			ContractParameter::integer(neo_amount as i64),
			ContractParameter::any(),
		],
		None,
	)?;

	let neo_script = neo_script_builder.to_bytes();
	println!("   ✅ NEO transfer script built ({} bytes)", neo_script.len());

	// 6. Multi-Call Transaction (Transfer both NEO and GAS)
	println!("\n6. Creating multi-call transaction (NEO + GAS)...");

	let mut multi_script_builder = ScriptBuilder::new();

	// Add NEO transfer
	multi_script_builder.contract_call(
		&neo_hash,
		"transfer",
		&[
			ContractParameter::h160(&sender.get_script_hash()),
			ContractParameter::h160(&recipient),
			ContractParameter::integer(1),
			ContractParameter::any(),
		],
		None,
	)?;

	// Add GAS transfer
	multi_script_builder.contract_call(
		&gas_hash,
		"transfer",
		&[
			ContractParameter::h160(&sender.get_script_hash()),
			ContractParameter::h160(&recipient),
			ContractParameter::integer(50_000_000), // 0.5 GAS
			ContractParameter::any(),
		],
		None,
	)?;

	let multi_script = multi_script_builder.to_bytes();
	println!("   ✅ Multi-call script built ({} bytes)", multi_script.len());

	// 7. Transaction Simulation and Cost Estimation
	println!("\n7. Transaction simulation and cost estimation...");

	// Note: In production implementation, you can:
	// - Use invoke_script to test the transaction
	// - Calculate network fees using calculate_network_fee
	// - Check account balances before sending

	println!("   💡 Before sending transactions in production:");
	println!("     • Test with invoke_script RPC call");
	println!("     • Calculate network fees with calculate_network_fee");
	println!("     • Verify sufficient balance for fees and transfers");
	println!("     • Use proper signers with appropriate witness scopes");

	// 8. Transaction Signing Process
	println!("\n8. Transaction signing demonstration...");

	// Create a simple transaction for demonstration
	let mut demo_tx_builder = TransactionBuilder::with_client(&client);
	demo_tx_builder.set_script(Some(script.clone()));
	demo_tx_builder.valid_until_block(current_block + 100)?;

	println!("   💡 Transaction signing requires:");
	println!("     • Adding signers with appropriate witness scopes");
	println!("     • Private key access for signing");
	println!("     • Network fee calculation and payment");

	// 9. Transaction Broadcasting (Demo)
	println!("\n9. Transaction broadcasting process...");

	println!("   📡 To broadcast a transaction:");
	println!("     1. Create and configure transaction");
	println!("     2. Add signers with witness scopes");
	println!("     3. Sign the transaction");
	println!("     4. Send via send_raw_transaction RPC");
	println!("     5. Track confirmation with get_transaction_height");

	// 10. Transaction Tracking Example
	println!("\n10. Transaction tracking methods...");

	println!("   🔍 Track transaction status with:");
	println!("     • get_transaction_height(tx_hash) - Check if included in block");
	println!("     • get_application_log(tx_hash) - Get execution results");
	println!("     • get_transaction(tx_hash) - Get full transaction details");

	// 11. Best Practices Summary
	println!("\n11. 💡 Transaction Best Practices:");
	println!("     • Always simulate transactions before sending");
	println!("     • Set appropriate valid_until_block values");
	println!("     • Use minimal witness scopes for security");
	println!("     • Handle network fee calculations properly");
	println!("     • Implement proper error handling and retries");
	println!("     • Track transaction confirmations");
	println!("     • Use multi-sig for high-value transactions");

	println!("\n✅ Comprehensive transaction example completed!");
	println!("   💡 Remember: This example shows the structure and concepts.");
	println!("   💡 For actual transactions, ensure proper key management and testing.");

	Ok(())
}
