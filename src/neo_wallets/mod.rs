#![deny(unsafe_code)]

//! # Neo Wallets
//!
//! Wallet management for the Neo N3 blockchain.
//!
//! ## Overview
//!
//! The neo_wallets module provides comprehensive wallet management functionality for the Neo N3 blockchain.
//! It includes:
//!
//! - Wallet creation and loading
//! - NEP-6 wallet standard support
//! - BIP-39 mnemonic phrase support
//! - Transaction signing
//! - Key management and derivation
//! - Hardware wallet integration (Ledger)
//! - Secure key storage
//! - Wallet backup and recovery
//!
//! This module enables secure management of private keys and accounts, allowing users to interact
//! with the Neo N3 blockchain in a secure manner.
//!
//! ## Examples
//!
//! ### Creating and using a wallet
//!
//! ```rust
//! use neo3::neo_wallets::Wallet;
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a new wallet
//!     let mut wallet = Wallet::new();
//!     
//!     // Create a new account in the wallet
//!     let account = wallet.create_new_account()?;
//!     println!("New account address: {}", account.get_address());
//!     
//!     // Save the wallet to a file
//!     wallet.save_to_file(PathBuf::from("my_wallet.json"))?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### Working with BIP-39 accounts
//!
//! ```no_run
//! use neo3::neo_wallets::Bip39Account;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a new BIP-39 account
//!     let account = Bip39Account::create("password123")?;
//!     println!("Generated mnemonic: {}", account.mnemonic());
//!     
//!     // Recover an account from a mnemonic
//!     let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
//!     let recovered = Bip39Account::from_bip39_mnemonic("password123", mnemonic)?;
//!     
//!     Ok(())
//! }
//! ```

#[cfg(feature = "ledger")]
pub use ledger::{HDPath, LedgerWallet};
#[cfg(feature = "yubi")]
use p256::NistP256;
#[cfg(all(feature = "yubihsm", not(target_arch = "wasm32")))]
pub use yubihsm;

use crate::neo_protocol::Account;
pub use bip39_account::*;
pub use error::*;
pub use wallet::*;
pub use wallet_signer::WalletSigner;
pub use wallet_trait::WalletTrait;

#[cfg(feature = "ledger")]
mod ledger;
mod wallet;
mod wallet_trait;

/// A wallet instantiated with a locally stored private key
pub type LocalWallet = WalletSigner<Account>;
// pub type LocalWallet = Wallet<ethers_core::k256::ecdsa::SigningKey>;

/// A wallet instantiated with a YubiHSM
#[cfg(feature = "yubi")]
pub type YubiWallet = WalletSigner<yubihsm::ecdsa::Signer<NistP256>>;

// #[cfg(all(feature = "yubihsm", not(target_arch = "wasm32")))]
mod yubi;

mod bip39_account;
mod error;
mod wallet_signer;
