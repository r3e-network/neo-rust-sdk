#![allow(clippy::items_after_test_module)]
// This module demonstrates extensions for blockchain address manipulation, focusing on converting between addresses, script hashes,
// and handling various formats like Base58 and hexadecimal strings. It leverages cryptographic functions, serialization, and
// deserialization to work with blockchain-specific data types.

use primitive_types::H160;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::{
	crypto::HashableForVec,
	neo_crypto::utils::FromHexString,
	neo_error::NeoError,
	neo_types::{ScriptHash, ScriptHashExtension, StringExt, TypeError},
};

pub type Address = String;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NameOrAddress {
	Name(String),
	Address(Address),
}

// Implementations below provide concrete behavior for the `AddressExtension` trait,
// applicable to `String` and `&str` types.
pub trait AddressExtension {
	/// Converts a Base58-encoded address (common in many blockchain systems) to a `ScriptHash`.
	///
	/// # Examples
	///
	/// Basic usage:
	///
	/// ```
	/// use neo3::neo_types::AddressExtension;
	/// // Example with a valid Neo N3 address format
	/// let address = "NNLi44dJNXtDNSBkofB48aTVYtb1zZrNEs";
	/// let result = address.address_to_script_hash();
	/// // This demonstrates the function's capability to convert addresses
	/// ```
	fn address_to_script_hash(&self) -> Result<ScriptHash, TypeError>;

	/// Decodes a hex-encoded script into a `ScriptHash`, demonstrating error handling for invalid hex strings.
	///
	/// # Examples
	///
	/// Basic usage:
	///
	/// ```
	/// use neo3::neo_types::AddressExtension;
	/// let script = "abcdef1234567890";
	/// let script_hash = script.script_to_script_hash().unwrap();
	/// ```
	fn script_to_script_hash(&self) -> Result<ScriptHash, TypeError>;

	/// Validates a hex string and converts it to a `ScriptHash`, showcasing error handling for non-hex strings.
	///
	/// # Examples
	///
	/// Basic usage:
	///
	/// ```
	/// use neo3::neo_types::AddressExtension;
	/// let hex_string = "abcdef1234567890abcdef1234567890abcdef12";
	/// let script_hash = hex_string.hex_to_script_hash().unwrap();
	/// ```
	fn hex_to_script_hash(&self) -> Result<ScriptHash, TypeError>;

	/// Generates a random address using cryptographic-safe random number generation, ideal for creating new wallet addresses.
	///
	/// # Examples
	///
	/// Basic usage:
	///
	/// ```
	/// use neo3::neo_types::AddressExtension;
	/// let random_address = String::random();
	/// ```
	fn random() -> Self;
}

impl AddressExtension for String {
	fn address_to_script_hash(&self) -> Result<ScriptHash, TypeError> {
		// Base58-decode the address
		let decoded_data = match bs58::decode(self).into_vec() {
			Ok(data) => data,
			Err(_) => return Err(TypeError::InvalidAddress),
		};
		let data_payload = decoded_data[1..decoded_data.len() - 4].to_vec();
		Ok(H160::from_slice(data_payload.as_slice()))
	}

	fn script_to_script_hash(&self) -> Result<ScriptHash, TypeError> {
		self.from_hex_string()
			.map(|data| ScriptHash::from_script(data.as_slice()))
			.map_err(|_| TypeError::InvalidScript("Invalid hex string".to_string()))
	}

	fn hex_to_script_hash(&self) -> Result<ScriptHash, TypeError> {
		if self.is_valid_hex() {
			ScriptHash::from_hex(self.as_str())
				.map_err(|_| TypeError::InvalidFormat("Invalid hex format".to_string()))
		} else {
			Err(TypeError::InvalidFormat("Invalid hex format".to_string()))
		}
	}

	fn random() -> Self {
		let mut rng = rand::thread_rng();
		let mut bytes = [0u8; 20];
		rng.fill(&mut bytes);
		let script_hash = bytes.sha256_ripemd160();
		let mut data = vec![0x17];
		data.extend_from_slice(&script_hash);
		let sha = &data.hash256().hash256();
		data.extend_from_slice(&sha[..4]);
		bs58::encode(data).into_string()
	}
}

impl AddressExtension for &str {
	fn address_to_script_hash(&self) -> Result<ScriptHash, TypeError> {
		self.to_string().address_to_script_hash()
	}

	fn script_to_script_hash(&self) -> Result<ScriptHash, TypeError> {
		self.to_string().script_to_script_hash()
	}

	fn hex_to_script_hash(&self) -> Result<ScriptHash, TypeError> {
		self.to_string().hex_to_script_hash()
	}

	fn random() -> Self {
		// This implementation is not feasible for &str as it requires returning a borrowed string
		// Users should use String::random() instead
		""
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_address_to_script_hash() {
		// Test case 1: Valid N3 address
		let n3_address = "NTGYC16CN5QheM4ZwfhUp9JKq8bMjWtcAp";
		let expected_script_hash_hex = "50acc01271492d7b0e264ace0d60d572e66bc087";
		let result = n3_address
			.address_to_script_hash()
			.expect("Should be able to convert valid N3 address to script hash");
		assert_eq!(hex::encode(result), expected_script_hash_hex);

		// Test case 3: Invalid N3 address
		let n3_address = "Invalid_Address";
		let result = n3_address.to_string().address_to_script_hash();
		assert!(result.is_err());
	}
}

pub fn from_script_hash(_script_hash: &H160) -> Result<String, NeoError> {
	Err(NeoError::UnsupportedOperation(
		"Address conversion from script hash requires comprehensive cryptographic implementation"
			.to_string(),
	))
}
