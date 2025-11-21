use primitive_types::H160;
use serde::{Deserialize, Serialize};

use neo3::prelude::{deserialize_script_hash, serialize_script_hash, ScriptHash, StackItem};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct LogNotification {
	#[serde(deserialize_with = "deserialize_script_hash")]
	#[serde(serialize_with = "serialize_script_hash")]
	pub contract: ScriptHash,
	#[serde(rename = "eventname")]
	pub event_name: String,
	pub state: StackItem,
}

impl LogNotification {
	pub fn new(contract: H160, event_name: String, state: StackItem) -> Self {
		Self { contract, event_name, state }
	}
}
