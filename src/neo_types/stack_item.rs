use std::{collections::HashMap, fmt};
use base64::Engine;

/// This module defines the `StackItem` enum and `MapEntry` struct, which are used to represent items on the Neo virtual machine stack.
/// `StackItem` is a recursive enum that can represent any type of value that can be stored on the stack, including arrays, maps, and custom types.
/// `MapEntry` is a simple struct that represents a key-value pair in a `StackItem::Map`.
/// The `StackItem` enum also provides several utility methods for converting between different types and formats.
use primitive_types::{H160, H256};
use serde::{de::Visitor, Deserialize, Deserializer, Serialize};

use crate::crypto::Secp256r1PublicKey;
use neo3::prelude::{Address, ScriptHashExtension};

/// The `StackItem` enum represents an item on the Neo virtual machine stack.
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StackItem {
	/// Represents any type of value.
	#[serde(rename = "Any")]
	Any,

	/// Represents a pointer to another stack item.
	#[serde(rename = "Pointer")]
	Pointer {
		#[serde(deserialize_with = "deserialize_integer_from_string")]
		value: i64,
	},

	/// Represents a boolean value.
	#[serde(rename = "Boolean")]
	Boolean { value: bool },

	/// Represents an integer value.
	#[serde(rename = "Integer")]
	Integer {
		#[serde(deserialize_with = "deserialize_integer_from_string")]
		value: i64,
	},

	/// Represents a byte string value.
	#[serde(rename = "ByteString")]
	ByteString {
		value: String, // base64 encoded
	},

	/// Represents a buffer value.
	#[serde(rename = "Buffer")]
	Buffer {
		value: String, // base64 encoded
	},

	/// Represents an array of stack items.
	#[serde(rename = "Array")]
	Array { value: Vec<StackItem> },

	/// Represents a struct of stack items.
	#[serde(rename = "Struct")]
	Struct { value: Vec<StackItem> },

	/// Represents a map of stack items.
	#[serde(rename = "Map")]
	Map { value: Vec<MapEntry> },

	/// Represents an interop interface.
	#[serde(rename = "InteropInterface")]
	InteropInterface { id: String, interface: String },
}

fn deserialize_integer_from_string<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
	D: Deserializer<'de>,
{
	// let value_str = String::deserialize(deserializer)?;
	// value_str.parse::<i64>().map_err(serde::de::Error::custom)
	// First, try to deserialize the input as a string
	struct StringOrIntVisitor;

	impl<'de> Visitor<'de> for StringOrIntVisitor {
		type Value = i64;

		fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
			formatter.write_str("a string or integer")
		}

		fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
		where
			E: serde::de::Error,
		{
			value.parse::<i64>().map_err(serde::de::Error::custom)
		}

		fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
		where
			E: serde::de::Error,
		{
			value.parse::<i64>().map_err(serde::de::Error::custom)
		}

		fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
		where
			E: serde::de::Error,
		{
			Ok(value)
		}

		fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
		where
			E: serde::de::Error,
		{
			Ok(value as i64)
		}
	}

	deserializer.deserialize_any(StringOrIntVisitor)
}

/// The `MapEntry` struct represents a key-value pair in a `StackItem::Map`.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct MapEntry {
	key: StackItem,
	value: StackItem,
}

impl StackItem {
	/// The string value for `StackItem::Any`.
	pub const ANY_VALUE: &'static str = "Any";

	/// The string value for `StackItem::Pointer`.
	pub const POINTER_VALUE: &'static str = "Pointer";

	/// The string value for `StackItem::Boolean`.
	pub const BOOLEAN_VALUE: &'static str = "Boolean";

	/// The string value for `StackItem::Integer`.
	pub const INTEGER_VALUE: &'static str = "Integer";

	/// The string value for `StackItem::ByteString`.
	pub const BYTE_STRING_VALUE: &'static str = "ByteString";

	/// The string value for `StackItem::Buffer`.
	pub const BUFFER_VALUE: &'static str = "Buffer";

	/// The string value for `StackItem::Array`.
	pub const ARRAY_VALUE: &'static str = "Array";

	/// The string value for `StackItem::Struct`.
	pub const STRUCT_VALUE: &'static str = "Struct";

	/// The string value for `StackItem::Map`.
	pub const MAP_VALUE: &'static str = "Map";

	/// The string value for `StackItem::InteropInterface`.
	pub const INTEROP_INTERFACE_VALUE: &'static str = "InteropInterface";

	/// The byte value for `StackItem::Any`.
	pub const ANY_BYTE: u8 = 0x00;

	/// The byte value for `StackItem::Pointer`.
	pub const POINTER_BYTE: u8 = 0x10;

	/// The byte value for `StackItem::Boolean`.
	pub const BOOLEAN_BYTE: u8 = 0x20;

	/// The byte value for `StackItem::Integer`.
	pub const INTEGER_BYTE: u8 = 0x21;

	/// The byte value for `StackItem::ByteString`.
	pub const BYTE_STRING_BYTE: u8 = 0x28;

	/// The byte value for `StackItem::Buffer`.
	pub const BUFFER_BYTE: u8 = 0x30;

	/// The byte value for `StackItem::Array`.
	pub const ARRAY_BYTE: u8 = 0x40;

	/// The byte value for `StackItem::Struct`.
	pub const STRUCT_BYTE: u8 = 0x41;

	/// The byte value for `StackItem::Map`.
	pub const MAP_BYTE: u8 = 0x48;

	/// The byte value for `StackItem::InteropInterface`.
	pub const INTEROP_INTERFACE_BYTE: u8 = 0x60;

	pub fn new_byte_string(byte_array: Vec<u8>) -> Self {
		let byte_string = base64::engine::general_purpose::STANDARD.encode(byte_array);
		StackItem::ByteString { value: byte_string }
	}

	/// Returns the boolean value of a `StackItem::Boolean` or `StackItem::Integer`.
	pub fn as_bool(&self) -> Option<bool> {
		match self {
			StackItem::Boolean { value } => Some(*value),
			StackItem::Integer { value } => Some(value != &0),
			_ => None,
		}
	}

	/// Returns the string value of a `StackItem::ByteString`, `StackItem::Buffer`, `StackItem::Integer`, or `StackItem::Boolean`.
	pub fn as_string(&self) -> Option<String> {
		match self {
				StackItem::ByteString { value } | StackItem::Buffer { value } => Some(
					String::from_utf8_lossy(
						&base64::engine::general_purpose::STANDARD.decode(value).unwrap(),
					)
					.to_string(),
				),
			StackItem::Integer { value } => Some(value.to_string()),
			StackItem::Boolean { value } => Some(value.to_string()),
			_ => None,
		}
	}

	/// Returns the string representation of a `StackItem`.
	#[allow(clippy::inherent_to_string)]
	pub fn to_string(&self) -> String {
		match self {
			StackItem::Any => "Any".to_string(),
			StackItem::Pointer { value: pointer } => format!("Pointer{{value={}}}", pointer),
			StackItem::Boolean { value: boolean } => format!("Boolean{{value={}}}", boolean),
			StackItem::Integer { value: integer } => format!("Integer{{value={}}}", integer),
			StackItem::ByteString { value: byte_string } => {
				format!("ByteString{{value={:?}}}", byte_string)
			},
			StackItem::Buffer { value: buffer } => format!("Buffer{{value={:?}}}", buffer),
			StackItem::Array { value: array } => {
				let values = array.iter().map(StackItem::to_string).collect::<Vec<_>>().join(", ");
				format!("Array{{value=[{}]}}", values)
			},
			StackItem::Struct { value: _struct } => {
				let values =
					_struct.iter().map(StackItem::to_string).collect::<Vec<_>>().join(", ");
				format!("Struct{{value=[{}]}}", values)
			},
			StackItem::Map { value: map_value } => {
				// Iterate over pairs of elements in the vector
				// (assuming the vector has an even number of elements)
				let entries = map_value
					.iter()
					.map(|entry| {
						format!("{} -> {}", entry.key.to_string(), entry.value.to_string())
					})
					.collect::<Vec<_>>()
					.join(", ");
				format!("Map{{{{{}}}}}", entries)
			},
			StackItem::InteropInterface { id, interface } => {
				format!("InteropInterface{{id={}, interface={}}}", id, interface)
			},
		}
	}

	/// Returns the byte representation of a `StackItem::ByteString`, `StackItem::Buffer`, or `StackItem::Integer`.
	pub fn as_bytes(&self) -> Option<Vec<u8>> {
		match self {
			StackItem::ByteString { value } | StackItem::Buffer { value } =>
			// Some(hex::decode(value).unwrap()),
			{
				Some(
					base64::engine::general_purpose::STANDARD
						.decode(value.trim_end())
						.unwrap_or_else(|_| panic!("Failed to decode the string: {}", value)),
				)
			},
			//Some(value.trim_end().as_bytes().to_vec()),
			StackItem::Integer { value } => {
				let mut bytes = value.to_be_bytes().to_vec();
				bytes.reverse();
				Some(bytes)
			},
			_ => None,
		}
	}

	/// Returns the array value of a `StackItem::Array` or `StackItem::Struct`.
	pub fn as_array(&self) -> Option<Vec<StackItem>> {
		match self {
			StackItem::Array { value } | StackItem::Struct { value } => Some(value.clone()),
			_ => None,
		}
	}

	/// Returns the integer value of a `StackItem::Integer` or `StackItem::Boolean`.
	pub fn as_int(&self) -> Option<i64> {
		match self {
			StackItem::Integer { value } => Some(*value),
			StackItem::Boolean { value } => Some(if *value { 1 } else { 0 }),
			StackItem::Pointer { value } => Some(*value),
			_ => None,
		}
	}

	/// Returns the map value of a `StackItem::Map`.
	pub fn as_map(&self) -> Option<HashMap<StackItem, StackItem>> {
		match self {
			StackItem::Map { value } => {
				let mut map = HashMap::new();
				for entry in value {
					map.insert(entry.key.clone(), entry.value.clone());
				}
				Some(map)
			},
			_ => None,
		}
	}

	/// Returns the `Address` value of a `StackItem::ByteString` or `StackItem::Buffer`.
	pub fn as_address(&self) -> Option<Address> {
		self.as_bytes().map(|mut bytes| {
			bytes.reverse();
			H160::from_slice(&bytes).to_address()
		})
	}

	/// Returns the `Secp256r1PublicKey` value of a `StackItem::ByteString` or `StackItem::Buffer`.
	pub fn as_public_key(&self) -> Option<Secp256r1PublicKey> {
		self.as_bytes().and_then(|bytes| Secp256r1PublicKey::from_bytes(&bytes).ok())
	}

	/// Returns the `H160` value of a `StackItem::ByteString` or `StackItem::Buffer`.
	pub fn as_hash160(&self) -> Option<H160> {
		self.as_bytes().map(|bytes| H160::from_slice(&bytes))
	}

	/// Returns the `H256` value of a `StackItem::ByteString` or `StackItem::Buffer`.
	pub fn as_hash256(&self) -> Option<H256> {
		self.as_bytes().map(|bytes| H256::from_slice(&bytes))
	}

	pub fn as_interop(&self, interface_name: &str) -> Option<StackItem> {
		match self {
			StackItem::Integer { value } => Some(StackItem::InteropInterface {
				id: value.to_string(),
				interface: interface_name.to_string(),
			}),
			StackItem::Boolean { value } => Some(StackItem::InteropInterface {
				id: value.to_string(),
				interface: interface_name.to_string(),
			}),
			StackItem::ByteString { value } => Some(StackItem::InteropInterface {
				id: value.to_string(),
				interface: interface_name.to_string(),
			}),
			StackItem::Buffer { value } => Some(StackItem::InteropInterface {
				id: value.to_string(),
				interface: interface_name.to_string(),
			}),
			_ => None,
		}
	}

	pub fn len(&self) -> Option<usize> {
		match self {
			StackItem::Array { value } | StackItem::Struct { value } => Some(value.len()),
			_ => None,
		}
	}

	pub fn is_empty(&self) -> Option<bool> {
		self.len().map(|len| len == 0)
	}

	pub fn get(&self, index: usize) -> Option<StackItem> {
		self.as_array().and_then(|arr| arr.get(index).cloned())
	}

	pub fn get_iterator_id(&self) -> Option<&String> {
		if let StackItem::InteropInterface { id, .. } = self {
			Some(id)
		} else {
			None
		}
	}

	pub fn get_interface_name(&self) -> Option<&String> {
		if let StackItem::InteropInterface { interface, .. } = self {
			Some(interface)
		} else {
			None
		}
	}
}

impl From<String> for StackItem {
	fn from(value: String) -> Self {
		StackItem::ByteString { value }
	}
}

impl From<H160> for StackItem {
	fn from(value: H160) -> Self {
		StackItem::ByteString { value: ToString::to_string(&value) }
	}
}

impl From<u8> for StackItem {
	fn from(value: u8) -> Self {
		StackItem::Integer { value: value as i64 }
	}
}

impl From<i8> for StackItem {
	fn from(value: i8) -> Self {
		StackItem::Integer { value: value as i64 }
	}
}

impl From<u16> for StackItem {
	fn from(value: u16) -> Self {
		StackItem::Integer { value: value as i64 }
	}
}

impl From<i16> for StackItem {
	fn from(value: i16) -> Self {
		StackItem::Integer { value: value as i64 }
	}
}

impl From<u32> for StackItem {
	fn from(value: u32) -> Self {
		StackItem::Integer { value: value as i64 }
	}
}

impl From<i32> for StackItem {
	fn from(value: i32) -> Self {
		StackItem::Integer { value: value as i64 }
	}
}

impl From<u64> for StackItem {
	fn from(value: u64) -> Self {
		StackItem::Integer { value: value as i64 }
	}
}
impl From<&str> for StackItem {
	fn from(value: &str) -> Self {
		StackItem::ByteString { value: value.to_string() }
	}
}
