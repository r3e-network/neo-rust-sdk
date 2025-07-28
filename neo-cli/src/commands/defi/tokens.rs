#![allow(dead_code)]
// Token operations for Neo CLI
//
// This module provides commands for interacting with NEP-17 tokens on the Neo N3 blockchain.
// It supports token information retrieval, balance checking, and token transfers.
//
// NOTE: This is currently an early implementation with some functional limitations.
// Token operations require connection to a Neo N3 RPC node and a properly configured wallet.

use super::utils::{
	format_token_amount, get_token_decimals, parse_amount,
	resolve_token_to_scripthash_with_network, NetworkTypeCli,
};
use crate::{commands::wallet::CliState, errors::CliError};
use colored::*;
use hex;
use neo3::{
	builder::{AccountSigner, CallFlags, ScriptBuilder, Signer},
	neo_clients::APITrait,
	neo_codec::NeoSerializable,
	neo_protocol::AccountTrait,
	neo_types::AddressExtension,
	prelude::*,
};
use primitive_types::{H160, H256};
use rand;
use std::str::FromStr;

// Local helper functions
fn print_success(message: &str) {
	println!("{}", message.green());
}

fn print_info(message: &str) {
	println!("{}", message.blue());
}

fn print_error(message: &str) {
	eprintln!("{}", message.red());
}

fn prompt_password(prompt: &str) -> Result<String, CliError> {
	use std::io::{self, Write};

	print!("{}: ", prompt);
	io::stdout().flush().map_err(|e| CliError::Io(e))?;

	let mut password = String::new();
	io::stdin().read_line(&mut password).map_err(|e| CliError::Io(e))?;

	Ok(password.trim().to_string())
}

fn prompt_yes_no(prompt: &str) -> bool {
	use std::io::{self, Write};

	print!("{} [y/N]: ", prompt);
	io::stdout().flush().unwrap();

	let mut input = String::new();
	io::stdin().read_line(&mut input).unwrap();

	let input = input.trim().to_lowercase();
	input == "y" || input == "yes"
}

// Ensure account is loaded
fn ensure_account_loaded(state: &mut CliState) -> Result<neo3::neo_protocol::Account, CliError> {
	state.get_account()
}

/// Get token information
///
/// Retrieves detailed information about a NEP-17 token including name,
/// symbol, decimals, and total supply.
///
/// This function attempts to resolve token symbols to their script hashes and
/// queries the blockchain for token information. There are currently some type
/// compatibility issues being addressed between the wallet and neo3 libraries.
///
/// # Arguments
/// * `contract` - Token contract address or symbol
/// * `state` - CLI state containing wallet and RPC client
///
/// # Returns
/// * `Result<(), CliError>` - Success or error
pub async fn get_token_info(contract: &str, state: &CliState) -> Result<(), CliError> {
	let rpc_client = state.get_rpc_client()?;

	// Convert network string to NetworkTypeCli
	let network_type = NetworkTypeCli::from_network_string(&state.get_network_type_string());

	// Resolve token to script hash
	let token_hash =
		resolve_token_to_scripthash_with_network(contract, rpc_client, network_type).await?;

	// Get token name
	match rpc_client
		.invoke_function_diagnostics(token_hash, "name".to_string(), vec![], vec![])
		.await
	{
		Ok(result) => {
			if let Some(stack_item) = result.stack.first() {
				if let Some(bytes) = stack_item.as_bytes() {
					let name = String::from_utf8_lossy(&bytes);
					print_info(&format!("Token Name: {}", name));
				} else {
					print_info("Token Name: <Cannot decode name>");
				}
			} else {
				print_info("Token Name: <No result>");
			}
		},
		Err(e) => {
			print_error(&format!("Failed to get token name: {}", e));
		},
	}

	// Get token symbol
	match rpc_client
		.invoke_function_diagnostics(token_hash, "symbol".to_string(), vec![], vec![])
		.await
	{
		Ok(result) => {
			if let Some(stack_item) = result.stack.first() {
				if let Some(bytes) = stack_item.as_bytes() {
					let symbol = String::from_utf8_lossy(&bytes);
					print_info(&format!("Token Symbol: {}", symbol));
				} else {
					print_info("Token Symbol: <Cannot decode symbol>");
				}
			} else {
				print_info("Token Symbol: <No result>");
			}
		},
		Err(e) => {
			print_error(&format!("Failed to get token symbol: {}", e));
		},
	}

	// Get token decimals
	match get_token_decimals(&token_hash, rpc_client, network_type).await {
		Ok(decimals) => {
			print_info(&format!("Token Decimals: {}", decimals));
		},
		Err(e) => {
			print_error(&format!("Failed to get token decimals: {}", e));
		},
	}

	// Get token total supply
	match rpc_client
		.invoke_function_diagnostics(token_hash, "totalSupply".to_string(), vec![], vec![])
		.await
	{
		Ok(result) => {
			if let Some(stack_item) = result.stack.first() {
				if let Some(amount) = stack_item.as_int() {
					if let Ok(decimals) =
						get_token_decimals(&token_hash, rpc_client, network_type).await
					{
						let formatted = format_token_amount(amount, decimals);
						print_info(&format!("Total Supply: {}", formatted));
					} else {
						print_info(&format!("Total Supply (raw): {}", amount));
					}
				} else {
					print_info("Total Supply: <Cannot decode amount>");
				}
			} else {
				print_info("Total Supply: <No result>");
			}
		},
		Err(e) => {
			print_error(&format!("Failed to get token total supply: {}", e));
		},
	}

	Ok(())
}

/// Get token balance for an address
pub async fn get_token_balance(
	contract: &str,
	target_address: &str,
	state: &CliState,
) -> Result<(), CliError> {
	let rpc_client = state.get_rpc_client()?;

	// Convert network string to NetworkTypeCli
	let network_type = NetworkTypeCli::from_network_string(&state.get_network_type_string());

	// Resolve token to script hash
	let token_hash =
		resolve_token_to_scripthash_with_network(contract, rpc_client, network_type).await?;

	// Convert address to script hash
	let addr_script_hash = address_to_script_hash(target_address).map_err(|e| {
		CliError::Wallet(format!("Failed to convert address to script hash: {}", e))
	})?;

	// Call balanceOf method
	match rpc_client
		.invoke_function_diagnostics(
			token_hash,
			"balanceOf".to_string(),
			vec![ContractParameter::h160(&addr_script_hash)],
			vec![],
		)
		.await
	{
		Ok(result) => {
			if let Some(stack_item) = result.stack.first() {
				if let Some(amount) = stack_item.as_int() {
					// Get token symbol for display
					let token_symbol = match rpc_client
						.invoke_function_diagnostics(
							token_hash,
							"symbol".to_string(),
							vec![],
							vec![],
						)
						.await
					{
						Ok(result) => {
							if let Some(stack_item) = result.stack.first() {
								if let Some(bytes) = stack_item.as_bytes() {
									String::from_utf8_lossy(&bytes).to_string()
								} else {
									"Unknown".to_string()
								}
							} else {
								"Unknown".to_string()
							}
						},
						Err(_) => "Unknown".to_string(),
					};

					// Format with decimals if available
					if let Ok(decimals) =
						get_token_decimals(&token_hash, rpc_client, network_type).await
					{
						let formatted = format_token_amount(amount, decimals);
						print_info(&format!("Balance: {} {}", formatted, token_symbol));
					} else {
						print_info(&format!("Balance (raw): {} {}", amount, token_symbol));
					}
				} else {
					print_error("Could not parse balance from response");
				}
			} else {
				print_error("Empty response from balanceOf call");
			}
		},
		Err(e) => {
			print_error(&format!("Failed to get balance: {}", e));
		},
	}

	Ok(())
}

/// Transfer tokens to an address
pub async fn transfer_token(
	contract: &str,
	to_address: &str,
	amount: &str,
	state: &mut CliState,
) -> Result<(), CliError> {
	// Ensure account is loaded
	let account = ensure_account_loaded(state)?;

	// Convert address to script hash
	let _to_address_obj = Address::from_str(to_address)
		.map_err(|_| CliError::Wallet(format!("Failed to parse address: {}", to_address)))?;
	let to_script_hash = address_to_script_hash(to_address).map_err(|e| {
		CliError::Wallet(format!("Failed to convert address to script hash: {}", e))
	})?;

	let rpc_client = state.get_rpc_client()?;

	// Convert network string to NetworkTypeCli
	let network_type = NetworkTypeCli::from_network_string(&state.get_network_type_string());

	// Resolve token to script hash
	let token_hash =
		resolve_token_to_scripthash_with_network(contract, rpc_client, network_type).await?;

	// Get token symbol for display
	let token_symbol = match rpc_client
		.invoke_function_diagnostics(token_hash, "symbol".to_string(), vec![], vec![])
		.await
	{
		Ok(result) => {
			if let Some(stack_item) = result.stack.first() {
				if let Some(bytes) = stack_item.as_bytes() {
					String::from_utf8_lossy(&bytes).to_string()
				} else {
					"Unknown".to_string()
				}
			} else {
				"Unknown".to_string()
			}
		},
		Err(_) => "Unknown".to_string(),
	};

	// Get token decimals
	let decimals = get_token_decimals(&token_hash, rpc_client, network_type).await?;

	// Parse and validate amount
	let token_amount = parse_amount(amount, &token_hash, rpc_client, network_type).await?;

	// Confirm transfer with user
	let formatted_amount = format_token_amount(token_amount, decimals);
	print_info(&format!(
		"Preparing to transfer {} {} to {}",
		formatted_amount, token_symbol, to_address
	));

	if !prompt_yes_no("Do you want to proceed with this transfer?") {
		return Err(CliError::UserCancelled("Transfer cancelled by user".to_string()));
	}

	// Check if account is encrypted and prompt for password if needed
	let password = if account.encrypted_private_key().is_some() && account.key_pair().is_none() {
		Some(prompt_password("Enter password to decrypt account")?)
	} else {
		None
	};

	// Prepare parameters for transfer method
	let mut params = vec![
		ContractParameter::h160(&account.get_script_hash()),
		ContractParameter::h160(&to_script_hash),
		ContractParameter::integer(token_amount),
	];

	// Add data parameter if specified
	let data_param = ContractParameter::any();
	params.push(data_param);

	print_info("Testing transfer transaction...");

	// Create a signer with appropriate scope
	let signers = vec![Signer::from(
		AccountSigner::called_by_entry_hash160(account.get_script_hash()).unwrap(),
	)];

	// Test invoke the transfer
	let result = rpc_client
		.invoke_function_diagnostics(
			token_hash,
			"transfer".to_string(),
			params.clone(),
			signers.clone(),
		)
		.await?;

	// Check if the result indicates success
	let will_succeed = if let Some(stack_item) = result.stack.first() {
		stack_item.as_bool().unwrap_or(false)
	} else {
		false
	};

	if !will_succeed {
		return Err(CliError::TransactionFailed(
			"Transfer validation failed - insufficient balance or invalid recipient address"
				.to_string(),
		));
	}

	// Confirm final execution
	if prompt_yes_no("Ready to submit transaction. Continue?") {
		// Build and send transaction
		print_info("Building and sending transaction...");

		// Create a script builder for the transfer
		let mut script_builder = ScriptBuilder::new();
		script_builder.contract_call(&token_hash, "transfer", &params, Some(CallFlags::All))?;

		let script = script_builder.to_bytes();

		// Send the transaction to the network
		print_info("Sending transaction to the network...");

		// Get the RPC client from state
		let rpc_client = state.rpc_client.as_ref().ok_or_else(|| {
			CliError::Network(
				"No RPC client connected. Please connect to a node first.".to_string(),
			)
		})?;

		// Build a proper transaction
		let mut tx_builder = neo3::builder::TransactionBuilder::with_client(rpc_client);

		// Set transaction parameters
		tx_builder.version(0);
		tx_builder
			.nonce((rand::random::<u32>() % 1000000) as u32)
			.map_err(|e| CliError::Transaction(format!("Failed to set nonce: {}", e)))?;

		// Get current block count for valid until block
		let block_count = rpc_client
			.get_block_count()
			.await
			.map_err(|e| CliError::Network(format!("Failed to get block count: {}", e)))?;
		tx_builder.valid_until_block(block_count + 100).map_err(|e| {
			CliError::Transaction(format!("Failed to set valid until block: {}", e))
		})?;

		// Set the script
		tx_builder.set_script(Some(script));

		// Set signers
		tx_builder
			.set_signers(signers)
			.map_err(|e| CliError::Transaction(format!("Failed to set signers: {}", e)))?;

		// Build the transaction
		let mut tx = tx_builder
			.build()
			.await
			.map_err(|e| CliError::Transaction(format!("Failed to build transaction: {}", e)))?;

		// Sign the transaction if we have a password
		if let Some(password) = password {
			// Decrypt the account
			let mut account_clone = account.clone();
			account_clone
				.decrypt_private_key(&password)
				.map_err(|e| CliError::Wallet(format!("Failed to decrypt private key: {}", e)))?;

			// Get the key pair
			let key_pair = account_clone
				.key_pair()
				.as_ref()
				.ok_or_else(|| {
					CliError::Wallet("No key pair available after decryption".to_string())
				})?
				.clone();

			// Create a witness for the transaction
			let tx_hash = tx.get_hash_data().await.map_err(|e| {
				CliError::Transaction(format!("Failed to get transaction hash: {}", e))
			})?;

			let witness = neo3::builder::Witness::create(tx_hash, &key_pair)
				.map_err(|e| CliError::Transaction(format!("Failed to create witness: {}", e)))?;

			// Add the witness to the transaction
			tx.add_witness(witness);
		}

		// Encode the transaction for sending
		let mut encoder = neo3::codec::Encoder::new();
		tx.encode(&mut encoder);
		let tx_bytes = encoder.to_bytes();
		let tx_hex = hex::encode(&tx_bytes);

		// Send the raw transaction
		match rpc_client.send_raw_transaction(tx_hex).await {
			Ok(result) => {
				print_success("✅ Transaction sent successfully!");
				println!("   Transaction Hash: {}", result.hash);
				println!("   Status: Pending confirmation");
				println!("   Note: Transaction will be confirmed in the next block");

				// Optionally wait for confirmation
				if prompt_yes_no("Wait for transaction confirmation?") {
					match wait_for_transaction_confirmation(rpc_client, &result.hash.to_string())
						.await
					{
						Ok((block_hash, confirmations)) => {
							print_success(&format!(
								"✅ Transaction confirmed! (Block: {}, Confirmations: {})",
								block_hash, confirmations
							));
						},
						Err(e) => {
							print_error(&format!("❌ Transaction confirmation failed: {}", e));
							print_info("Transaction may still be pending. Check manually later.");
						},
					}
				}

				Ok(())
			},
			Err(e) => {
				print_error(&format!("❌ Failed to send transaction: {}", e));
				Err(CliError::Network(format!("Transaction failed: {}", e)))
			},
		}
	} else {
		Err(CliError::UserCancelled("Transaction cancelled by user".to_string()))
	}
}

async fn resolve_token_to_address(state: &mut CliState, token: &str) -> Result<String, CliError> {
	let network_type = network_type_from_state(state);
	let token_hash = resolve_token_to_scripthash_with_network(
		token,
		&state
			.rpc_client
			.as_ref()
			.ok_or(CliError::Config("RPC client not initialized".to_string()))?
			.clone(),
		network_type,
	)
	.await
	.map_err(|e| CliError::Config(format!("Failed to resolve token: {}", e)))?;

	Ok(token_hash.to_address())
}

/// Convert CliState.network_type to NetworkTypeCli
fn network_type_from_state(state: &CliState) -> NetworkTypeCli {
	match &state.network_type {
		Some(network) => match network.to_lowercase().as_str() {
			"mainnet" => NetworkTypeCli::MainNet,
			"testnet" => NetworkTypeCli::TestNet,
			_ => NetworkTypeCli::MainNet, // Default to MainNet
		},
		None => NetworkTypeCli::TestNet, // Default to TestNet if not specified
	}
}

// Helper function to convert address to script hash
fn address_to_script_hash(address: &str) -> Result<H160, CliError> {
	Address::from_str(address)
		.map_err(|_| CliError::Wallet(format!("Invalid address format: {}", address)))?
		.address_to_script_hash()
		.map_err(|e| CliError::Wallet(format!("Failed to convert address to script hash: {}", e)))
}

/// Production-ready transaction confirmation monitoring with exponential backoff and robust error handling
async fn wait_for_transaction_confirmation<T: APITrait>(
	rpc_client: &T,
	tx_hash: &str,
) -> Result<(String, u32), CliError> {
	const MAX_ATTEMPTS: u32 = 60; // Up to 10 minutes with exponential backoff
	const INITIAL_DELAY_MS: u64 = 1000; // Start with 1 second
	const MAX_DELAY_MS: u64 = 30000; // Max 30 seconds between attempts
	const BACKOFF_MULTIPLIER: f64 = 1.5;

	print_info("🔄 Monitoring transaction confirmation...");

	let mut attempt = 1;
	let mut delay_ms = INITIAL_DELAY_MS;
	let mut consecutive_errors = 0;
	const MAX_CONSECUTIVE_ERRORS: u32 = 5;

	while attempt <= MAX_ATTEMPTS {
		// Sleep with exponential backoff (except first attempt)
		if attempt > 1 {
			tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
		}

		let tx_hash_h256 = H256::from_str(&tx_hash)
			.map_err(|_| CliError::InvalidInput("Invalid transaction hash format".to_string()))?;
		match rpc_client.get_transaction(tx_hash_h256).await {
			Ok(tx_result) => {
				consecutive_errors = 0; // Reset error counter on success

				if tx_result.confirmations > 0 {
					// Transaction confirmed!
					return Ok((
						format!("{}", tx_result.block_hash),
						tx_result.confirmations.max(0) as u32,
					));
				} else {
					// Transaction found but not yet confirmed
					if attempt % 10 == 0 {
						print_info(&format!(
							"⏳ Transaction found in mempool, waiting for confirmation... (attempt {}/{})",
							attempt, MAX_ATTEMPTS
						));
					}
				}
			},
			Err(e) => {
				consecutive_errors += 1;

				// Check if it's a "transaction not found" error (normal while pending)
				let error_message = format!("{}", e);
				if error_message.contains("not found")
					|| error_message.contains("Unknown transaction")
				{
					// This is expected while transaction is in mempool
					if attempt % 15 == 0 {
						print_info(&format!(
							"🔍 Searching for transaction... (attempt {}/{})",
							attempt, MAX_ATTEMPTS
						));
					}
				} else {
					// Unexpected error
					print_error(&format!("⚠️  Network error during confirmation check: {}", e));

					if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
						return Err(CliError::Network(format!(
							"Too many consecutive network errors ({}). Last error: {}",
							consecutive_errors, e
						)));
					}
				}
			},
		}

		// Update delay for exponential backoff
		delay_ms = ((delay_ms as f64) * BACKOFF_MULTIPLIER) as u64;
		if delay_ms > MAX_DELAY_MS {
			delay_ms = MAX_DELAY_MS;
		}

		attempt += 1;
	}

	// Timeout reached
	Err(CliError::Network(format!(
		"Transaction confirmation timeout after {} attempts. Transaction may still be pending.",
		MAX_ATTEMPTS
	)))
}
