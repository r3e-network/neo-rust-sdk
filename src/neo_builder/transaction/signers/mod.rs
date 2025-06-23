//! This module contains implementations for different types of signers in the NEO blockchain.
//!
//! It includes:
//! - `AccountSigner`: Represents an account-based signer.
//! - `ContractSigner`: Represents a contract-based signer.
//! - `TransactionSigner`: Represents a transaction-specific signer.
//! - `Signer`: An enum that can be any of the above signer types.
//!
//! This module also provides traits and utilities for working with signers,
//! including serialization, deserialization, and common signer operations.
//!
//! # Usage
//!
//! To use the signers in your NEO blockchain transactions:
//!
//! 1. Import the necessary types:
//!    ```rust
//!    use neo3::neo_builder::{AccountSigner, ContractSigner, TransactionSigner, Signer};
//!    ```
//!
//! 2. Create a signer based on your needs:
//!    ```rust
//!    use neo3::neo_protocol::{Account, AccountTrait};
//!    use neo3::neo_builder::{AccountSigner, ContractSigner, TransactionSigner, WitnessScope};
//!    use neo3::prelude::H160;
//!    use std::str::FromStr;
//!
//!    // For an account-based signer
//!    let account = Account::from_wif("KxDgvEKzgSBPPfuVfw67oPQBSjidEiqTHURKSDL1R7yGaGYAeYnr").unwrap();
//!    let account_signer = AccountSigner::called_by_entry(&account).unwrap();
//!
//!    // For a contract-based signer
//!    let contract_hash = H160::from_str("0xef4073a0f2b305a38ec4050e4d3d28bc40ea63f5").unwrap();
//!    let contract_signer = ContractSigner::called_by_entry(contract_hash, &[]);
//!
//!    // For a transaction-specific signer
//!    let transaction_signer = TransactionSigner::new(account.get_script_hash(), vec![WitnessScope::CalledByEntry]);
//!    ```
//!
//! 3. Use the signer in your transaction:
//!    ```rust
//!    # use neo3::neo_builder::{TransactionBuilder, AccountSigner};
//!    # use neo3::neo_protocol::{Account, AccountTrait};
//!    # let account = Account::from_wif("KxDgvEKzgSBPPfuVfw67oPQBSjidEiqTHURKSDL1R7yGaGYAeYnr").unwrap();
//!    # let account_signer = AccountSigner::called_by_entry(&account).unwrap();
//!    # use neo3::neo_clients::HttpProvider;
//!    let mut tx_builder: TransactionBuilder<'_, HttpProvider> = TransactionBuilder::default();
//!    tx_builder.set_signers(vec![account_signer.into()]);
//!    // ... add other transaction details ...
//!    # async {
//!    let tx = tx_builder.build().await.unwrap();
//!    # };
//!    ```
//!
//! 4. You can also convert between signer types using the `Signer` enum:
//!    ```rust
//!    # use neo3::neo_builder::{Signer, AccountSigner};
//!    # use neo3::neo_protocol::{Account, AccountTrait};
//!    # let account = Account::from_wif("KxDgvEKzgSBPPfuVfw67oPQBSjidEiqTHURKSDL1R7yGaGYAeYnr").unwrap();
//!    # let account_signer = AccountSigner::called_by_entry(&account).unwrap();
//!    let generic_signer: Signer = account_signer.into();
//!    ```
//!
//! Remember to handle errors and manage scopes, allowed contracts, and other signer properties as needed for your specific use case.

pub use account_signer::*;
pub use contract_signer::*;
pub use signer::*;
pub use transaction_signer::*;

mod account_signer;
mod contract_signer;
mod signer;
mod transaction_signer;
