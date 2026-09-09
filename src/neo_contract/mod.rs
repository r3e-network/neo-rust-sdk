//! # Neo Contract Module
//!
//! Comprehensive interfaces for interacting with Neo N3 smart contracts and tokens.
//!
//! ## Overview
//!
//! The neo_contract module provides a robust set of interfaces for interacting with
//! various types of smart contracts on the Neo N3 blockchain. This module abstracts
//! away the complexity of contract calls and state management, providing easy-to-use
//! APIs for developers.
//!
//! ## Key Features
//!
//! - **System Contracts**: Built-in interfaces for Neo N3 system contracts:
//!   - NEO Token contract
//!   - GAS Token contract
//!   - Policy contract
//!   - RoleManagement contract
//!   - ContractManagement contract
//!
//! - **Token Standards**:
//!   - NEP-17 fungible token standard (similar to Ethereum's ERC-20)
//!   - NEP-11 non-fungible token standard (similar to Ethereum's ERC-721)
//!
//! - **Advanced Contract Interactions**:
//!   - Neo Name Service (NNS) domain resolution
//!   - Neo URI parsing and validation
//!   - Contract iterator support for handling large result sets
//!
//! - **Famous Contract Integrations**:
//!   - Flamingo Finance DeFi ecosystem
//!   - NeoburgerNeo (bNEO) staking contract
//!   - GrandShare voting and proposals
//!   - NeoCompound yield aggregator
//!
//! - **Developer Tools**:
//!   - Contract deployment helpers
//!   - ABI and manifest handling utilities
//!   - Contract invocation result parsing
//!
//! ## Examples
//!
//! ### Working with Standard Contracts
//!
//! ```no_run
//! use neo3::neo_contract::{GasToken, PolicyContract, FungibleTokenContract, FungibleTokenTrait};
//! use neo3::neo_protocol::{Account, AccountTrait};
//! use neo3::neo_clients::{HttpProvider, RpcClient};
//! use neo3::neo_types::ScriptHash;
//! use std::str::FromStr;
//!
//! async fn contract_examples() -> Result<(), Box<dyn std::error::Error>> {
//!     // Set up a client connection
//!     let provider = HttpProvider::new("https://testnet1.neo.org:443").unwrap();
//!     let client = RpcClient::new(provider);
//!
//!     // Create accounts for testing
//!     let sender = Account::create()?;
//!
//!     // 1. Working with the GAS token contract
//!     let gas_token = GasToken::new(Some(&client));
//!     let gas_balance = gas_token.get_balance_of(&sender.get_script_hash()).await?;
//!     println!("GAS Balance: {}", gas_balance);
//!
//!     // 2. Working with the Policy contract
//!     let policy = PolicyContract::new(Some(&client));
//!     let exec_fee_factor = policy.get_exec_fee_factor().await?;
//!     let storage_price = policy.get_storage_price().await?;
//!     println!("Execution Fee Factor: {}", exec_fee_factor);
//!     println!("Storage Price: {}", storage_price);
//!
//!     // 3. Working with a custom NEP-17 token
//!     let token_hash = ScriptHash::from_str("0xd2a4cff31913016155e38e474a2c06d08be276cf")?;
//!     let custom_token = FungibleTokenContract::new(&token_hash, Some(&client));
//!     let custom_balance = custom_token.get_balance_of(&sender.get_script_hash()).await?;
//!     println!("Token Balance: {}", custom_balance);
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Deploying a Smart Contract
//!
//! ```no_run
//! use neo3::neo_contract::ContractManagement;
//! use neo3::neo_types::NefFile;
//! use neo3::neo_clients::{HttpProvider, RpcClient};
//! use std::fs;
//!
//! async fn deploy_contract_example() -> Result<(), Box<dyn std::error::Error>> {
//!     // Set up a client connection
//!     let provider = HttpProvider::new("https://testnet1.neo.org:443").unwrap();
//!     let client = RpcClient::new(provider);
//!
//!     // Load contract files (NEF and manifest)
//!     let nef_bytes = fs::read("path/to/contract.nef")?;
//!     let manifest_bytes = fs::read("path/to/contract.manifest.json")?;
//!
//!     // Parse the NEF file
//!     let nef = NefFile::deserialize(&nef_bytes)?;
//!
//!     // Create a native ContractManagement instance
//!     let contract_mgmt = ContractManagement::new(Some(&client));
//!
//!     // Deploy the contract
//!     println!("Deploying contract...");
//!     let tx_builder = contract_mgmt.deploy(
//!         &nef,
//!         &manifest_bytes,
//!         None, // No deployment data
//!     ).await?;
//!
//!     println!("Contract deployment transaction built successfully!");
//!
//!     Ok(())
//! }
//! ```

pub use account_events::{AccountEventQuery, AccountEventResult};
pub use contract_error::*;
pub use contract_management::*;
pub use events::*;
pub use famous::*;
pub use fungible_token_contract::*;
pub use gasless::*;
pub use gas_token::*;
pub use iterator::*;
pub use name_service::*;
pub use neo_token::*;
pub use neo_uri::*;
pub use nft_contract::*;
pub use notary::*;
pub use policy_contract::*;
pub use role_management::*;
pub use traits::*;
pub use treasury::*;

mod account_events;
mod contract_error;
mod contract_management;
mod events;
mod famous;
mod fungible_token_contract;
pub mod gasless;
mod gas_token;
mod iterator;
mod name_service;
mod neo_token;
mod neo_uri;
mod nft_contract;
mod notary;
mod policy_contract;
mod role_management;
mod traits;
mod treasury;

pub(crate) fn checked_vm_integer<T>(value: i64, field: &str) -> Result<T, ContractError>
where
	T: TryFrom<i64>,
{
	T::try_from(value).map_err(|_| {
		ContractError::InvalidResponse(format!(
			"{field} value {value} is outside the supported range"
		))
	})
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod checked_integer_tests {
	use super::*;

	#[test]
	fn checked_vm_integer_rejects_negative_and_oversized_values() {
		assert!(matches!(
			checked_vm_integer::<u32>(-1, "block height"),
			Err(ContractError::InvalidResponse(message)) if message.contains("block height")
		));
		assert!(matches!(
			checked_vm_integer::<u8>(256, "decimals"),
			Err(ContractError::InvalidResponse(message)) if message.contains("decimals")
		));
		assert_eq!(checked_vm_integer::<u32>(42, "block height").unwrap(), 42);
	}
}
