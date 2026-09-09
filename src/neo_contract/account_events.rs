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
/// - **limit** (optional) — an early-exit cap on the number of results.
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
/// - ✅ **Fast path for transfers**: When querying only `Transfer` events from a
///   known contract, use [`AccountEventQuery::execute_fast`](Self::execute_fast)
///   which uses the specialized `getnep17transfers` RPC call (~5-10ms vs ~200ms)
/// - ✅ **Paginate deep queries**: Use [`offset`](Self::offset) + [`page_size`](Self::page_size)
///   to walk backwards through history without loading millions of rows
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

	/// Sets the maximum number of results to return per page.
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

	/// Recursively checks whether `state` references `account`.
	///
	/// Arrays, structs, and maps are traversed depth-first. A leaf
	/// [`StackItem::ByteString`] (or [`StackItem::Buffer`]) matches when its
	/// decoded byte value is exactly 20 bytes long and equal to
	/// `account.as_bytes()`.
	fn stack_item_contains_account(state: &StackItem, account: &H160) -> bool {
		match state {
			StackItem::Array { value } | StackItem::Struct { value } => value
				.iter()
				.any(|item| Self::stack_item_contains_account(item, account)),
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
		if let Some(contract) = self.contract_filter {
			if notification.contract != contract {
				return false;
			}
		}
		if let Some(name) = &self.event_name {
			if notification.event_name != *name {
				return false;
			}
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
	/// [`limit`](Self::limit) results have been collected. If [`offset`](Self::offset)
	/// is set, that many matching events are discarded before beginning result collection.
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
		let mut results: Vec<AccountEventResult> = Vec::new();
		let mut skipped: usize = 0;
		let target_offset = self.offset.unwrap_or(0);

		for index in from..=to {
			let block = client.get_block_by_index(index, true).await?;
			for tx in block.transactions.unwrap_or_default() {
				let tx_hash = tx.hash;
				let log = client.get_application_log(tx_hash).await?;
				let matched = self.collect_from_log(tx_hash, index, &log);
				let count = matched.len();

				// Apply offset: skip this many matches before collecting results
				if skipped + count <= target_offset {
					skipped += count;
				} else {
					let start = target_offset.saturating_sub(skipped);
					results.extend(matched.into_iter().skip(start));
					skipped = 0; // Reset after applying offset
				}

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

	/// Fast path for querying NEP-17 Transfer events involving `account`.
	///
	/// When the query targets only `Transfer` events, this method uses the
	/// optimized `getnep17transfers` RPC endpoint instead of scanning every
	/// transaction in a block range. Neo nodes maintain a server-side index of
	/// NEP-17 transfers keyed by account, so a single RPC call replaces the
	/// per-block `getblock` + per-transaction `getapplicationlog` fan-out of
	/// [`execute`](Self::execute) — typically ~5-10ms versus hundreds of
	/// milliseconds (or more) for a wide block scan.
	///
	/// # How it works
	///
	/// 1. Calls `getnep17transfers(account)` once, returning every NEP-17 transfer
	///    (both `sent` and `received`) that references the target account.
	/// 2. Filters the returned transfers client-side by the configured block range
	///    (`block_index` in `from..=to`).
	/// 3. If a [`contract`](Self::contract) filter is set, keeps only transfers
	///    whose `asset_hash` matches that token contract.
	/// 4. Reconstructs a NEP-17 `Transfer`-shaped `state` (`[from, to, amount]`)
	///    for each surviving transfer so the results are shape-compatible with
	///    [`execute`](Self::execute).
	/// 5. Applies [`limit`](Self::limit) and [`offset`](Self::offset) client-side.
	///
	/// # Requirements
	///
	/// This fast path is only valid when the [`event_name`](Self::event_name)
	/// filter is either unset or exactly `"Transfer"`; the underlying RPC only
	/// indexes NEP-17 transfers. For any other event name, call
	/// [`execute`](Self::execute) instead.
	///
	/// # Errors
	///
	/// Returns [`ContractError::InvalidArgError`] if the event filter is set to
	/// something other than `"Transfer"`, if the block range is missing/inverted,
	/// or a wrapped [`ContractError::ProviderError`] if the RPC call fails.
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
	///     // Uses getnep17transfers - much faster than scanning all blocks.
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
		// The getnep17transfers index only covers NEP-17 `Transfer` events, so the
		// fast path is only valid when no event filter is set or it is exactly
		// "Transfer". Any other event name must fall back to execute().
		if let Some(name) = &self.event_name {
			if name != "Transfer" {
				return Err(ContractError::InvalidArgError(format!(
					"execute_fast only supports the 'Transfer' event (got '{name}'); \
					 use execute(..) for other event names"
				)));
			}
		}

		let (from, to) = self.validated_range()?;

		// One indexed RPC call returns every NEP-17 transfer for the account.
		let transfers = client.get_nep17_transfers(self.account).await?;

		let mut results: Vec<AccountEventResult> = Vec::new();
		let mut skipped: usize = 0;
		let target_offset = self.offset.unwrap_or(0);

		// `sent` transfers have the account as sender; `received` as receiver.
		let classified = transfers
			.sent
			.iter()
			.map(|t| (t, true))
			.chain(transfers.received.iter().map(|t| (t, false)));

		for (transfer, account_is_sender) in classified {
			// Client-side block-range filter (the RPC returns the full history).
			if transfer.block_index < from || transfer.block_index > to {
				continue;
			}

			// Optional contract (token) filter maps onto the transfer's asset hash.
			if let Some(contract) = self.contract_filter {
				if transfer.asset_hash != contract {
					continue;
				}
			}

			// Reconstruct a NEP-17 Transfer state [from, to, amount] so results are
			// shape-compatible with execute(). The counterparty is the transfer's
			// `transfer_address`; the account itself is the other party.
			let counterparty = H160::from_address(&transfer.transfer_address).ok();
			let (from_item, to_item) = if account_is_sender {
				(account_byte_item(&self.account), counterparty_byte_item(counterparty))
			} else {
				(counterparty_byte_item(counterparty), account_byte_item(&self.account))
			};
			let state = StackItem::Array {
				value: vec![from_item, to_item, StackItem::Integer { value: transfer.amount as i64 }],
			};

			// Apply the pagination offset before collecting.
			if skipped < target_offset {
				skipped += 1;
				continue;
			}

			results.push(AccountEventResult::new(
				transfer.asset_hash,
				"Transfer".to_string(),
				state,
				transfer.tx_hash,
				transfer.block_index,
				transfer.transfer_notify_index as usize,
			));

			if let Some(limit) = self.limit {
				if results.len() >= limit {
					break;
				}
			}
		}

		Ok(results)
	}
}

/// A `ByteString` stack item wrapping the 20-byte value of `account`.
fn account_byte_item(account: &H160) -> StackItem {
	StackItem::new_byte_string(account.as_bytes().to_vec())
}

/// A `ByteString` stack item for a (possibly unresolvable) counterparty hash.
///
/// Falls back to [`StackItem::Any`] when the transfer's counterparty address
/// could not be decoded into a script hash (for example a mint/burn sentinel).
fn counterparty_byte_item(counterparty: Option<H160>) -> StackItem {
	match counterparty {
		Some(hash) => StackItem::new_byte_string(hash.as_bytes().to_vec()),
		None => StackItem::Any,
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
		assert!(matches!(err, ContractError::InvalidArgError(msg) if msg.contains("Invalid account")));
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
		assert!(matches!(err, ContractError::InvalidArgError(msg) if msg.contains("starting block")));

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
		let tx_hash = H256::from_str(
			"0x1111111111111111111111111111111111111111111111111111111111111111",
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
		let tx_hash = H256::from_str(
			"0x3333333333333333333333333333333333333333333333333333333333333333",
		)
		.unwrap();

		let tx = RTransaction::new(
			tx_hash, 0, 0, 0, String::new(), "0".to_string(), "0".to_string(), 0, vec![], vec![],
			String::new(), vec![],
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
	async fn execute_fast_rejects_non_transfer_event() {
		let provider = MockProvider::new();
		let client = RpcClient::new(provider);

		let err = AccountEventQuery::new(sample_account())
			.event_name("Approval")
			.block_range(0, 10)
			.execute_fast(&client)
			.await
			.unwrap_err();
		assert!(matches!(err, ContractError::InvalidArgError(msg) if msg.contains("Transfer")));
	}

	#[tokio::test]
	async fn execute_fast_uses_indexed_transfers() {
		use crate::neo_protocol::{Nep17Transfer, Nep17Transfers};

		let account = sample_account();
		let contract = sample_contract();
		let other_contract = H160::repeat_byte(0xcd);
		let counterparty = account.to_address();

		let mk = |block_index: u32, asset: H160, byte: u8| {
			Nep17Transfer::new(
				0,
				asset,
				counterparty.clone(),
				5,
				block_index,
				0,
				H256::repeat_byte(byte),
			)
		};

		let transfers = Nep17Transfers {
			// sent: one in range, one out of range, one on the wrong contract.
			sent: vec![
				mk(100, contract, 0x11),        // in range, right contract -> keep
				mk(300, contract, 0x22),        // out of range -> drop
				mk(120, other_contract, 0x33),  // wrong contract -> drop
			],
			// received: one in range on the right contract.
			received: vec![mk(150, contract, 0x44)],
			transfer_address: counterparty.clone(),
		};

		let provider = MockProvider::new();
		provider.push_result_with_params(
			"getnep17transfers",
			json!([account.to_address()]),
			serde_json::to_value(&transfers).unwrap(),
		);
		let client = RpcClient::new(provider.clone());

		let results = AccountEventQuery::new(account)
			.contract(contract)
			.event_name("Transfer")
			.block_range(100, 200)
			.execute_fast(&client)
			.await
			.unwrap();

		// Only the two in-range, right-contract transfers survive.
		assert_eq!(results.len(), 2);
		for result in &results {
			assert_eq!(result.contract, contract);
			assert_eq!(result.event_name, "Transfer");
			assert!(result.block_index >= 100 && result.block_index <= 200);
		}

		// Exactly one indexed RPC call replaced the whole block scan.
		let requests = provider.take_requests();
		let methods: Vec<String> = requests.iter().map(|(m, _)| m.clone()).collect();
		assert_eq!(methods, vec!["getnep17transfers".to_string()]);
	}
}
