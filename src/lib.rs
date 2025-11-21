#![allow(
	clippy::result_large_err,
	clippy::too_many_arguments,
	clippy::wrong_self_convention,
	clippy::module_inception,
	clippy::type_complexity
)]
//! ![Neo Logo](https://neo.org/images/neo-logo/NEO-logo.svg)
//! # NeoRust SDK v0.5.1
//!
//! A production-ready Rust SDK for the Neo N3 blockchain with enterprise-grade features.
//!
//! [![Crates.io](https://img.shields.io/crates/v/neo3.svg)](https://crates.io/crates/neo3)
//! [![Documentation](https://docs.rs/neo3/badge.svg)](https://docs.rs/neo3)
//! [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
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
//! - **sgx**: Enables Intel SGX (Software Guard Extensions) support for running Neo operations
//!   in secure enclaves. This feature enables `no_std` compilation and provides hardware-based
//!   security for sensitive operations like key management and transaction signing.
//!
//! - **no_std**: Enables `no_std` compilation for embedded systems and SGX environments.
//!   This feature removes dependencies on the standard library, allowing the SDK to run in
//!   constrained environments with custom allocators.
//!
//! To enable specific features in your project, modify your `Cargo.toml` as follows:
//!
//! ```toml
//! [dependencies]
//! neo3 = { version = "0.5.1", features = ["futures", "ledger"] }
//! ```
//!
//! You can disable default features with:
//!
//! ```toml
//! neo3 = { version = "0.5.1", default-features = false, features = ["futures"] }
//! ```
//!
//! ## Overview
//!
//! NeoRust is a complete SDK designed to make Neo N3 blockchain development in Rust
//! intuitive, type-safe, and productive. The library provides full support for all
//! Neo N3 features and follows Rust best practices for reliability and performance.
//!
//! ### New in v0.5.x
//! - **WebSocket Support**: Real-time blockchain events with automatic reconnection
//! - **HD Wallets (BIP-39/44)**: Deterministic wallet generation and derivation
//! - **Transaction Simulation**: Preview fees, VM state, and state changes before sending
//! - **High-Level SDK API**: Simplified entrypoint (`Neo`) for common operations
//! - **Enhanced Error Handling**: Consistent `NeoError` with recovery suggestions
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
//! use neo3::neo_protocol::{Account, AccountTrait};
//! use neo3::neo_clients::{HttpProvider, RpcClient, APITrait};
//!
//! async fn neo_example() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to Neo TestNet
//!     let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
//!     let client = RpcClient::new(provider);
//!     
//!     // Get basic blockchain information
//!     let block_height = client.get_block_count().await?;
//!     println!("Connected to Neo TestNet at height: {}", block_height);
//!     
//!     // Create a new wallet account
//!     let account = Account::create()?;
//!     println!("New account created:");
//!     println!("  Address:     {}", account.get_address());
//!     println!("  Script Hash: {}", account.get_script_hash());
//!     
//!     // Get version information
//!     let version = client.get_version().await?;
//!     println!("Node version: {}", version.user_agent);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Usage Examples
//!
//! ### Connecting to a Neo N3 node
//!
//! ```no_run
//! use neo3::neo_clients::{HttpProvider, RpcClient, APITrait};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to Neo N3 MainNet
//!     let provider = HttpProvider::new("https://mainnet1.neo.org:443")?;
//!     let client = RpcClient::new(provider);
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
//! ```no_run
//! use neo3::neo_clients::{HttpProvider, RpcClient, APITrait};
//! use neo3::neo_protocol::{Account, AccountTrait};
//! use neo3::neo_types::{ScriptHash, ContractParameter, ScriptHashExtension};
//! use neo3::neo_builder::{ScriptBuilder, TransactionBuilder, AccountSigner};
//! use std::str::FromStr;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize the JSON-RPC provider
//!     let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
//!     let client = RpcClient::new(provider);
//!
//!     // Create accounts for the sender and recipient
//!     // Using TestNet test account - replace with your own WIF for actual use
//!     let sender = Account::from_wif("L1eV34wPoj9weqhGijdDLtVQzUpWGHszXXpdU9dPuh2nRFFzFa7E")?;
//!     let recipient = ScriptHash::from_address("NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc")?;
//!
//!     // Get the GAS token contract
//!     let gas_token_hash = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
//!     
//!     // Build the transaction using the ScriptBuilder
//!     let script = ScriptBuilder::new()
//!         .contract_call(
//!             &gas_token_hash,
//!             "transfer",
//!             &[
//!                 ContractParameter::h160(&sender.get_script_hash()),
//!                 ContractParameter::h160(&recipient),
//!                 ContractParameter::integer(1_0000_0000), // 1 GAS (8 decimals)
//!                 ContractParameter::any(),
//!             ],
//!             None,
//!         )?
//!         .to_bytes();
//!     
//!     // Create and configure the transaction
//!     let mut tx_builder = TransactionBuilder::with_client(&client);
//!     tx_builder
//!         .set_script(Some(script))
//!         .set_signers(vec![AccountSigner::called_by_entry(&sender)?.into()])?
//!         .valid_until_block(client.get_block_count().await? + 5760)?; // Valid for ~1 day
//!
//!     // Sign the transaction
//!     let mut tx = tx_builder.sign().await?;
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
//! ```no_run
//! use neo3::neo_clients::{HttpProvider, RpcClient, APITrait};
//! use neo3::neo_types::{ScriptHash, ContractParameter, StackItem};
//! use std::str::FromStr;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to Neo N3 TestNet
//!     let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
//!     let client = RpcClient::new(provider);
//!     
//!     // Get the NEO token contract
//!     let neo_token = ScriptHash::from_str("ef4073a0f2b305a38ec4050e4d3d28bc40ea63f5")?;
//!     
//!     // Call a read-only method (doesn't require signing)
//!     let result = client.invoke_function(
//!         &neo_token,
//!         "symbol",
//!         &[],
//!         vec![]
//!     ).await?;
//!     
//!     // Parse the result
//!     if let Some(stack) = result.stack {
//!         if let Some(item) = stack.first() {
//!             println!("NEO Token Symbol: {:?}", item);
//!         }
//!     }
//!     
//!     // Get the total supply
//!     let supply_result = client.invoke_function(
//!         &neo_token,
//!         "totalSupply",
//!         &[],
//!         vec![]
//!     ).await?;
//!     
//!     println!("Total Supply Result: {:?}", supply_result);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### Working with NEP-17 tokens
//!
//! ```no_run
//! use neo3::neo_clients::{HttpProvider, RpcClient, APITrait};
//! use neo3::neo_protocol::{Account, AccountTrait};
//! use neo3::neo_types::{ScriptHash, ContractParameter};
//! use std::str::FromStr;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to Neo N3 TestNet
//!     let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
//!     let client = RpcClient::new(provider);
//!     
//!     // Create an account from WIF (Wallet Import Format)
//!     // This is a TestNet test account - replace with your own WIF for actual use
//!     let account = Account::from_wif("L1eV34wPoj9weqhGijdDLtVQzUpWGHszXXpdU9dPuh2nRFFzFa7E")?;
//!     
//!     // Get account information
//!     println!("Account address: {}", account.get_address());
//!     println!("Account script hash: {}", account.get_script_hash());
//!     
//!     // Get GAS token balance for the account
//!     let gas_token = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
//!     
//!     let balance_result = client.invoke_function(
//!         &gas_token,
//!         "balanceOf",
//!         &[ContractParameter::h160(&account.get_script_hash())],
//!         vec![]
//!     ).await?;
//!     
//!     // Parse the balance result
//!     if let Some(stack) = balance_result.stack {
//!         if let Some(item) = stack.first() {
//!             println!("GAS Balance: {:?}", item);
//!         }
//!     }
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### Using the Neo Name Service (NNS)
//!
//! ```no_run
//! use neo3::neo_clients::{HttpProvider, RpcClient, APITrait};
//! use neo3::neo_types::{ScriptHash, ContractParameter, NNSName};
//! use std::str::FromStr;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to Neo N3 TestNet
//!     let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
//!     let client = RpcClient::new(provider);
//!     
//!     // NNS contract on TestNet (use MainNet hash for production)
//!     let nns_contract = ScriptHash::from_str("50ac1c37690cc2cfc594472833cf57505d5f46de")?;
//!     
//!     // Check if a name is available
//!     let available_result = client.invoke_function(
//!         &nns_contract,
//!         "isAvailable",
//!         &[ContractParameter::string("myname.neo")],
//!         vec![]
//!     ).await?;
//!     
//!     if let Some(stack) = available_result.stack {
//!         if let Some(item) = stack.first() {
//!             println!("Is 'myname.neo' available: {:?}", item);
//!         }
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
//! ```text
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
//!     ├── neo_sgx            - SGX support for secure enclave execution
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

// Production-ready Neo N3 SDK - warnings are treated as errors in CI
#![cfg_attr(feature = "sgx", no_std)]
#![cfg_attr(feature = "sgx", feature(rustc_private))]
#![allow(elided_lifetimes_in_paths, missing_docs, missing_debug_implementations)]
#![warn(unreachable_pub)]
#![doc(test(no_crate_inject, attr(deny(rust_2018_idioms), allow(dead_code, unused_variables))))]

// SGX support
#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;

// Required for no_std
#[cfg(feature = "no_std")]
extern crate alloc;

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
#[cfg(any(feature = "sgx", feature = "no_std"))]
pub mod neo_sgx;
pub mod neo_wallets;
pub mod neo_x;

// High-level SDK API (new in v0.5.x)
pub mod sdk;

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

#[cfg(test)]
mod tests {
	use super::prelude::*;
	use primitive_types::H160;
	use std::str::FromStr;

	use crate::{
		builder::{AccountSigner, ScriptBuilder, TransactionBuilder},
		neo_clients::{APITrait, HttpProvider, RpcClient},
		neo_protocol::{Account, AccountTrait},
	};

	#[cfg(test)]
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
		let signed_tx = tx_builder.sign().await?;

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
	use serde_json::Value;

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
pub use futures;

#[cfg(feature = "ledger")]
pub use coins_ledger;

// AWS feature is disabled in v0.4.1 due to security vulnerabilities
// #[cfg(feature = "aws")]
// #[cfg_attr(docsrs, doc(cfg(feature = "aws")))]
// pub use rusoto_core;
//
// #[cfg(feature = "aws")]
// #[cfg_attr(docsrs, doc(cfg(feature = "aws")))]
// pub use rusoto_kms;
