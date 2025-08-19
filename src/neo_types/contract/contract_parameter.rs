use crate::{
	codec::NeoSerializable,
	crypto::Secp256r1PublicKey,
	deserialize_map,
	neo_contract::Role,
	neo_crypto::utils::{FromBase64String, FromHexString, ToHexString},
	neo_types::script_hash::ScriptHashExtension,
	serialize_map, Base64Encode, ContractParameterType, NNSName, NefFile, ValueExtension,
};
use getset::Getters;
use hex;
use primitive_types::{H160, H256};
use serde::{
	de,
	de::{MapAccess, Visitor},
	Deserialize, Deserializer, Serialize,
};
use serde_json::Value;
use sha3::Digest;
use std::{
	collections::HashMap,
	fmt,
	hash::{Hash, Hasher},
};
use strum_macros::{Display, EnumString};

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
pub struct ContractParameter2 {
	pub name: String,
	#[serde(rename = "type")]
	pub typ: ContractParameterType,
}

impl ContractParameter2 {
	pub fn new(name: String, typ: ContractParameterType) -> Self {
		Self { name, typ }
	}
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Clone, Getters)]
pub struct ContractParameter {
	#[getset(get = "pub")]
	#[serde(skip_serializing_if = "Option::is_none")]
	name: Option<String>,
	#[getset(get = "pub")]
	#[serde(rename = "type")]
	typ: ContractParameterType,
	#[getset(get = "pub")]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub value: Option<ParameterValue>,
}

impl<'de> Deserialize<'de> for ContractParameter {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		#[derive(Deserialize)]
		#[serde(field_identifier, rename_all = "lowercase")]
		enum Field {
			Name,
			#[serde(rename = "type")]
			Typ,
			Value,
		}

		struct ContractParameterVisitor;

		impl<'de> Visitor<'de> for ContractParameterVisitor {
			type Value = ContractParameter;

			fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
				formatter.write_str("struct ContractParameter")
			}

			fn visit_map<V>(self, mut map: V) -> Result<ContractParameter, V::Error>
			where
				V: MapAccess<'de>,
			{
				let mut name = None;
				let mut typ = None;
				let mut value = None;

				while let Some(key) = map.next_key()? {
					match key {
						Field::Name => {
							if name.is_some() {
								return Err(de::Error::duplicate_field("name"));
							}
							name = Some(map.next_value()?);
						},
						Field::Typ => {
							if typ.is_some() {
								return Err(de::Error::duplicate_field("type"));
							}
							typ = Some(map.next_value()?);
						},
						Field::Value => {
							if value.is_some() {
								return Err(de::Error::duplicate_field("value"));
							}
							value = Some(map.next_value()?);
						},
					}
				}

				let typ: ContractParameterType =
					typ.ok_or_else(|| de::Error::missing_field("type"))?;
				let value: Option<ParameterValue> = match typ {
					ContractParameterType::Boolean => value
						.map(|v| {
							serde_json::from_value(v).map(ParameterValue::Boolean).map_err(|e| {
								de::Error::custom(format!("Failed to deserialize Boolean: {}", e))
							})
						})
						.transpose()?,
					ContractParameterType::Integer => value
						.map(|v| {
							serde_json::from_value(v).map(ParameterValue::Integer).map_err(|e| {
								de::Error::custom(format!("Failed to deserialize Integer: {}", e))
							})
						})
						.transpose()?,
					ContractParameterType::ByteArray => value
						.map(|v| {
							serde_json::from_value(v).map(ParameterValue::ByteArray).map_err(|e| {
								de::Error::custom(format!("Failed to deserialize ByteArray: {}", e))
							})
						})
						.transpose()?,
					ContractParameterType::String => value
						.map(|v| {
							serde_json::from_value(v).map(ParameterValue::String).map_err(|e| {
								de::Error::custom(format!("Failed to deserialize String: {}", e))
							})
						})
						.transpose()?,
					ContractParameterType::H160 => value
						.map(|v| {
							serde_json::from_value(v).map(ParameterValue::H160).map_err(|e| {
								de::Error::custom(format!("Failed to deserialize H160: {}", e))
							})
						})
						.transpose()?,
					ContractParameterType::H256 => value
						.map(|v| {
							serde_json::from_value(v).map(ParameterValue::H256).map_err(|e| {
								de::Error::custom(format!("Failed to deserialize H256: {}", e))
							})
						})
						.transpose()?,
					ContractParameterType::PublicKey => value
						.map(|v| {
							serde_json::from_value(v).map(ParameterValue::PublicKey).map_err(|e| {
								de::Error::custom(format!("Failed to deserialize PublicKey: {}", e))
							})
						})
						.transpose()?,
					ContractParameterType::Signature => value
						.map(|v| {
							serde_json::from_value(v).map(ParameterValue::Signature).map_err(|e| {
								de::Error::custom(format!("Failed to deserialize Signature: {}", e))
							})
						})
						.transpose()?,
					ContractParameterType::Array => value
						.map(|v| {
							serde_json::from_value(v).map(ParameterValue::Array).map_err(|e| {
								de::Error::custom(format!("Failed to deserialize Array: {}", e))
							})
						})
						.transpose()?,
					ContractParameterType::Map => value
						.map(|v| {
							serde_json::from_value(v).map(ParameterValue::Map).map_err(|e| {
								de::Error::custom(format!("Failed to deserialize Map: {}", e))
							})
						})
						.transpose()?,
					ContractParameterType::Any => Some(ParameterValue::Any),
					_ => None,
				};

				Ok(ContractParameter { name, typ, value })
			}
		}

		const FIELDS: &[&str] = &["name", "type", "value"];
		deserializer.deserialize_struct("ContractParameter", FIELDS, ContractParameterVisitor)
	}
}

impl From<&H160> for ContractParameter {
	fn from(value: &H160) -> Self {
		Self::h160(value)
	}
}

impl From<H160> for ContractParameter {
	fn from(value: H160) -> Self {
		Self::h160(&value)
	}
}

impl From<u8> for ContractParameter {
	fn from(value: u8) -> Self {
		Self::integer(value as i64)
	}
}

impl From<i32> for ContractParameter {
	fn from(value: i32) -> Self {
		Self::integer(value as i64)
	}
}

impl From<u32> for ContractParameter {
	fn from(value: u32) -> Self {
		Self::integer(value as i64)
	}
}

impl From<u64> for ContractParameter {
	fn from(value: u64) -> Self {
		Self::integer(value as i64)
	}
}

impl From<&Role> for ContractParameter {
	fn from(value: &Role) -> Self {
		Self::integer(value.clone() as i64)
	}
}

impl From<&str> for ContractParameter {
	fn from(value: &str) -> Self {
		Self::string(value.to_string())
	}
}

impl From<usize> for ContractParameter {
	fn from(value: usize) -> Self {
		Self::integer(value as i64)
	}
}

impl From<&[u8]> for ContractParameter {
	fn from(value: &[u8]) -> Self {
		Self::byte_array(value.to_vec())
	}
}

impl From<Vec<u8>> for ContractParameter {
	fn from(value: Vec<u8>) -> Self {
		Self::byte_array(value)
	}
}

impl From<&Secp256r1PublicKey> for ContractParameter {
	fn from(value: &Secp256r1PublicKey) -> Self {
		Self::public_key(value)
	}
}

impl From<&H256> for ContractParameter {
	fn from(value: &H256) -> Self {
		Self::h256(value)
	}
}

impl From<&Vec<ContractParameter>> for ContractParameter {
	fn from(value: &Vec<ContractParameter>) -> Self {
		Self::array(value.clone())
	}
}

impl From<&NefFile> for ContractParameter {
	fn from(value: &NefFile) -> Self {
		Self::byte_array(value.to_array())
	}
}

impl From<String> for ContractParameter {
	fn from(value: String) -> Self {
		Self::string(value)
	}
}

impl From<bool> for ContractParameter {
	fn from(value: bool) -> Self {
		Self::bool(value)
	}
}
impl From<&String> for ContractParameter {
	fn from(value: &String) -> Self {
		Self::string(value.to_string())
	}
}

impl From<NNSName> for ContractParameter {
	fn from(value: NNSName) -> Self {
		Self::string(value.to_string())
	}
}

impl From<Value> for ContractParameter {
	fn from(value: Value) -> Self {
		match value {
			Value::Null => Self::new(ContractParameterType::Any),
			Value::Bool(b) => Self::bool(b),
			Value::Number(n) => {
				if let Some(i) = n.as_i64() {
					Self::integer(i)
				} else {
					// For numbers that can't be represented as i64, we'll use a string representation
					Self::string(n.to_string())
				}
			},
			Value::String(s) => Self::string(s),
			Value::Array(a) => {
				Self::array(a.into_iter().map(|v| ContractParameter::from(v)).collect())
			},
			Value::Object(o) => Self::map(ContractParameterMap::from_map(
				o.into_iter()
					.map(|(k, v)| (ContractParameter::from(k), ContractParameter::from(v)))
					.collect(),
			)),
		}
	}
}

impl ContractParameter {
	/// Creates a ContractParameter from a JSON value
	///
	/// # Arguments
	///
	/// * `value` - The JSON value to convert
	///
	/// # Returns
	///
	/// A new ContractParameter
	pub fn from_json(value: Value) -> Self {
		Self::from(value)
	}

	/// Safely extracts a public key from the parameter
	pub fn as_public_key(&self) -> Result<Secp256r1PublicKey, String> {
		match &self.value {
			Some(ParameterValue::PublicKey(bytes)) => {
				Secp256r1PublicKey::from_bytes(bytes.as_bytes())
					.map_err(|_| "Invalid public key bytes".to_string())
			},
			Some(ParameterValue::ByteArray(bytes)) => {
				Secp256r1PublicKey::from_bytes(bytes.as_bytes())
					.map_err(|_| "Invalid public key bytes".to_string())
			},
			_ => Err(format!(
				"Invalid type for public key conversion. Expected PublicKey or ByteArray, got {:?}",
				self.typ
			)),
		}
	}

	/// Safely extracts a signature from the parameter
	pub fn as_signature(&self) -> Result<Vec<u8>, String> {
		match &self.value {
			Some(ParameterValue::Signature(bytes)) => Ok(bytes.as_bytes().to_vec()),
			Some(ParameterValue::ByteArray(bytes)) => Ok(bytes.as_bytes().to_vec()),
			_ => Err(format!(
				"Invalid type for signature conversion. Expected Signature or ByteArray, got {:?}",
				self.typ
			)),
		}
	}

	/// Safely extracts bytes from the parameter
	pub fn as_bytes(&self) -> Result<Vec<u8>, String> {
		match &self.value {
			Some(ParameterValue::ByteArray(b)) => Ok(b.clone().into_bytes()),
			_ => Err(format!(
				"Cannot convert parameter of type {:?} to Vec<u8>. Expected ByteArray.",
				self.typ
			)),
		}
	}

	/// Safely extracts a string from the parameter
	pub fn as_string(&self) -> Result<String, String> {
		match &self.value {
			Some(ParameterValue::String(s)) => Ok(s.clone()),
			_ => Err(format!(
				"Cannot convert parameter of type {:?} to String. Expected String.",
				self.typ
			)),
		}
	}
}

impl Into<Value> for ContractParameter {
	fn into(self) -> Value {
		match self.value {
			Some(ParameterValue::Boolean(b)) => Value::Bool(b),
			Some(ParameterValue::Integer(i)) => Value::Number(serde_json::Number::from(i)),
			Some(ParameterValue::ByteArray(b)) => Value::String(b),
			Some(ParameterValue::String(s)) => Value::String(s),
			Some(ParameterValue::H160(h)) => Value::String(h),
			Some(ParameterValue::H256(h)) => Value::String(h),
			Some(ParameterValue::PublicKey(p)) => Value::String(p),
			Some(ParameterValue::Signature(s)) => Value::String(s),
			Some(ParameterValue::Array(a)) => {
				Value::Array(a.into_iter().map(|v| v.into()).collect())
			},
			Some(ParameterValue::Map(m)) => Value::Array(
				m.0.iter()
					.flat_map(|(key, value)| vec![key.clone().into(), value.clone().into()])
					.collect(),
			),
			Some(ParameterValue::Any) => Value::Null,
			None => Value::Null, // Handle the case where value is None
		}
	}
}

impl From<Vec<Value>> for ContractParameter {
	fn from(value: Vec<Value>) -> Self {
		Self::array(value.into_iter().map(|v| ContractParameter::from(v)).collect())
	}
}

impl ValueExtension for ContractParameter {
	fn to_value(&self) -> Value {
		match serde_json::to_string(self) {
			Ok(s) => Value::String(s),
			Err(e) => {
				// Professional error handling with logging for debugging purposes
				// Since the trait doesn't allow for Result, we'll log the error and return a null value
				eprintln!("Error serializing ContractParameter: {}", e);
				Value::Null
			},
		}
	}
}

impl ValueExtension for Vec<ContractParameter> {
	fn to_value(&self) -> Value {
		self.iter().map(|x| x.to_value()).collect()
	}
}

#[derive(Display, EnumString, Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
// #[serde(tag = "type", content = "content")]
#[serde(untagged)]
pub enum ParameterValue {
	Boolean(bool),
	Integer(i64),
	ByteArray(String),
	String(String),
	H160(String),
	H256(String),
	PublicKey(String),
	Signature(String),
	Array(Vec<ContractParameter>),
	Map(ContractParameterMap),
	Any,
}

impl Hash for ParameterValue {
	fn hash<H: Hasher>(&self, state: &mut H) {
		match self {
			ParameterValue::Boolean(b) => b.hash(state),
			ParameterValue::Integer(i) => i.hash(state),
			ParameterValue::ByteArray(b) => b.hash(state),
			ParameterValue::String(s) => s.hash(state),
			ParameterValue::H160(h) => h.hash(state),
			ParameterValue::H256(h) => h.hash(state),
			ParameterValue::PublicKey(p) => p.hash(state),
			ParameterValue::Signature(s) => s.hash(state),
			ParameterValue::Array(a) => a.hash(state),
			ParameterValue::Map(m) => {
				// Hash the map by iterating through its entries
				// Note: HashMap iteration order is not guaranteed, but for hashing purposes
				// we'll hash each key-value pair individually
				for (k, v) in &m.0 {
					k.hash(state);
					v.hash(state);
				}
			},
			ParameterValue::Any => "Any".hash(state),
		}
	}
}

impl ContractParameter {
	pub fn new(typ: ContractParameterType) -> Self {
		Self { name: None, typ, value: None }
	}

	pub fn get_type(&self) -> ContractParameterType {
		self.typ.clone()
	}

	pub fn with_value(typ: ContractParameterType, value: ParameterValue) -> Self {
		Self { name: None, typ, value: Some(value) }
	}

	pub fn bool(value: bool) -> Self {
		Self::with_value(ContractParameterType::Boolean, ParameterValue::Boolean(value))
	}

	pub fn to_bool(&self) -> Result<bool, String> {
		match self.value.as_ref() {
			Some(ParameterValue::Boolean(b)) => Ok(*b),
			Some(other) => Err(format!("Cannot convert {:?} to bool", other)),
			None => Err("Parameter value is None".to_string()),
		}
	}

	pub fn integer(value: i64) -> Self {
		Self::with_value(ContractParameterType::Integer, ParameterValue::Integer(value))
	}

	pub fn to_integer(&self) -> Result<i64, String> {
		match self.value.as_ref() {
			Some(ParameterValue::Integer(i)) => Ok(*i),
			Some(other) => Err(format!("Cannot convert {:?} to i64", other)),
			None => Err("Parameter value is None".to_string()),
		}
	}

	pub fn byte_array(value: Vec<u8>) -> Self {
		let encoded = value.to_base64();
		Self::with_value(ContractParameterType::ByteArray, ParameterValue::ByteArray(encoded))
	}

	pub fn to_byte_array(&self) -> Result<Vec<u8>, String> {
		match &self.value {
			Some(ParameterValue::ByteArray(b)) => {
				b.from_base64_string().map_err(|e| format!("Failed to decode base64: {}", e))
			},
			_ => Err(format!("Cannot convert {:?} to Vec<u8>", self)),
		}
	}

	pub fn string(value: String) -> Self {
		Self::with_value(ContractParameterType::String, ParameterValue::String(value))
	}

	pub fn to_string(&self) -> Result<String, String> {
		match self.value.as_ref() {
			Some(ParameterValue::String(s)) => Ok(s.clone()),
			Some(other) => Err(format!("Cannot convert {:?} to String", other)),
			None => Err("Parameter value is None".to_string()),
		}
	}
	pub fn h160(value: &H160) -> Self {
		Self::with_value(ContractParameterType::H160, ParameterValue::H160(value.to_hex()))
	}

	pub fn to_h160(&self) -> Result<H160, String> {
		match &self.value {
			Some(ParameterValue::H160(h)) => {
				let bytes =
					h.from_hex_string().map_err(|e| format!("Failed to decode hex: {}", e))?;
				if bytes.len() != 20 {
					return Err("Invalid H160 length".to_string());
				}
				let mut arr = [0u8; 20];
				arr.copy_from_slice(&bytes);
				Ok(H160(arr))
			},
			_ => Err(format!("Cannot convert {:?} to H160", self)),
		}
	}

	pub fn h256(value: &H256) -> Self {
		Self::with_value(ContractParameterType::H256, ParameterValue::H256(value.0.to_hex_string()))
	}

	pub fn to_h256(&self) -> Result<H256, String> {
		match &self.value {
			Some(ParameterValue::H256(h)) => {
				let bytes =
					h.from_hex_string().map_err(|e| format!("Failed to decode hex: {}", e))?;
				if bytes.len() != 32 {
					return Err("Invalid H256 length".to_string());
				}
				let mut arr = [0u8; 32];
				arr.copy_from_slice(&bytes);
				Ok(H256(arr))
			},
			_ => Err(format!("Cannot convert {:?} to H256", self)),
		}
	}

	pub fn public_key(value: &Secp256r1PublicKey) -> Self {
		Self::with_value(
			ContractParameterType::PublicKey,
			ParameterValue::PublicKey(hex::encode(value.get_encoded(true))),
		)
	}

	pub fn to_public_key(&self) -> Result<Secp256r1PublicKey, String> {
		match self.value.as_ref() {
			Some(ParameterValue::PublicKey(p)) => {
				let bytes = hex::decode(p).map_err(|e| format!("Failed to decode hex: {}", e))?;

				Secp256r1PublicKey::from_bytes(&bytes)
					.map_err(|e| format!("Failed to create public key: {}", e))
			},
			Some(other) => Err(format!("Cannot convert {:?} to PublicKey", other)),
			None => Err("Parameter value is None".to_string()),
		}
	}

	pub fn signature(value: &str) -> Self {
		Self::with_value(
			ContractParameterType::Signature,
			ParameterValue::Signature(value.to_string()),
		)
	}

	pub fn to_signature(&self) -> Result<String, String> {
		match self.value.as_ref() {
			Some(ParameterValue::Signature(s)) => Ok(s.clone()),
			Some(other) => Err(format!("Cannot convert {:?} to Signature", other)),
			None => Err("Parameter value is None".to_string()),
		}
	}

	pub fn array(values: Vec<Self>) -> Self {
		Self::with_value(ContractParameterType::Array, ParameterValue::Array(values))
	}

	pub fn to_array(&self) -> Result<Vec<ContractParameter>, String> {
		match self.value.as_ref() {
			Some(ParameterValue::Array(a)) => Ok(a.clone()),
			Some(other) => Err(format!("Cannot convert {:?} to Vec<ContractParameter>", other)),
			None => Err("Parameter value is None".to_string()),
		}
	}

	pub fn map(values: ContractParameterMap) -> Self {
		Self::with_value(ContractParameterType::Map, ParameterValue::Map(values))
	}

	pub fn to_map(&self) -> Result<ContractParameterMap, String> {
		match self.value.as_ref() {
			Some(ParameterValue::Map(m)) => Ok(m.clone()),
			Some(other) => Err(format!(
				"Cannot convert {:?} to HashMap<ContractParameter, ContractParameter>",
				other
			)),
			None => Err("Parameter value is None".to_string()),
		}
	}

	pub fn any() -> Self {
		Self::new(ContractParameterType::Any)
	}

	pub fn hash(self) -> Vec<u8> {
		let mut hasher = std::collections::hash_map::DefaultHasher::new();
		Hash::hash(&self, &mut hasher);
		hasher.finish().to_be_bytes().to_vec()
	}
}

#[derive(Default, Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct ContractParameterMap(
	#[serde(serialize_with = "serialize_map", deserialize_with = "deserialize_map")]
	pub  HashMap<ContractParameter, ContractParameter>,
);

impl ContractParameterMap {
	pub fn new() -> Self {
		Self(HashMap::new())
	}

	pub fn from_map(map: HashMap<ContractParameter, ContractParameter>) -> Self {
		Self(map)
	}

	pub fn to_map(&mut self) -> &HashMap<ContractParameter, ContractParameter> {
		&mut self.0
	}
}

impl ParameterValue {
	pub fn to_bool(&self) -> Result<bool, String> {
		match self {
			ParameterValue::Boolean(b) => Ok(*b),
			_ => Err(format!("Cannot convert {:?} to bool", self)),
		}
	}

	pub fn to_integer(&self) -> Result<i64, String> {
		match self {
			ParameterValue::Integer(i) => Ok(*i),
			_ => Err(format!("Cannot convert {:?} to i64", self)),
		}
	}

	pub fn to_byte_array(&self) -> Result<Vec<u8>, String> {
		match self {
			ParameterValue::ByteArray(b) => {
				b.from_base64_string().map_err(|e| format!("Failed to decode base64: {}", e))
			},
			_ => Err(format!("Cannot convert {:?} to Vec<u8>", self)),
		}
	}

	pub fn to_string(&self) -> Result<String, String> {
		match self {
			ParameterValue::String(s) => Ok(s.clone()),
			_ => Err(format!("Cannot convert {:?} to String", self)),
		}
	}

	pub fn to_h160(&self) -> Result<H160, String> {
		match self {
			ParameterValue::H160(h) => {
				let bytes =
					h.from_hex_string().map_err(|e| format!("Failed to decode hex: {}", e))?;
				if bytes.len() != 20 {
					return Err("Invalid H160 length".to_string());
				}
				let mut arr = [0u8; 20];
				arr.copy_from_slice(&bytes);
				Ok(H160(arr))
			},
			_ => Err(format!("Cannot convert {:?} to H160", self)),
		}
	}

	pub fn to_h256(&self) -> Result<H256, String> {
		match self {
			ParameterValue::H256(h) => {
				let bytes =
					h.from_hex_string().map_err(|e| format!("Failed to decode hex: {}", e))?;
				if bytes.len() != 32 {
					return Err("Invalid H256 length".to_string());
				}
				let mut arr = [0u8; 32];
				arr.copy_from_slice(&bytes);
				Ok(H256(arr))
			},
			_ => Err(format!("Cannot convert {:?} to H256", self)),
		}
	}

	pub fn to_public_key(&self) -> Result<Secp256r1PublicKey, String> {
		match self {
			ParameterValue::PublicKey(p) => {
				let bytes = hex::decode(p).map_err(|e| format!("Failed to decode hex: {}", e))?;

				Secp256r1PublicKey::from_bytes(&bytes)
					.map_err(|e| format!("Failed to create public key: {}", e))
			},
			_ => Err(format!("Cannot convert {:?} to PublicKey", self)),
		}
	}

	pub fn to_signature(&self) -> Result<String, String> {
		match self {
			ParameterValue::Signature(s) => Ok(s.clone()),
			_ => Err(format!("Cannot convert {:?} to Signature", self)),
		}
	}

	pub fn to_array(&self) -> Result<Vec<ContractParameter>, String> {
		match self {
			ParameterValue::Array(a) => Ok(a.clone()),
			_ => Err(format!("Cannot convert {:?} to Vec<ContractParameter>", self)),
		}
	}

	pub fn to_map(&self) -> Result<ContractParameterMap, String> {
		match self {
			ParameterValue::Map(m) => Ok(m.clone()),
			_ => Err(format!(
				"Cannot convert {:?} to HashMap<ContractParameter, ContractParameter>",
				self
			)),
		}
	}

	pub fn hash(&self) -> Vec<u8> {
		let mut hasher = std::collections::hash_map::DefaultHasher::new();
		Hash::hash(&self, &mut hasher);
		hasher.finish().to_be_bytes().to_vec()
	}
}

#[cfg(test)]
mod tests {
	use primitive_types::{H160, H256};

	use crate::{crypto::Secp256r1PublicKey, neo_types::ContractParameterMap};
	use neo3::prelude::{ContractParameter, ContractParameterType};

	#[test]
	fn test_string_from_string() {
		let param = ContractParameter::string("value".to_string());
		// assert_param(&param, "value", ContractParameterType::String);
		assert_eq!(param.typ, ContractParameterType::String);
		assert_eq!(
			param
				.value
				.as_ref()
				.expect("Parameter value should not be None")
				.to_string()
				.expect("Should be able to convert to string"),
			"value"
		);
	}

	#[test]
	fn test_bytes_from_bytes() {
		let bytes = vec![0x01, 0x01];
		let param = ContractParameter::byte_array(bytes.clone());
		// assert_param(&param, bytes, ContractParameterType::ByteArray);
		assert_eq!(param.typ, ContractParameterType::ByteArray);
		assert_eq!(
			param
				.value
				.as_ref()
				.expect("Parameter value should not be None")
				.to_byte_array()
				.expect("Should be able to convert to byte array"),
			bytes
		);
	}

	#[test]
	fn test_bytes_from_hex_string() {
		let param = ContractParameter::byte_array(
			hex::decode("a602").expect("Should be able to decode valid hex string in test"),
		);
		// assert_param(&param, vec![0xa6, 0x02], ContractParameterType::ByteArray);
		assert_eq!(param.typ, ContractParameterType::ByteArray);
		assert_eq!(
			param
				.value
				.as_ref()
				.expect("Parameter value should not be None")
				.to_byte_array()
				.expect("Should be able to convert to byte array"),
			vec![0xa6, 0x02]
		);
	}

	#[test]
	fn test_array_from_array() {
		let params = vec![
			ContractParameter::string("value".to_string()),
			ContractParameter::byte_array(
				hex::decode("0101").expect("Should be able to decode valid hex string in test"),
			),
		];

		let param = ContractParameter::array(params.clone());
		// assert_param(&param, params, ContractParameterType::Array);
		assert_eq!(param.typ, ContractParameterType::Array);
		assert_eq!(
			param
				.value
				.as_ref()
				.expect("Parameter value should not be None")
				.to_array()
				.expect("Should be able to convert to array"),
			params
		);
	}

	#[test]
	fn test_array_from_empty() {
		let param = ContractParameter::array(Vec::new());

		// assert!(matches!(param.value, Some([])));
	}

	#[test]
	fn test_nested_array() {
		let nested_params = vec![
			ContractParameter::integer(420),
			ContractParameter::integer(1024),
			ContractParameter::string("neow3j:)".to_string()),
			ContractParameter::integer(10),
		];

		let params = vec![
			ContractParameter::string("value".to_string()),
			ContractParameter::byte_array(
				hex::decode("0101").expect("Should be able to decode valid hex string in test"),
			),
			ContractParameter::array(nested_params),
			ContractParameter::integer(55),
		];

		let param = ContractParameter::array(params);

		assert_eq!(param.typ, ContractParameterType::Array);

		// let nested_vec = nested.value.as_ref().expect("Parameter value should not be None");
		// assert_eq!(nested_vec.len(), 4);
		//
		// let nested_nested = &nested_vec[3];
		// assert_eq!(nested_nested.typ, ContractParameterType::Array);
	}

	#[test]
	fn test_map() {
		let mut map = ContractParameterMap::new();
		map.0
			.insert(ContractParameter::integer(1), ContractParameter::string("first".to_string()));

		let param = ContractParameter::map(map);

		assert_eq!(param.typ, ContractParameterType::Map);
		let map = param.value.as_ref().expect("Parameter value should not be None");

		let map = map.to_map().expect("Should be able to convert to map");
		let (key, val) = map.0.iter().next().expect("Map should not be empty in test");
		assert_eq!(*key, ContractParameter::integer(1));
		assert_eq!(*val, ContractParameter::string("first".to_string()));
	}

	#[test]
	fn test_nested_map() {
		let inner_map = {
			let mut map = ContractParameterMap::new();
			map.0.insert(
				ContractParameter::string("halo".to_string()),
				ContractParameter::integer(1234),
			);
			ContractParameter::map(map)
		};

		let mut map = ContractParameterMap::new();
		map.0.insert(ContractParameter::integer(16), inner_map);

		let param = ContractParameter::map(map);

		let outer_map = param.value.as_ref().expect("Parameter value should not be None");
		assert_eq!(outer_map.to_map().expect("Should be able to convert to map").0.len(), 1);

		let outer_map = outer_map.to_map().expect("Should be able to convert to map");
		let inner_param = outer_map
			.0
			.get(&ContractParameter::integer(16))
			.expect("Map should contain key 16 in test");
		let inner_map =
			inner_param.value.as_ref().expect("Inner parameter value should not be None");

		assert_eq!(inner_map.to_map().expect("Should be able to convert to map").0.len(), 1);
		let inner_map = inner_map.to_map().expect("Should be able to convert to map");
		let (key, val) = inner_map.0.iter().next().expect("Inner map should not be empty in test");
		assert_eq!(*key, ContractParameter::string("halo".to_string()));
		assert_eq!(*val, ContractParameter::integer(1234));
	}

	#[test]
	fn test_serialize_deserialize() {
		let array_param_1 = ContractParameter::integer(1000);
		let array_param_2 = ContractParameter::integer(2000);

		let mut inner_map = ContractParameterMap::new();
		inner_map
			.0
			.insert(ContractParameter::integer(5), ContractParameter::string("value".to_string()));
		inner_map.0.insert(
			ContractParameter::byte_array(vec![0x01, 0x02, 0x03]),
			ContractParameter::integer(5),
		);
		let inner_map_param = ContractParameter::map(inner_map);

		let array_params = vec![array_param_1, array_param_2, inner_map_param];

		let param = ContractParameter::array(array_params);

		// Serialize
		let json = serde_json::to_string(&param)
			.expect("Should be able to serialize ContractParameter to JSON in test");

		// Deserialize
		let deserialized: ContractParameter = serde_json::from_str(&json)
			.expect("Should be able to deserialize ContractParameter from JSON in test");

		// Assert
		assert_eq!(deserialized, param);

		// Round trip
		let roundtrip_json = serde_json::to_string(&deserialized)
			.expect("Should be able to serialize ContractParameter to JSON in test");
		let roundtrip = serde_json::from_str::<ContractParameter>(&roundtrip_json)
			.expect("Should be able to deserialize ContractParameter from JSON in test");

		assert_eq!(roundtrip, param);
	}
	#[test]
	fn test_bytes_equals() {
		let param1 = ContractParameter::byte_array(
			hex::decode("796573").expect("Should be able to decode valid hex string in test"),
		);
		let param2 = ContractParameter::byte_array(vec![0x79, 0x65, 0x73]);
		assert_eq!(param1, param2);
	}

	#[test]
	fn test_bytes_from_string() {
		let param = ContractParameter::byte_array("Neo".as_bytes().to_vec());
		// assert_param(&param, b"Neo", ContractParameterType::ByteArray);
		assert_eq!(param.typ, ContractParameterType::ByteArray);
		assert_eq!(
			param
				.value
				.as_ref()
				.expect("Parameter value should not be None")
				.to_byte_array()
				.expect("Should be able to convert to byte array"),
			b"Neo"
		);
	}

	#[test]
	fn test_bool() {
		let param = ContractParameter::bool(false);
		// assert_param(&param, false, ContractParameterType::Boolean);
		assert_eq!(param.typ, ContractParameterType::Boolean);
		assert_eq!(
			param
				.value
				.as_ref()
				.expect("Parameter value should not be None")
				.to_bool()
				.expect("Should be able to convert to bool"),
			false
		);
	}

	#[test]
	fn test_int() {
		let param = ContractParameter::integer(10);
		// assert_param(&param, 10, ContractParameterType::Integer);
		assert_eq!(param.typ, ContractParameterType::Integer);
		assert_eq!(
			param
				.value
				.as_ref()
				.expect("Parameter value should not be None")
				.to_integer()
				.expect("Should be able to convert to integer"),
			10
		);
	}

	#[test]
	fn test_H160() {
		let hash = H160::from([0u8; 20]);
		let param = ContractParameter::h160(&hash);
		// assert_param(&param, hash.into(), ContractParameterType::H160);
		assert_eq!(param.typ, ContractParameterType::H160);
		assert_eq!(
			param
				.value
				.as_ref()
				.expect("Parameter value should not be None")
				.to_h160()
				.expect("Should be able to convert to H160"),
			hash
		);
	}

	#[test]
	fn test_H256() {
		let hash = H256::from([0u8; 32]);
		let param = ContractParameter::h256(&hash);
		// assert_param(&param, hash.into(), ContractParameterType::H256);
		assert_eq!(param.typ, ContractParameterType::H256);
		assert_eq!(
			param
				.value
				.as_ref()
				.expect("Parameter value should not be None")
				.to_h256()
				.expect("Should be able to convert to H256"),
			hash
		);
	}

	#[test]
	fn test_public_key() {
		let key = hex::decode("03b4af8efe55d98b44eedfcfaa39642fd5d53ad543d18d3cc2db5880970a4654f6")
			.expect("Should be able to decode valid hex string in test")
			.to_vec();
		let key = Secp256r1PublicKey::from_bytes(&key)
			.expect("Should be able to create public key from valid bytes in test");
		let param = ContractParameter::public_key(&key);
		// assert_param(&param, key, ContractParameterType::PublicKey);
		assert_eq!(param.typ, ContractParameterType::PublicKey);
		assert_eq!(
			param
				.value
				.as_ref()
				.expect("Parameter value should not be None")
				.to_public_key()
				.expect("Should be able to convert to public key"),
			key
		);
	}

	#[test]
	fn test_signature() {
		let sig = "010203..."; // 64 byte signature
		let param = ContractParameter::signature(sig);
		// assert_param(&param, sig, ContractParameterType::Signature);
		assert_eq!(param.typ, ContractParameterType::Signature);
		assert_eq!(
			param
				.value
				.as_ref()
				.expect("Parameter value should not be None")
				.to_signature()
				.expect("Should be able to convert to signature"),
			sig
		);
	}

	#[test]
	fn create_from_various_types() {
		let string_param = ContractParameter::from("hello");
		// assert_param(&string_param, "hello".as_bytes(), ContractParameterType::String);

		assert_eq!(string_param.typ, ContractParameterType::String);
		assert_eq!(
			string_param
				.value
				.as_ref()
				.expect("Parameter value should not be None")
				.to_string()
				.expect("Should be able to convert to string"),
			"hello"
		);

		let bool_param = ContractParameter::from(true);
		// assert_param(&bool_param, true, ContractParameterType::Boolean);
		assert_eq!(bool_param.typ, ContractParameterType::Boolean);
		assert_eq!(
			bool_param
				.value
				.as_ref()
				.expect("Parameter value should not be None")
				.to_bool()
				.expect("Should be able to convert to bool"),
			true
		);

		let int_param = ContractParameter::from(10);
		// assert_param(&int_param, 10, ContractParameterType::Integer);
		assert_eq!(int_param.typ, ContractParameterType::Integer);
		assert_eq!(
			int_param
				.value
				.as_ref()
				.expect("Parameter value should not be None")
				.to_integer()
				.expect("Should be able to convert to integer"),
			10
		);
	}

	#[test]
	fn create_array_from_vec() {
		let vec = vec![ContractParameter::from(1), ContractParameter::from("test")];

		let param = ContractParameter::from(&vec);

		assert_eq!(param.typ, ContractParameterType::Array);

		let array = param
			.value
			.as_ref()
			.expect("Parameter value should not be None")
			.to_array()
			.expect("Should be able to convert to array");
		assert_eq!(array.len(), 2);
		// assert_param(&array[0], 1, ContractParameterType::Integer);
		assert_eq!(&array[0].typ, &ContractParameterType::Integer);
		assert_eq!(
			&array[0]
				.value
				.clone()
				.expect("Parameter value should not be None")
				.to_integer()
				.expect("Should be able to convert to integer"),
			&1
		);
		// assert_param(&array[1], "test".as_bytes(), ContractParameterType::String);
		assert_eq!(&array[1].typ, &ContractParameterType::String);
		assert_eq!(
			&array[1]
				.value
				.clone()
				.expect("Parameter value should not be None")
				.to_string()
				.expect("Should be able to convert to string"),
			"test"
		);
	}

	#[test]
	fn create_map_from_hashmap() {
		let mut map = ContractParameterMap::new();
		map.0.insert("key".to_owned().into(), ContractParameter::from(1));

		let param = ContractParameter::map(map);

		assert_eq!(param.typ, ContractParameterType::Map);

		let map = param.value.as_ref().expect("Parameter value should not be None");

		let map = map.to_map().expect("Should be able to convert to map");
		let (key, val) = map.0.iter().next().expect("Map should not be empty in test");
		assert_eq!(key.typ, ContractParameterType::String);
		assert_eq!(
			key.value
				.clone()
				.expect("Parameter value should not be None")
				.to_string()
				.expect("Should be able to convert to string"),
			"key"
		);
	}

	#[test]
	fn equality_operator() {
		let p1 = ContractParameter::from(1);
		let p2 = ContractParameter::from(1);

		assert_eq!(p1, p2);

		let p3 = ContractParameter::from("test");
		assert_ne!(p1, p3);
	}

	// #[test]
	// fn invalid_type_errors() {
	// 	let result = ContractParameter::from(MyStruct);
	//
	// 	assert!(result.is_err());
	// 	assert_eq!(result.err(), Some(InvalidTypeError));
	// }
}
