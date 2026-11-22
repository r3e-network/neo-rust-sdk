use crate::{
	NeoVMStateType,
	builder::{TransactionAttribute, TransactionSigner, WitnessRule, WitnessScope},
	deserialize_h256, deserialize_h256_option,
	neo_protocol::NeoWitness,
	serialize_h256, serialize_h256_option,
};
use primitive_types::{H160, H256};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Hash, Clone, Debug)]
pub struct TransactionResult {
	#[serde(serialize_with = "serialize_h256")]
	#[serde(deserialize_with = "deserialize_h256")]
	pub hash: H256,
	pub size: i32,
	pub version: i32,
	pub nonce: u32,
	pub sender: String,
	#[serde(rename = "sysfee")]
	pub sys_fee: String,
	#[serde(rename = "netfee")]
	pub net_fee: String,
	#[serde(rename = "validuntilblock")]
	pub valid_until_block: i32,
	pub signers: Vec<TransactionSigner>,
	pub attributes: Vec<TransactionAttribute>,
	pub script: String,
	pub witnesses: Vec<NeoWitness>,
	#[serde(rename = "blockhash")]
	#[serde(serialize_with = "serialize_h256_option")]
	#[serde(deserialize_with = "deserialize_h256_option")]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub block_hash: Option<H256>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub confirmations: Option<i32>,
	#[serde(rename = "blocktime")]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub block_time: Option<u64>,
	#[serde(rename = "vmstate")]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub vm_state: Option<NeoVMStateType>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct NeoTransactionSigner {
	account: H160,
	scopes: Vec<WitnessScope>,
	allowed_contracts: Option<Vec<String>>,
	allowed_groups: Option<Vec<String>>,
	rules: Option<Vec<WitnessRule>>,
}
