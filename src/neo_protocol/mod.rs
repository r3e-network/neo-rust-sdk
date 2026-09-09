//! # Neo Protocol
//!
//! Core protocol types and interfaces for the Neo N3 blockchain.
//!
//! ## Overview
//!
//! The neo_protocol module provides the core types and interfaces for interacting with
//! the Neo N3 blockchain protocol. It includes:
//!
//! - Account management and address handling
//! - NEP-2 password-protected key format support
//! - Protocol error definitions and handling
//! - Response types for Neo N3 RPC calls
//! - Role-based access control definitions
//!
//! This module forms the foundation for blockchain interactions, defining the data structures
//! and interfaces that represent the Neo N3 protocol.
//!
//! ## Examples
//!
//! ### Working with Neo N3 accounts
//!
//! ```no_run
//! use neo3::neo_protocol::{Account, AccountTrait};
//!
//! // Create a new account
//! let account = Account::create().unwrap();
//! println!("Address: {}", account.get_address());
//! println!("Script Hash: {}", account.get_script_hash());
//!
//! // Create an account from a WIF (Wallet Import Format) string
//! let wif = "your-private-key-wif";
//! let account = Account::from_wif(wif).unwrap();
//! ```
//!
//! ### Using NEP-2 password-protected keys
//!
//! ```no_run
//! use neo3::neo_crypto::KeyPair;
//! use neo3::neo_protocol::NEP2;
//!
//! // Create a key pair and encrypt with NEP-2
//! let key_pair = KeyPair::new_random();
//! let password = "your_secure_password_here";
//! let nep2_string = NEP2::encrypt(password, &key_pair).unwrap();
//!
//! // Decrypt a NEP-2 string back to a key pair
//! let decrypted = NEP2::decrypt(password, &nep2_string).unwrap();
//! ```

pub use account::*;
pub use nep2::*;
pub use protocol_error::*;
pub use responses::*;
pub use test_node::*;

mod account;
mod nep2;
mod protocol_error;
mod responses;
mod role;
mod test_node;
