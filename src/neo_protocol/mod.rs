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
//! ```rust
//! use neo3::neo_protocol::Account;
//!
//! // Create a new account
//! let account = Account::create().unwrap();
//! println!("Address: {}", account.address());
//! println!("Script Hash: {}", account.script_hash());
//!
//! // Create an account from a WIF (Wallet Import Format) string
//! let wif = "KwYgW8gcxj1JWJXhPSu4Fqwzfhp5Yfi42mdYmMa4XqK7NJxXUSK7";
//! let account = Account::from_wif(wif).unwrap();
//!
//! // Sign a message
//! let message = b"Hello, Neo!";
//! let signature = account.sign(message).unwrap();
//!
//! // Verify the signature
//! let is_valid = account.verify(message, &signature).unwrap();
//! assert!(is_valid);
//! ```
//!
//! ### Using NEP-2 password-protected keys
//!
//! ```rust
//! use neo3::neo_crypto::PrivateKey;
//! use neo3::neo_protocol::nep2::{encrypt_to_nep2, decrypt_from_nep2};
//!
//! // Encrypt a private key with a password (NEP-2 format)
//! let private_key = PrivateKey::from_slice(&[/* 32 bytes */]).unwrap();
//! let password = "your_secure_password_here";
//! let nep2_string = encrypt_to_nep2(&private_key, password).unwrap();
//!
//! // Decrypt a NEP-2 string back to a private key
//! let nep2_string = "6PYVPVe1fQznphjbUxXP9KZJqPMVnVwCx5s5pr5axRJ8uHkMtZg97eT5kL";
//! let password = "your_secure_password_here";
//! let private_key = decrypt_from_nep2(nep2_string, password).unwrap();
//! ```

pub use account::*;
pub use nep2::*;
pub use protocol_error::*;
pub use responses::*;

mod account;
mod nep2;
mod protocol_error;
mod responses;
mod role;
