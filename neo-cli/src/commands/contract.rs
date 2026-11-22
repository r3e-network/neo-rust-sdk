use crate::{
	commands::defi::create_h160_param, errors::CliError, print_error, print_info, print_success,
	prompt_password,
};
use clap::{Args, Subcommand};
use neo3::{
	builder::{AccountSigner, ScriptBuilder, Signer, TransactionBuilder},
	codec::NeoSerializable,
	neo_clients::APITrait,
	neo_contract::PolicyContract,
	neo_protocol::AccountTrait,
	neo_types::{ContractManifest, NefFile},
	prelude::*,
};
use primitive_types::H160;
use std::{path::PathBuf, str::FromStr};

#[derive(Args, Debug)]
pub struct ContractArgs {
	#[command(subcommand)]
	pub command: ContractCommands,
}

#[derive(Subcommand, Debug)]
pub enum ContractCommands {
	/// Get deployed contract metadata and manifest
	Get {
		/// Contract script hash
		#[arg(short, long)]
		script_hash: String,
	},

	/// Get a native contract by name
	Native {
		/// Native contract name (for example NeoToken, GasToken, PolicyContract)
		#[arg(short, long)]
		name: String,
	},

	/// Deploy a smart contract
	Deploy {
		/// Path to the contract file (.nef)
		#[arg(short, long)]
		nef: PathBuf,

		/// Path to the contract manifest file (.json)
		#[arg(short, long)]
		manifest: PathBuf,

		/// Account to pay for deployment
		#[arg(short, long)]
		account: Option<String>,
	},

	/// Update an existing contract
	Update {
		/// Contract script hash
		#[arg(short, long)]
		script_hash: String,

		/// Path to the new contract file (.nef)
		#[arg(short, long)]
		nef: PathBuf,

		/// Path to the new contract manifest file (.json)
		#[arg(short, long)]
		manifest: PathBuf,

		/// Account to pay for update
		#[arg(short, long)]
		account: Option<String>,
	},

	/// Invoke a contract method
	Invoke {
		/// Contract script hash
		#[arg(short, long)]
		script_hash: String,

		/// Method name
		#[arg(short, long)]
		method: String,

		/// Method parameters as JSON array
		#[arg(short, long)]
		params: Option<String>,

		/// Account to pay for invocation
		#[arg(short, long)]
		account: Option<String>,

		/// Whether to just test the invocation without submitting to the blockchain
		#[arg(short, long, default_value = "false")]
		test_invoke: bool,
	},

	/// Call a contract method without submitting a transaction
	Call {
		/// Contract script hash
		#[arg(short, long)]
		script_hash: String,

		/// Method name
		#[arg(short, long)]
		method: String,

		/// Method parameters as JSON array
		#[arg(short, long)]
		params: Option<String>,
	},

	/// Read contract storage
	Storage {
		/// Contract script hash
		#[arg(short, long)]
		script_hash: String,

		/// Storage key as hex string
		#[arg(short, long)]
		key: String,
	},

	/// Find contract storage by key prefix
	FindStorage {
		/// Contract script hash
		#[arg(short, long)]
		script_hash: String,

		/// Storage key prefix as hex string
		#[arg(short, long)]
		prefix: String,

		/// Start index for paged results
		#[arg(long, default_value = "0")]
		start: u64,
	},

	/// List native contracts
	ListNativeContracts,

	/// Show current network policy values
	Policy,
}

/// CLI state is defined in wallet.rs
pub async fn handle_contract_command(
	args: ContractArgs,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	match args.command {
		ContractCommands::Get { script_hash } => get_contract(script_hash, state).await,
		ContractCommands::Native { name } => get_native_contract(name, state).await,
		ContractCommands::Deploy { nef, manifest, account } => {
			deploy_contract(nef, manifest, account, state).await
		},
		ContractCommands::Update { script_hash, nef, manifest, account } => {
			update_contract(script_hash, nef, manifest, account, state).await
		},
		ContractCommands::Invoke { script_hash, method, params, account, test_invoke } => {
			invoke_contract(script_hash, method, params, account, test_invoke, state).await
		},
		ContractCommands::Call { script_hash, method, params } => {
			invoke_contract(script_hash, method, params, None, true, state).await
		},
		ContractCommands::Storage { script_hash, key } => {
			get_contract_storage(script_hash, key, state).await
		},
		ContractCommands::FindStorage { script_hash, prefix, start } => {
			find_contract_storage(script_hash, prefix, start, state).await
		},
		ContractCommands::ListNativeContracts => list_native_contracts(state).await,
		ContractCommands::Policy => show_policy(state).await,
	}
}

fn strip_hex_prefix(input: &str) -> &str {
	input.strip_prefix("0x").unwrap_or(input)
}

fn parse_h160(input: &str, label: &str) -> Result<H160, CliError> {
	let bytes = hex::decode(strip_hex_prefix(input))
		.map_err(|e| CliError::Input(format!("Invalid {label} hex: {e}")))?;
	if bytes.len() != 20 {
		return Err(CliError::Input(format!("{label} must be 20 bytes")));
	}
	Ok(H160::from_slice(&bytes))
}

async fn get_contract(
	script_hash: String,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	let rpc_client = state.get_rpc_client()?;
	let contract_hash = parse_h160(&script_hash, "contract hash")?;
	let contract = rpc_client
		.get_contract_state(contract_hash)
		.await
		.map_err(|e| CliError::Network(format!("Failed to get contract: {}", e)))?;
	println!("{}", serde_json::to_string_pretty(&contract)?);
	Ok(())
}

async fn get_native_contract(
	name: String,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	let rpc_client = state.get_rpc_client()?;
	let contract = rpc_client
		.get_native_contract_state(&name)
		.await
		.map_err(|e| CliError::Network(format!("Failed to get native contract '{name}': {e}")))?;
	println!("{}", serde_json::to_string_pretty(&contract)?);
	Ok(())
}

async fn get_contract_storage(
	script_hash: String,
	key: String,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	let rpc_client = state.get_rpc_client()?;
	let contract_hash = parse_h160(&script_hash, "contract hash")?;
	let value = rpc_client
		.get_storage(contract_hash, strip_hex_prefix(&key))
		.await
		.map_err(|e| CliError::Network(format!("Failed to get storage: {}", e)))?;
	println!("{value}");
	Ok(())
}

async fn find_contract_storage(
	script_hash: String,
	prefix: String,
	start: u64,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	let rpc_client = state.get_rpc_client()?;
	let contract_hash = parse_h160(&script_hash, "contract hash")?;
	let result = rpc_client
		.find_storage(contract_hash, strip_hex_prefix(&prefix), start)
		.await
		.map_err(|e| CliError::Network(format!("Failed to find storage: {}", e)))?;
	println!("{result}");
	Ok(())
}

async fn deploy_contract(
	nef_path: PathBuf,
	manifest_path: PathBuf,
	account: Option<String>,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	if state.wallet.is_none() {
		print_error("No wallet is currently open");
		return Err(CliError::Wallet("No wallet is currently open".to_string()));
	}

	if state.rpc_client.is_none() {
		print_error("No RPC client is connected. Please connect to a node first.");
		return Err(CliError::Network("No RPC client is connected".to_string()));
	}

	// Check if files exist
	if !nef_path.exists() {
		print_error(&format!("NEF file not found: {:?}", nef_path));
		return Err(CliError::Input(format!("NEF file not found: {:?}", nef_path)));
	}

	if !manifest_path.exists() {
		print_error(&format!("Manifest file not found: {:?}", manifest_path));
		return Err(CliError::Input(format!("Manifest file not found: {:?}", manifest_path)));
	}

	print_info("Deploying smart contract...");

	// Read NEF and manifest files
	let nef_bytes = std::fs::read(&nef_path).map_err(|e| CliError::Io(e))?;
	let manifest_json = std::fs::read_to_string(&manifest_path).map_err(|e| CliError::Io(e))?;

	// Parse NEF and manifest
	let _nef = NefFile::deserialize(&nef_bytes)
		.map_err(|e| CliError::Input(format!("Failed to parse NEF file: {}", e)))?;
	let _manifest: ContractManifest = serde_json::from_str(&manifest_json)
		.map_err(|e| CliError::Input(format!("Failed to parse manifest file: {}", e)))?;

	// Get account to pay for deployment
	let wallet = state.wallet.as_ref().unwrap();
	let account_address = match account {
		Some(addr) => addr,
		None => {
			// If no account specified, use the first account in the wallet
			let accounts = wallet.get_accounts();
			if accounts.is_empty() {
				print_error("No accounts in wallet");
				return Err(CliError::Wallet("No accounts in wallet".to_string()));
			}
			accounts[0].get_address().to_string()
		},
	};

	// Find account in wallet
	let account_obj = wallet
		.get_accounts()
		.into_iter()
		.find(|a| a.get_address() == account_address)
		.cloned()
		.ok_or_else(|| CliError::Wallet(format!("Account not found: {}", account_address)))?;

	// Get password for signing
	let password = prompt_password("Enter wallet password")?;

	// Create and sign deployment transaction
	let rpc_client = state.rpc_client.as_ref().unwrap();

	// Get system fee
	let params =
		vec![ContractParameter::byte_array(nef_bytes), ContractParameter::string(manifest_json)];

	let invocation_result = rpc_client
		.invoke_function(
			&H160::from_hex("fffdc93764dbaddd97c48f252a53ea4643faa3fd").unwrap(), // Management contract
			"deploy".to_string(),
			params.clone(),
			Some(vec![Signer::from(
				AccountSigner::called_by_entry(&account_obj)
					.map_err(|e| CliError::Builder(e.to_string()))?,
			)]),
		)
		.await
		.map_err(|e| CliError::Network(format!("Failed to test invoke deploy: {}", e)))?;

	let system_fee = invocation_result.gas_consumed;
	print_info(&format!("Estimated system fee: {} GAS", system_fee));

	// Get current block count and calculate validUntilBlock
	let block_count = rpc_client
		.get_block_count()
		.await
		.map_err(|e| CliError::Network(format!("Failed to get block count: {}", e)))?;
	let valid_until_block = block_count + 100; // Valid for ~16 minutes (assuming 10s blocks)

	// Build transaction
	let signer = AccountSigner::called_by_entry(&account_obj)
		.map_err(|e| CliError::Builder(e.to_string()))?;
	let signers = vec![Signer::AccountSigner(signer)];

	let mut tx_builder: TransactionBuilder<'_, neo3::neo_clients::HttpProvider> =
		TransactionBuilder::with_client(&rpc_client);

	// Set up the transaction builder with all required parameters
	tx_builder.version(0);
	tx_builder
		.nonce((rand::random::<u32>() % 1000000) as u32)
		.map_err(|e| CliError::from(e))?;
	tx_builder.valid_until_block(valid_until_block).map_err(|e| CliError::from(e))?;
	tx_builder.set_signers(signers).map_err(|e| CliError::from(e))?;

	// Add the script
	let method = "deploy".to_string();
	let script = ScriptBuilder::new()
		.contract_call(
			&H160::from_hex("fffdc93764dbaddd97c48f252a53ea4643faa3fd").unwrap(),
			&method,
			&params,
			None,
		)
		.map_err(|e| CliError::Builder(e.to_string()))?
		.to_bytes();

	tx_builder.set_script(Some(script));

	// Build and sign the transaction (network fee calculated automatically via RPC client)
	let mut tx = tx_builder
		.build()
		.await
		.map_err(|e| CliError::Transaction(format!("Failed to build transaction: {}", e)))?;

	// Sign the transaction with the account's private key
	print_info("Signing transaction with account's private key...");

	// Decrypt the account's private key using the password
	let mut account_clone = account_obj.clone();
	account_clone
		.decrypt_private_key(&password)
		.map_err(|e| CliError::Wallet(format!("Failed to decrypt private key: {}", e)))?;

	// Get the key pair from the decrypted account
	let key_pair = account_clone
		.key_pair()
		.as_ref()
		.ok_or_else(|| CliError::Wallet("No key pair available after decryption".to_string()))?
		.clone();

	// Create a witness for the transaction
	let tx_hash = tx
		.get_hash_data()
		.await
		.map_err(|e| CliError::Transaction(format!("Failed to get transaction hash: {}", e)))?;

	let witness = neo3::builder::Witness::create(tx_hash, &key_pair)
		.map_err(|e| CliError::Transaction(format!("Failed to create witness: {}", e)))?;

	// Add the witness to the transaction
	tx.add_witness(witness);

	let mut encoder = neo3::codec::Encoder::new();
	tx.encode(&mut encoder);
	let tx_hex = hex::encode(encoder.to_bytes());

	// Send transaction
	let result = rpc_client
		.send_raw_transaction(tx_hex)
		.await
		.map_err(|e| CliError::Network(format!("Failed to send transaction: {}", e)))?;

	print_success("Contract deployment transaction sent successfully");
	println!("Transaction hash: {}", result.hash);
	println!(
		"Note: The contract hash can be obtained from the transaction when it is confirmed on the blockchain."
	);

	Ok(())
}

async fn update_contract(
	script_hash: String,
	nef_path: PathBuf,
	manifest_path: PathBuf,
	account: Option<String>,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	if state.wallet.is_none() {
		print_error("No wallet is currently open");
		return Err(CliError::Wallet("No wallet is currently open".to_string()));
	}

	if state.rpc_client.is_none() {
		print_error("No RPC client is connected. Please connect to a node first.");
		return Err(CliError::Network("No RPC client is connected".to_string()));
	}

	// Check if files exist
	if !nef_path.exists() {
		print_error(&format!("NEF file not found: {:?}", nef_path));
		return Err(CliError::Input(format!("NEF file not found: {:?}", nef_path)));
	}

	if !manifest_path.exists() {
		print_error(&format!("Manifest file not found: {:?}", manifest_path));
		return Err(CliError::Input(format!("Manifest file not found: {:?}", manifest_path)));
	}

	print_info(&format!("Updating contract: {}", script_hash));

	// Read NEF and manifest files
	let nef_bytes = std::fs::read(&nef_path).map_err(|e| CliError::Io(e))?;
	let manifest_json = std::fs::read_to_string(&manifest_path).map_err(|e| CliError::Io(e))?;

	// Parse NEF and manifest
	let _nef = NefFile::deserialize(&nef_bytes)
		.map_err(|e| CliError::Input(format!("Failed to parse NEF file: {}", e)))?;
	let _manifest: ContractManifest = serde_json::from_str(&manifest_json)
		.map_err(|e| CliError::Input(format!("Failed to parse manifest file: {}", e)))?;

	// Get account to pay for update
	let wallet = state.wallet.as_ref().unwrap();
	let account_address = match account {
		Some(addr) => addr,
		None => {
			// If no account specified, use the first account in the wallet
			let accounts = wallet.get_accounts();
			if accounts.is_empty() {
				print_error("No accounts in wallet");
				return Err(CliError::Wallet("No accounts in wallet".to_string()));
			}
			accounts[0].get_address().to_string()
		},
	};

	// Find account in wallet
	let account_obj = wallet
		.get_accounts()
		.into_iter()
		.find(|a| a.get_address() == account_address)
		.cloned()
		.ok_or_else(|| CliError::Wallet(format!("Account not found: {}", account_address)))?;

	// Get password for signing
	let password = prompt_password("Enter wallet password")?;

	// Parse contract hash
	let contract_hash = H160::from_str(&script_hash)
		.map_err(|_| CliError::Input("Invalid script hash format".to_string()))?;

	// Create and sign update transaction
	let rpc_client = state.rpc_client.as_ref().unwrap();

	// Get system fee
	let params = vec![
		ContractParameter::h160(&contract_hash),
		ContractParameter::byte_array(nef_bytes),
		ContractParameter::string(manifest_json),
	];

	let invocation_result = rpc_client
		.invoke_function(
			&contract_hash,
			"update".to_string(),
			params.clone(),
			Some(vec![Signer::from(
				AccountSigner::called_by_entry(&account_obj)
					.map_err(|e| CliError::Builder(e.to_string()))?,
			)]),
		)
		.await
		.map_err(|e| CliError::Network(format!("Failed to test invoke update: {}", e)))?;

	let system_fee = invocation_result.gas_consumed;
	print_info(&format!("Estimated system fee: {} GAS", system_fee));

	// Get current block count and calculate validUntilBlock
	let block_count = rpc_client
		.get_block_count()
		.await
		.map_err(|e| CliError::Network(format!("Failed to get block count: {}", e)))?;
	let valid_until_block = block_count + 100; // Valid for ~16 minutes (assuming 10s blocks)

	// Build transaction
	let signer = AccountSigner::called_by_entry(&account_obj)
		.map_err(|e| CliError::Builder(e.to_string()))?;
	let signers = vec![Signer::AccountSigner(signer)];

	let mut tx_builder: TransactionBuilder<'_, neo3::neo_clients::HttpProvider> =
		TransactionBuilder::with_client(&rpc_client);

	// Set up the transaction builder with all required parameters
	tx_builder.version(0);
	tx_builder
		.nonce((rand::random::<u32>() % 1000000) as u32)
		.map_err(|e| CliError::from(e))?;
	tx_builder.valid_until_block(valid_until_block).map_err(|e| CliError::from(e))?;
	tx_builder.set_signers(signers).map_err(|e| CliError::from(e))?;

	// Add the script
	let method = "update".to_string();
	let script = ScriptBuilder::new()
		.contract_call(&contract_hash, &method, &params, None)
		.map_err(|e| CliError::Builder(e.to_string()))?
		.to_bytes();

	tx_builder.set_script(Some(script));

	// Build and sign the transaction (network fee calculated automatically via RPC client)
	let mut tx = tx_builder
		.build()
		.await
		.map_err(|e| CliError::Transaction(format!("Failed to build transaction: {}", e)))?;

	// Sign the transaction with the account's private key
	print_info("Signing transaction with account's private key...");

	// Decrypt the account's private key using the password
	let mut account_clone = account_obj.clone();
	account_clone
		.decrypt_private_key(&password)
		.map_err(|e| CliError::Wallet(format!("Failed to decrypt private key: {}", e)))?;

	// Get the key pair from the decrypted account
	let key_pair = account_clone
		.key_pair()
		.as_ref()
		.ok_or_else(|| CliError::Wallet("No key pair available after decryption".to_string()))?
		.clone();

	// Create a witness for the transaction
	let tx_hash = tx
		.get_hash_data()
		.await
		.map_err(|e| CliError::Transaction(format!("Failed to get transaction hash: {}", e)))?;

	let witness = neo3::builder::Witness::create(tx_hash, &key_pair)
		.map_err(|e| CliError::Transaction(format!("Failed to create witness: {}", e)))?;

	// Add the witness to the transaction
	tx.add_witness(witness);

	let mut encoder = neo3::codec::Encoder::new();
	tx.encode(&mut encoder);
	let tx_hex = hex::encode(encoder.to_bytes());

	// Send transaction
	let result = rpc_client
		.send_raw_transaction(tx_hex)
		.await
		.map_err(|e| CliError::Network(format!("Failed to send transaction: {}", e)))?;

	print_success("Contract updated successfully");
	println!("Transaction hash: {}", result.hash);

	Ok(())
}

async fn invoke_contract(
	script_hash: String,
	method: String,
	params: Option<String>,
	account: Option<String>,
	test_invoke: bool,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	if state.rpc_client.is_none() {
		print_error("No RPC client is connected. Please connect to a node first.");
		return Err(CliError::Network("No RPC client is connected".to_string()));
	}

	// Parse parameters if provided
	let parameters = match params {
		Some(p) => {
			let params_json: Vec<serde_json::Value> = serde_json::from_str(&p)
				.map_err(|e| CliError::Input(format!("Invalid JSON parameters: {}", e)))?;

			// Convert JSON parameters to ContractParameter
			params_json
				.into_iter()
				.map(|v| contract_parameter_from_json(v))
				.collect::<Result<Vec<_>, _>>()?
		},
		None => Vec::new(),
	};

	// Convert script hash
	let contract_hash = H160::from_str(&script_hash)
		.map_err(|_| CliError::Input("Invalid script hash format".to_string()))?;

	let rpc_client = state.rpc_client.as_ref().unwrap();

	if test_invoke {
		print_info(&format!("Test invoking method '{}' on contract {}", method, script_hash));

		// Test invoke
		let result = rpc_client
			.invoke_function(&contract_hash, method.clone(), parameters, None)
			.await
			.map_err(|e| CliError::Network(format!("Failed to invoke function: {}", e)))?;

		// Display result
		println!("Invocation result:");
		println!("  State: {:?}", result.state);
		println!("  Gas consumed: {}", result.gas_consumed);
		println!("  Stack:");
		for (i, item) in result.stack.iter().enumerate() {
			println!("    {}: {:?}", i, item);
		}
	} else {
		// Real invocation
		if state.wallet.is_none() {
			print_error("No wallet is currently open");
			return Err(CliError::Wallet("No wallet is currently open".to_string()));
		}

		print_info(&format!("Invoking method '{}' on contract {}", method, script_hash));

		// Get account to pay for invocation
		let wallet = state.wallet.as_ref().unwrap();
		let account_address = match account {
			Some(addr) => addr,
			None => {
				// If no account specified, use the first account in the wallet
				let accounts = wallet.get_accounts();
				if accounts.is_empty() {
					print_error("No accounts in wallet");
					return Err(CliError::Wallet("No accounts in wallet".to_string()));
				}
				accounts[0].get_address().to_string()
			},
		};

		// Find account in wallet
		let account_obj = wallet
			.get_accounts()
			.into_iter()
			.find(|a| a.get_address() == account_address)
			.cloned()
			.ok_or_else(|| CliError::Wallet(format!("Account not found: {}", account_address)))?;

		// Get password for signing
		let password = prompt_password("Enter wallet password")?;

		// Get system fee
		let invocation_result = rpc_client
			.invoke_function(
				&contract_hash,
				method.clone(),
				parameters.clone(),
				Some(vec![Signer::from(
					AccountSigner::called_by_entry(&account_obj)
						.map_err(|e| CliError::Builder(e.to_string()))?,
				)]),
			)
			.await
			.map_err(|e| CliError::Network(format!("Failed to test invoke: {}", e)))?;

		let system_fee = invocation_result.gas_consumed;
		print_info(&format!("Estimated system fee: {} GAS", system_fee));

		// Get current block count and calculate validUntilBlock
		let block_count = rpc_client
			.get_block_count()
			.await
			.map_err(|e| CliError::Network(format!("Failed to get block count: {}", e)))?;
		let valid_until_block = block_count + 100; // Valid for ~16 minutes (assuming 10s blocks)

		// Build transaction
		let signer = AccountSigner::called_by_entry(&account_obj)
			.map_err(|e| CliError::Builder(e.to_string()))?;
		let signers = vec![Signer::AccountSigner(signer)];

		let mut tx_builder: TransactionBuilder<'_, neo3::neo_clients::HttpProvider> =
			TransactionBuilder::with_client(&rpc_client);

		// Set up the transaction builder with all required parameters
		tx_builder.version(0);
		tx_builder
			.nonce((rand::random::<u32>() % 1000000) as u32)
			.map_err(|e| CliError::from(e))?;
		tx_builder.valid_until_block(valid_until_block).map_err(|e| CliError::from(e))?;
		tx_builder.set_signers(signers).map_err(|e| CliError::from(e))?;

		// Add the script
		let script = ScriptBuilder::new()
			.contract_call(&contract_hash, &method, &parameters, None)
			.map_err(|e| CliError::Builder(e.to_string()))?
			.to_bytes();

		tx_builder.set_script(Some(script));

		// Build and sign the transaction (network fee calculated automatically via RPC client)
		let mut tx = tx_builder
			.build()
			.await
			.map_err(|e| CliError::Transaction(format!("Failed to build transaction: {}", e)))?;

		// Sign the transaction with the account's private key
		print_info("Signing transaction with account's private key...");

		// Decrypt the account's private key using the password
		let mut account_clone = account_obj.clone();
		account_clone
			.decrypt_private_key(&password)
			.map_err(|e| CliError::Wallet(format!("Failed to decrypt private key: {}", e)))?;

		// Get the key pair from the decrypted account
		let key_pair = account_clone
			.key_pair()
			.as_ref()
			.ok_or_else(|| CliError::Wallet("No key pair available after decryption".to_string()))?
			.clone();

		// Create a witness for the transaction
		let tx_hash = tx
			.get_hash_data()
			.await
			.map_err(|e| CliError::Transaction(format!("Failed to get transaction hash: {}", e)))?;

		let witness = neo3::builder::Witness::create(tx_hash, &key_pair)
			.map_err(|e| CliError::Transaction(format!("Failed to create witness: {}", e)))?;

		// Add the witness to the transaction
		tx.add_witness(witness);

		let mut encoder = neo3::codec::Encoder::new();
		tx.encode(&mut encoder);
		let tx_hex = hex::encode(encoder.to_bytes());

		// Send transaction
		let result = rpc_client
			.send_raw_transaction(tx_hex)
			.await
			.map_err(|e| CliError::Network(format!("Failed to send transaction: {}", e)))?;

		print_success("Contract method invoked successfully");
		println!("Transaction hash: {}", result.hash);
	}

	Ok(())
}

async fn list_native_contracts(
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	if state.rpc_client.is_none() {
		print_error("No RPC client is connected. Please connect to a node first.");
		return Err(CliError::Network("No RPC client is connected".to_string()));
	}

	print_info("Native contracts:");

	// List native contracts
	let rpc_client = state.rpc_client.as_ref().unwrap();
	let native_contracts = rpc_client
		.get_native_contracts()
		.await
		.map_err(|e| CliError::Network(format!("Failed to get native contracts: {}", e)))?;

	for (i, contract) in native_contracts.iter().enumerate() {
		println!(
			"{}. {} ({})",
			i + 1,
			contract.manifest().name.as_ref().unwrap_or(&"Unknown".to_string()),
			contract.hash()
		);
		println!("  Supported Standards: {:?}", contract.manifest().supported_standards);
		println!();
	}

	print_success("Native contracts retrieved successfully");
	Ok(())
}

async fn show_policy(state: &mut crate::commands::wallet::CliState) -> Result<(), CliError> {
	if state.rpc_client.is_none() {
		print_error("No RPC client is connected. Please connect to a node first.");
		return Err(CliError::Network("No RPC client is connected".to_string()));
	}

	let policy = PolicyContract::new(state.rpc_client.as_ref());

	print_info("Fetching policy values...");

	let fee_per_byte = policy
		.get_fee_per_byte()
		.await
		.map_err(|e| CliError::Network(format!("Failed to get fee per byte: {}", e)))?;

	let exec_fee_factor = policy
		.get_exec_fee_factor()
		.await
		.map_err(|e| CliError::Network(format!("Failed to get exec fee factor: {}", e)))?;

	let storage_price = policy
		.get_storage_price()
		.await
		.map_err(|e| CliError::Network(format!("Failed to get storage price: {}", e)))?;

	let pico_factor = match policy.get_exec_pico_fee_factor().await {
		Ok(val) => val.to_string(),
		Err(_) => "Not supported (Neo 3.9+)".to_string(),
	};

	let milliseconds_per_block = match policy.get_milliseconds_per_block().await {
		Ok(val) => format!("{} ms", val),
		Err(_) => "Not supported".to_string(),
	};

	println!("Policy Contract State:");
	println!("  Fee Per Byte:         {}", fee_per_byte);
	println!("  Exec Fee Factor:      {}", exec_fee_factor);
	println!("  Storage Price:        {}", storage_price);
	println!("  Exec Pico Fee Factor: {}", pico_factor);
	println!("  Milliseconds/Block:   {}", milliseconds_per_block);

	Ok(())
}

// Helper to convert JSON to ContractParameter
fn contract_parameter_from_json(value: serde_json::Value) -> Result<ContractParameter, CliError> {
	match value {
		serde_json::Value::Null => Ok(ContractParameter::any()),
		serde_json::Value::Bool(b) => Ok(ContractParameter::bool(b)),
		serde_json::Value::Number(n) => {
			if n.is_i64() {
				Ok(ContractParameter::integer(n.as_i64().unwrap()))
			} else if n.is_f64() {
				Ok(ContractParameter::string(n.to_string()))
			} else {
				Err(CliError::Input("Invalid number type".to_string()))
			}
		},
		serde_json::Value::String(s) => {
			// Check if it's a hex string (for ByteArray)
			if let Some(hex_str) = s.strip_prefix("0x") {
				match hex::decode(hex_str) {
					Ok(bytes) => Ok(ContractParameter::byte_array(bytes)),
					Err(_) => Ok(ContractParameter::string(s)),
				}
			} else if let Some(hash_str) = s.strip_prefix("@") {
				// Special format for Hash160
				match H160::from_str(hash_str) {
					Ok(hash) => create_h160_param(&format!("{:x}", hash)),
					Err(_) => Ok(ContractParameter::string(s)),
				}
			} else {
				Ok(ContractParameter::string(s))
			}
		},
		serde_json::Value::Array(arr) => {
			let mut params = Vec::new();
			for item in arr {
				params.push(contract_parameter_from_json(item)?);
			}
			Ok(ContractParameter::array(params))
		},
		serde_json::Value::Object(_) => {
			Err(CliError::Input("Object parameters not supported".to_string()))
		},
	}
}
