use super::ScriptHashExtension;
use crate::{Bytes, crypto::Secp256r1PublicKey};
use primitive_types::{H160, H256};
use serde_json::Value;

pub trait ValueExtension {
	fn to_value(&self) -> Value;
}

impl ValueExtension for Bytes {
	fn to_value(&self) -> Value {
		Value::String(hex::encode(self))
	}
}

impl ValueExtension for String {
	fn to_value(&self) -> Value {
		Value::String(self.clone())
	}
}

impl ValueExtension for &str {
	fn to_value(&self) -> Value {
		Value::String(self.to_string())
	}
}

impl ValueExtension for H160 {
	fn to_value(&self) -> Value {
		Value::String(ScriptHashExtension::to_hex(self))
	}
}

impl ValueExtension for Secp256r1PublicKey {
	fn to_value(&self) -> Value {
		Value::String(hex::encode(self.get_encoded(true)))
	}
}

impl ValueExtension for H256 {
	fn to_value(&self) -> Value {
		Value::String(hex::encode(self))
	}
}

impl ValueExtension for u32 {
	fn to_value(&self) -> Value {
		Value::Number(serde_json::Number::from(*self))
	}
}

impl ValueExtension for u64 {
	fn to_value(&self) -> Value {
		Value::Number(serde_json::Number::from(*self))
	}
}

impl ValueExtension for i32 {
	fn to_value(&self) -> Value {
		Value::Number(serde_json::Number::from(*self))
	}
}

impl ValueExtension for i64 {
	fn to_value(&self) -> Value {
		Value::Number(serde_json::Number::from(*self))
	}
}

impl ValueExtension for bool {
	fn to_value(&self) -> Value {
		Value::Bool(*self)
	}
}
