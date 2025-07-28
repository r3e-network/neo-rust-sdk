/// Neo N3 Ledger Hardware Wallet Integration Example
///
/// This example demonstrates a simulation of Ledger hardware wallet integration
/// for Neo N3, including device discovery, address derivation, and transaction signing.
/// In production, this would interface with the actual Ledger device via HID.

use neo3::{
	neo_builder::ScriptBuilder,
	neo_clients::{APITrait, HttpProvider, RpcClient},
	neo_crypto::KeyPair,
	neo_types::{ContractParameter, ScriptHash, ScriptHashExtension},
};
use std::{collections::HashMap, str::FromStr};

/// Simulated Ledger device interface
#[derive(Debug)]
struct LedgerDevice {
	device_id: String,
	neo_app_version: String,
	connected: bool,
	unlocked: bool,
}

/// Ledger signing result
#[derive(Debug)]
struct LedgerSignature {
	signature: Vec<u8>,
	public_key: Vec<u8>,
}

impl LedgerDevice {
	/// Simulate connecting to a Ledger device
	fn new() -> Self {
		Self {
			device_id: "ledger_nano_s_plus_001".to_string(),
			neo_app_version: "1.0.3".to_string(),
			connected: true,
			unlocked: false,
		}
	}

	/// Simulate unlocking the device with PIN
	fn unlock(&mut self, pin: &str) -> Result<(), Box<dyn std::error::Error>> {
		if pin == "1234" {
			// Simulated PIN check
			self.unlocked = true;
			println!("   ✅ Device unlocked successfully");
			Ok(())
		} else {
			Err("Invalid PIN".into())
		}
	}

	/// Simulate getting Neo app info
	fn get_neo_app_info(&self) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
		if !self.connected {
			return Err("Device not connected".into());
		}

		let mut info = HashMap::new();
		info.insert("name".to_string(), "Neo".to_string());
		info.insert("version".to_string(), self.neo_app_version.clone());
		info.insert("flags".to_string(), "0x40".to_string());
		info.insert("mcuVersion".to_string(), "1.12".to_string());

		Ok(info)
	}

	/// Simulate deriving public key from BIP44 path
	fn get_public_key(&self, path: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
		if !self.unlocked {
			return Err("Device locked - please unlock first".into());
		}

		// Simulate deriving a public key for the given path
		// In reality, this would request the key from the hardware device
		let key_pair = KeyPair::new_random(); // Simulated - would be deterministic from device
		let public_key = key_pair.public_key().get_encoded_point(false);

		println!("   🔑 Derived public key for path {path}");
		println!("       Key: {}", hex::encode(&public_key));

		Ok(public_key.as_bytes().to_vec())
	}

	/// Simulate getting Neo address from BIP44 path
	fn get_address(&self, path: &str) -> Result<String, Box<dyn std::error::Error>> {
		let _public_key_bytes = self.get_public_key(path)?;

		// Convert public key to Neo address
		// In reality, this would be done by the device
		let key_pair = KeyPair::new_random(); // Simulated
		let script_hash = key_pair.get_script_hash();
		let address = script_hash.to_address();

		println!("   📍 Derived address for path {path}: {address}");
		Ok(address.to_string())
	}

	/// Simulate signing transaction data
	fn sign_transaction(
		&self,
		path: &str,
		transaction_data: &[u8],
	) -> Result<LedgerSignature, Box<dyn std::error::Error>> {
		if !self.unlocked {
			return Err("Device locked - please unlock first".into());
		}

		println!("   📱 Please confirm transaction on Ledger device...");
		println!("   💼 Transaction size: {} bytes", transaction_data.len());

		// Simulate user confirmation on device
		std::thread::sleep(std::time::Duration::from_millis(100));

		// Simulate signature generation
		let key_pair = KeyPair::new_random(); // In reality, uses hardware-stored key
		let signature = key_pair
			.private_key()
			.sign_tx(transaction_data)
			.map_err(|e| format!("Signing failed: {e}"))?;

		let public_key = self.get_public_key(path)?;

		println!("   ✅ Transaction signed successfully");

		Ok(LedgerSignature {
			signature: signature.to_bytes().to_vec(),
			public_key,
		})
	}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("🔐 Neo N3 Ledger Hardware Wallet Integration Example");
	println!("===================================================");

	// 1. Discover and connect to Ledger device
	println!("\n1️⃣ Discovering Ledger devices...");
	let mut ledger = LedgerDevice::new();
	println!("   ✅ Found device: {}", ledger.device_id);
	println!("   📱 Device connected: {}", ledger.connected);

	// 2. Check Neo app
	println!("\n2️⃣ Checking Neo app on device...");
	match ledger.get_neo_app_info() {
		Ok(app_info) => {
			println!("   ✅ Neo app detected");
			for (key, value) in app_info {
				println!("   📋 {key}: {value}");
			}
		},
		Err(e) => {
			println!("   ❌ Neo app not found: {e}");
			println!("   💡 Please install the Neo app on your Ledger device");
		},
	}

	// 3. Unlock device
	println!("\n3️⃣ Unlocking device...");
	if let Err(e) = ledger.unlock("1234") {
		println!("   ❌ Failed to unlock: {e}");
		return Ok(());
	}

	// 4. Derive Neo addresses
	println!("\n4️⃣ Deriving Neo addresses...");
	let bip44_paths = vec![
		"m/44'/888'/0'/0/0", // First Neo address
		"m/44'/888'/0'/0/1", // Second Neo address
		"m/44'/888'/0'/0/2", // Third Neo address
	];

	let mut derived_addresses = Vec::new();
	for path in &bip44_paths {
		match ledger.get_address(path) {
			Ok(address) => {
				derived_addresses.push((path.to_string(), address));
			},
			Err(e) => {
				println!("   ❌ Failed to derive address for {path}: {e}");
			},
		}
	}

	// 5. Connect to Neo network
	println!("\n5️⃣ Connecting to Neo TestNet...");
	let provider = HttpProvider::new("https://testnet1.neo.org:443/")?;
	let client = RpcClient::new(provider);

	if let Ok(block_count) = client.get_block_count().await {
		println!("   ✅ Connected to TestNet");
		println!("   📦 Current block: {block_count}");
	} else {
		println!("   ❌ Failed to connect to TestNet");
	}

	// 6. Check balances for derived addresses
	println!("\n6️⃣ Checking balances...");
	let gas_hash = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;

	for (path, address) in &derived_addresses {
		println!("   📍 Address: {address} ({})", path);

		if let Ok(address_hash) = ScriptHash::from_address(address) {
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
							println!(
								"       💰 GAS Balance: {} GAS",
								balance as f64 / 100_000_000.0
							);
						}
					}
				},
				Err(e) => {
					println!("       ❌ Failed to check balance: {e}");
				},
			}
		}
	}

	// 7. Demonstrate transaction signing
	println!("\n7️⃣ Demonstrating transaction signing...");
	if let Some((first_path, first_address)) = derived_addresses.first() {
		println!("   📝 Creating test transaction from {first_address}...");

		// Create a simple transaction
		let script = ScriptBuilder::new()
			.contract_call(&gas_hash, "symbol", &[], None)?
			.to_bytes();

		// Simulate transaction signing with Ledger
		let tx_data = b"simulated_transaction_data"; // In reality, this would be the actual transaction hash
		match ledger.sign_transaction(first_path, tx_data) {
			Ok(signature) => {
				println!("   ✅ Transaction signed with Ledger");
				println!("   🔏 Signature length: {} bytes", signature.signature.len());
			},
			Err(e) => {
				println!("   ❌ Signing failed: {e}");
			},
		}
	}

	// 8. Security best practices
	println!("\n8️⃣ Security Best Practices:");
	println!("   🔒 Always verify transaction details on device screen");
	println!("   🛡️ Keep Ledger firmware updated");
	println!("   🔐 Use strong PIN and recovery phrase");
	println!("   💾 Store recovery phrase offline and secure");
	println!("   🚫 Never share recovery phrase or private keys");
	println!("   ✅ Verify addresses before sending funds");

	println!("\n✅ Ledger integration example completed!");
	println!("💡 This demonstrates the workflow for integrating");
	println!("   hardware wallet security with Neo N3 applications");

	Ok(())
}
