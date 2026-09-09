//! # NEP-27 Contract Event Queries
//!
//! This module provides a dedicated, chainable query-builder API for retrieving
//! [NEP-27](https://github.com/neo-project/proposals/blob/master/nep-27.mdk)
//! contract events (notifications) emitted by a smart contract.
//!
//! Neo's JSON-RPC surface does not expose a single "events by contract + block
//! range" endpoint. Instead, notifications are embedded inside the application log
//! of each transaction ([`getapplicationlog`]). [`ContractEventQuery`] builds on
//! the existing [`RpcClient`] / [`APITrait`] primitives (`getblock` and
//! `getapplicationlog`) to scan a block range, decode every transaction's
//! application log, and return only the notifications that match the requested
//! contract script hash and (optionally) event name.
//!
//! Because the query is built on top of the shared RPC client rather than
//! duplicating transport logic, it works transparently with any
//! [`JsonRpcProvider`] — HTTP, WebSocket, or the in-memory `MockProvider` used in
//! tests.
//!
//! ## Example
//!
//! ```no_run
//! use neo3::neo_clients::{HttpProvider, RpcClient};
//! use neo3::neo_contract::ContractEventQuery;
//! use neo3::neo_types::ScriptHash;
//! use std::str::FromStr;
//!
//! async fn transfer_events() -> Result<(), Box<dyn std::error::Error>> {
//!     let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
//!     let client = RpcClient::new(provider);
//!
//!     let gas = ScriptHash::from_str("0xd2a4cff31913016155e38e474a2c06d08be276cf")?;
//!
//!     // All NEP-27 "Transfer" events emitted by GAS between blocks 100 and 200.
//!     let events = ContractEventQuery::new(gas)
//!         .event_name("Transfer")
//!         .block_range(100, 200)
//!         .limit(50)
//!         .execute(&client)
//!         .await?;
//!
//!     for event in events {
//!         println!("block {} tx {:?} -> {:?}", event.block_index, event.tx_hash, event.state);
//!     }
//!     Ok(())
//! }
//! ```

use primitive_types::{H160, H256};
use serde::{Deserialize, Serialize};

use crate::{
	neo_clients::{APITrait, JsonRpcProvider, RpcClient},
	neo_contract::ContractError,
	neo_protocol::{ApplicationLog, LogNotification},
	neo_types::StackItem,
};

/// A single NEP-27 contract event notification, paired with the chain context
/// (transaction hash and block index) in which it was emitted.
///
/// This is the typed result returned by [`ContractEventQuery::execute`]. It
/// mirrors the on-chain [`LogNotification`] payload (`contract`, `event_name`,
/// `state`) while adding the location metadata that callers typically need to
/// correlate an event with the transaction that produced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractEventResult {
	/// The script hash of the contract that emitted the notification.
	pub contract: H160,
	/// The name of the emitted event (for example `"Transfer"`).
	pub event_name: String,
	/// The event payload, as an arbitrary NeoVM [`StackItem`].
	pub state: StackItem,
	/// The hash of the transaction whose application log contained this event.
	pub tx_hash: H256,
	/// The index of the block that included the transaction.
	pub block_index: u32,
	/// The position of this notification within the transaction execution.
	pub notification_index: usize,
}

impl ContractEventResult {
	/// Creates a new [`ContractEventResult`] from its raw components.
	pub fn new(
		contract: H160,
		event_name: String,
		state: StackItem,
		tx_hash: H256,
		block_index: u32,
		notification_index: usize,
	) -> Self {
		Self { contract, event_name, state, tx_hash, block_index, notification_index }
	}

	/// Builds a result from a decoded [`LogNotification`] plus its chain context.
	fn from_notification(
		notification: &LogNotification,
		tx_hash: H256,
		block_index: u32,
		notification_index: usize,
	) -> Self {
		Self {
			contract: notification.contract,
			event_name: notification.event_name.clone(),
			state: notification.state.clone(),
			tx_hash,
			block_index,
			notification_index,
		}
	}
}

/// A chainable builder for querying NEP-27 contract events over JSON-RPC.
///
/// Filters supported:
/// - **contract script hash** (required) — only notifications emitted by this
///   contract are returned.
/// - **event name** (optional) — when set, only notifications whose `eventname`
///   matches exactly are returned; when unset, all events of the contract match.
/// - **block range** (required by [`execute`](Self::execute)) — the inclusive
///   `from..=to` span of blocks to scan.
/// - **limit** (optional) — an early-exit cap on the number of results.
///
/// Construct with [`ContractEventQuery::new`], chain the filter setters, then run
/// [`execute`](Self::execute) against an [`RpcClient`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContractEventQuery {
	contract: H160,
	event_name: Option<String>,
	from_block: Option<u32>,
	to_block: Option<u32>,
	limit: Option<usize>,
}

impl ContractEventQuery {
	/// Starts a new query for events emitted by `contract`.
	pub fn new(contract: H160) -> Self {
		Self { contract, ..Default::default() }
	}

	/// Restricts results to a single event name (for example `"Transfer"`).
	pub fn event_name(mut self, name: impl Into<String>) -> Self {
		self.event_name = Some(name.into());
		self
	}

	/// Sets the inclusive block range `from..=to` to scan.
	pub fn block_range(mut self, from: u32, to: u32) -> Self {
		self.from_block = Some(from);
		self.to_block = Some(to);
		self
	}

	/// Sets the first (inclusive) block of the scan range.
	pub fn from_block(mut self, from: u32) -> Self {
		self.from_block = Some(from);
		self
	}

	/// Sets the last (inclusive) block of the scan range.
	pub fn to_block(mut self, to: u32) -> Self {
		self.to_block = Some(to);
		self
	}

	/// Caps the number of returned events, stopping the scan early once reached.
	pub fn limit(mut self, limit: usize) -> Self {
		self.limit = Some(limit);
		self
	}

	/// The contract script hash this query targets.
	pub fn contract(&self) -> H160 {
		self.contract
	}

	/// The event-name filter, if any.
	pub fn event_name_filter(&self) -> Option<&str> {
		self.event_name.as_deref()
	}

	/// The inclusive lower bound of the block range, if set.
	pub fn from_block_index(&self) -> Option<u32> {
		self.from_block
	}

	/// The inclusive upper bound of the block range, if set.
	pub fn to_block_index(&self) -> Option<u32> {
		self.to_block
	}

	/// The result cap, if set.
	pub fn result_limit(&self) -> Option<usize> {
		self.limit
	}

	/// Returns `true` when `notification` matches this query's contract and
	/// (optional) event-name filters.
	///
	/// This is the pure predicate used during scanning and is exposed so callers
	/// can filter notifications they obtained through other means (for example a
	/// WebSocket subscription) using the exact same rules.
	pub fn matches(&self, notification: &LogNotification) -> bool {
		if notification.contract != self.contract {
			return false;
		}
		match &self.event_name {
			Some(name) => notification.event_name == *name,
			None => true,
		}
	}

	/// Extracts every matching event from a decoded [`ApplicationLog`], tagging
	/// each with the supplied `tx_hash` and `block_index`.
	///
	/// This performs no I/O: it is the response-parsing half of the query and can
	/// be reused on logs fetched independently of [`execute`](Self::execute).
	pub fn collect_from_log(
		&self,
		tx_hash: H256,
		block_index: u32,
		log: &ApplicationLog,
	) -> Vec<ContractEventResult> {
		let mut results = Vec::new();
		for execution in &log.executions {
			for (index, notification) in execution.notifications.iter().enumerate() {
				if self.matches(notification) {
					results.push(ContractEventResult::from_notification(
						notification,
						tx_hash,
						block_index,
						index,
					));
				}
			}
		}
		results
	}

	/// Validates the configured block range, returning the inclusive bounds.
	fn validated_range(&self) -> Result<(u32, u32), ContractError> {
		let from = self.from_block.ok_or_else(|| {
			ContractError::InvalidArgError(
				"ContractEventQuery requires a starting block; call from_block(..) or \
				 block_range(..) before execute(..)"
					.to_string(),
			)
		})?;
		let to = self.to_block.ok_or_else(|| {
			ContractError::InvalidArgError(
				"ContractEventQuery requires an ending block; call to_block(..) or \
				 block_range(..) before execute(..)"
					.to_string(),
			)
		})?;
		if from > to {
			return Err(ContractError::InvalidArgError(format!(
				"Invalid block range: start block {from} is greater than end block {to}"
			)));
		}
		Ok((from, to))
	}

	/// Executes the query against `client`, scanning the configured block range.
	///
	/// For each block in `from..=to` the full block is fetched (`getblock`) to
	/// enumerate its transactions, and each transaction's application log
	/// (`getapplicationlog`) is decoded and filtered. Scanning stops early once
	/// [`limit`](Self::limit) results have been collected.
	///
	/// # Errors
	///
	/// Returns [`ContractError::InvalidArgError`] if the block range is missing or
	/// inverted, or a wrapped [`ContractError::ProviderError`] if any RPC call
	/// fails.
	pub async fn execute<P>(
		&self,
		client: &RpcClient<P>,
	) -> Result<Vec<ContractEventResult>, ContractError>
	where
		P: JsonRpcProvider,
	{
		let (from, to) = self.validated_range()?;
		let mut results: Vec<ContractEventResult> = Vec::new();

		for index in from..=to {
			let block = client.get_block_by_index(index, true).await?;
			for tx in block.transactions.unwrap_or_default() {
				let tx_hash = tx.hash;
				let log = client.get_application_log(tx_hash).await?;
				results.extend(self.collect_from_log(tx_hash, index, &log));

				if let Some(limit) = self.limit {
					if results.len() >= limit {
						results.truncate(limit);
						return Ok(results);
					}
				}
			}
		}

		Ok(results)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		neo_clients::{MockProvider, RpcClient},
		neo_protocol::{Execution, NeoBlock, RTransaction},
	};
	use serde_json::json;
	use std::str::FromStr;

	fn sample_contract() -> H160 {
		H160::repeat_byte(0xef)
	}

	fn notification(contract: H160, event_name: &str) -> LogNotification {
		LogNotification::new(contract, event_name.to_string(), StackItem::Any)
	}

	fn application_log(tx_hash: H256, notifications: Vec<LogNotification>) -> ApplicationLog {
		ApplicationLog {
			transaction_id: tx_hash,
			executions: vec![Execution { notifications, ..Default::default() }],
		}
	}

	#[test]
	fn builder_stores_filters() {
		let contract = H160::repeat_byte(0xab);
		let query = ContractEventQuery::new(contract)
			.event_name("Transfer")
			.block_range(10, 20)
			.limit(5);

		assert_eq!(query.contract(), contract);
		assert_eq!(query.event_name_filter(), Some("Transfer"));
		assert_eq!(query.from_block_index(), Some(10));
		assert_eq!(query.to_block_index(), Some(20));
		assert_eq!(query.result_limit(), Some(5));
	}

	#[test]
	fn from_and_to_block_setters_are_independent() {
		let query = ContractEventQuery::new(H160::repeat_byte(0x01)).from_block(7).to_block(9);
		assert_eq!(query.from_block_index(), Some(7));
		assert_eq!(query.to_block_index(), Some(9));
	}

	#[test]
	fn matches_filters_by_contract_and_event_name() {
		let contract = H160::repeat_byte(0xab);
		let other = H160::repeat_byte(0xcd);

		// No event-name filter: only the contract matters.
		let any_event = ContractEventQuery::new(contract);
		assert!(any_event.matches(&notification(contract, "Transfer")));
		assert!(any_event.matches(&notification(contract, "Approval")));
		assert!(!any_event.matches(&notification(other, "Transfer")));

		// With event-name filter: both must match.
		let transfers = ContractEventQuery::new(contract).event_name("Transfer");
		assert!(transfers.matches(&notification(contract, "Transfer")));
		assert!(!transfers.matches(&notification(contract, "Approval")));
		assert!(!transfers.matches(&notification(other, "Transfer")));
	}

	#[test]
	fn collect_from_log_extracts_only_matching_notifications() {
		let contract = H160::repeat_byte(0xab);
		let other = H160::repeat_byte(0xcd);
		let tx_hash = H256::repeat_byte(0x11);

		let log = application_log(
			tx_hash,
			vec![
				notification(contract, "Transfer"), // index 0: match
				notification(other, "Transfer"),    // index 1: wrong contract
				notification(contract, "Approval"), // index 2: wrong event
				notification(contract, "Transfer"), // index 3: match
			],
		);

		let query = ContractEventQuery::new(contract).event_name("Transfer");
		let results = query.collect_from_log(tx_hash, 42, &log);

		assert_eq!(results.len(), 2);
		assert_eq!(results[0].notification_index, 0);
		assert_eq!(results[1].notification_index, 3);
		for result in &results {
			assert_eq!(result.contract, contract);
			assert_eq!(result.event_name, "Transfer");
			assert_eq!(result.tx_hash, tx_hash);
			assert_eq!(result.block_index, 42);
		}
	}

	#[tokio::test]
	async fn execute_requires_a_block_range() {
		let provider = MockProvider::new();
		let client = RpcClient::new(provider);

		// No range configured at all.
		let err = ContractEventQuery::new(H160::repeat_byte(0xab))
			.execute(&client)
			.await
			.unwrap_err();
		assert!(matches!(err, ContractError::InvalidArgError(msg) if msg.contains("starting block")));

		// Only the start block configured.
		let err = ContractEventQuery::new(H160::repeat_byte(0xab))
			.from_block(5)
			.execute(&client)
			.await
			.unwrap_err();
		assert!(matches!(err, ContractError::InvalidArgError(msg) if msg.contains("ending block")));

		// Inverted range.
		let err = ContractEventQuery::new(H160::repeat_byte(0xab))
			.block_range(20, 10)
			.execute(&client)
			.await
			.unwrap_err();
		assert!(matches!(err, ContractError::InvalidArgError(msg) if msg.contains("greater than")));
	}

	#[tokio::test]
	async fn execute_scans_blocks_and_filters_notifications() {
		let contract = sample_contract();
		let other = H160::repeat_byte(0xcd);
		let tx_hash = H256::from_str(
			"0x1111111111111111111111111111111111111111111111111111111111111111",
		)
		.unwrap();

		// Build a block that contains exactly one transaction.
		let tx = RTransaction::new(
			tx_hash,
			0,
			0,
			0,
			String::new(),
			"0".to_string(),
			"0".to_string(),
			0,
			vec![],
			vec![],
			String::new(),
			vec![],
		);
		let block = NeoBlock {
			hash: H256::zero(),
			size: 0,
			version: 0,
			prev_block_hash: H256::zero(),
			merkle_root_hash: H256::zero(),
			time: 0,
			nonce: "0".to_string(),
			index: 100,
			primary: None,
			next_consensus: String::new(),
			witnesses: None,
			transactions: Some(vec![tx]),
			confirmations: 1,
			next_block_hash: None,
		};

		// Application log with one matching and one non-matching notification.
		let log = application_log(
			tx_hash,
			vec![notification(other, "Transfer"), notification(contract, "Transfer")],
		);

		let provider = MockProvider::new();
		// getblock params are [index, 1] (full transactions).
		provider.push_result_with_params(
			"getblock",
			json!([100, 1]),
			serde_json::to_value(&block).unwrap(),
		);
		// getapplicationlog params are [hex(tx_hash)] (no 0x prefix, little-endian bytes).
		provider.push_result_with_params(
			"getapplicationlog",
			json!([hex::encode(tx_hash.0)]),
			serde_json::to_value(&log).unwrap(),
		);
		let client = RpcClient::new(provider.clone());

		let results = ContractEventQuery::new(contract)
			.event_name("Transfer")
			.block_range(100, 100)
			.execute(&client)
			.await
			.unwrap();

		assert_eq!(results.len(), 1);
		assert_eq!(results[0].contract, contract);
		assert_eq!(results[0].event_name, "Transfer");
		assert_eq!(results[0].tx_hash, tx_hash);
		assert_eq!(results[0].block_index, 100);
		assert_eq!(results[0].notification_index, 1);

		// Both RPC methods were exercised exactly once.
		let requests = provider.take_requests();
		let methods: Vec<String> = requests.iter().map(|(m, _)| m.clone()).collect();
		assert_eq!(methods, vec!["getblock".to_string(), "getapplicationlog".to_string()]);
	}

	#[tokio::test]
	async fn execute_honours_limit() {
		let contract = sample_contract();
		let tx_hash = H256::from_str(
			"0x2222222222222222222222222222222222222222222222222222222222222222",
		)
		.unwrap();

		let tx = RTransaction::new(
			tx_hash,
			0,
			0,
			0,
			String::new(),
			"0".to_string(),
			"0".to_string(),
			0,
			vec![],
			vec![],
			String::new(),
			vec![],
		);
		let block = NeoBlock {
			hash: H256::zero(),
			size: 0,
			version: 0,
			prev_block_hash: H256::zero(),
			merkle_root_hash: H256::zero(),
			time: 0,
			nonce: "0".to_string(),
			index: 200,
			primary: None,
			next_consensus: String::new(),
			witnesses: None,
			transactions: Some(vec![tx]),
			confirmations: 1,
			next_block_hash: None,
		};

		// Three matching notifications, but the limit caps the result at two.
		let log = application_log(
			tx_hash,
			vec![
				notification(contract, "Transfer"),
				notification(contract, "Transfer"),
				notification(contract, "Transfer"),
			],
		);

		let provider = MockProvider::new();
		provider.push_result_with_params(
			"getblock",
			json!([200, 1]),
			serde_json::to_value(&block).unwrap(),
		);
		provider.push_result_with_params(
			"getapplicationlog",
			json!([hex::encode(tx_hash.0)]),
			serde_json::to_value(&log).unwrap(),
		);
		let client = RpcClient::new(provider);

		let results = ContractEventQuery::new(contract)
			.event_name("Transfer")
			.block_range(200, 200)
			.limit(2)
			.execute(&client)
			.await
			.unwrap();

		assert_eq!(results.len(), 2);
	}
}
