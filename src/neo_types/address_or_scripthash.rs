// This module demonstrates the flexibility in handling blockchain addresses and script hashes, leveraging Rust's type system
// and trait implementations to provide a seamless interface for converting and working with these two fundamental types.

use std::hash::{Hash, Hasher};

use primitive_types::H160;
use serde::{Deserialize, Deserializer, Serialize, de};

use crate::neo_types::TypeError;
use neo3::prelude::{Address, Bytes, ScriptHashExtension};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
/// An enum that can represent either a blockchain `Address` or a `ScriptHash`,
/// offering flexibility for APIs that can work with either.
pub enum AddressOrScriptHash {
	/// An address type
	Address(Address),
	/// A bytes type
	ScriptHash(H160),
}

impl Hash for AddressOrScriptHash {
	/// Implements the `Hash` trait to allow `AddressOrScriptHash`
	/// instances to be used as keys in hash maps or elements in hash sets.
	///
	/// # Examples
	///
	/// ```
	/// use std::collections::HashSet;
	/// use neo3::neo_types::AddressOrScriptHash;
	/// let mut set = HashSet::new();
	/// set.insert(AddressOrScriptHash::Address("myAddress".into()));
	/// ```
	fn hash<H: Hasher>(&self, state: &mut H) {
		match self {
			AddressOrScriptHash::Address(a) => a.hash(state),
			AddressOrScriptHash::ScriptHash(s) => s.hash(state),
		}
	}
}

impl Default for AddressOrScriptHash {
	fn default() -> Self {
		AddressOrScriptHash::Address(Default::default())
	}
}

impl From<Address> for AddressOrScriptHash {
	/// Allows creating an `AddressOrScriptHash` directly from an `Address`.
	///
	/// # Examples
	///
	/// ```
	/// use neo3::neo_types::AddressOrScriptHash;
	/// let from_address = AddressOrScriptHash::from("NNLi44dJNXtDNSBkofB48aTVYtb1zZrNEs".to_string());
	/// assert!(matches!(from_address, AddressOrScriptHash::Address(_)));
	/// ```
	fn from(s: Address) -> Self {
		H160::from_address(&s).unwrap_or_else(|err| {
			panic!(
				"invalid address; use AddressOrScriptHash::Address plus try_script_hash for fallible handling: {}",
				err
			)
		});
		Self::Address(s)
	}
}

impl From<Bytes> for AddressOrScriptHash {
	/// Allows creating an `AddressOrScriptHash` from a `Bytes` array, automatically converting it into a `ScriptHash`.
	///
	/// # Examples
	///
	/// ```
	/// use neo3::neo_types::{AddressOrScriptHash, Bytes};
	/// let bytes: Bytes = vec![0xde, 0xad, 0xbe, 0xef, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10];
	/// let from_bytes = AddressOrScriptHash::from(bytes);
	/// assert!(matches!(from_bytes, AddressOrScriptHash::ScriptHash(_)));
	/// ```
	fn from(s: Bytes) -> Self {
		Self::try_from_script_hash_bytes(&s).unwrap_or_else(|err| {
			panic!(
				"invalid script hash bytes; use AddressOrScriptHash::try_from for fallible handling: {}",
				err
			)
		})
	}
}

impl TryFrom<&[u8]> for AddressOrScriptHash {
	type Error = TypeError;

	fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
		Self::try_from_script_hash_bytes(value)
	}
}

#[derive(Deserialize)]
enum AddressOrScriptHashRepr {
	Address(Address),
	ScriptHash(H160),
}

impl<'de> Deserialize<'de> for AddressOrScriptHash {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		match AddressOrScriptHashRepr::deserialize(deserializer)? {
			AddressOrScriptHashRepr::Address(address) => {
				H160::from_address(&address).map_err(|err| {
					de::Error::custom(format!("invalid address '{}': {}", address, err))
				})?;
				Ok(Self::Address(address))
			},
			AddressOrScriptHashRepr::ScriptHash(script_hash) => Ok(Self::ScriptHash(script_hash)),
		}
	}
}

impl AddressOrScriptHash {
	/// Creates an `AddressOrScriptHash` from script hash bytes, rejecting invalid lengths.
	pub fn try_from_script_hash_bytes(bytes: &[u8]) -> Result<Self, TypeError> {
		if bytes.len() != 20 {
			return Err(TypeError::InvalidAddress);
		}

		Ok(Self::ScriptHash(H160::from_slice(bytes)))
	}

	/// Retrieves the `Address` representation. If the instance is a `ScriptHash`, converts it to an `Address`.
	///
	/// # Examples
	///
	/// ```
	/// use primitive_types::H160;
	/// use neo3::neo_types::AddressOrScriptHash;
	/// let script_hash = AddressOrScriptHash::ScriptHash(H160::repeat_byte(0x01));
	/// let address = script_hash.address();
	/// // The address will be a valid Neo address derived from the script hash
	/// assert!(address.starts_with("N"));
	/// ```
	pub fn address(&self) -> Address {
		match self {
			AddressOrScriptHash::Address(a) => a.clone(),
			AddressOrScriptHash::ScriptHash(s) => s.to_address(),
		}
	}

	/// Retrieves the `ScriptHash` representation. If the instance is an `Address`, converts it to a `ScriptHash`.
	///
	/// # Errors
	///
	/// Returns `TypeError::InvalidAddress` when the stored address is invalid.
	pub fn try_script_hash(&self) -> Result<H160, TypeError> {
		match self {
			AddressOrScriptHash::Address(address) => H160::from_address(address),
			AddressOrScriptHash::ScriptHash(script_hash) => Ok(*script_hash),
		}
	}

	/// Retrieves the `ScriptHash` representation. If the instance is an `Address`, converts it to a `ScriptHash`.
	///
	/// # Examples
	///
	/// ```
	/// use primitive_types::H160;
	/// use neo3::neo_types::AddressOrScriptHash;
	/// let address = AddressOrScriptHash::Address("NNLi44dJNXtDNSBkofB48aTVYtb1zZrNEs".to_string());
	/// let script_hash = address.script_hash();
	/// // The script hash will be derived from the address
	/// assert!(script_hash != H160::zero());
	/// ```
	pub fn script_hash(&self) -> H160 {
		self.try_script_hash().unwrap_or_else(|e| {
			panic!(
				"BUG: AddressOrScriptHash::Address contains invalid address '{}': {}",
				self.address(),
				e
			)
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_try_from_bytes_rejects_invalid_length() {
		let result = AddressOrScriptHash::try_from_script_hash_bytes(&[0u8; 19]);
		assert!(result.is_err());
	}

	#[test]
	fn test_try_script_hash_rejects_invalid_address() {
		let value = AddressOrScriptHash::Address("invalid-address".to_string());
		let result = value.try_script_hash();
		assert!(result.is_err());
	}

	#[test]
	#[should_panic(expected = "invalid address")]
	fn test_from_address_panics_on_invalid_address() {
		let _ = AddressOrScriptHash::from("invalid-address".to_string());
	}

	#[test]
	#[should_panic(expected = "invalid script hash bytes")]
	fn test_from_bytes_panics_on_invalid_length_instead_of_zero_hash() {
		let _ = AddressOrScriptHash::from(vec![0u8; 19]);
	}

	#[test]
	fn test_deserialize_rejects_invalid_address_variant() {
		let result: Result<AddressOrScriptHash, _> =
			serde_json::from_str(r#"{"Address":"invalid-address"}"#);
		assert!(result.is_err());
	}

	#[test]
	fn test_deserialize_accepts_valid_address_variant() {
		let result: AddressOrScriptHash =
			serde_json::from_str(r#"{"Address":"NNLi44dJNXtDNSBkofB48aTVYtb1zZrNEs"}"#).unwrap();
		assert!(matches!(result, AddressOrScriptHash::Address(_)));
	}
}
