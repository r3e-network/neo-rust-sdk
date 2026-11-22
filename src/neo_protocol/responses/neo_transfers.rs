use primitive_types::H256;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;

use neo3::prelude::{
	ScriptHash, deserialize_h256, deserialize_script_hash, serialize_h256, serialize_script_hash,
};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Nep11Transfers {
	pub sent: Vec<Nep11Transfer>,
	pub received: Vec<Nep11Transfer>,
	#[serde(rename = "address")]
	pub transfer_address: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Nep11Transfer {
	#[serde(rename = "tokenid")]
	pub token_id: String,
	pub timestamp: u64,
	#[serde(rename = "assethash")]
	#[serde(deserialize_with = "deserialize_script_hash")]
	#[serde(serialize_with = "serialize_script_hash")]
	pub asset_hash: ScriptHash,
	#[serde(rename = "transferaddress")]
	pub transfer_address: String,
	#[serde(deserialize_with = "deserialize_amount")]
	#[serde(serialize_with = "serialize_amount")]
	pub amount: u64,
	#[serde(rename = "blockindex")]
	pub block_index: u32,
	#[serde(rename = "transfernotifyindex")]
	pub transfer_notify_index: u32,
	#[serde(rename = "txhash")]
	#[serde(serialize_with = "serialize_h256")]
	#[serde(deserialize_with = "deserialize_h256")]
	pub tx_hash: H256,
}

impl Nep11Transfer {
	pub fn new(
		timestamp: u64,
		asset_hash: ScriptHash,
		transfer_address: String,
		amount: u64,
		block_index: u32,
		transfer_notify_index: u32,
		tx_hash: H256,
		token_id: String,
	) -> Self {
		Nep11Transfer {
			token_id,
			timestamp,
			asset_hash,
			transfer_address,
			amount,
			block_index,
			transfer_notify_index,
			tx_hash,
		}
	}
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Nep17Transfers {
	pub sent: Vec<Nep17Transfer>,
	pub received: Vec<Nep17Transfer>,
	#[serde(rename = "address")]
	pub transfer_address: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Nep17Transfer {
	pub timestamp: u64,
	#[serde(rename = "assethash")]
	#[serde(deserialize_with = "deserialize_script_hash")]
	#[serde(serialize_with = "serialize_script_hash")]
	pub asset_hash: ScriptHash,
	#[serde(rename = "transferaddress")]
	pub transfer_address: String,
	#[serde(deserialize_with = "deserialize_amount")]
	#[serde(serialize_with = "serialize_amount")]
	pub amount: u64,
	#[serde(rename = "blockindex")]
	pub block_index: u32,
	#[serde(rename = "transfernotifyindex")]
	pub transfer_notify_index: u32,
	#[serde(rename = "txhash")]
	#[serde(serialize_with = "serialize_h256")]
	#[serde(deserialize_with = "deserialize_h256")]
	pub tx_hash: H256,
}

impl Nep17Transfer {
	// Constructor function for Nep17Transfer
	pub fn new(
		timestamp: u64,
		asset_hash: ScriptHash,
		transfer_address: String,
		amount: u64,
		block_index: u32,
		transfer_notify_index: u32,
		tx_hash: H256,
	) -> Self {
		Self {
			timestamp,
			asset_hash,
			transfer_address,
			amount,
			block_index,
			transfer_notify_index,
			tx_hash,
		}
	}
}

// Custom deserialization function to convert a string into a u64
fn deserialize_amount<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
	D: Deserializer<'de>,
{
	let s = String::deserialize(deserializer)?;
	u64::from_str(&s).map_err(serde::de::Error::custom)
}

// Optional: custom serialization function if you want to serialize `u64` back into `String`
fn serialize_amount<S>(amount: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	serializer.serialize_str(&amount.to_string())
}
