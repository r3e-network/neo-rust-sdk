//! # NEP-91 Account Event Queries
//!
//! This module provides a dedicated, chainable query-builder API for retrieving
//! [NEP-91](https://github.com/neo-project/proposals/blob/master/nep-91.mediawiki)
//! account events — contract notifications (in the NEP-27 sense) whose payload
//! references a particular account script hash.
//!
//! Neo's JSON-RPC surface does not expose an "events by account + block range"
//! endpoint. Notifications are embedded inside the application log of each
//! transaction ([`getapplicationlog`]). [`AccountEventQuery`] builds on the
//! existing [`RpcClient`] / [`APITrait`] primitives (`getblock` and
//! `getapplicationlog`) to scan a block range, decode every transaction's
//! application log, and return only the notifications whose event `state`
//! contains the requested account script hash — optionally narrowed by an
//! emitting-contract filter and/or an event-name filter.
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
//! use neo3::neo_contract::AccountEventQuery;
//! use neo3::neo_types::ScriptHash;
//! use std::str::FromStr;
//!
//! async fn account_activity() -> Result<(), Box<dyn std::error::Error>> {
//!     let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
//!     let client = RpcClient::new(provider);
//!
//!     let account = ScriptHash::from_str("0x0000000000000000000000000000000000000001")?;
//!
//!     // Every notification referencing `account` between blocks 100 and 200.
//!     let events = AccountEventQuery::new(account)
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

use crate::{
	neo_clients::{APITrait, JsonRpcProvider, RpcClient},
	neo_contract::{ContractError, ContractEventResult},
	neo_protocol::{ApplicationLog, LogNotification},
	neo_types::{ScriptHashExtension, StackItem},
};

/// A single account event notification, paired with the chain context
/// (transaction hash and block index) in which it was emitted.
///
/// This is a type alias for [`ContractEventResult`]: the on-chain payload shape
/// (`contract`, `event_name`, `state`) and location metadata are identical to a
/// NEP-27 contract event; an account event is simply one whose `state` references
/// a particular account.
pub type AccountEventResult = ContractEventResult;

/// A chainable builder for querying account events over JSON-RPC.
///
/// Filters supported:
/// - **account script hash** (required) — only notifications whose `state`
///   references this account are returned.
/// - **contract filter** (optional) — when set, only notifications emitted by
///   this contract are considered.
/// - **event name** (optional) — when set, only notifications whose `eventname`
///   matches exactly are returned; when unset, all event names match.
/// - **block range** (required by [`execute`](Self::execute)) — the inclusive
///   `from..=to` span of blocks to scan.
/// - **limit** (optional) — an early-exit cap on the number of results. When
///   combined with `page_size`, the smaller value is used.
/// - **pagination** (optional) — [`offset`](Self::offset) and [`page_size`](Self::page_size)
///   allow clients to page through large result sets without loading everything into memory.
///
/// Construct with [`AccountEventQuery::new`] (or the [`address`](Self::address)
/// convenience constructor), chain the filter setters, then run
/// [`execute`](Self::execute) against an [`RpcClient`].
///
/// Unlike [`ContractEventQuery`](crate::neo_contract::ContractEventQuery), this
/// builder does not implement [`Default`] because an account is always required.
///
/// # Performance
///
/// ## RPC Cost Analysis
///
/// [`execute`](Self::execute) has no server-side index to lean on: it fetches
/// every block in the configured range (`getblock`) and then the application log
/// of *every* transaction in those blocks (`getapplicationlog`). The number of
/// RPC round-trips therefore grows with both the width of the block range and
/// the transaction density of each block. A range of 100 blocks with an average
/// of 5 transactions per block requires ~600 sequential RPC calls (1 getblock +
/// 500 getapplicationlog + overhead).
///
/// ### 🚨 Cost Warnings
/// - **Wide ranges are expensive**: Scanning 10,000 blocks can require >50,000 RPC calls
/// - **Sequential latency adds up**: With 200ms avg latency, 1000 calls = ~200s
/// - **Rate limits apply**: Many public nodes limit requests/second; consider backoff
///
/// ### Optimization Strategies
/// - ✅ **Use smaller ranges**: Query by day (≈720 blocks) instead of months
/// - ✅ **Set `limit` early**: Stop scanning once you have enough results
/// - ✅ **Filter by event name**: Reduces filtering work on client side
/// - ✅ **Paginate deep queries**: Use [`offset`](Self::offset) + [`page_size`](Self::page_size)
///   to walk through a fixed block range without retaining every matching event
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountEventQuery {
	account: H160,
	contract_filter: Option<H160>,
	event_name: Option<String>,
	from_block: Option<u32>,
	to_block: Option<u32>,
	limit: Option<usize>,
	offset: Option<usize>,
	page_size: Option<usize>,
}

impl AccountEventQuery {
	/// Starts a new query for events referencing `account`.
	pub fn new(account: H160) -> Self {
		Self {
			account,
			contract_filter: None,
			event_name: None,
			from_block: None,
			to_block: None,
			limit: None,
			offset: None,
			page_size: None,
		}
	}

	/// Starts a new query from an account's address string (for example a
	/// Base58Check `N...` address).
	///
	/// # Errors
	///
	/// Returns [`ContractError::InvalidArgError`] if `addr` is not a valid Neo
	/// address.
	pub fn address(addr: &str) -> Result<Self, ContractError> {
		let account = H160::from_address(addr).map_err(|e| {
			ContractError::InvalidArgError(format!("Invalid account address '{addr}': {e}"))
		})?;
		Ok(Self::new(account))
	}

	/// Restricts results to notifications emitted by a single contract.
	pub fn contract(mut self, contract: H160) -> Self {
		self.contract_filter = Some(contract);
		self
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
	/// If [`page_size`](Self::page_size) is also set, the smaller cap applies.
	pub fn limit(mut self, limit: usize) -> Self {
		self.limit = Some(limit);
		self
	}

	/// Sets the pagination offset (number of results to skip). Combined with
	/// [`page_size`](Self::page_size) this allows streaming through large result sets.
	///
	/// # Example
	///
	/// ```no_run
	/// use neo3::neo_contract::AccountEventQuery;
	/// use neo3::neo_types::ScriptHash;
	/// # use std::str::FromStr;
	/// let account = ScriptHash::from_str("0x0000000000000000000000000000000000000001").unwrap();
	/// // Get results 101-200
	/// let _query = AccountEventQuery::new(account)
	///     .offset(100)
	///     .page_size(100)
	///     .block_range(1000, 2000);
	/// ```
	pub fn offset(mut self, offset: usize) -> Self {
		self.offset = Some(offset);
		self
	}

	/// Sets the maximum number of results to return per page, after applying
	/// [`offset`](Self::offset). If [`limit`](Self::limit) is also set, the
	/// smaller cap applies.
	///
	/// Recommended values: 50-200. Avoid very large pages (>1000) as they may cause
	/// out-of-memory conditions when scanning dense blocks.
	pub fn page_size(mut self, size: usize) -> Self {
		self.page_size = Some(size);
		self
	}

	/// The account script hash this query targets.
	pub fn account(&self) -> H160 {
		self.account
	}

	/// The emitting-contract filter, if any.
	pub fn contract_filter(&self) -> Option<H160> {
		self.contract_filter
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

	/// The pagination offset, if set.
	pub fn pagination_offset(&self) -> Option<usize> {
		self.offset
	}

	/// The page size, if set.
	pub fn page_size_value(&self) -> Option<usize> {
		self.page_size
	}

	/// Returns the maximum number of results this query may return.
	///
	/// `limit` and `page_size` are both upper bounds, so their intersection is
	/// represented by the smaller value when both are configured.
	fn effective_result_cap(&self) -> Option<usize> {
		match (self.limit, self.page_size) {
			(Some(limit), Some(page_size)) => Some(limit.min(page_size)),
			(Some(limit), None) => Some(limit),
			(None, Some(page_size)) => Some(page_size),
			(None, None) => None,
		}
	}

	/// Recursively checks whether `state` references `account`.
	///
	/// Arrays, structs, and maps are traversed depth-first. A leaf
	/// [`StackItem::ByteString`] (or [`StackItem::Buffer`]) matches when its
	/// decoded byte value is exactly 20 bytes long and equal to
	/// `account.as_bytes()`.
	fn stack_item_contains_account(state: &StackItem, account: &H160) -> bool {
		match state {
			StackItem::Array { value } | StackItem::Struct { value } => {
				value.iter().any(|item| Self::stack_item_contains_account(item, account))
			},
			StackItem::Map { value } => value.iter().any(|entry| {
				Self::stack_item_contains_account(&entry.key, account)
					|| Self::stack_item_contains_account(&entry.value, account)
			}),
			StackItem::ByteString { .. } | StackItem::Buffer { .. } => match state.as_bytes() {
				Some(bytes) => bytes.len() == 20 && bytes.as_slice() == account.as_bytes(),
				None => false,
			},
			_ => false,
		}
	}

	/// Returns `true` when `notification` matches this query's contract filter,
	/// event-name filter, and references the target account within its `state`.
	///
	/// This is the pure predicate used during scanning and is exposed so callers
	/// can filter notifications they obtained through other means (for example a
	/// WebSocket subscription) using the exact same rules.
	pub fn matches_account(&self, notification: &LogNotification) -> bool {
		if let Some(contract) = self.contract_filter
			&& notification.contract != contract
		{
			return false;
		}
		if let Some(name) = &self.event_name
			&& notification.event_name != *name
		{
			return false;
		}
		Self::stack_item_contains_account(&notification.state, &self.account)
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
	) -> Vec<AccountEventResult> {
		let mut results = Vec::new();
		for execution in &log.executions {
			for (index, notification) in execution.notifications.iter().enumerate() {
				if self.matches_account(notification) {
					results.push(AccountEventResult::new(
						notification.contract,
						notification.event_name.clone(),
						notification.state.clone(),
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
				"AccountEventQuery requires a starting block; call from_block(..) or \
				 block_range(..) before execute(..)"
					.to_string(),
			)
		})?;
		let to = self.to_block.ok_or_else(|| {
			ContractError::InvalidArgError(
				"AccountEventQuery requires an ending block; call to_block(..) or \
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
	/// the effective result cap has been collected. The cap is the smaller of
	/// [`limit`](Self::limit) and [`page_size`](Self::page_size) when both are set.
	/// If [`offset`](Self::offset) is set, that many matching events are discarded
	/// once across the complete scan before beginning result collection.
	///
	/// # Performance Warning
	///
	/// This method scans **every transaction in every block** within the configured
	/// range. A range of 10,000 blocks with average transaction density can require
	/// tens of thousands of sequential RPC calls. See the module-level documentation
	/// for optimization strategies.
	///
	/// # Errors
	///
	/// Returns [`ContractError::InvalidArgError`] if the block range is missing or
	/// inverted, or a wrapped [`ContractError::ProviderError`] if any RPC call
	/// fails.
	pub async fn execute<P>(
		&self,
		client: &RpcClient<P>,
	) -> Result<Vec<AccountEventResult>, ContractError>
	where
		P: JsonRpcProvider,
	{
		let (from, to) = self.validated_range()?;
		let result_cap = self.effective_result_cap();
		if result_cap == Some(0) {
			return Ok(Vec::new());
		}

		let mut results: Vec<AccountEventResult> = Vec::new();
		let mut remaining_offset = self.offset.unwrap_or(0);

		for index in from..=to {
			let block = client.get_block_by_index(index, true).await?;
			for tx in block.transactions.unwrap_or_default() {
				let tx_hash = tx.hash;
				let log = client.get_application_log(tx_hash).await?;
				let matched = self.collect_from_log(tx_hash, index, &log);
				let skip_count = remaining_offset.min(matched.len());
				remaining_offset -= skip_count;
				results.extend(matched.into_iter().skip(skip_count));

				if let Some(cap) = result_cap {
					if results.len() >= cap {
						results.truncate(cap);
						return Ok(results);
					}				}
			}
		}

		Ok(results)
	}

	/// Compatibility wrapper for [`execute`](Self::execute).
	///
	/// Earlier versions used the indexed `getnep17transfers` RPC here. That RPC can
	/// apply server-side history and result limits, and it returns sent and received
	/// transfers as separate collections. Those constraints make it impossible to
	/// guarantee the same complete, canonical block/transaction/notification order
	/// as [`execute`](Self::execute), which in turn makes offset pagination unsafe.
	///
	/// For correctness and API compatibility, this method now delegates directly to
	/// [`execute`](Self::execute). It accepts the same filters and has identical
	/// ordering, pagination, performance, and error behavior. New code should call
	/// [`execute`](Self::execute) directly.
	///
	/// # Errors
	///
	/// Returns the same errors as [`execute`](Self::execute).
	///
	/// # Example
	///
	/// ```no_run
	/// use neo3::{neo_clients::{HttpProvider, RpcClient}, neo_contract::AccountEventQuery};
	/// use neo3::neo_types::ScriptHash;
	/// use std::str::FromStr;
	///
	/// async fn query_transfers_fast() -> Result<(), Box<dyn std::error::Error>> {
	///     let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
	///     let client = RpcClient::new(provider);
	///
	///     let account = ScriptHash::from_str("0x0000000000000000000000000000000000000001")?;
	///
	///     // Compatibility entry point; equivalent to execute().
	///     let transfers = AccountEventQuery::new(account)
	///         .event_name("Transfer")
	///         .block_range(1_000_000, 1_000_100)
	///         .execute_fast(&client)
	///         .await?;
	///
	///     for transfer in transfers {
	///         println!("block {} tx {:?}", transfer.block_index, transfer.tx_hash);
	///     }
	///     Ok(())
	/// }
	/// ```
	pub async fn execute_fast<P>(
		&self,
		client: &RpcClient<P>,
	) -> Result<Vec<AccountEventResult>, ContractError>
	where
		P: JsonRpcProvider,
	{
		self.execute(client).await	}
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

	fn sample_account() -> H160 {
		H160::repeat_byte(0xaa)
	}

	fn sample_contract() -> H160 {
		H160::repeat_byte(0xef)
	}

	/// A `ByteString` stack item wrapping the 20-byte value of `hash`.
	fn account_byte_string(hash: H160) -> StackItem {
		StackItem::new_byte_string(hash.as_bytes().to_vec())
	}

	/// A `Buffer` stack item wrapping the raw 20-byte value of `hash`.
	///
	/// Mirrors [`StackItem::new_byte_string`] (base64-encoded payload) but emits
	/// the `Buffer` variant so traversal of both leaf byte types is exercised.
	fn account_buffer(hash: H160) -> StackItem {
		use base64::Engine;
		StackItem::Buffer {
			value: base64::engine::general_purpose::STANDARD.encode(hash.as_bytes()),
		}
	}

	/// A `Transfer`-shaped notification whose state references `account`.
	fn account_notification(contract: H160, event_name: &str, account: H160) -> LogNotification {
		let state = StackItem::Array {
			value: vec![account_byte_string(account), StackItem::Integer { value: 42 }],
		};
		LogNotification::new(contract, event_name.to_string(), state)
	}

	fn application_log(tx_hash: H256, notifications: Vec<LogNotification>) -> ApplicationLog {
		ApplicationLog {
			transaction_id: tx_hash,
			executions: vec![Execution { notifications, ..Default::default() }],
		}
	}

	fn transaction(tx_hash: H256) -> RTransaction {
		RTransaction::new(
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
		)
	}

	fn block(index: u32, transactions: Vec<RTransaction>) -> NeoBlock {
		NeoBlock {
			hash: H256::zero(),
			size: 0,
			version: 0,
			prev_block_hash: H256::zero(),
			merkle_root_hash: H256::zero(),
			time: 0,
			nonce: "0".to_string(),
			index,
			primary: None,
			next_consensus: String::new(),
			witnesses: None,
			transactions: Some(transactions),
			confirmations: 1,
			next_block_hash: None,
		}
	}

	fn push_block_with_logs(provider: &MockProvider, index: u32, logs: &[ApplicationLog]) {
		let transactions = logs.iter().map(|log| transaction(log.transaction_id)).collect();
		provider.push_result_with_params(
			"getblock",
			json!([index, 1]),
			serde_json::to_value(block(index, transactions)).unwrap(),
		);
		for log in logs {
			provider.push_result_with_params(
				"getapplicationlog",
				json!([hex::encode(log.transaction_id.0)]),
				serde_json::to_value(log).unwrap(),
			);
		}
	}

	#[test]
	fn builder_stores_filters() {
		let account = sample_account();
		let contract = sample_contract();
		let query = AccountEventQuery::new(account)
			.contract(contract)
			.event_name("Transfer")
			.block_range(10, 20)
			.limit(5);

		assert_eq!(query.account(), account);
		assert_eq!(query.contract_filter(), Some(contract));
		assert_eq!(query.event_name_filter(), Some("Transfer"));
		assert_eq!(query.from_block_index(), Some(10));
		assert_eq!(query.to_block_index(), Some(20));
		assert_eq!(query.result_limit(), Some(5));
	}

	#[test]
	fn effective_result_cap_uses_tighter_bound() {
		let account = sample_account();

		assert_eq!(AccountEventQuery::new(account).effective_result_cap(), None);
		assert_eq!(AccountEventQuery::new(account).limit(5).effective_result_cap(), Some(5));
		assert_eq!(AccountEventQuery::new(account).page_size(3).effective_result_cap(), Some(3));
		assert_eq!(
			AccountEventQuery::new(account).limit(5).page_size(3).effective_result_cap(),
			Some(3)
		);
		assert_eq!(
			AccountEventQuery::new(account).limit(2).page_size(3).effective_result_cap(),
			Some(2)
		);
	}

	#[test]
	fn from_and_to_block_setters_are_independent() {
		let query = AccountEventQuery::new(sample_account()).from_block(7).to_block(9);
		assert_eq!(query.from_block_index(), Some(7));
		assert_eq!(query.to_block_index(), Some(9));
	}

	#[test]
	fn address_constructor_round_trips() {
		let account = sample_account();
		let query = AccountEventQuery::address(&account.to_address()).unwrap();
		assert_eq!(query.account(), account);

		let err = AccountEventQuery::address("not-a-valid-address").unwrap_err();
		assert!(
			matches!(err, ContractError::InvalidArgError(msg) if msg.contains("Invalid account"))
		);
	}

	#[test]
	fn matches_account_filters_by_state_content() {
		let account = sample_account();
		let other_account = H160::repeat_byte(0xbb);
		let contract = sample_contract();

		let query = AccountEventQuery::new(account);

		// Positive: state references the target account.
		assert!(query.matches_account(&account_notification(contract, "Transfer", account)));

		// Negative: state references a different account.
		assert!(!query.matches_account(&account_notification(contract, "Transfer", other_account)));

		// Negative: no byte-string payload at all.
		let empty = LogNotification::new(contract, "Transfer".to_string(), StackItem::Any);
		assert!(!query.matches_account(&empty));
	}

	#[test]
	fn matches_account_with_contract_filter() {
		let account = sample_account();
		let contract = sample_contract();
		let other_contract = H160::repeat_byte(0xcd);

		let query = AccountEventQuery::new(account).contract(contract).event_name("Transfer");

		// Right contract, right event, references account.
		assert!(query.matches_account(&account_notification(contract, "Transfer", account)));
		// Wrong contract.
		assert!(!query.matches_account(&account_notification(other_contract, "Transfer", account)));
		// Wrong event name.
		assert!(!query.matches_account(&account_notification(contract, "Approval", account)));
	}

	#[test]
	fn matches_account_traverses_complex_state_and_rejects_wrong_length() {
		use crate::neo_types::MapEntry;

		let account = sample_account();
		let other_account = H160::repeat_byte(0xbb);
		let contract = sample_contract();
		let query = AccountEventQuery::new(account);

		// Positive: a `Buffer` leaf (not a `ByteString`) that matches the account.
		let buffer_state = StackItem::Array { value: vec![account_buffer(account)] };
		assert!(query.matches_account(&LogNotification::new(
			contract,
			"Transfer".to_string(),
			buffer_state,
		)));

		// Positive: the account is buried deep inside a nested Struct -> Map -> Array.
		let nested_state = StackItem::Struct {
			value: vec![
				StackItem::Integer { value: 7 },
				StackItem::Map {
					value: vec![MapEntry {
						key: StackItem::new_byte_string(b"to".to_vec()),
						value: StackItem::Array { value: vec![account_byte_string(account)] },
					}],
				},
			],
		};
		assert!(query.matches_account(&LogNotification::new(
			contract,
			"Transfer".to_string(),
			nested_state,
		)));

		// Negative: a 19-byte `ByteString` must NOT match (wrong length).
		let short_state =
			StackItem::Array { value: vec![StackItem::new_byte_string(vec![0xaa; 19])] };
		assert!(!query.matches_account(&LogNotification::new(
			contract,
			"Transfer".to_string(),
			short_state,
		)));

		// Negative: an equally deep structure that only references a different account.
		let other_nested = StackItem::Struct {
			value: vec![StackItem::Map {
				value: vec![MapEntry {
					key: StackItem::new_byte_string(b"to".to_vec()),
					value: StackItem::Array { value: vec![account_buffer(other_account)] },
				}],
			}],
		};
		assert!(!query.matches_account(&LogNotification::new(
			contract,
			"Transfer".to_string(),
			other_nested,
		)));
	}

	#[test]
	fn collect_from_log_extracts_account_events() {
		let account = sample_account();
		let other_account = H160::repeat_byte(0xbb);
		let contract = sample_contract();
		let tx_hash = H256::repeat_byte(0x11);

		let log = application_log(
			tx_hash,
			vec![
				account_notification(contract, "Transfer", account), // index 0: match
				account_notification(contract, "Transfer", other_account), // index 1: wrong account
				account_notification(contract, "Transfer", account), // index 2: match
			],
		);

		let query = AccountEventQuery::new(account).event_name("Transfer");
		let results = query.collect_from_log(tx_hash, 42, &log);

		assert_eq!(results.len(), 2);
		assert_eq!(results[0].notification_index, 0);
		assert_eq!(results[1].notification_index, 2);
		for result in &results {
			assert_eq!(result.contract, contract);
			assert_eq!(result.event_name, "Transfer");
			assert_eq!(result.tx_hash, tx_hash);
			assert_eq!(result.block_index, 42);
		}
	}

	#[tokio::test]
	async fn execute_requires_block_range() {
		let provider = MockProvider::new();
		let client = RpcClient::new(provider);

		// No range configured at all.
		let err = AccountEventQuery::new(sample_account()).execute(&client).await.unwrap_err();
		assert!(
			matches!(err, ContractError::InvalidArgError(msg) if msg.contains("starting block"))
		);

		// Only the start block configured.
		let err = AccountEventQuery::new(sample_account())
			.from_block(5)
			.execute(&client)
			.await
			.unwrap_err();
		assert!(matches!(err, ContractError::InvalidArgError(msg) if msg.contains("ending block")));

		// Inverted range.
		let err = AccountEventQuery::new(sample_account())
			.block_range(20, 10)
			.execute(&client)
			.await
			.unwrap_err();
		assert!(matches!(err, ContractError::InvalidArgError(msg) if msg.contains("greater than")));
	}

	#[tokio::test]
	async fn execute_scans_and_filters() {
		let account = sample_account();
		let other_account = H160::repeat_byte(0xbb);
		let contract = sample_contract();
		let tx_hash =
			H256::from_str("0x1111111111111111111111111111111111111111111111111111111111111111")
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
			index: 100,
			primary: None,
			next_consensus: String::new(),
			witnesses: None,
			transactions: Some(vec![tx]),
			confirmations: 1,
			next_block_hash: None,
		};

		// One notification references the target account, one does not.
		let log = application_log(
			tx_hash,
			vec![
				account_notification(contract, "Transfer", other_account),
				account_notification(contract, "Transfer", account),
			],
		);

		let provider = MockProvider::new();
		provider.push_result_with_params(
			"getblock",
			json!([100, 1]),
			serde_json::to_value(&block).unwrap(),
		);
		provider.push_result_with_params(
			"getapplicationlog",
			json!([hex::encode(tx_hash.0)]),
			serde_json::to_value(&log).unwrap(),
		);
		let client = RpcClient::new(provider.clone());

		let results = AccountEventQuery::new(account)
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

		let requests = provider.take_requests();
		let methods: Vec<String> = requests.iter().map(|(m, _)| m.clone()).collect();
		assert_eq!(methods, vec!["getblock".to_string(), "getapplicationlog".to_string()]);
	}

	#[tokio::test]
	async fn execute_honours_limit() {
		let account = sample_account();
		let contract = sample_contract();
		let tx_hash =
			H256::from_str("0x2222222222222222222222222222222222222222222222222222222222222222")
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
				account_notification(contract, "Transfer", account),
				account_notification(contract, "Transfer", account),
				account_notification(contract, "Transfer", account),
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

		let results = AccountEventQuery::new(account)
			.event_name("Transfer")
			.block_range(200, 200)
			.limit(2)
			.execute(&client)
			.await
			.unwrap();

		assert_eq!(results.len(), 2);
	}

	#[tokio::test]
	async fn execute_honours_offset() {
		let account = sample_account();
		let contract = sample_contract();
		let tx_hash =
			H256::from_str("0x3333333333333333333333333333333333333333333333333333333333333333")
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
			index: 300,
			primary: None,
			next_consensus: String::new(),
			witnesses: None,
			transactions: Some(vec![tx]),
			confirmations: 1,
			next_block_hash: None,
		};

		// Three matching notifications; offset(1) drops the first, so two remain.
		let log = application_log(
			tx_hash,
			vec![
				account_notification(contract, "Transfer", account), // index 0: skipped by offset
				account_notification(contract, "Transfer", account), // index 1
				account_notification(contract, "Transfer", account), // index 2
			],
		);

		let provider = MockProvider::new();
		provider.push_result_with_params(
			"getblock",
			json!([300, 1]),
			serde_json::to_value(&block).unwrap(),
		);
		provider.push_result_with_params(
			"getapplicationlog",
			json!([hex::encode(tx_hash.0)]),
			serde_json::to_value(&log).unwrap(),
		);
		let client = RpcClient::new(provider);

		let results = AccountEventQuery::new(account)
			.event_name("Transfer")
			.block_range(300, 300)
			.offset(1)
			.execute(&client)
			.await
			.unwrap();

		// Two of the three matches survive the offset, and they are the later ones.
		assert_eq!(results.len(), 2);
		assert_eq!(results[0].notification_index, 1);
		assert_eq!(results[1].notification_index, 2);
	}

	#[tokio::test]
	async fn execute_applies_offset_once_across_transactions() {
		let account = sample_account();
		let contract = sample_contract();
		let tx_hashes = [
			H256::repeat_byte(0x41),
			H256::repeat_byte(0x42),
			H256::repeat_byte(0x43),
			H256::repeat_byte(0x44),
		];
		let block = block(301, tx_hashes.iter().copied().map(transaction).collect());
		let provider = MockProvider::new();
		provider.push_result_with_params(
			"getblock",
			json!([301, 1]),
			serde_json::to_value(&block).unwrap(),
		);
		for tx_hash in tx_hashes {
			let log =
				application_log(tx_hash, vec![account_notification(contract, "Transfer", account)]);
			provider.push_result_with_params(
				"getapplicationlog",
				json!([hex::encode(tx_hash.0)]),
				serde_json::to_value(&log).unwrap(),
			);
		}
		let client = RpcClient::new(provider);

		let results = AccountEventQuery::new(account)
			.event_name("Transfer")
			.block_range(301, 301)
			.offset(1)
			.execute(&client)
			.await
			.unwrap();

		assert_eq!(results.iter().map(|event| event.tx_hash).collect::<Vec<_>>(), tx_hashes[1..]);
	}

	#[tokio::test]
	async fn execute_honours_page_size_without_limit() {
		let account = sample_account();
		let contract = sample_contract();
		let tx_hash = H256::repeat_byte(0x51);
		let block = block(302, vec![transaction(tx_hash)]);
		let log = application_log(
			tx_hash,
			vec![
				account_notification(contract, "Transfer", account),
				account_notification(contract, "Transfer", account),
				account_notification(contract, "Transfer", account),
			],
		);

		let provider = MockProvider::new();
		provider.push_result_with_params(
			"getblock",
			json!([302, 1]),
			serde_json::to_value(&block).unwrap(),
		);
		provider.push_result_with_params(
			"getapplicationlog",
			json!([hex::encode(tx_hash.0)]),
			serde_json::to_value(&log).unwrap(),
		);
		let client = RpcClient::new(provider);

		let results = AccountEventQuery::new(account)
			.event_name("Transfer")
			.block_range(302, 302)
			.page_size(2)
			.execute(&client)
			.await
			.unwrap();

		assert_eq!(results.len(), 2);
		assert_eq!(results[0].notification_index, 0);
		assert_eq!(results[1].notification_index, 1);
	}

	#[tokio::test]
	async fn execute_uses_smaller_of_limit_and_page_size_after_offset() {
		let account = sample_account();
		let contract = sample_contract();
		let tx_hash = H256::repeat_byte(0x52);
		let log = application_log(
			tx_hash,
			(0..5).map(|_| account_notification(contract, "Transfer", account)).collect(),
		);

		let provider = MockProvider::new();
		push_block_with_logs(&provider, 303, &[log]);
		let client = RpcClient::new(provider);

		let results = AccountEventQuery::new(account)
			.event_name("Transfer")
			.block_range(303, 303)
			.offset(1)
			.limit(3)
			.page_size(2)
			.execute(&client)
			.await
			.unwrap();

		assert_eq!(results.len(), 2);
		assert_eq!(results[0].notification_index, 1);
		assert_eq!(results[1].notification_index, 2);
	}

	#[tokio::test]
	async fn execute_applies_offset_across_blocks() {
		let account = sample_account();
		let contract = sample_contract();
		let first_tx = H256::repeat_byte(0x53);
		let second_tx = H256::repeat_byte(0x54);
		let first_log =
			application_log(first_tx, vec![account_notification(contract, "Transfer", account)]);
		let second_log = application_log(
			second_tx,
			vec![
				account_notification(contract, "Transfer", account),
				account_notification(contract, "Transfer", account),
			],
		);

		let provider = MockProvider::new();
		push_block_with_logs(&provider, 304, &[first_log]);
		push_block_with_logs(&provider, 305, &[second_log]);
		let client = RpcClient::new(provider);

		let results = AccountEventQuery::new(account)
			.event_name("Transfer")
			.block_range(304, 305)
			.offset(2)
			.execute(&client)
			.await
			.unwrap();

		assert_eq!(results.len(), 1);
		assert_eq!(results[0].block_index, 305);
		assert_eq!(results[0].tx_hash, second_tx);
		assert_eq!(results[0].notification_index, 1);
	}

	#[tokio::test]
	async fn zero_result_cap_issues_no_rpc_requests() {
		let page_provider = MockProvider::new();
		let page_client = RpcClient::new(page_provider.clone());
		let page_results = AccountEventQuery::new(sample_account())
			.block_range(1, 2)
			.page_size(0)
			.execute(&page_client)
			.await
			.unwrap();

		assert!(page_results.is_empty());
		assert!(page_provider.take_requests().is_empty());

		let limit_provider = MockProvider::new();
		let limit_client = RpcClient::new(limit_provider.clone());
		let limit_results = AccountEventQuery::new(sample_account())
			.block_range(1, 2)
			.limit(0)
			.execute_fast(&limit_client)
			.await
			.unwrap();

		assert!(limit_results.is_empty());
		assert!(limit_provider.take_requests().is_empty());
	}

	#[tokio::test]
	async fn execute_fast_delegates_with_identical_order_and_pagination() {
		let account = sample_account();
		let contract = sample_contract();
		let first_tx = H256::repeat_byte(0x61);
		let second_tx = H256::repeat_byte(0x62);
		let first_log = application_log(
			first_tx,
			vec![
				account_notification(contract, "Approval", account),
				account_notification(contract, "Transfer", account),
			],
		);
		let second_log =
			application_log(second_tx, vec![account_notification(contract, "Mint", account)]);
		let configured_provider = || {
			let provider = MockProvider::new();
			push_block_with_logs(&provider, 400, std::slice::from_ref(&first_log));
			push_block_with_logs(&provider, 401, std::slice::from_ref(&second_log));
			provider
		};
		let query = AccountEventQuery::new(account).block_range(400, 401).offset(1).page_size(2);

		let canonical_provider = configured_provider();
		let canonical_client = RpcClient::new(canonical_provider.clone());
		let canonical = query.execute(&canonical_client).await.unwrap();

		let compatibility_provider = configured_provider();
		let compatibility_client = RpcClient::new(compatibility_provider.clone());
		let compatibility = query.execute_fast(&compatibility_client).await.unwrap();

		assert_eq!(compatibility, canonical);
		assert_eq!(
			compatibility.iter().map(|event| event.event_name.as_str()).collect::<Vec<_>>(),
			vec!["Transfer", "Mint"]
		);
		assert_eq!(compatibility_provider.take_requests(), canonical_provider.take_requests());
	}
}
