#![allow(warnings)]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! ![Neo Logo](https://neo.org/images/neo-logo/NEO-logo.svg)
//! # NeoRust SDK v0.4.1
//!
//! A comprehensive Rust library for building applications on the Neo N3 blockchain ecosystem.
//!
//! [![Crates.io](https://img.shields.io/crates/v/neo3.svg)](https://crates.io/crates/neo3)
//! [![Documentation](https://docs.rs/neo3/badge.svg)](https://docs.rs/neo3)
//!
//! ## Features
//!
//! This crate provides several feature flags to customize functionality:
//!
//! - **futures**: Enables async/futures support for asynchronous blockchain operations. This is recommended
//!   for most applications that need to interact with the Neo blockchain without blocking.
//!
//! - **ledger**: Enables hardware wallet support via Ledger devices. When enabled, you can use Ledger
//!   hardware wallets for transaction signing and key management. This feature provides an additional
//!   security layer by keeping private keys on dedicated hardware.
//!
//! - **aws**: ⚠️ **DISABLED in v0.4.1** due to security vulnerabilities in rusoto dependencies.
//!   Will be re-enabled in a future version with modern AWS SDK. For AWS KMS integration,
//!   please use v0.3.0 or wait for the next major release with updated AWS dependencies.
//!
//! To enable specific features in your project, modify your `Cargo.toml` as follows:
//!
//! ```toml
//! [dependencies]
//! neo3 = { version = "0.4.1", features = ["futures", "ledger"] }
//! ```
//!
//! You can disable default features with:
//!
//! ```toml
//! neo3 = { version = "0.4.1", default-features = false, features = ["futures"] }
//! ```
//!
//! ## Overview
//!
//! NeoRust is a complete SDK designed to make Neo N3 blockchain development in Rust
//! intuitive, type-safe, and productive. The library provides full support for all
//! Neo N3 features and follows Rust best practices for reliability and performance.
//!
//! ## Core Modules
//!
//! NeoRust is organized into specialized modules, each handling specific aspects of Neo N3:
//!
//! - [**neo_builder**](neo_builder): Transaction construction and script building
//! - [**neo_clients**](neo_clients): Neo node interaction and RPC client implementations
//! - [**neo_codec**](neo_codec): Serialization and deserialization of Neo data structures
//! - [**neo_config**](neo_config): Configuration for networks and client settings
//! - [**neo_contract**](neo_contract): Smart contract interaction and token standards
//! - [**neo_crypto**](neo_crypto): Cryptographic primitives and operations
//! - [**neo_error**](neo_error): Unified error handling
//! - [**neo_fs**](neo_fs): NeoFS distributed storage system integration
//! - [**neo_protocol**](neo_protocol): Core blockchain protocol implementations
//! - [**neo_types**](neo_types): Core data types and primitives for Neo N3
//! - [**neo_utils**](neo_utils): General utility functions
//! - [**neo_wallets**](neo_wallets): Wallet management for Neo N3
//! - [**neo_x**](neo_x): Neo X EVM compatibility layer
//!
//! ## Quick Start
//!
//! Import all essential types and traits using the `prelude`:
//!
//! ```rust
//! use neo3::prelude::*;
//! ```
//!
//! ## Complete Example
//!
//! Here's a comprehensive example showcasing common operations with the NeoRust SDK:
//!
//! ```no_run
//! use neo3::prelude::*;
//! use neo3::neo_protocol::Account;
//! use neo3::neo_contract::{NeoToken, GasToken};
//! use neo3::neo_builder::{TransactionBuilder, ScriptBuilder};
//! use neo3::neo_clients::{HttpProvider, RpcClient};
//! use std::str::FromStr;
//!
//! async fn neo_example() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to Neo TestNet
//!     let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
//!     let client = RpcClient::new(provider);
//!     
//!     // Get basic blockchain information
//!     let block_height = client.get_block_count().await?;
//!     let best_block_hash = client.get_best_block_hash().await?;
//!     println!("Connected to Neo TestNet at height: {}", block_height);
//!     
//!     // Create a new wallet account
//!     let account = Account::create()?;
//!     println!("New account created:");
//!     println!("  Address:     {}", account.get_address());
//!     println!("  Script Hash: {}", account.get_script_hash());
//!     
//!     // Connect to system token contracts
//!     let neo_token = NeoToken::new(&client);
//!     let gas_token = GasToken::new(&client);
//!     
//!     // Query token information
//!     let neo_symbol = neo_token.symbol().await?;
//!     let neo_total_supply = neo_token.total_supply().await?;
//!     let gas_symbol = gas_token.symbol().await?;
//!     let gas_decimals = gas_token.decimals().await?;
//!     
//!     println!("{} token supply: {}", neo_symbol, neo_total_supply);
//!     println!("{} token decimals: {}", gas_symbol, gas_decimals);
//!     
//!     // Query account balances
//!     // Note: A newly created account will have zero balance
//!     // until it receives tokens from somewhere
//!     let test_address = "NUVPACTpQvd2HHmBgFjJJRWwVXJiR3uAEh";
//!     let test_account = Account::from_address(test_address)?;
//!     let script_hash = test_account.get_script_hash();
//!     
//!     let neo_balance = neo_token.balance_of(&script_hash).await?;
//!     let gas_balance = gas_token.balance_of(&script_hash).await?;
//!     
//!     println!("Account {} balances:", test_address);
//!     println!("  {}: {}", neo_symbol, neo_balance);
//!     println!("  {}: {} (÷ 10^{})", gas_symbol, gas_balance, gas_decimals);
//!     
//!     // Build a transaction to transfer GAS
//!     // (production deployment requires the account to have GAS for this to work)
//!     let recipient = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
//!     let amount = 1_0000_0000; // 1 GAS (with 8 decimals)
//!     
//!     // Create the transfer script
//!     let script = ScriptBuilder::build_contract_call(
//!         &gas_token.script_hash(),
//!         "transfer",
//!         &[
//!             ContractParameter::hash160(&script_hash),
//!             ContractParameter::hash160(&recipient),
//!             ContractParameter::integer(amount),
//!             ContractParameter::any(None),
//!         ],
//!     )?;
//!     
//!     // Build transaction
//!     let transaction = TransactionBuilder::new()
//!         .version(0)
//!         .nonce(rand::random::<u32>())
//!         .valid_until_block(block_height + 100)
//!         .script(script)
//!         .add_signer(Signer::called_by_entry(script_hash.clone()))
//!         .build();
//!     
//!     println!("Transaction built successfully:");
//!     println!("  Size: {} bytes", transaction.get_size());
//!     println!("  Hash: {}", transaction.hash());
//!     
//!     // Note: Signing and sending requires the account to have a private key
//!     // and sufficient GAS for fees. This part is for illustration only.
//!     /*
//!     // Sign transaction (requires account with private key)
//!     let signed_tx = transaction.sign(&client, &test_account).await?;
//!     
//!     // Send transaction to network
//!     let tx_id = client.send_raw_transaction(&signed_tx).await?;
//!     println!("Transaction sent: {}", tx_id);
//!     */
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Usage Examples
//!
//! ### Connecting to a Neo N3 node
//!
//! ```rust
//! use neo3::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to Neo N3 MainNet
//!     let provider = providers::HttpProvider::new("https://mainnet1.neo.org:443")?;
//!     let client = providers::RpcClient::new(provider);
//!     
//!     // Get basic blockchain information
//!     let block_count = client.get_block_count().await?;
//!     println!("Current block count: {}", block_count);
//!     
//!     let version = client.get_version().await?;
//!     println!("Node version: {}", version.user_agent);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### Creating and sending a transaction
//!
//! ```rust
//! use neo3::prelude::*;
//! use std::str::FromStr;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize the JSON-RPC provider
//!     let provider = providers::HttpProvider::new("https://testnet1.neo.org:443")?;
//!     let client = providers::RpcClient::new(provider);
//!
//!     // Create accounts for the sender and recipient
//!     let sender = protocol::Account::from_wif("YOUR_SENDER_WIF_HERE")?;
//!     let recipient = ScriptHash::from_address("NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc")?;
//!
//!     // Get the GAS token contract
//!     let gas_token_hash = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
//!     
//!     // Build the transaction using the ScriptBuilder
//!     let script = builder::ScriptBuilder::new()
//!         .contract_call(
//!             &gas_token_hash,
//!             "transfer",
//!             &[
//!                 ContractParameter::hash160(&sender.get_script_hash()),
//!                 ContractParameter::hash160(&recipient),
//!                 ContractParameter::integer(1_0000_0000), // 1 GAS (8 decimals)
//!                 ContractParameter::any(None),
//!             ],
//!             None,
//!         )?
//!         .to_bytes();
//!     
//!     // Create and configure the transaction
//!     let mut tx_builder = builder::TransactionBuilder::with_client(&client);
//!     tx_builder
//!         .script(Some(script))
//!         .set_signers(vec![sender.clone().into()])
//!         .valid_until_block(client.get_block_count().await? + 5760)?; // Valid for ~1 day
//!
//!     // Sign the transaction
//!     let tx = tx_builder.sign().await?;
//!
//!     // Send the transaction
//!     let result = tx.send_tx().await?;
//!     println!("Transaction sent: {}", result.hash);
//!
//!     // Wait for the transaction to be confirmed
//!     println!("Waiting for confirmation...");
//!     tx.track_tx(10).await?;
//!     println!("Transaction confirmed!");
//!
//!     // Get the application log
//!     let app_log = tx.get_application_log(&client).await?;
//!     println!("Application log: {:?}", app_log);
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Interacting with a smart contract
//!
//! ```rust
//! use neo3::prelude::*;
//! use std::str::FromStr;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to Neo N3 TestNet
//!     let provider = providers::HttpProvider::new("https://testnet1.neo.org:443")?;
//!     let client = providers::RpcClient::new(provider);
//!     
//!     // Define the smart contract to interact with (GAS token)
//!     let contract_hash = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
//!     let contract = neo_contract::SmartContract::new(contract_hash, Some(&client));
//!     
//!     // Call a read-only method
//!     let result = contract.call_function("symbol", vec![]).await?;
//!     let symbol = result.stack[0].as_string().unwrap_or_default();
//!     
//!     println!("Contract symbol: {}", symbol);
//!     
//!     // Get token decimals
//!     let decimals_result = contract.call_function("decimals", vec![]).await?;
//!     let decimals = decimals_result.stack[0].as_int().unwrap_or_default();
//!     println!("Token decimals: {}", decimals);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### Working with NEP-17 tokens
//!
//! ```rust
//! use neo3::prelude::*;
//! use std::str::FromStr;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to Neo N3 TestNet
//!     let provider = providers::HttpProvider::new("https://testnet1.neo.org:443")?;
//!     let client = providers::RpcClient::new(provider);
//!     
//!     // Create a reference to the GAS token contract
//!     let gas_hash = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
//!     let gas_token = neo_contract::FungibleTokenContract::new(gas_hash, Some(&client));
//!     
//!     // Get token information
//!     let symbol = gas_token.symbol().await?;
//!     let decimals = gas_token.decimals().await?;
//!     let total_supply = gas_token.total_supply().await?;
//!     
//!     println!("Token: {}", symbol);
//!     println!("Decimals: {}", decimals);
//!     println!("Total Supply: {}", total_supply);
//!     
//!     // Check an account's balance
//!     let account = protocol::Account::from_wif("YOUR_PRIVATE_KEY_WIF_HERE")?;
//!     let balance = gas_token.balance_of(&account.get_script_hash()).await?;
//!     
//!     println!("Account balance: {} {}",
//!              balance as f64 / 10f64.powi(decimals as i32),
//!              symbol);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### Using the Neo Name Service (NNS)
//!
//! ```rust
//! use neo3::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to Neo N3 TestNet
//!     let provider = providers::HttpProvider::new("https://testnet1.neo.org:443")?;
//!     let client = providers::RpcClient::new(provider);
//!     
//!     // Create a reference to the NNS contract
//!     let nns_service = neo_contract::NeoNameService::new(Some(&client));
//!     
//!     // Check domain availability
//!     let domain_name = "example.neo";
//!     let is_available = nns_service.is_available(domain_name).await?;
//!     
//!     if is_available {
//!         println!("Domain '{}' is available for registration", domain_name);
//!     } else {
//!         println!("Domain '{}' is already registered", domain_name);
//!     }
//!     
//!     Ok(())
//! }
//! ```
//!
//! For more usage examples, refer to the [`examples` directory](https://github.com/R3E-Network/NeoRust/tree/master/examples) in the repository.
//!
//! ## Project Structure
//!
//! ```
//! NeoRust
//! ├── examples
//! │   ├── neo_nodes          - Examples for connecting to Neo nodes
//! │   ├── neo_transactions   - Examples for creating and sending transactions
//! │   ├── neo_smart_contracts - Examples for interacting with smart contracts
//! │   ├── neo_wallets        - Examples for wallet management
//! │   ├── neo_nep17_tokens   - Examples for working with NEP-17 tokens
//! │   └── neo_nns            - Examples for using the Neo Name Service
//! └── src
//!     ├── neo_builder        - Transaction and script building utilities
//!     ├── neo_clients        - Neo node interaction clients (RPC and WebSocket)
//!     ├── neo_codec          - Encoding and decoding for Neo-specific data structures
//!     ├── neo_config         - Network and client configuration management
//!     ├── neo_contract       - Smart contract interaction abstractions
//!     ├── neo_crypto         - Neo-specific cryptographic operations
//!     ├── neo_protocol       - Neo network protocol implementation
//!     ├── neo_types          - Core Neo ecosystem data types
//!     └── neo_wallets        - Neo asset and account management
//! ```
//!
//! ## Module Overview
//!
//! - **neo_builder**: Transaction and script building utilities.
//!   - Transaction construction and signing
//!   - Script building for contract calls
//!   - Network fee calculation
//!
//! - **neo_clients**: Neo node interaction clients.
//!   - HTTP, WebSocket, and IPC providers
//!   - JSON-RPC client implementation
//!   - Event subscription and notification handling
//!
//! - **neo_codec**: Encoding and decoding for Neo-specific data structures.
//!   - Binary serialization and deserialization
//!   - Neo VM script encoding
//!
//! - **neo_config**: Network and client configuration management.
//!   - Network magic numbers
//!   - Client settings
//!
//! - **neo_contract**: Smart contract interaction abstractions.
//!   - Contract invocation and deployment
//!   - NEP-17 token standard implementation
//!   - Native contracts (GAS, NEO, etc.)
//!   - Neo Name Service (NNS) support
//!
//! - **neo_crypto**: Neo-specific cryptographic operations.
//!   - Key generation and management
//!   - Signing and verification
//!   - Hashing functions
//!
//! - **neo_protocol**: Neo network protocol implementation.
//!   - Account management
//!   - Address formats and conversions
//!
//! - **neo_types**: Core Neo ecosystem data types.
//!   - Script hashes
//!   - Contract parameters
//!   - Block and transaction types
//!   - NNS name types
//!
//! - **neo_wallets**: Neo asset and account management.
//!   - Wallet creation and management
//!   - NEP-6 wallet standard support
//!   - Account import/export
//!   - Wallet backup and recovery
//!
//! For detailed information, consult the documentation of each module.

#![warn(missing_debug_implementations, missing_docs, rust_2018_idioms, unreachable_pub)]
// Only enable doc_cfg feature on nightly builds or when explicitly building docs
#![cfg_attr(all(feature = "nightly", docsrs), feature(doc_cfg))]
#![doc(test(no_crate_inject, attr(deny(rust_2018_idioms), allow(dead_code, unused_variables))))]

// For macro expansions only, not public API.
#[doc(hidden)]
#[allow(unused_extern_crates)]
extern crate self as neo3;

// Core modules - always available
pub mod neo_error;
pub mod neo_types;
pub mod neo_utils;

// All modules unconditionally available
pub mod neo_builder;
pub mod neo_clients;
pub mod neo_codec;
pub mod neo_config;
pub mod neo_contract;
pub mod neo_crypto;
pub mod neo_fs;
pub mod neo_protocol;
pub mod neo_wallets;
pub mod neo_x;

// Re-exports for convenience
#[doc(inline)]
pub use neo_builder as builder;
#[doc(inline)]
pub use neo_clients as providers;
#[doc(inline)]
pub use neo_codec as codec;
#[doc(inline)]
pub use neo_config as config;
#[doc(inline)]
pub use neo_crypto as crypto;
#[doc(inline)]
pub use neo_protocol as protocol;
#[doc(inline)]
pub use neo_wallets as wallets;
#[doc(inline)]
pub use neo_x as x;
// No need to re-export specialized modules as they're already public with their full names

// Re-export common types directly in lib.rs for easy access
pub use crate::neo_types::{
	deserialize_address_or_script_hash,
	deserialize_h256,
	deserialize_h256_option,
	deserialize_hash_map_h160_account,
	deserialize_script_hash,
	deserialize_script_hash_option,
	deserialize_url_option,
	serialize_address_or_script_hash,
	serialize_h256,
	serialize_h256_option,
	serialize_hash_map_h160_account,
	// Serialization/deserialization helpers
	serialize_script_hash,
	serialize_script_hash_option,
	serialize_url_option,
	var_size,
	vec_to_array32,
	Address,
	AddressOrScriptHash,
	// Additional types
	Base64Encode,
	Bytes,
	ContractIdentifiers,
	// Contract types
	ContractManifest,
	ContractParameter,
	ContractParameterType,
	ContractState,
	InvocationResult,
	// NNS types
	NNSName,
	NefFile,
	OpCode,
	OperandSize,
	ParameterValue,
	ScriptHash,
	ScriptHashExtension,
	// Additional types
	ScryptParamsDef,
	StackItem,
	StringExt,
	TypeError,
	VMState,
};

// Add direct re-exports for commonly used serde utils
pub use crate::neo_types::serde_with_utils::{
	deserialize_boolean_expression, deserialize_bytes, deserialize_h160, deserialize_hardforks,
	deserialize_hashmap_address_u256, deserialize_hashmap_u256_hashset_h256,
	deserialize_hashmap_u256_hashset_u256, deserialize_hashmap_u256_vec_u256,
	deserialize_hashset_u256, deserialize_map, deserialize_private_key, deserialize_public_key,
	deserialize_public_key_option, deserialize_scopes, deserialize_vec_script_hash,
	deserialize_vec_script_hash_option, deserialize_wildcard, serialize_boolean_expression,
	serialize_bytes, serialize_h160, serialize_hashmap_address_u256,
	serialize_hashmap_u256_hashset_h256, serialize_hashmap_u256_hashset_u256,
	serialize_hashmap_u256_vec_u256, serialize_hashset_u256, serialize_map, serialize_private_key,
	serialize_public_key, serialize_public_key_option, serialize_scopes, serialize_vec_script_hash,
	serialize_vec_script_hash_option, serialize_wildcard,
};

// Re-export additional contract types
pub use crate::neo_types::contract::{
	ContractMethodToken, ContractNef, NativeContractState, NeoVMStateType,
};

// Re-export value extension trait
pub use crate::neo_types::serde_value::ValueExtension;

/// Convenient imports for commonly used types and traits.
///
/// This prelude module provides a single import to access the most commonly used
/// components of the NeoRust SDK. Import it with:
///
/// ```rust
/// use neo3::prelude::*;
/// ```
pub mod prelude;

#[cfg(all(test))]
mod tests {
	use super::prelude::*;
	use primitive_types::H160;
	use std::str::FromStr;

	use tokio;

	use crate::{
		builder::{AccountSigner, ScriptBuilder, TransactionBuilder},
		neo_clients::{APITrait, HttpProvider, RpcClient},
		neo_protocol::{Account, AccountTrait},
	};
	use url::Url;

	#[cfg(all(test))]
	#[tokio::test]
	#[ignore] // Ignoring this test as it requires a live Neo N3 node and real tokens
	async fn test_create_and_send_transaction() -> Result<(), Box<dyn std::error::Error>> {
		// Initialize the JSON-RPC provider - using TestNet for safer testing
		let http_provider = HttpProvider::new("https://testnet1.neo.org:443")?;
		let rpc_client = RpcClient::new(http_provider);

		// Create accounts for the sender and recipient
		let sender = Account::from_wif("L1WMhxazScMhUrdv34JqQb1HFSQmWeN2Kpc1R9JGKwL7CDNP21uR")?;
		let recipient = Account::from_address("NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc")?;

		// Use the correct GAS token hash for Neo N3 TestNet
		let gas_token_hash = "d2a4cff31913016155e38e474a2c06d08be276cf"; // GAS token on Neo N3

		// Create a new TransactionBuilder
		let mut tx_builder = TransactionBuilder::with_client(&rpc_client);

		// Build the transaction
		tx_builder
			.set_script(Some(
				ScriptBuilder::new()
					.contract_call(
						&H160::from_str(gas_token_hash)?,
						"transfer",
						&[
							ContractParameter::h160(&sender.get_script_hash()),
							ContractParameter::h160(&recipient.get_script_hash()),
							ContractParameter::integer(1_0000_0000), // 1 GAS (8 decimals)
							ContractParameter::any(),
						],
						None,
					)
					.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?
					.to_bytes(),
			))
			.set_signers(vec![AccountSigner::called_by_entry(&sender)?.into()])
			.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?
			.valid_until_block(rpc_client.get_block_count().await? + 5760)?; // Valid for ~1 day

		// Sign the transaction
		let mut signed_tx = tx_builder.sign().await?;

		// For testing purposes, we'll just verify that we can create and sign the transaction
		// without actually sending it to the network
		println!("Transaction created and signed successfully");
		println!("Transaction size: {} bytes", signed_tx.size());
		println!("System fee: {} GAS", signed_tx.sys_fee as f64 / 100_000_000.0);
		println!("Network fee: {} GAS", signed_tx.net_fee as f64 / 100_000_000.0);

		Ok(())
	}
}

// Adding trait implementations for serde JSON serialization
// These extensions will be used by the http-client feature
pub mod extensions {
	use serde_json::{Result as JsonResult, Value};

	pub trait ToValue {
		fn to_value(&self) -> Value;
	}

	impl ToValue for String {
		fn to_value(&self) -> Value {
			serde_json::Value::String(self.clone())
		}
	}

	impl ToValue for &str {
		fn to_value(&self) -> Value {
			serde_json::Value::String((*self).to_string())
		}
	}

	impl ToValue for u32 {
		fn to_value(&self) -> Value {
			serde_json::Value::Number(serde_json::Number::from(*self))
		}
	}

	impl ToValue for i32 {
		fn to_value(&self) -> Value {
			serde_json::Value::Number(serde_json::Number::from(*self))
		}
	}

	impl ToValue for bool {
		fn to_value(&self) -> Value {
			serde_json::Value::Bool(*self)
		}
	}
}

// Explicitly mark external dependencies with cfg_attr for docs.rs
#[cfg(feature = "futures")]
#[cfg_attr(docsrs, doc(cfg(feature = "futures")))]
pub use futures;

#[cfg(feature = "ledger")]
#[cfg_attr(docsrs, doc(cfg(feature = "ledger")))]
pub use coins_ledger;

// AWS feature is disabled in v0.4.1 due to security vulnerabilities
// #[cfg(feature = "aws")]
// #[cfg_attr(docsrs, doc(cfg(feature = "aws")))]
// pub use rusoto_core;
//
// #[cfg(feature = "aws")]
// #[cfg_attr(docsrs, doc(cfg(feature = "aws")))]
// pub use rusoto_kms;
