use super::{RTransactionSigner, TransactionAttributeEnum};
use crate::{neo_protocol::NeoWitness, TypeError};
use getset::{CopyGetters, Getters, MutGetters, Setters};
use neo3::VMState;
use primitive_types::H256;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Getters, Setters, MutGetters, CopyGetters, Debug, Clone)]
pub struct RTransaction {
	#[serde(rename = "hash")]
	#[getset(get = "pub", set = "pub")]
	pub hash: H256,

	#[serde(rename = "size")]
	#[getset(get = "pub", set = "pub")]
	pub size: u64,

	#[serde(rename = "version")]
	#[getset(get = "pub", set = "pub")]
	pub version: u8,

	#[serde(rename = "nonce")]
	#[getset(get = "pub", set = "pub")]
	pub nonce: u64,

	#[serde(rename = "sender")]
	#[getset(get = "pub", set = "pub")]
	pub sender: String,

	#[serde(rename = "sysfee")]
	#[getset(get = "pub", set = "pub")]
	pub sys_fee: String,

	#[serde(rename = "netfee")]
	#[getset(get = "pub", set = "pub")]
	pub net_fee: String,

	#[serde(rename = "validuntilblock")]
	#[getset(get = "pub", set = "pub")]
	pub valid_until_block: u64,

	#[serde(rename = "signers", default)]
	#[getset(get = "pub", set = "pub")]
	pub signers: Vec<RTransactionSigner>,

	#[serde(rename = "attributes", default)]
	#[getset(get = "pub", set = "pub")]
	pub attributes: Vec<TransactionAttributeEnum>,

	#[serde(rename = "script")]
	#[getset(get = "pub", set = "pub")]
	pub script: String,

	#[serde(rename = "witnesses", default)]
	#[getset(get = "pub", set = "pub")]
	pub witnesses: Vec<NeoWitness>,

	#[serde(rename = "blockhash", default)]
	#[getset(get = "pub", set = "pub")]
	pub block_hash: H256,

	#[serde(rename = "confirmations", default)]
	#[getset(get = "pub", set = "pub")]
	pub confirmations: i32,

	#[serde(rename = "blocktime", default)]
	#[getset(get = "pub", set = "pub")]
	pub block_time: i64,

	#[serde(rename = "vmstate", default)]
	#[getset(get = "pub", set = "pub")]
	pub vmstate: VMState,
}

impl RTransaction {
	pub fn new(
		hash: H256,
		size: u64,
		version: u8,
		nonce: u64,
		sender: String,
		sys_fee: String,
		net_fee: String,
		valid_until_block: u64,
		signers: Vec<RTransactionSigner>,
		attributes: Vec<TransactionAttributeEnum>,
		script: String,
		witnesses: Vec<NeoWitness>,
	) -> Self {
		Self {
			hash,
			size,
			version,
			nonce,
			sender,
			sys_fee,
			net_fee,
			valid_until_block,
			signers,
			attributes,
			script,
			witnesses,
			block_hash: Default::default(),
			confirmations: Default::default(),
			block_time: Default::default(),
			vmstate: Default::default(),
		}
	}

	pub fn get_first_signer(&self) -> Result<&RTransactionSigner, TypeError> {
		if self.signers.is_empty() {
			return Err(TypeError::IndexOutOfBounds(
				"This transaction does not have any signers. It might be malformed, since every transaction requires at least one signer.".to_string(),
			));
		}
		self.get_signer(0)
	}

	pub fn get_signer(&self, index: usize) -> Result<&RTransactionSigner, TypeError> {
		if index >= self.signers.len() {
			return Err(TypeError::IndexOutOfBounds(format!(
				"This transaction only has {} signers.",
				self.signers.len()
			)));
		}
		Ok(&self.signers[index])
	}

	pub fn get_first_attribute(&self) -> Result<&TransactionAttributeEnum, TypeError> {
		if self.attributes.is_empty() {
			return Err(TypeError::IndexOutOfBounds(
				"This transaction does not have any attributes.".to_string(),
			));
		}
		self.get_attribute(0)
	}

	pub fn get_attribute(&self, index: usize) -> Result<&TransactionAttributeEnum, TypeError> {
		if index >= self.attributes.len() {
			return Err(TypeError::IndexOutOfBounds(format!(
				"This transaction only has {} attributes. Tried to access index {}.",
				self.attributes.len(),
				index
			)));
		}
		Ok(&self.attributes[index])
	}
}

// Production-ready RTransaction uses derive macros for proper deserialization
// Professional implementation with comprehensive transaction field handling

impl Eq for RTransaction {}

impl PartialEq for RTransaction {
	fn eq(&self, other: &Self) -> bool {
		self.size == other.size
			&& self.version == other.version
			&& self.hash == other.hash
			&& self.nonce == other.nonce
			&& self.sender == other.sender
			&& self.sys_fee == other.sys_fee
			&& self.net_fee == other.net_fee
			&& self.valid_until_block == other.valid_until_block
			&& self.signers == other.signers
			&& self.attributes == other.attributes
			&& self.script == other.script
			&& self.witnesses == other.witnesses
			&& self.block_hash == other.block_hash
			&& self.confirmations == other.confirmations
			&& self.block_time == other.block_time
			&& self.vmstate == other.vmstate
	}
}

// impl PartialEq for Transaction {
// 	fn eq(&self, other: &Self) -> bool {
// 		self.to_array() == other.to_array()
// 	}
// }
