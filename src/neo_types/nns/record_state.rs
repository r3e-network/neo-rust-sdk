use std::hash::Hash;

use serde::{Deserialize, Serialize};

use crate::neo_types::StackItem;

use super::RecordType;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecordState {
	pub name: String,
	pub record_type: RecordType,
	pub data: String,
}

impl RecordState {
	pub fn new(name: String, record_type: RecordType, data: String) -> Self {
		Self { name, record_type, data }
	}

	pub fn from_stack_item(item: &StackItem) -> Result<Self, &'static str> {
		match item {
			StackItem::Array { value: vec } if vec.len() == 3 => {
				if let Some(name) = vec[0].as_string()
					&& let Some(byte) = vec[1].as_int()
					&& let Ok(record_type) = RecordType::try_from(byte as u8)
					&& let Some(data) = vec[2].as_string()
				{
					return Ok(Self::new(name, record_type, data));
				}
				Err("Could not deserialize RecordState")
			},
			_ => Err("Expected a StackItem array of length 3"),
		}
	}
}
