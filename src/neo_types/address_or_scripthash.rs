// This module demonstrates the flexibility in handling blockchain addresses and script hashes, leveraging Rust's type system
// and trait implementations to provide a seamless interface for converting and working with these two fundamental types.

use std::hash::{Hash, Hasher};

use primitive_types::H160;
use serde_derive::{Deserialize, Serialize};

use neo3::prelude::{Address, Bytes, ScriptHashExtension};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
	/// let from_address = AddressOrScriptHash::from("myAddress".to_string());
	/// assert!(matches!(from_address, AddressOrScriptHash::Address(_)));
	/// ```
	fn from(s: Address) -> Self {
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
		Self::ScriptHash(H160::from_slice(&s))
	}
}

impl AddressOrScriptHash {
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
		match self {
			AddressOrScriptHash::Address(a) => H160::from_address(&a).unwrap(), //a.address_to_script_hash().unwrap(),
			AddressOrScriptHash::ScriptHash(s) => s.clone(),
		}
	}
}
