#![allow(clippy::items_after_test_module)]

//! # Neo Types (v0.4.1)
//!
//! Core data types for the Neo N3 blockchain.
//!
//! ## Overview
//!
//! The neo_types module provides fundamental data types and structures for working with
//! the Neo N3 blockchain. It includes:
//!
//! - Address and ScriptHash types for identifying accounts and contracts
//! - Contract-related types (parameters, manifests, NEF files)
//! - Neo Name Service (NNS) types
//! - Numeric types with blockchain-specific operations
//! - Serialization and deserialization utilities
//! - Stack item representations for VM operations
//! - Blockchain-specific enumerations (OpCode, VMState)
//!
//! This module forms the type foundation for the entire SDK, providing the core data structures
//! that represent Neo N3 blockchain concepts.
//!
//! ## Examples
//!
//! ### Working with Neo N3 addresses and script hashes
//!
//! ```rust,no_run
//! use neo3::prelude::*;
//! use std::str::FromStr;
//!
//! // Create a script hash from a string
//! let script_hash = ScriptHash::from_str("0xd2a4cff31913016155e38e474a2c06d08be276cf").unwrap();
//! println!("Script hash: {}", script_hash);
//!
//! // Convert between address and script hash
//! let address = script_hash.to_address();
//! println!("Address: {}", address);
//!
//! let recovered_hash = ScriptHash::from_str(&address).unwrap();
//! assert_eq!(script_hash, recovered_hash);
//! ```
//!
//! ### Working with contract parameters
//!
//! ```rust,no_run
//! use neo3::prelude::*;
//! use std::str::FromStr;
//!
//! // Create different types of contract parameters
//! let string_param = ContractParameter::string("Hello, Neo!".to_string());
//! let integer_param = ContractParameter::integer(42);
//! let bool_param = ContractParameter::bool(true);
//!
//! // Create a parameter array for a contract invocation
//! let params = vec![string_param, integer_param, bool_param];
//! ```
//!
//! ### Working with stack items
//!
//! ```ignore
//! use neo3::neo_types::StackItem;
//! use serde_json::json;
//!
//! // Create stack items of various types
//! let int_item = StackItem::Integer { value: 123 };
//! let bool_item = StackItem::Boolean { value: true };
//! let bytes_item = StackItem::new_byte_string(b"Neo".to_vec());
//!
//! // Convert between stack items and JSON values
//! let json_value = int_item.to_json_value();
//! assert_eq!(json_value, json!(123));
//! ```

use base64::{engine::general_purpose, Engine};
pub use log::*;
use primitive_types::H256;
use serde_derive::{Deserialize, Serialize};

// Re-export everything from these modules
pub use address::*;
pub use address_or_scripthash::*;
pub use bytes::*;
pub use contract::*;
pub use error::*;
pub use nns::*;
pub use numeric::*;
pub use op_code::*;
pub use path_or_string::*;
pub use plugin_type::*;
pub mod script_hash;
pub use script_hash::{ScriptHash, ScriptHashExtension};
pub use serde_with_utils::*;
pub use stack_item::*;
pub use syncing::*;
pub use tx_pool::*;
pub use url_session::*;
pub use util::*;

// Make modules public for direct access
pub mod contract;
pub mod error;
pub mod nns;
pub mod serde_value;
pub mod serde_with_utils;

mod address;
mod address_or_scripthash;
pub mod block;
mod bytes;
mod numeric;
mod op_code;
mod path_or_string;
mod plugin_type;
mod stack_item;
mod string;
mod syncing;
mod tx_pool;
mod url_session;
mod util;
mod vm_state;

pub type Byte = u8;
pub type Bytes = Vec<u8>;
pub type TxHash = H256;

pub trait ExternBase64 {
	fn to_base64(&self) -> String;
}

impl ExternBase64 for String {
	fn to_base64(&self) -> String {
		general_purpose::STANDARD.encode(self.as_bytes())
	}
}

// ScryptParams
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScryptParamsDef {
	#[serde(rename = "n")]
	pub log_n: u8,
	#[serde(rename = "r")]
	pub r: u32,
	#[serde(rename = "p")]
	pub p: u32,
}

impl Default for ScryptParamsDef {
	fn default() -> Self {
		Self { log_n: 14, r: 8, p: 8 }
	}
}

// Extend Vec<u8> with a to_base64 method
pub trait Base64Encode {
	fn to_base64(&self) -> String;
}

impl Base64Encode for Vec<u8> {
	fn to_base64(&self) -> String {
		base64::engine::general_purpose::STANDARD.encode(self)
	}
}

impl Base64Encode for &[u8] {
	fn to_base64(&self) -> String {
		base64::engine::general_purpose::STANDARD.encode(self)
	}
}

impl Base64Encode for String {
	fn to_base64(&self) -> String {
		match hex::decode(self) {
			Ok(bytes) => general_purpose::STANDARD.encode(&bytes),
			Err(_) => {
				// If hex decoding fails, return an empty string
				// Professional error handling with logging for debugging
				eprintln!("Failed to decode hex string: {}", self);
				String::new()
			},
		}
	}
}

// pub fn secret_key_to_script_hash(secret_key: &Secp256r1PrivateKey) -> ScriptHash {
// 	let public_key = secret_key.to_public_key();
// 	public_key_to_script_hash(&public_key)
// }

// pub fn public_key_to_script_hash(pubkey: &Secp256r1PublicKey) -> ScriptHash {
// 	raw_public_key_to_script_hash(&pubkey.get_encoded(true)[1..])
// }
//
// pub fn raw_public_key_to_script_hash<T: AsRef<[u8]>>(pubkey: T) -> ScriptHash {
// 	let pubkey = pubkey.as_ref();
// 	let script = format!(
// 		"{}21{}{}{}{}",
// 		OpCode::PushData1.to_string(),
// 		"03",
// 		pubkey.to_hex(),
// 		OpCode::Syscall.to_string(),
// 		InteropService::SystemCryptoCheckSig.hash()
// 	)
// 	.from_hex()
// 	.unwrap();
// 	let mut script = script.sha256_ripemd160();
// 	script.reverse();
// 	ScriptHash::from_slice(&script)
// }

pub fn to_checksum(addr: &ScriptHash, chain_id: Option<u8>) -> String {
	// if !addr.is_valid_address(){
	// 	panic!("invalid address");
	// }
	let prefixed_addr = match chain_id {
		Some(chain_id) => format!("{chain_id}0x{addr:x}"),
		None => format!("{addr:x}"),
	};
	let hash = hex::encode(prefixed_addr);
	let hash = hash.as_bytes();

	let addr_hex = hex::encode(addr.as_bytes());
	let addr_hex = addr_hex.as_bytes();

	addr_hex.iter().zip(hash).fold("0x".to_owned(), |mut encoded, (addr, hash)| {
		encoded.push(if *hash >= 56 {
			addr.to_ascii_uppercase() as char
		} else {
			addr.to_ascii_lowercase() as char
		});
		encoded
	})
}

#[cfg(test)]
mod tests {
	use crate::neo_crypto::utils::FromBase64String;
	use hex;

	use super::*;

	#[test]
	fn test_base64_encode_bytes() {
		let input = hex::decode("150c14242dbf5e2f6ac2568b59b7822278d571b75f17be0c14242dbf5e2f6ac2568b59b7822278d571b75f17be13c00c087472616e736665720c14897720d8cd76f4f00abfa37c0edd889c208fde9b41627d5b5238").unwrap();
		let expected = "FQwUJC2/Xi9qwlaLWbeCInjVcbdfF74MFCQtv14vasJWi1m3giJ41XG3Xxe+E8AMCHRyYW5zZmVyDBSJdyDYzXb08Aq/o3wO3YicII/em0FifVtSOA==";

		let encoded = input.to_base64();

		assert_eq!(encoded, expected);
	}

	#[test]
	fn test_base64_decode() {
		let encoded = "FQwUJC2/Xi9qwlaLWbeCInjVcbdfF74MFCQtv14vasJWi1m3giJ41XG3Xxe+E8AMCHRyYW5zZmVyDBSJdyDYzXb08Aq/o3wO3YicII/em0FifVtSOA==";
		let expected = "150c14242dbf5e2f6ac2568b59b7822278d571b75f17be0c14242dbf5e2f6ac2568b59b7822278d571b75f17be13c00c087472616e736665720c14897720d8cd76f4f00abfa37c0edd889c208fde9b41627d5b5238";

		let decoded = encoded.from_base64_string().unwrap();
		let decoded_hex = hex::encode(decoded);

		assert_eq!(decoded_hex, expected);
	}
}

// Re-export serialization functions from serde_with_utils
pub use serde_with_utils::{
	deserialize_h160, deserialize_h256, deserialize_script_hash, deserialize_u256, deserialize_u64,
	deserialize_vec_h256, deserialize_vec_u256, deserialize_wildcard, serialize_h160,
	serialize_h256, serialize_script_hash, serialize_u256, serialize_u64, serialize_vec_h256,
	serialize_vec_u256, serialize_wildcard,
};

// Re-export ValueExtension trait
pub use serde_value::ValueExtension;

// Re-export address types
pub use address::NameOrAddress;

// Other re-exports
pub use string::StringExt;
pub use vm_state::VMState;
