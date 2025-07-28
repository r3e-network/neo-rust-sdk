/// Neo N3 YubiHSM Hardware Security Module Integration Example
///
/// This example demonstrates a simulation of YubiHSM integration for Neo N3,
/// including key generation, storage, and transaction signing using enterprise-grade
/// hardware security. In production, this would interface with an actual YubiHSM device.

use neo3::{
	neo_builder::{ScriptBuilder, TransactionBuilder},
	neo_clients::{APITrait, HttpProvider, RpcClient},
	neo_crypto::KeyPair,
	neo_types::{ContractParameter, ScriptHash, ScriptHashExtension},
};
use std::collections::HashMap;

/// Simulated YubiHSM connection configuration
#[derive(Debug, Clone)]
struct YubiHsmConfig {
	connector_url: String,
	auth_key_id: u16,
	password: String,
	timeout_ms: u32,
}

/// Simulated YubiHSM device interface
#[derive(Debug)]
struct YubiHsm {
	config: YubiHsmConfig,
	connected: bool,
	authenticated: bool,
	session_id: Option<u16>,
	stored_keys: HashMap<u16, String>, // key_id -> key_info
}

/// Key metadata stored in YubiHSM
#[derive(Debug, Clone)]
struct KeyMetadata {
	id: u16,
	algorithm: String,
	capabilities: Vec<String>,
	delegated_capabilities: Vec<String>,
	domains: u16,
	label: String,
}

/// HSM signing operation result
#[derive(Debug)]
struct HsmSignature {
	signature: Vec<u8>,
	key_id: u16,
	algorithm: String,
}

impl YubiHsm {
	/// Create new YubiHSM instance with configuration
	fn new(config: YubiHsmConfig) -> Self {
		Self {
			config,
			connected: false,
			authenticated: false,
			session_id: None,
			stored_keys: HashMap::new(),
		}
	}

	/// Connect to YubiHSM device
	fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
		println!("   🔌 Connecting to YubiHSM at {}...", self.config.connector_url);
		
		// Simulate connection establishment
		std::thread::sleep(std::time::Duration::from_millis(200));
		
		self.connected = true;
		println!("   ✅ Successfully connected to YubiHSM");
		Ok(())
	}

	/// Authenticate with YubiHSM using auth key
	fn authenticate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
		if !self.connected {
			return Err("Not connected to device".into());
		}

		println!("   🔐 Authenticating with auth key {}...", self.config.auth_key_id);
		
		// Simulate authentication process
		std::thread::sleep(std::time::Duration::from_millis(100));
		
		self.authenticated = true;
		self.session_id = Some(1234); // Simulated session ID
		println!("   ✅ Authentication successful (Session: {})", self.session_id.unwrap());
		Ok(())
	}

	/// Generate a new asymmetric key pair in the HSM
	fn generate_key_pair(&mut self, key_id: u16, label: &str) -> Result<KeyMetadata, Box<dyn std::error::Error>> {
		if !self.authenticated {
			return Err("Not authenticated".into());
		}

		println!("   🔑 Generating key pair with ID {key_id} (label: {label})...");
		
		// Simulate key generation
		std::thread::sleep(std::time::Duration::from_millis(500));
		
		let metadata = KeyMetadata {
			id: key_id,
			algorithm: "secp256r1".to_string(),
			capabilities: vec!["sign-ecdsa".to_string(), "exportable-under-wrap".to_string()],
			delegated_capabilities: vec!["sign-ecdsa".to_string()],
			domains: 1,
			label: label.to_string(),
		};

		// Store key metadata
		self.stored_keys.insert(key_id, format!("Neo key: {label}"));
		
		println!("   ✅ Key pair generated successfully");
		println!("       ID: {}", metadata.id);
		println!("       Algorithm: {}", metadata.algorithm);
		println!("       Capabilities: {:?}", metadata.capabilities);
		
		Ok(metadata)
	}

	/// Get public key for a stored key ID
	fn get_public_key(&self, key_id: u16) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
		if !self.authenticated {
			return Err("Not authenticated".into());
		}

		if !self.stored_keys.contains_key(&key_id) {
			return Err(format!("Key {key_id} not found").into());
		}

		println!("   📤 Retrieving public key for key ID {key_id}...");
		
		// Simulate public key retrieval
		let key_pair = KeyPair::new_random(); // In reality, this would be the actual stored key
		let public_key = key_pair.public_key().get_encoded_point(false);
		
		println!("   ✅ Public key retrieved");
		println!("       Key: {}", hex::encode(&public_key));
		
		Ok(public_key.as_bytes().to_vec())
	}

	/// Sign data using a stored private key
	fn sign_data(&self, key_id: u16, data: &[u8]) -> Result<HsmSignature, Box<dyn std::error::Error>> {
		if !self.authenticated {
			return Err("Not authenticated".into());
		}

		if !self.stored_keys.contains_key(&key_id) {
			return Err(format!("Key {key_id} not found").into());
		}

		println!("   ✍️ Signing {} bytes with key ID {key_id}...", data.len());
		
		// Simulate hardware signing operation
		std::thread::sleep(std::time::Duration::from_millis(300));
		
		// In reality, this would use the HSM's cryptographic capabilities
		let key_pair = KeyPair::new_random();
		let signature = key_pair.private_key().sign_tx(data)
			.map_err(|e| format!("HSM signing failed: {e}"))?;
		
		println!("   ✅ Data signed successfully with HSM");
		
		Ok(HsmSignature {
			signature: signature.to_bytes(),
			key_id,
			algorithm: "secp256r1".to_string(),
		})
	}

	/// List all stored keys in the HSM
	fn list_keys(&self) -> Result<Vec<u16>, Box<dyn std::error::Error>> {
		if !self.authenticated {
			return Err("Not authenticated".into());
		}

		let key_ids: Vec<u16> = self.stored_keys.keys().cloned().collect();
		println!("   📋 Found {} stored keys: {:?}", key_ids.len(), key_ids);
		
		Ok(key_ids)
	}

	/// Get Neo address from stored key
	fn get_neo_address(&self, key_id: u16) -> Result<String, Box<dyn std::error::Error>> {
		let _public_key = self.get_public_key(key_id)?;
		
		// Convert public key to Neo address
		// In reality, this would use the actual public key from HSM
		let key_pair = KeyPair::new_random(); // Simulated
		let script_hash = key_pair.get_script_hash();
		let address = script_hash.to_address();
		
		println!("   📍 Neo address for key {key_id}: {address}");
		Ok(address.to_string())
	}

	/// Close the HSM session
	fn close(&mut self) {
		if self.authenticated {
			println!("   🔒 Closing HSM session...");
			self.authenticated = false;
			self.session_id = None;
		}
		if self.connected {
			self.connected = false;
			println!("   ✅ Disconnected from YubiHSM");
		}
	}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("🔐 Neo N3 YubiHSM Hardware Security Module Integration Example");
	println!("==============================================================");

	// 1. Configure YubiHSM connection
	println!("\n1️⃣ Configuring YubiHSM connection...");
	let config = YubiHsmConfig {
		connector_url: "http://localhost:12345".to_string(),
		auth_key_id: 1,
		password: "password".to_string(),
		timeout_ms: 5000,
	};
	
	let mut hsm = YubiHsm::new(config.clone());
	println!("   ⚙️ HSM Connector: {}", config.connector_url);
	println!("   🆔 Auth Key ID: {}", config.auth_key_id);

	// 2. Connect and authenticate
	println!("\n2️⃣ Establishing HSM connection...");
	hsm.connect()?;
	hsm.authenticate()?;

	// 3. Generate Neo key pairs in HSM
	println!("\n3️⃣ Generating Neo key pairs in HSM...");
	let key_labels = vec![
		(100, "neo-main-wallet"),
		(101, "neo-hot-wallet"),
		(102, "neo-cold-storage"),
	];

	let mut generated_keys = Vec::new();
	for (key_id, label) in key_labels {
		match hsm.generate_key_pair(key_id, label) {
			Ok(metadata) => {
				generated_keys.push(metadata);
			},
			Err(e) => {
				println!("   ❌ Failed to generate key {key_id}: {e}");
			},
		}
	}

	// 4. List all keys in HSM
	println!("\n4️⃣ Listing keys in HSM...");
	if let Ok(key_ids) = hsm.list_keys() {
		for key_id in key_ids {
			if let Ok(address) = hsm.get_neo_address(key_id) {
				println!("   🔑 Key {key_id}: {address}");
			}
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

	// 6. Check balances for HSM-managed addresses
	println!("\n6️⃣ Checking balances for HSM addresses...");
	let gas_hash = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;

	for metadata in &generated_keys {
		if let Ok(address) = hsm.get_neo_address(metadata.id) {
			println!("   📍 Address for key {}: {}", metadata.id, address);

			if let Ok(address_hash) = ScriptHash::from_address(&address) {
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
	}

	// 7. Demonstrate transaction signing with HSM
	println!("\n7️⃣ Demonstrating HSM transaction signing...");
	if let Some(first_key) = generated_keys.first() {
		println!("   📝 Creating transaction for HSM signing...");

		// Create a simple transaction
		let script = ScriptBuilder::new()
			.contract_call(&gas_hash, "symbol", &[], None)?
			.to_bytes();

		// Sign with HSM
		let tx_data = b"neo_transaction_to_sign"; // In reality, this would be the transaction hash
		match hsm.sign_data(first_key.id, tx_data) {
			Ok(signature) => {
				println!("   ✅ Transaction signed with HSM");
				println!("   🔑 Key ID: {}", signature.key_id);
				println!("   🔐 Algorithm: {}", signature.algorithm);
				println!("   🔏 Signature length: {} bytes", signature.signature.len());
			},
			Err(e) => {
				println!("   ❌ HSM signing failed: {e}");
			},
		}
	}

	// 8. Enterprise security features
	println!("\n8️⃣ Enterprise Security Features:");
	println!("   🏢 Multi-tenant key isolation");
	println!("   🔐 Hardware-backed key generation");
	println!("   📊 Comprehensive audit logging");
	println!("   🔒 Tamper-evident hardware design");
	println!("   ⚡ High-performance signing operations");
	println!("   🛡️ FIPS 140-2 Level 3 certification");
	println!("   🌐 Network-based secure access");

	// 9. Use cases for YubiHSM with Neo
	println!("\n9️⃣ Enterprise Use Cases:");
	println!("   💱 Cryptocurrency exchanges");
	println!("   🏦 Custodial wallet services");
	println!("   🏭 Manufacturing blockchain applications");
	println!("   🔗 Supply chain management systems");
	println!("   🗳️ Voting and governance systems");
	println!("   💰 DeFi protocol treasury management");

	// 10. Clean up
	println!("\n🔟 Cleaning up HSM session...");
	hsm.close();

	println!("\n✅ YubiHSM integration example completed!");
	println!("💡 This demonstrates enterprise-grade security");
	println!("   for Neo N3 applications using hardware security modules");

	Ok(())
}
