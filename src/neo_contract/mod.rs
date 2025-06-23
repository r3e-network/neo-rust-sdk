#![feature(const_trait_impl)]

//! # Neo Contract Module (v0.1.8)
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
//! use neo3::neo_contract::{NeoToken, GasToken, PolicyContract};
//! use neo3::neo_protocol::Account;
//! use neo3::neo_clients::{HttpProvider, RpcClient};
//! use std::str::FromStr;
//!
//! async fn contract_examples() -> Result<(), Box<dyn std::error::Error>> {
//!     // Set up a client connection
//!     let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
//!     let client = RpcClient::new(provider);
//!     
//!     // Create accounts for testing
//!     let sender = Account::create()?;
//!     let recipient = Account::from_address("NUVPACTpQvd2HHmBgFjJJRWwVXJiR3uAEh")?;
//!     
//!     // 1. Working with the NEO token contract
//!     let neo_token = NeoToken::new(&client);
//!     
//!     // Get token information
//!     let symbol = neo_token.symbol().await?;
//!     let decimals = neo_token.decimals().await?;
//!     let total_supply = neo_token.total_supply().await?;
//!     
//!     println!("NEO Token: Symbol={}, Decimals={}, Total Supply={}",
//!              symbol, decimals, total_supply);
//!     
//!     // Check account balance
//!     let balance = neo_token.balance_of(&sender.get_script_hash()).await?;
//!     println!("NEO Balance: {}", balance);
//!     
//!     // Get voting data
//!     let candidates = neo_token.get_all_candidates().await?;
//!     println!("Current NEO committee candidates: {}", candidates.len());
//!     
//!     // 2. Working with the GAS token contract
//!     let gas_token = GasToken::new(&client);
//!     
//!     // Transfer GAS (requires sender to have GAS)
//!     if gas_token.balance_of(&sender.get_script_hash()).await? > 0 {
//!         let tx_hash = gas_token.transfer(
//!             &sender,
//!             &recipient.get_script_hash(),
//!             1_00000000, // 1 GAS
//!             None,       // No data
//!         ).await?;
//!         
//!         println!("GAS transfer transaction: {}", tx_hash);
//!     }
//!     
//!     // 3. Working with the Policy contract
//!     let policy = PolicyContract::new(&client);
//!     
//!     // Get network policies
//!     let exec_fee_factor = policy.get_exec_fee_factor().await?;
//!     let storage_price = policy.get_storage_price().await?;
//!     
//!     println!("Network Policies:");
//!     println!("  Execution Fee Factor: {}", exec_fee_factor);
//!     println!("  Storage Price: {}", storage_price);
//!     
//!     // 4. Working with a custom NEP-17 token
//!     let token_hash = ScriptHash::from_str("0xd2a4cff31913016155e38e474a2c06d08be276cf")?;
//!     let custom_token = FungibleTokenContract::new(token_hash, &client);
//!     
//!     let custom_symbol = custom_token.symbol().await?;
//!     let custom_balance = custom_token.balance_of(&sender.get_script_hash()).await?;
//!     
//!     println!("{} Balance: {}", custom_symbol, custom_balance);
//!     
//!     // 5. Using the Neo Name Service
//!     let nns = NameService::new(&client);
//!     
//!     // Resolve a domain name to a script hash
//!     let domain = "example.neo";
//!     if let Ok(resolved_address) = nns.resolve(domain).await {
//!         println!("Domain {} resolves to: {}", domain, resolved_address);
//!     } else {
//!         println!("Domain {} is not registered", domain);
//!     }
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### Deploying a Smart Contract
//!
//! ```no_run
//! use neo3::neo_contract::{ContractManagement, ContractState};
//! use neo3::neo_protocol::Account;
//! use neo3::neo_types::{ContractManifest, NefFile};
//! use neo3::neo_clients::{HttpProvider, RpcClient};
//! use std::fs;
//!
//! async fn deploy_contract_example() -> Result<(), Box<dyn std::error::Error>> {
//!     // Set up a client connection
//!     let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
//!     let client = RpcClient::new(provider);
//!     
//!     // Create or load an account with GAS for deployment
//!     let account = Account::from_wif("KwVEKk78X65fDrJ3VgqHLcpPpbQVfJLjXrkFUCozHQBJ5nT2xwP8")?;
//!     
//!     // Load contract files (NEF and manifest)
//!     let nef_bytes = fs::read("path/to/contract.nef")?;
//!     let manifest_json = fs::read_to_string("path/to/contract.manifest.json")?;
//!     
//!     // Parse contract files
//!     let nef = NefFile::from_bytes(&nef_bytes)?;
//!     let manifest = ContractManifest::from_json(&manifest_json)?;
//!     
//!     // Create contract management instance
//!     let contract_mgmt = ContractManagement::new(&client);
//!     
//!     // Deploy the contract
//!     println!("Deploying contract...");
//!     let result = contract_mgmt.deploy(
//!         &nef,
//!         &manifest,
//!         None, // No deployment data
//!         &account,
//!     ).await?;
//!     
//!     // Get the contract hash
//!     let contract_hash = result.script_hash;
//!     println!("Contract deployed successfully!");
//!     println!("Contract hash: {}", contract_hash);
//!     
//!     // Get detailed contract information
//!     let contract_state: ContractState = contract_mgmt.get_contract(&contract_hash).await?;
//!     println!("Contract ID: {}", contract_state.id);
//!     println!("Contract update counter: {}", contract_state.update_counter);
//!     
//!     Ok(())
//! }
//! ```

pub use contract_error::*;
pub use contract_management::*;
pub use famous::*;
pub use fungible_token_contract::*;
pub use gas_token::*;
pub use iterator::*;
pub use name_service::*;
pub use neo_token::*;
pub use neo_uri::*;
pub use nft_contract::*;
pub use policy_contract::*;
pub use role_management::*;
pub use traits::*;

mod contract_error;
mod contract_management;
mod famous;
mod fungible_token_contract;
mod gas_token;
mod iterator;
mod name_service;
mod neo_token;
mod neo_uri;
mod nft_contract;
mod policy_contract;
mod role_management;
mod traits;

#[cfg(test)]
mod tests;
