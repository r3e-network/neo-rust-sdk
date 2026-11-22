use primitive_types::H160;
use serde::{Deserialize, Serialize};

use crate::ContractNef;
use neo3::prelude::{
	ContractManifest, InvocationResult, StackItem, deserialize_script_hash, serialize_script_hash,
	*,
};

#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq)]
pub struct ContractState {
	#[serde(default)]
	pub id: i32,
	pub nef: ContractNef,
	#[serde(rename = "updatecounter", default)]
	pub update_counter: i32,
	#[serde(serialize_with = "serialize_h160", deserialize_with = "deserialize_h160", default)]
	pub hash: H160,
	pub manifest: ContractManifest,
}

impl ContractState {
	pub fn new(
		id: i32,
		update_counter: i32,
		hash: H160,
		nef: ContractNef,
		manifest: ContractManifest,
	) -> Self {
		Self { id, nef, update_counter, hash, manifest }
	}

	pub fn contract_identifiers(
		stack_item: &StackItem,
	) -> Result<ContractIdentifiers, &'static str> {
		match stack_item {
			StackItem::Struct { value } if value.len() >= 2 => {
				let id = value[0]
					.as_int()
					.ok_or("Failed to get contract ID as integer from stack item")?;

				let mut v = value[1]
					.as_bytes()
					.ok_or("Failed to get contract hash as bytes from stack item")?;

				if v.len() != 20 {
					return Err("Contract hash must be 20 bytes");
				}
				v.reverse();
				let hash = H160::from_slice(&v);
				Ok(ContractIdentifiers { id: id as i32, hash })
			},
			_ => Err("Could not deserialize ContractIdentifiers from stack item"),
		}
	}
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ContractIdentifiers {
	pub id: i32,
	#[serde(deserialize_with = "deserialize_script_hash")]
	#[serde(serialize_with = "serialize_script_hash")]
	pub hash: H160,
}

impl ContractIdentifiers {
	/// Safely converts from InvocationResult
	pub fn from_invocation_result(result: InvocationResult) -> Result<Self, String> {
		if result.stack.is_empty() {
			return Err("InvocationResult has empty stack".to_string());
		}

		let stack_item = &result.stack[0];
		ContractState::contract_identifiers(stack_item).map_err(|e| {
			format!("Failed to convert InvocationResult to ContractIdentifiers: {}", e)
		})
	}
}
