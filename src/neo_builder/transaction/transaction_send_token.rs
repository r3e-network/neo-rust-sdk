use getset::Getters;
use primitive_types::H160;
use serde::{Deserialize, Serialize};

use neo3::prelude::{deserialize_script_hash, serialize_script_hash};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Getters)]
pub struct TransactionSendToken {
	#[serde(rename = "asset")]
	#[serde(deserialize_with = "deserialize_script_hash")]
	#[serde(serialize_with = "serialize_script_hash")]
	pub token: H160,
	pub value: i32,
	// #[serde(deserialize_with = "deserialize_script_hash")]
	// #[serde(serialize_with = "serialize_script_hash")]
	pub address: String,
}

impl TransactionSendToken {
	pub fn new(token: H160, value: i32, address: String) -> Self {
		Self { token, value, address }
	}
}
