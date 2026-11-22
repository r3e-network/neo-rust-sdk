//! # Neo Crypto
//!
//! Cryptographic utilities for the Neo N3 blockchain.
//!
//! ## Overview
//!
//! The neo_crypto module provides cryptographic primitives and utilities for working with
//! the Neo N3 blockchain. It includes:
//!
//! - Key pair generation and management
//! - Cryptographic signing and verification
//! - Hashing functions (SHA256, RIPEMD160, etc.)
//! - Base58 encoding and decoding
//! - WIF (Wallet Import Format) utilities
//! - Secure random number generation
//! - Encryption and decryption utilities
//!
//! This module forms the cryptographic foundation for wallet management, transaction signing,
//! and secure communication within the Neo N3 ecosystem.
//!
//! ## Examples
//!
//! ### Creating a key pair
//!
//! ```rust,no_run
//! use neo3::neo_crypto::KeyPair;
//!
//! // Generate a new random key pair
//! let key_pair = KeyPair::new_random();
//! println!("Public key: {:?}", key_pair.public_key());
//! // SECURITY: Avoid logging private keys.
//!
//! // Create a key pair from a private key (32 bytes)
//! let private_key_bytes = [1u8; 32]; // Replace with actual private key bytes
//! let key_pair = KeyPair::from_private_key(&private_key_bytes).unwrap();
//! ```
//!
//! ### Signing and verifying data
//!
//! ```rust,no_run
//! use neo3::neo_crypto::KeyPair;
//!
//! // Generate a key pair
//! let key_pair = KeyPair::new_random();
//!
//! // Data to sign
//! let data = b"Hello, Neo!";
//!
//! // Sign the data using the private key
//! let signature = key_pair.sign(data).unwrap();
//! ```
//!
//! ### Working with WIF format
//!
//! ```rust,no_run
//! use neo3::neo_crypto::KeyPair;
//!
//! // Import a private key from WIF format
//! let wif = "your-private-key-wif";
//! let key_pair = KeyPair::from_wif(wif).unwrap();
//!
//! // Export a private key to WIF format
//! let exported_wif = key_pair.export_as_wif().unwrap();
//! assert_eq!(wif, exported_wif);
//! ```

pub use base58_helper::*;
pub use error::*;
pub use keys::*;
pub use utils::*;
pub use wif::*;

mod base58_helper;
pub mod crypto_lib;
mod error;
pub mod hash;
mod key_pair;
mod keys;
pub mod utils;
mod wif;

// Re-export important types
pub use crypto_lib::{
	CryptoLibHashable, blake2b_512, recover_secp256k1, sha3_512, verify_with_ed25519,
};
pub use error::CryptoError;
pub use hash::HashableForVec;
pub use key_pair::KeyPair;
pub use keys::{Secp256r1PublicKey, Secp256r1Signature};
