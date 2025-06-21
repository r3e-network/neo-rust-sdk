use neo3::{
	neo_builder::{AccountSigner, ScriptBuilder, TransactionBuilder},
	neo_clients::{APITrait, HttpProvider, RpcClient},
	neo_protocol::{Account, AccountTrait},
	prelude::*,
};
use std::error::Error;

/// This example demonstrates how to use a local signer to sign messages in Neo N3.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	println!("🔐 Neo3 Local Signer Example");
	println!("============================");

	// 1. Create an account from a WIF (Wallet Import Format)
	println!("\n1. Creating account from WIF:");
	let wif = "L1WMhxazScMhUrdv34JqQb1HFSQmWeN2Kpc1R9JGKwL7CDNP21uR";
	let account = Account::from_wif(wif)?;
	println!("   ✅ Account created successfully");
	println!("   📍 Address: {}", account.get_address());
	println!("   🔑 Script Hash: {:?}", account.get_script_hash());

	// 2. Create a new random account
	println!("\n2. Creating random account:");
	let random_account = Account::create()?;
	println!("   ✅ Random account created");
	println!("   📍 Address: {}", random_account.get_address());
	// Get the WIF from the key pair
	if let Some(key_pair) = random_account.key_pair() {
		println!("   🔐 WIF: {}", key_pair.export_as_wif());
	}

	// 3. Connect to Neo testnet
	println!("\n3. Connecting to Neo testnet:");
	let provider = HttpProvider::new("https://testnet1.neo.coz.io:443")?;
	let client = RpcClient::new(provider);

	// Test connection
	let version = client.get_version().await?;
	println!("   ✅ Connected to Neo node");
	if let Some(protocol) = version.protocol {
		println!("   🌐 Network Magic: {}", protocol.network);
		println!("   ⏱️  Block time: {} ms", protocol.ms_per_block);
	}

	// 4. Check account balance
	println!("\n4. Checking account balance:");
	let neo_token = H160::from_hex("ef4073a0f2b305a38ec4050e4d3d28bc40ea63f5")?;
	let gas_token = H160::from_hex("d2a4cff31913016155e38e474a2c06d08be276cf")?;

	// Get NEO balance
	let neo_balance = client
		.invoke_function(
			&neo_token,
			"balanceOf".to_string(),
			vec![ContractParameter::h160(&account.get_script_hash())],
			None,
		)
		.await?;

	// Get GAS balance
	let gas_balance = client
		.invoke_function(
			&gas_token,
			"balanceOf".to_string(),
			vec![ContractParameter::h160(&account.get_script_hash())],
			None,
		)
		.await?;

	println!("   💰 NEO Balance: {:?}", neo_balance.stack.first());
	println!("   ⛽ GAS Balance: {:?}", gas_balance.stack.first());

	// 5. Create and sign a transaction
	println!("\n5. Creating and signing a transaction:");

	// Create a simple script that calls NEO's symbol method
	let script = ScriptBuilder::new().contract_call(&neo_token, "symbol", &[], None)?.to_bytes();

	// Build transaction
	let mut tx_builder = TransactionBuilder::with_client(&client);
	tx_builder
		.set_script(Some(script))
		.set_signers(vec![AccountSigner::called_by_entry(&account)?.into()])?
		.valid_until_block(client.get_block_count().await? + 100)?;

	// Get unsigned transaction
	let unsigned_tx = tx_builder.get_unsigned_tx().await?;
	println!("   📝 Transaction created");
	println!("   🆔 Nonce: {}", unsigned_tx.nonce());
	println!("   ⏰ Valid until block: {}", unsigned_tx.valid_until_block());
	println!("   💸 System fee: {}", unsigned_tx.sys_fee());
	println!("   🌐 Network fee: {}", unsigned_tx.net_fee());

	// Sign the transaction
	let signed_tx = tx_builder.sign().await?;
	println!("   ✅ Transaction signed successfully");
	println!("   🔏 Witnesses: {}", signed_tx.witnesses().len());

	// 6. Demonstrate message signing
	println!("\n6. Message signing:");
	let message = "Hello, Neo blockchain!";
	let message_bytes = message.as_bytes();

	// Sign the message
	if let Some(key_pair) = account.key_pair() {
		let signature = key_pair.private_key().sign_tx(message_bytes)?;
		println!("   ✅ Message signed");
		println!("   📝 Message: {message}");
		println!("   🔏 Signature length: {} bytes", signature.to_bytes().len());

		// Note: Signature verification would be done by the network when the transaction is submitted
		println!("   ℹ️  Signature verification is performed by the Neo network");
	}

	// 7. Create a multi-signature account
	println!("\n7. Multi-signature account:");
	let account1 = Account::create()?;
	let account2 = Account::create()?;
	let account3 = Account::create()?;

	let mut public_keys = vec![
		account1.get_public_key().unwrap(),
		account2.get_public_key().unwrap(),
		account3.get_public_key().unwrap(),
	];

	let multi_sig_account = Account::multi_sig_from_public_keys(&mut public_keys, 2)?;
	println!("   ✅ Multi-sig account created (2-of-3)");
	println!("   📍 Address: {}", multi_sig_account.get_address());

	// 8. Demonstrate contract interaction
	println!("\n8. Contract interaction example:");

	// Test invoke a contract method
	let invoke_result = client
		.invoke_function(&neo_token, "totalSupply".to_string(), vec![], None)
		.await?;

	println!("   ✅ Contract invoked successfully");
	println!("   📊 Total NEO supply: {:?}", invoke_result.stack.first());
	println!("   ⛽ Gas consumed: {}", invoke_result.gas_consumed);

	println!("\n🎉 Local signer example completed successfully!");
	println!("\n💡 Key takeaways:");
	println!("   • Accounts can be created from WIF or generated randomly");
	println!("   • Transactions must be properly signed before submission");
	println!("   • Multi-signature accounts provide enhanced security");
	println!("   • Contract interactions can be tested before execution");
	println!("   • Always verify signatures and check balances before operations");

	Ok(())
}
