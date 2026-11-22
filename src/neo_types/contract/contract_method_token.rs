use primitive_types::H160;
use serde::{Deserialize, Serialize};

use neo3::prelude::{ScriptHash, deserialize_script_hash, serialize_script_hash};

#[derive(Serialize, Deserialize, Debug, Hash, Clone, PartialEq)]
pub struct ContractMethodToken {
	#[serde(deserialize_with = "deserialize_script_hash")]
	#[serde(serialize_with = "serialize_script_hash")]
	hash: ScriptHash,
	method: String,
	#[serde(rename = "paramcount")]
	param_count: u32,
	#[serde(rename = "hasreturnvalue")]
	has_return_value: bool,
	#[serde(rename = "callflags")]
	call_flags: String,
}

impl ContractMethodToken {
	pub fn new(
		hash: H160,
		method: String,
		param_count: u32,
		has_return_value: bool,
		call_flags: String,
	) -> Self {
		Self { hash, method, param_count, has_return_value, call_flags }
	}
}
