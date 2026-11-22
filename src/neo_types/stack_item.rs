use base64::Engine;
use std::{collections::HashMap, fmt};

/// This module defines the `StackItem` enum and `MapEntry` struct, which are used to represent items on the Neo virtual machine stack.
/// `StackItem` is a recursive enum that can represent any type of value that can be stored on the stack, including arrays, maps, and custom types.
/// `MapEntry` is a simple struct that represents a key-value pair in a `StackItem::Map`.
/// The `StackItem` enum also provides several utility methods for converting between different types and formats.
use primitive_types::{H160, H256};
use serde::{Deserialize, Deserializer, Serialize, de::Visitor};

use crate::crypto::Secp256r1PublicKey;
use neo3::prelude::{Address, ScriptHashExtension};

/// The `StackItem` enum represents an item on the Neo virtual machine stack.
///
/// # Memory Layout Considerations
///
/// This enum is designed for use in Neo VM stack operations and JSON-RPC serialization.
/// Current size: **48 bytes** (on 64-bit platforms).
///
/// ## Variant Size Analysis
///
/// | Variant | Payload Size | Notes |
/// |---------|-------------|-------|
/// | `Any` | 0 bytes | Unit type |
/// | `Pointer` | 8 bytes | i64 value |
/// | `Boolean` | 8 bytes | bool + padding |
/// | `Integer` | 8 bytes | i64 value |
/// | `ByteString` | 24 bytes | String (ptr + len + cap) |
/// | `Buffer` | 24 bytes | String (ptr + len + cap) |
/// | `Array` | 24 bytes | Vec (ptr + len + cap on heap) |
/// | `Struct` | 24 bytes | Vec (ptr + len + cap on heap) |
/// | `Map` | 24 bytes | `Vec<MapEntry>` (ptr + len + cap on heap) |
/// | `InteropInterface` | **48 bytes** | Two Strings (largest variant) |
///
/// ## Boxing Considerations
///
/// While boxing large enum variants is a common Rust optimization technique, **boxing `Array`,
/// `Struct`, or `Map` would NOT reduce the enum size** because `InteropInterface` (48 bytes)
/// is already larger than the `Vec` variants (24 bytes each).
///
/// To actually reduce the enum size, we would need to:
/// 1. **Box `InteropInterface`** - This would reduce the enum from 48 bytes to 32 bytes
/// 2. **Use a single `String` or `Arc<str>` for `InteropInterface`** - Depends on usage patterns
///
/// ## Performance Trade-offs
///
/// The current design prioritizes:
/// - **Cache locality**: No pointer indirection for most variants
/// - **Zero-allocation access**: Direct access to Array/Map elements without dereferencing
/// - **Clone efficiency**: Simple memcpy for small variants
///
/// Boxing would trade:
/// - **Extra heap allocation** for Array/Map/InteropInterface creation
/// - **Pointer indirection** on every Array/Map access
/// - **Better memory density** when storing many small variants (Boolean, Integer)
///
/// ## Future Optimization
///
/// If profiling shows memory pressure from `StackItem` allocations, consider:
/// 1. Boxing only `InteropInterface` (gains 16 bytes per enum)
/// 2. Using `Box<str>` instead of `String` where mutability isn't needed
/// 3. Investigating arena allocation patterns for VM execution
///
/// # Serialization Note
///
/// This type uses externally tagged serialization (`#[serde(tag = "type")]`) to match
/// Neo's JSON-RPC response format. Any changes to variant structure must maintain
/// backward compatibility with the Neo node RPC API.
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
	///
	/// # Performance Note
	/// The `Vec<StackItem>` is stored inline in the enum variant. While `Vec` itself is only
	/// 24 bytes (pointer + length + capacity on the stack), the heap-allocated elements
	/// are separate. See the enum-level documentation for memory layout considerations.
	///
	/// Arrays are commonly used in Neo VM execution results and smart contract return values.
	/// Use `as_array()` to access elements without cloning when possible.
	#[serde(rename = "Array")]
	Array { value: Vec<StackItem> },

	/// Represents a struct of stack items.
	///
	/// # Note
	/// Similar to `Array`, but represents a struct type in the Neo VM. The memory layout
	/// and performance characteristics are identical to `Array`.
	#[serde(rename = "Struct")]
	Struct { value: Vec<StackItem> },

	/// Represents a map of stack items.
	///
	/// # Performance Note
	/// Maps are stored as `Vec<MapEntry>` rather than `HashMap` to maintain deterministic
	/// iteration order (required for Neo VM consistency) and reduce memory overhead.
	/// Use `as_map()` to convert to `HashMap<StackItem, StackItem>` when needed.
	#[serde(rename = "Map")]
	Map { value: Vec<MapEntry> },

	/// Represents an interop interface.
	///
	/// # Memory Note
	/// This is the **largest variant** (48 bytes) due to storing two `String` values.
	/// It determines the overall enum size. If memory optimization becomes critical,
	/// consider boxing this variant or using `Box<str>` for immutable strings.
	///
	/// InteropInterface is used to represent external objects (like iterators) that
	/// cannot be directly serialized but can be referenced by ID in subsequent operations.
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
			i64::try_from(value)
				.map_err(|_| serde::de::Error::custom(format!("u64 value {} overflows i64", value)))
		}
	}

	deserializer.deserialize_any(StringOrIntVisitor)
}

/// The `MapEntry` struct represents a key-value pair in a `StackItem::Map`.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct MapEntry {
	/// The key of the map entry.
	pub key: StackItem,
	/// The value of the map entry.
	pub value: StackItem,
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
			StackItem::Integer { value } => Some(*value != 0),
			_ => None,
		}
	}

	/// Returns the string value of a `StackItem::ByteString`, `StackItem::Buffer`, `StackItem::Integer`, or `StackItem::Boolean`.
	pub fn as_string(&self) -> Option<String> {
		match self {
			StackItem::ByteString { value } | StackItem::Buffer { value } => {
				let bytes =
					base64::engine::general_purpose::STANDARD.decode(value.trim_end()).ok()?;
				Some(String::from_utf8_lossy(&bytes).into_owned())
			},
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
			StackItem::ByteString { value } | StackItem::Buffer { value } => {
				base64::engine::general_purpose::STANDARD.decode(value.trim_end()).ok()
			},
			//Some(value.trim_end().as_bytes().to_vec()),
			StackItem::Integer { value } => {
				let bytes = value.to_le_bytes();
				let mut result = bytes.to_vec();
				// Trim to minimal two's complement representation
				if *value >= 0 {
					while result.len() > 1 && *result.last().unwrap() == 0 {
						if result[result.len() - 2] & 0x80 != 0 {
							break;
						}
						result.pop();
					}
				} else {
					while result.len() > 1 && *result.last().unwrap() == 0xFF {
						if result[result.len() - 2] & 0x80 == 0 {
							break;
						}
						result.pop();
					}
				}
				Some(result)
			},
			_ => None,
		}
	}

	/// Returns the array value of a `StackItem::Array` or `StackItem::Struct`.
	///
	/// # Performance Note
	/// This method clones the entire `Vec`. For read-only access, prefer `as_array_ref()`.
	/// Use this when you need ownership of the array elements.
	pub fn as_array(&self) -> Option<Vec<StackItem>> {
		match self {
			StackItem::Array { value } | StackItem::Struct { value } => Some(value.clone()),
			_ => None,
		}
	}

	/// Returns a reference to the array value without cloning.
	///
	/// # Performance
	/// This is more efficient than `as_array()` when you only need to read the data,
	/// as it avoids cloning the `Vec<StackItem>` and all its elements.
	///
	/// # Example
	/// ```rust
	/// use neo3::neo_types::StackItem;
	///
	/// let stack_item = StackItem::Array { value: vec![StackItem::Integer { value: 1 }] };
	/// if let Some(arr) = stack_item.as_array_ref() {
	///     for item in arr {
	///         println!("{:?}", item);
	///     }
	/// }
	/// ```
	pub fn as_array_ref(&self) -> Option<&[StackItem]> {
		match self {
			StackItem::Array { value } | StackItem::Struct { value } => Some(value.as_slice()),
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
	///
	/// # Performance Note
	/// This method constructs a new `HashMap` by cloning all key-value pairs.
	/// For read-only access without HashMap conversion, prefer `as_map_entries()`.
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

	/// Returns a reference to the underlying map entries without conversion.
	///
	/// # Performance
	/// This is more efficient than `as_map()` when you only need to iterate over
	/// entries, as it avoids creating a `HashMap` and cloning all elements.
	/// Note that lookup by key is O(n) in the returned slice.
	///
	/// # Example
	/// ```rust
	/// use neo3::neo_types::{StackItem, MapEntry};
	///
	/// let stack_item = StackItem::Map {
	///     value: vec![MapEntry {
	///         key: StackItem::Integer { value: 1 },
	///         value: StackItem::Boolean { value: true },
	///     }],
	/// };
	/// if let Some(entries) = stack_item.as_map_entries() {
	///     for entry in entries {
	///         println!("{:?} -> {:?}", entry.key, entry.value);
	///     }
	/// }
	/// ```
	pub fn as_map_entries(&self) -> Option<&[MapEntry]> {
		match self {
			StackItem::Map { value } => Some(value.as_slice()),
			_ => None,
		}
	}

	/// Returns the `Address` value of a `StackItem::ByteString` or `StackItem::Buffer`.
	pub fn as_address(&self) -> Option<Address> {
		self.as_bytes().and_then(|mut bytes| {
			if bytes.len() != 20 {
				return None;
			}
			bytes.reverse();
			Some(H160::from_slice(&bytes).to_address())
		})
	}

	/// Returns the `Secp256r1PublicKey` value of a `StackItem::ByteString` or `StackItem::Buffer`.
	pub fn as_public_key(&self) -> Option<Secp256r1PublicKey> {
		self.as_bytes().and_then(|bytes| Secp256r1PublicKey::from_bytes(&bytes).ok())
	}

	/// Returns the `H160` value of a `StackItem::ByteString` or `StackItem::Buffer`.
	pub fn as_hash160(&self) -> Option<H160> {
		self.as_bytes().and_then(|bytes| {
			if bytes.len() != 20 {
				return None;
			}
			Some(H160::from_slice(&bytes))
		})
	}

	/// Returns the `H256` value of a `StackItem::ByteString` or `StackItem::Buffer`.
	pub fn as_hash256(&self) -> Option<H256> {
		self.as_bytes().and_then(|bytes| {
			if bytes.len() != 32 {
				return None;
			}
			Some(H256::from_slice(&bytes))
		})
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
		if let StackItem::InteropInterface { id, .. } = self { Some(id) } else { None }
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
		StackItem::ByteString {
			value: base64::engine::general_purpose::STANDARD.encode(value.as_bytes()),
		}
	}
}

impl From<H160> for StackItem {
	fn from(value: H160) -> Self {
		StackItem::ByteString {
			value: base64::engine::general_purpose::STANDARD.encode(value.as_bytes()),
		}
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
		if let Ok(v) = i64::try_from(value) {
			StackItem::Integer { value: v }
		} else {
			StackItem::ByteString {
				value: base64::engine::general_purpose::STANDARD.encode(value.to_le_bytes()),
			}
		}
	}
}
impl From<&str> for StackItem {
	fn from(value: &str) -> Self {
		StackItem::ByteString {
			value: base64::engine::general_purpose::STANDARD.encode(value.as_bytes()),
		}
	}
}
