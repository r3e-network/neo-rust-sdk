use crate::neo_protocol::NeoBlock;
use futures::stream::Stream;
use std::{error::Error, pin::Pin};

/// The JSON-RPC client event API for Neo.
pub(crate) trait NeoRustRxTrait {
	/// Create a stream that emits newly created blocks on the blockchain.
	///
	/// # Arguments
	///
	/// * `full_transaction_objects` - If true, provides transactions embedded in blocks, otherwise transaction hashes
	///
	/// # Returns
	///
	/// A stream that emits all new blocks as they are added to the blockchain
	fn block_stream(
		&self,
		full_transaction_objects: bool,
	) -> Pin<Box<dyn Stream<Item = Result<NeoBlock, Box<dyn Error>>> + Send>>;

	/// Create a stream that emits all blocks from the blockchain contained within the requested range.
	///
	/// # Arguments
	///
	/// * `start_block` - The block number to commence with
	/// * `end_block` - The block number to finish with
	/// * `full_transaction_objects` - If true, provides transactions embedded in blocks, otherwise transaction hashes
	///
	/// # Returns
	///
	/// A stream to emit these blocks
	fn replay_blocks_stream(
		&self,
		start_block: u32,
		end_block: u32,
		full_transaction_objects: bool,
	) -> Pin<Box<dyn Stream<Item = Result<NeoBlock, Box<dyn Error>>> + Send>>;

	/// Create a stream that emits all blocks from the blockchain contained within the requested range.
	///
	/// # Arguments
	///
	/// * `start_block` - The block number to commence with
	/// * `end_block` - The block number to finish with
	/// * `full_transaction_objects` - If true, provides transactions embedded in blocks, otherwise transaction hashes
	/// * `ascending` - If true, emits blocks in ascending order between range, otherwise, in descending order
	///
	/// # Returns
	///
	/// A stream to emit these blocks
	fn replay_blocks_stream_ordered(
		&self,
		start_block: u32,
		end_block: u32,
		full_transaction_objects: bool,
		ascending: bool,
	) -> Pin<Box<dyn Stream<Item = Result<NeoBlock, Box<dyn Error>>> + Send>>;

	/// Create a stream that emits all transactions from the blockchain starting with a provided block number.
	/// Once it has replayed up to the most current block, the stream completes.
	///
	/// # Arguments
	///
	/// * `start_block` - The block number to commence with
	/// * `full_transaction_objects` - If true, provides transactions embedded in blocks, otherwise transaction hashes
	///
	/// # Returns
	///
	/// A stream to emit all requested blocks
	fn catch_up_to_latest_block_stream(
		&self,
		start_block: u32,
		full_transaction_objects: bool,
	) -> Pin<Box<dyn Stream<Item = Result<NeoBlock, Box<dyn Error>>> + Send>>;

	/// Creates a stream that emits all blocks from the requested block number to the most current.
	/// Once it has emitted the most current block, it starts emitting new blocks as they are created.
	///
	/// # Arguments
	///
	/// * `start_block` - The block number to commence with
	/// * `full_transaction_objects` - If true, provides transactions embedded in blocks, otherwise transaction hashes
	///
	/// # Returns
	///
	/// A stream to emit all requested blocks and future blocks
	fn catch_up_to_latest_and_subscribe_to_new_blocks_stream(
		&self,
		start_block: u32,
		full_transaction_objects: bool,
	) -> Pin<Box<dyn Stream<Item = Result<NeoBlock, Box<dyn Error>>> + Send>>;

	/// Creates a stream that emits new blocks as they are created on the blockchain (starting from the latest block).
	///
	/// # Arguments
	///
	/// * `full_transaction_objects` - If true, provides transactions embedded in blocks, otherwise transaction hashes
	///
	/// # Returns
	///
	/// A stream to emit all future blocks
	fn subscribe_to_new_blocks_stream(
		&self,
		full_transaction_objects: bool,
	) -> Pin<Box<dyn Stream<Item = Result<NeoBlock, Box<dyn Error>>> + Send>>;
}
