use crate::builder::OracleResponseCode;
use primitive_types::H256;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionAttributeType {
	HighPriority,
	OracleResponse,
	NotValidBefore,
	Conflicts,
	// Add other types as needed
}

// pub trait TransactionAttribute {
//     fn get_type(&self) -> TransactionAttributeType;
// }

#[derive(Debug, Serialize, Deserialize, Hash, Clone, PartialEq)]
pub struct HighPriorityAttribute {
	// #[serde(rename = "type")]
	// pub attribute_type: TransactionAttributeType,
	// Add other fields specific to HighPriorityAttribute if needed
}

// impl TransactionAttribute for HighPriorityAttribute {
//     fn get_type(&self) -> TransactionAttributeType {
//         self.attribute_type.clone()
//     }
// }

#[derive(Debug, Serialize, Deserialize, Hash, Clone, PartialEq)]
pub struct OracleResponseAttribute {
	// #[serde(rename = "type")]
	// pub attribute_type: TransactionAttributeType,
	#[serde(flatten)]
	pub oracle_response: OracleResponse,
	// Add other fields specific to OracleResponseAttribute if needed
}

// impl TransactionAttribute for OracleResponseAttribute {
//     fn get_type(&self) -> TransactionAttributeType {
//         self.attribute_type.clone()
//     }
// }

// NotValidBeforeAttribute Struct and Implementation
#[derive(Debug, Serialize, Deserialize, Hash, Clone, PartialEq)]
pub struct NotValidBeforeAttribute {
	// #[serde(rename = "type")]
	// pub attribute_type: TransactionAttributeType,
	#[serde(rename = "height", deserialize_with = "deserialize_height")]
	pub height: i64,
	// Add other fields specific to NotValidBeforeAttribute if needed
}

// impl TransactionAttribute for NotValidBeforeAttribute {
//     fn get_type(&self) -> TransactionAttributeType {
//         self.attribute_type.clone()
//     }
// }

// ConflictsAttribute Struct and Implementation
#[derive(Debug, Serialize, Deserialize, Hash, Clone, PartialEq)]
pub struct ConflictsAttribute {
	// #[serde(rename = "type")]
	// pub attribute_type: TransactionAttributeType,
	#[serde(rename = "hash")]
	pub hash: H256,
	// Add other fields specific to ConflictsAttribute if needed
}

// impl TransactionAttribute for ConflictsAttribute {
//     fn get_type(&self) -> TransactionAttributeType {
//         self.attribute_type.clone()
//     }
// }

// Add similar structs and implementations for NotValidBeforeAttribute, ConflictsAttribute, etc.
#[derive(Debug, Serialize, Deserialize, Hash, Clone, PartialEq)]
#[serde(tag = "type")] // Uses the "type" field in the JSON to determine the variant
pub enum TransactionAttributeEnum {
	#[serde(rename = "HighPriority")]
	HighPriority(HighPriorityAttribute),

	#[serde(rename = "OracleResponse")]
	OracleResponse(OracleResponseAttribute),

	#[serde(rename = "NotValidBefore")]
	NotValidBefore(NotValidBeforeAttribute),

	#[serde(rename = "Conflicts")]
	Conflicts(ConflictsAttribute),
	// Add other variants as needed
}

#[derive(Serialize, Deserialize, PartialEq, Hash, Debug, Clone)]
pub struct OracleResponse {
	pub(crate) id: u32,
	#[serde(rename = "code")]
	pub(crate) response_code: OracleResponseCode,
	pub(crate) result: String,
}

// Custom deserialization function for height
fn deserialize_height<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
	D: Deserializer<'de>,
{
	let value: serde_json::Value = Deserialize::deserialize(deserializer)?;
	match value {
		serde_json::Value::Number(num) => {
			num.as_i64().ok_or_else(|| serde::de::Error::custom("invalid number"))
		},
		serde_json::Value::String(s) => s.parse::<i64>().map_err(serde::de::Error::custom),
		_ => Err(serde::de::Error::custom("invalid type for height")),
	}
}
