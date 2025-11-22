use serde::{Deserialize, Serialize};

use crate::neo_types::{
	deserialize_h256,
	deserialize_h256_option,
	serialize_h256,
	serialize_h256_option,
	H256,
};

/// Lightweight transaction hash provider used for block equality checks.
#[allow(dead_code)]
pub(crate) trait TXTrait {
	/// Return the transaction hash for comparison.
	fn hash(&self) -> H256;
}

/// Compatibility helper so plain `H256` hashes satisfy `TXTrait`.
impl TXTrait for H256 {
	fn hash(&self) -> H256 {
		*self
	}
}

/// Basic N3 block model used by the legacy WebSocket transport.
#[derive(Serialize, Deserialize, Clone, Hash, Debug)]
#[allow(dead_code)]
pub(crate) struct Block<TX, W> {
	/// The hash of the block.
	#[serde(serialize_with = "serialize_h256")]
	#[serde(deserialize_with = "deserialize_h256")]
	pub hash: H256,
	/// The size of the block.
	pub size: u32,
	/// The version of the block.
	pub version: u32,
	/// The hash of the previous block in the blockchain.
	#[serde(rename = "previousblockhash")]
	#[serde(serialize_with = "serialize_h256")]
	#[serde(deserialize_with = "deserialize_h256")]
	pub prev_block_hash: H256,
	/// The hash of the Merkle root of all transactions in the block.
	#[serde(rename = "merkleroot")]
	#[serde(serialize_with = "serialize_h256")]
	#[serde(deserialize_with = "deserialize_h256")]
	pub merkle_root_hash: H256,
	/// The timestamp of the block.
	pub time: u32,
	/// The index of the block.
	pub index: u32,
	/// The index of the primary node that produced the block.
	pub primary: Option<u32>,
	/// The address of the next consensus node.
	#[serde(rename = "nextconsensus")]
	pub next_consensus: String,
	/// The list of witnesses for the block.
	pub witnesses: Option<Vec<W>>,
	/// The list of transactions in the block.
	#[serde(rename = "tx")]
	pub transactions: Option<Vec<TX>>,
	/// The number of confirmations for the block.
	pub confirmations: u32,
	/// The hash of the next block in the blockchain.
	#[serde(rename = "nextblockhash")]
	#[serde(serialize_with = "serialize_h256_option")]
	#[serde(deserialize_with = "deserialize_h256_option")]
	pub next_block_hash: Option<H256>,
}

impl<TX, W> PartialEq for Block<TX, W>
where
	TX: TXTrait,
{
	fn eq(&self, other: &Self) -> bool {
		// Compare transaction hashes when present, otherwise fall back to block hash.
		if let (Some(tx), Some(other_tx)) = (&self.transactions, &other.transactions) {
			if tx.len() != other_tx.len() {
				return false;
			}
			for (lhs, rhs) in tx.iter().zip(other_tx) {
				if lhs.hash() != rhs.hash() {
					return false;
				}
			}
		}
		self.hash == other.hash
	}
}
