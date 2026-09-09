#![deny(missing_docs)]
//! High-level, AWS-style entrypoint for Neo N3.
//!
//! The `sdk` module is the recommended starting point for application code.
//! It exposes a small, opinionated surface ([`Neo`], [`NeoBuilder`],
//! [`SdkConfig`], [`Token`], [`Balance`], …) that handles boilerplate the
//! lower-level modules leave to the caller: endpoint selection, retry,
//! caching, error normalization, and decimal-safe balance handling.
//!
//! ## Quick start
//!
//! ```no_run
//! use neo3::sdk::Neo;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Connect to TestNet with sensible defaults (30s timeout, 3 retries, caching on).
//! let neo = Neo::testnet().await?;
//!
//! // Connect to any RPC endpoint (private nodes, load balancers, sandboxes).
//! let neo = Neo::connect("https://my-node.example.com:443").await?;
//!
//! // Or honour `NEO_RPC_URL` from the environment for 12-factor deployments.
//! let neo = Neo::from_env().await?;
//! # Ok(()) }
//! ```
//!
//! ## Builder pattern
//!
//! When defaults don't fit (longer timeouts, more retries, custom endpoints,
//! disabling cache), reach for the builder:
//!
//! ```no_run
//! use neo3::sdk::{Neo, Network};
//! use std::time::Duration;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let neo = Neo::builder()
//!     .network(Network::MainNet)
//!     .timeout(Duration::from_secs(60))
//!     .retries(5)
//!     .cache(true)
//!     .build()
//!     .await?;
//! # Ok(()) }
//! ```
//!
//! ## Error handling
//!
//! Every fallible method returns the unified
//! [`crate::neo_error::unified::NeoError`] type. It supports
//! AWS-SDK-style classification:
//!
//! ```no_run
//! use neo3::sdk::Neo;
//! use neo3::neo_error::unified::NeoErrorKind;
//!
//! # #[tokio::main]
//! # async fn main() {
//! let neo = Neo::testnet().await.unwrap();
//! if let Err(err) = neo.get_block_height().await {
//!     if err.is_retryable() {
//!         tracing::warn!(kind = ?err.kind(), "transient failure; retrying");
//!     } else {
//!         tracing::error!(?err, "fatal");
//!     }
//!     match err.kind() {
//!         NeoErrorKind::RateLimit => { /* back off */ }
//!         NeoErrorKind::Network   => { /* health-check endpoint */ }
//!         _ => {}
//!     }
//! }
//! # }
//! ```
//!
//! ## Sub-modules
//!
//! - [`hd_wallet`] — BIP-39/44 hierarchical deterministic wallets.
//! - [`transaction_simulator`] — preview VM state, gas, and effects before
//!   submitting a transaction.
//! - [`unified`] — cross-chain [`EcosystemClient`](unified::EcosystemClient)
//!   bridging Neo N3 ↔ Neo X (EVM).
//! - `websocket` — push-based event subscriptions (requires `ws` feature).

pub mod fee;
pub mod hd_wallet;
mod retry;
pub mod transaction_simulator;
/// Cross-chain unified client bridging Neo N3 and Neo X (EVM).
///
/// See [`unified::EcosystemClient`] for the entry point. This is currently
/// the SDK's Alloy-backed cross-chain wrapper.
pub mod unified;
#[cfg(feature = "ws")]
pub mod websocket;

use crate::sdk::fee::{resolve_fee_adjustment, FeePolicy};
pub use self::fee::{FeeAdjustment, FeePriority};
use self::retry::{retry_network, DEFAULT_RETRY_DELAY};
use crate::{
	neo_clients::{APITrait, HttpProvider, RpcCache, RpcClient},
	neo_error::unified::{ErrorRecovery, NeoError},
	neo_types::VMState,
	neo_types::{ContractParameter, ScriptHash, ScriptHashExtension, StackItem},
	neo_wallets::wallet::Wallet,
};
use hex_literal::hex;
use num_bigint::{BigInt, Sign};
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

/// Main entry point for the Neo SDK
///
/// Provides high-level, user-friendly methods for common blockchain operations
/// while maintaining access to lower-level APIs when needed.
///
/// # Examples
///
/// ```no_run
/// use neo3::sdk::Neo;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Quick connection to testnet
///     let neo = Neo::testnet().await?;
///     
///     // Check balance
///     let balance = neo.get_balance("NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc").await?;
///     println!("Balance: {} GAS", balance.gas);
///     
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub struct Neo {
	client: Arc<RpcClient<HttpProvider>>,
	network: Network,
	endpoint: String,
	cache: Option<RpcCache>,
	config: SdkConfig,
	/// Fee strategy applied to transactions built by the send flow.
	///
	/// Defaults to dynamic [`GasEstimator`](crate::neo_builder::transaction::GasEstimator)-backed
	/// estimation with a [`FeePriority::Medium`] margin; override via
	/// [`NeoBuilder::fee_policy`] or [`Neo::with_fee_policy`].
	fee_policy: FeePolicy,
}

/// Network configuration
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Network {
	/// Neo MainNet
	MainNet,
	/// Neo TestNet
	TestNet,
	/// Custom network with RPC endpoint
	Custom(String),
}

/// SDK configuration options
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SdkConfig {
	/// Request timeout
	pub timeout: Duration,
	/// Number of retries for failed requests
	pub retries: u32,
	/// Enable caching
	pub cache_enabled: bool,
	/// Enable metrics collection
	pub metrics_enabled: bool,
}

impl SdkConfig {
	/// Creates a new builder for the configuration
	#[must_use]
	pub fn builder() -> SdkConfigBuilder {
		SdkConfigBuilder::default()
	}
}

/// Builder for `SdkConfig`
#[derive(Debug, Default, Clone)]
pub struct SdkConfigBuilder {
	timeout: Option<Duration>,
	retries: Option<u32>,
	cache_enabled: Option<bool>,
	metrics_enabled: Option<bool>,
}

impl SdkConfigBuilder {
	/// Sets the request timeout
	#[must_use]
	pub fn timeout(mut self, val: Duration) -> Self {
		self.timeout = Some(val);
		self
	}

	/// Sets the number of retries
	#[must_use]
	pub fn retries(mut self, val: u32) -> Self {
		self.retries = Some(val);
		self
	}

	/// Enables or disables caching
	#[must_use]
	pub fn cache_enabled(mut self, val: bool) -> Self {
		self.cache_enabled = Some(val);
		self
	}

	/// Enables or disables metrics
	#[must_use]
	pub fn metrics_enabled(mut self, val: bool) -> Self {
		self.metrics_enabled = Some(val);
		self
	}

	/// Builds the `SdkConfig`
	pub fn build(self) -> SdkConfig {
		let default = SdkConfig::default();
		SdkConfig {
			timeout: self.timeout.unwrap_or(default.timeout),
			retries: self.retries.unwrap_or(default.retries),
			cache_enabled: self.cache_enabled.unwrap_or(default.cache_enabled),
			metrics_enabled: self.metrics_enabled.unwrap_or(default.metrics_enabled),
		}
	}
}

impl Default for SdkConfig {
	fn default() -> Self {
		Self {
			timeout: Duration::from_secs(30),
			retries: 3,
			cache_enabled: true,
			metrics_enabled: false,
		}
	}
}

/// Exact token amount represented as a non-negative integer with an implied decimal scale.
///
/// Neo N3 NEP-17 balances are returned as raw integers (base units). This type keeps the raw
/// amount exact and provides safe, deterministic formatting without floating-point rounding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DecimalAmount {
	raw: String,
	decimals: u8,
}

impl<'de> Deserialize<'de> for DecimalAmount {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		#[derive(Deserialize)]
		struct DecimalAmountRepr {
			raw: String,
			decimals: u8,
		}

		let value = DecimalAmountRepr::deserialize(deserializer)?;
		Self::try_from_raw(value.raw, value.decimals).map_err(serde::de::Error::custom)
	}
}

/// Reasons [`DecimalAmount::parse`] can reject a string input.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DecimalAmountParseError {
	/// The supplied input was empty after trimming whitespace.
	Empty,
	/// The input started with `-`; negative balances are not representable.
	NegativeNotAllowed,
	/// The input did not match the expected `<digits>[.<digits>]` shape.
	InvalidFormat,
	/// The input contained a non-digit, non-`.` character.
	InvalidCharacter,
	/// The fractional part contained more digits than the declared `decimals` allow.
	TooManyFractionalDigits {
		/// How many fractional digits the input supplied.
		provided: usize,
		/// How many fractional digits are allowed (== the token's `decimals`).
		allowed: u8,
	},
}

impl fmt::Display for DecimalAmountParseError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Empty => f.write_str("amount is empty"),
			Self::NegativeNotAllowed => f.write_str("amount must be non-negative"),
			Self::InvalidFormat => f.write_str("invalid amount format"),
			Self::InvalidCharacter => f.write_str("amount contains invalid characters"),
			Self::TooManyFractionalDigits { provided, allowed } => {
				write!(f, "too many fractional digits: provided {}, allowed {}", provided, allowed)
			},
		}
	}
}

impl std::error::Error for DecimalAmountParseError {}

impl DecimalAmount {
	/// Try to create an amount from a raw (base-unit) integer string and a decimal scale.
	///
	/// `raw` must be a non-negative base-10 integer. Surrounding whitespace and a leading `+`
	/// are accepted, and leading zeros are normalized.
	pub fn try_from_raw(
		raw: impl Into<String>,
		decimals: u8,
	) -> Result<Self, DecimalAmountParseError> {
		let raw = raw.into();
		let raw = raw.trim();
		if raw.is_empty() {
			return Err(DecimalAmountParseError::Empty);
		}
		if raw.starts_with('-') {
			return Err(DecimalAmountParseError::NegativeNotAllowed);
		}

		let raw = raw.strip_prefix('+').unwrap_or(raw);
		if raw.is_empty() {
			return Err(DecimalAmountParseError::InvalidFormat);
		}
		if !raw.chars().all(|c| c.is_ascii_digit()) {
			return Err(DecimalAmountParseError::InvalidCharacter);
		}

		let raw = raw.trim_start_matches('0');
		let raw = if raw.is_empty() { "0" } else { raw };
		Ok(Self { raw: raw.to_string(), decimals })
	}

	/// Create an amount from a raw (base-unit) integer string and a decimal scale.
	///
	/// `raw` must be a non-negative base-10 integer. Prefer [`Self::try_from_raw`] when the
	/// value comes from an RPC response, cache, configuration, or user input. Malformed input
	/// produces zero here for backward compatibility with this deprecated constructor.
	#[deprecated(note = "use DecimalAmount::try_from_raw to handle malformed input")]
	pub fn from_raw(raw: impl Into<String>, decimals: u8) -> Self {
		Self::try_from_raw(raw, decimals)
			.unwrap_or_else(|_| Self { raw: "0".to_string(), decimals })
	}

	/// Parse a human-formatted decimal string into base units with a fixed `decimals` scale.
	///
	/// Examples:
	/// - `decimals = 8`, `"50.5"` -> raw `"5050000000"`
	/// - `decimals = 0`, `"10"` -> raw `"10"`
	pub fn parse(amount: &str, decimals: u8) -> Result<Self, DecimalAmountParseError> {
		let amount = amount.trim();
		if amount.is_empty() {
			return Err(DecimalAmountParseError::Empty);
		}
		if amount.starts_with('-') {
			return Err(DecimalAmountParseError::NegativeNotAllowed);
		}
		if !amount.chars().any(|c| c.is_ascii_digit()) {
			return Err(DecimalAmountParseError::InvalidFormat);
		}

		let mut iter = amount.split('.');
		let whole = iter.next().unwrap_or("");
		let frac = iter.next();
		if iter.next().is_some() {
			return Err(DecimalAmountParseError::InvalidFormat);
		}

		let whole = if whole.is_empty() { "0" } else { whole };
		let frac = frac.unwrap_or("");

		if !whole.chars().all(|c| c.is_ascii_digit()) || !frac.chars().all(|c| c.is_ascii_digit()) {
			return Err(DecimalAmountParseError::InvalidCharacter);
		}

		let allowed = decimals as usize;
		let mut frac = frac.to_string();
		if frac.len() > allowed {
			let (kept, extra) = frac.split_at(allowed);
			if extra.chars().all(|c| c == '0') {
				frac = kept.to_string();
			} else {
				return Err(DecimalAmountParseError::TooManyFractionalDigits {
					provided: frac.len(),
					allowed: decimals,
				});
			}
		}
		while frac.len() < allowed {
			frac.push('0');
		}

		let whole = whole.trim_start_matches('0');
		let whole = if whole.is_empty() { "0" } else { whole };
		let raw = format!("{}{}", whole, frac);
		Self::try_from_raw(raw, decimals)
	}

	/// Raw base-unit integer as a base-10 string.
	pub fn raw(&self) -> &str {
		&self.raw
	}

	/// Decimal scale (number of fractional digits).
	pub fn decimals(&self) -> u8 {
		self.decimals
	}

	/// Format as a fixed-scale decimal string (always prints exactly `decimals` fractional digits).
	pub fn to_fixed_string(&self) -> String {
		let decimals = self.decimals as usize;
		if decimals == 0 {
			return self.raw.clone();
		}

		let raw = self.raw.as_str();
		let len = raw.len();
		if len > decimals {
			let (int_part, frac_part) = raw.split_at(len - decimals);
			format!("{}.{}", int_part, frac_part)
		} else {
			let zeros = "0".repeat(decimals - len);
			format!("0.{}{}", zeros, raw)
		}
	}

	/// Convert raw base-units to `i64` when it fits (useful for Neo VM integers).
	pub fn raw_i64(&self) -> Option<i64> {
		self.raw.parse::<i64>().ok()
	}
}

impl fmt::Display for DecimalAmount {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&self.to_fixed_string())
	}
}

/// Balance information for an address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
	/// NEO token balance
	pub neo: u64,
	/// GAS token balance (exact; 8 decimals)
	pub gas: DecimalAmount,
	/// Other NEP-17 token balances
	pub tokens: Vec<TokenBalance>,
}

/// Individual token balance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
	/// Token contract hash
	pub contract: ScriptHash,
	/// Token symbol
	pub symbol: String,
	/// Token amount (exact; uses the token's on-chain decimals)
	pub amount: DecimalAmount,
}

/// Transaction hash type
pub type TxHash = String;

/// Common tokens
#[derive(Debug, Clone)]
pub enum Token {
	/// Native NEO token
	NEO,
	/// Native GAS token
	GAS,
	/// Custom NEP-17 token
	Custom(ScriptHash),
}

impl Token {
	/// Native NEO governance token script hash.
	pub const NEO_HASH: [u8; 20] = hex!("ef4073a0f2b305a38ec4050e4d3d28bc40ea63f5");
	/// Native GAS utility token script hash.
	pub const GAS_HASH: [u8; 20] = hex!("d2a4cff31913016155e38e474a2c06d08be276cf");

	/// Returns the on-chain script hash for this token.
	pub fn contract_hash(&self) -> ScriptHash {
		match self {
			Token::NEO => ScriptHash::from(Self::NEO_HASH),
			Token::GAS => ScriptHash::from(Self::GAS_HASH),
			Token::Custom(hash) => *hash,
		}
	}
}

/// Recovery hints shared by every "no default account configured" error.
pub(crate) fn no_default_account_error() -> NeoError {
	NeoError::Wallet {
		message: "No default account set in wallet".to_string(),
		source: None,
		recovery: ErrorRecovery::new()
			.suggest("Set a default account using set_default_account()")
			.suggest("Add an account to the wallet first"),
	}
}

/// Validation error helper for address-shaped inputs.
fn invalid_address_error<E: fmt::Display>(field: &str, value: &str, err: E) -> NeoError {
	NeoError::Validation {
		message: format!("Invalid {} address: {}", field, err),
		field: field.to_string(),
		value: Some(value.to_string()),
		recovery: ErrorRecovery::new()
			.suggest("Check the address format")
			.suggest("Ensure it's a valid Neo N3 address"),
	}
}

/// Send a signed transaction with bounded retry.
///
/// Mirrors [`retry_network`] but handles the borrow-checker constraint that
/// `Transaction::send_tx` takes `&mut self` — repeatedly borrowing the
/// transaction inside an `FnMut` closure is illegal, so we expand the loop
/// inline.
async fn send_tx_with_retry<'a>(
	tx: &mut crate::neo_builder::Transaction<'a, HttpProvider>,
	attempts: u32,
	delay: Duration,
	context: &str,
) -> Result<crate::neo_protocol::RawTransaction, NeoError> {
	let attempts = attempts.max(1);
	let tx_id = tx.tx_id().map_err(|err| NeoError::transaction(context, err))?;
	for attempt in 1..=attempts {
		match tx.send_tx().await {
			Ok(result) => return Ok(result),
			Err(err) => {
				if matches!(
					&err,
					crate::neo_builder::TransactionError::ProviderError(error)
						if error.is_already_known_transaction()
				) {
					tracing::debug!(transaction_hash = %tx_id, "transaction already accepted by node");
					return Ok(crate::neo_protocol::RawTransaction::new(tx_id));
				}

				let retryable = matches!(
					&err,
					crate::neo_builder::TransactionError::ProviderError(error)
						if error.is_retryable()
				);
				let retry_delay = match &err {
					crate::neo_builder::TransactionError::ProviderError(error) => {
						error.retry_after().unwrap_or(delay)
					},
					_ => delay,
				};
				if !retryable || attempt == attempts {
					return Err(match err {
						crate::neo_builder::TransactionError::ProviderError(error)
							if error.is_transaction_rejection() =>
						{
							NeoError::Transaction {
								message: format!("{}: {}", context, error),
								tx_hash: Some(format!("{tx_id:#x}")),
								source: Some(Box::new(error)),
								recovery: ErrorRecovery::new().suggest(
									"Review the transaction fees, policy, script, and signatures",
								),
							}
						},
						crate::neo_builder::TransactionError::ProviderError(error) => {
							NeoError::provider(context, error)
						},
						other => NeoError::transaction(context, other),
					});
				}

				tracing::warn!(
					attempt = attempt,
					max_attempts = attempts,
					context = %context,
					error = %err,
					retry_delay = ?retry_delay,
					"send_tx failed; retrying"
				);
				tokio::time::sleep(retry_delay).await;
			},
		}
	}

	unreachable!("the retry loop executes at least once")
}

/// Build, sign, and send a NEP-17 transfer with retry-aware broadcast.
///
/// Shared by [`Neo::transfer`] and [`Transfer::execute`] so both paths get
/// identical script construction, validity window, and send semantics. The
/// 4th transfer argument carries `memo` when supplied (the receiving contract
/// sees it); otherwise `any` is passed.
pub async fn build_and_send_transfer(
	client: &RpcClient<HttpProvider>,
	from_account: &crate::neo_protocol::Account,
	to: &str,
	amount: u64,
	token: Token,
	memo: Option<&str>,
	max_attempts: u32,
	fee_policy: FeePolicy,
	sponsor: Option<ScriptHash>,
) -> Result<TxHash, NeoError> {
	use crate::neo_builder::{AccountSigner, CallFlags, ScriptBuilder, TransactionBuilder};
	use crate::neo_types::ScriptHashExtension;

	let to_hash = ScriptHash::from_address(to).map_err(|e| invalid_address_error("to", to, e))?;
	let contract_hash = token.contract_hash();

	let amount_i64 = i64::try_from(amount).map_err(|_| NeoError::Validation {
		message: "Amount is too large to fit Neo VM integer".to_string(),
		field: "amount".to_string(),
		value: Some(amount.to_string()),
		recovery: ErrorRecovery::new()
			.suggest("Use a smaller amount")
			.suggest("Split into multiple transfers if needed"),
	})?;

	let mut sb = ScriptBuilder::new();
	// The 4th transfer argument is the NEP-17 `data` parameter: forward a
	// supplied memo there instead of silently dropping it.
	let data = match memo {
		Some(memo) => ContractParameter::string(memo.to_string()),
		None => ContractParameter::any(),
	};
	sb.contract_call(
		&contract_hash,
		"transfer",
		&[
			ContractParameter::h160(&from_account.get_script_hash()),
			ContractParameter::h160(&to_hash),
			ContractParameter::integer(amount_i64),
			data,
		],
		Some(CallFlags::All),
	)
	.map_err(|e| {
		NeoError::contract(
			"Failed to build transfer script",
			Some(contract_hash.to_hex()),
			Some("transfer".into()),
			e,
		)
	})?;

	let signer = AccountSigner::called_by_entry(from_account)
		.map_err(|e| NeoError::transaction("Failed to create signer", e))?;

	// When a sponsor is supplied, build a gas-less signer set: the sponsor is
	// placed first as the fee-payer (`None` scope) and the sender follows as
	// `CalledByEntry`. A placeholder relayer witness is prepared for the sponsor
	// to complete off-chain. Non-sponsored transfers keep their existing single
	// signer, so this is purely additive.
	let signers: Vec<crate::builder::Signer> = if let Some(sponsor_hash) = sponsor {
		use crate::neo_contract::gasless::{build_sponsored_signers, setup_relayer_witness_for_tx};

		// Prepare the placeholder relayer witness (sponsor signs off-chain).
		let _relayer_witness = setup_relayer_witness_for_tx(sponsor_hash);
		build_sponsored_signers(from_account, sponsor_hash)
			.map_err(|e| NeoError::transaction("Failed to build sponsored signers", e))?
	} else {
		vec![signer.into()]
	};

	let current_height =
		retry_network("fetch current block height", max_attempts, DEFAULT_RETRY_DELAY, || async {
			client.get_block_count().await
		})
		.await?;

	let script = sb.to_bytes();

	// A sponsored transaction has its fees covered by the sponsor, so estimate
	// with a fixed (non-priority) policy; otherwise use the caller's policy.
	let effective_fee_policy = if sponsor.is_some() { FeePolicy::default() } else { fee_policy };

	// Estimate fees dynamically (priority-aware) via the GasEstimator before
	// signing. When estimation is unavailable this degrades to a zero
	// adjustment, preserving the builder's existing fee behaviour.
	let adjustment = resolve_fee_adjustment(client, &script, &signers, &effective_fee_policy).await;

	let mut tb = TransactionBuilder::with_client(client);
	tb.extend_script(script);
	tb.set_signers(signers)
		.map_err(|e| NeoError::transaction("Failed to set signers", e))?;
	tb.valid_until_block(current_height + 5760)
		.map_err(|e| NeoError::transaction("Invalid valid-until-block", e))?;
	tb.set_additional_system_fee(adjustment.additional_system_fee);
	tb.set_additional_network_fee(adjustment.additional_network_fee);

	let mut tx = tb
		.sign()
		.await
		.map_err(|e| NeoError::transaction("Failed to sign transfer", e))?;

	let result =
		send_tx_with_retry(&mut tx, max_attempts, DEFAULT_RETRY_DELAY, "send transfer transaction")
			.await?;

	Ok(result.hash.to_string())
}

impl Neo {
	/// Connect to Neo TestNet with default configuration
	///
	/// # Examples
	///
	/// ```no_run
	/// use neo3::sdk::Neo;
	///
	/// # #[tokio::main]
	/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let neo = Neo::testnet().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn testnet() -> Result<Self, NeoError> {
		Self::builder().network(Network::TestNet).build().await
	}

	/// Connect to Neo MainNet with default configuration
	///
	/// # Examples
	///
	/// ```no_run
	/// use neo3::sdk::Neo;
	///
	/// # #[tokio::main]
	/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let neo = Neo::mainnet().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn mainnet() -> Result<Self, NeoError> {
		Self::builder().network(Network::MainNet).build().await
	}

	/// Connect to a custom RPC endpoint with default configuration.
	///
	/// Convenience for `Neo::builder().network(Network::Custom(...)).build()`,
	/// modeled after the AWS SDK's `Client::from_conf` quick path. Useful for
	/// private networks, local dev nodes, and load-balanced endpoints.
	///
	/// # Examples
	///
	/// ```no_run
	/// use neo3::sdk::Neo;
	///
	/// # #[tokio::main]
	/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let neo = Neo::connect("https://my-node.example.com:443").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn connect(endpoint: impl Into<String>) -> Result<Self, NeoError> {
		Self::builder().network(Network::Custom(endpoint.into())).build().await
	}

	/// Build a [`Neo`] using `$NEO_RPC_URL` from the environment, falling back to TestNet.
	///
	/// Mirrors the AWS SDK's `from_env()` / `load_defaults_from_env()`
	/// convention so 12-factor apps and CI pipelines can configure the SDK
	/// without touching code.
	///
	/// | Env var | Effect |
	/// |---------|--------|
	/// | `NEO_RPC_URL` | Connect to this RPC endpoint (overrides everything). |
	/// | _unset_ | Falls back to TestNet for safety. |
	///
	/// # Examples
	///
	/// ```no_run
	/// use neo3::sdk::Neo;
	///
	/// # #[tokio::main]
	/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// // export NEO_RPC_URL=https://mainnet1.neo.org:443
	/// let neo = Neo::from_env().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn from_env() -> Result<Self, NeoError> {
		match std::env::var("NEO_RPC_URL") {
			Ok(url) if !url.is_empty() => Self::connect(url).await,
			_ => Self::testnet().await,
		}
	}

	/// Create a new SDK builder for custom configuration
	///
	/// # Examples
	///
	/// ```no_run
	/// use neo3::sdk::{Neo, Network};
	/// use std::time::Duration;
	///
	/// # #[tokio::main]
	/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let neo = Neo::builder()
	///     .network(Network::TestNet)
	///     .timeout(Duration::from_secs(60))
	///     .retries(5)
	///     .build()
	///     .await?;
	/// # Ok(())
	/// # }
	/// ```
	#[must_use]
	pub fn builder() -> NeoBuilder {
		NeoBuilder::default()
	}

	/// Create a new instance with an overridden fee policy.
	///
	/// Returns a copy of this SDK instance with the supplied fee policy replacing
	/// the existing one. The underlying client, network, and config remain unchanged.
	///
	/// # Examples
	///
	/// ```no_run
	/// use neo3::sdk::{Neo, fee::{FeePolicy, FeePriority}};
	///
	/// # #[tokio::main]
	/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let neo = Neo::testnet().await?;
	///
	/// // High-priority dynamic estimation.
	/// let aggressive = neo.with_fee_policy(FeePolicy::dynamic(FeePriority::High));
	///
	/// // Fixed additional fees (bypasses estimation).
	/// let offline = neo.with_fee_policy(FeePolicy::fixed(1000, 500));
	/// # Ok(())
	/// # }
	/// ```
	#[must_use]
	pub fn with_fee_policy(&self, fee_policy: FeePolicy) -> Self {
		Self {
			client: self.client.clone(),
			network: self.network.clone(),
			endpoint: self.endpoint.clone(),
			cache: None,
			config: self.config.clone(),
			fee_policy,
		}
	}

	/// Get the balance of an address
	///
	/// Returns NEO, GAS, and all NEP-17 token balances.
	///
	/// # Examples
	///
	/// ```no_run
	/// use neo3::sdk::Neo;
	///
	/// # #[tokio::main]
	/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let neo = Neo::testnet().await?;
	/// let balance = neo.get_balance("NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc").await?;
	/// println!("NEO: {}, GAS: {}", balance.neo, balance.gas);
	/// # Ok(())
	/// # }
	/// ```
	pub async fn get_balance(&self, address: &str) -> Result<Balance, NeoError> {
		let started = std::time::Instant::now();
		let result = self.get_balance_inner(address).await;
		self.record_operation("get_balance", started.elapsed(), result.is_ok());
		result
	}

	async fn get_balance_inner(&self, address: &str) -> Result<Balance, NeoError> {
		use crate::neo_types::ScriptHashExtension;

		let script_hash = ScriptHash::from_address(address)
			.map_err(|e| invalid_address_error("address", address, e))?;

		if let Some(cache) = &self.cache {
			let cache_key = format!("balance:{}", address);
			if let Some(cached) = cache.get(&cache_key).await {
				if let Ok(balance) = serde_json::from_value::<Balance>(cached) {
					return Ok(balance);
				}
			}
		}

		let max_attempts = self.config.retries.saturating_add(1);

		let neo_hash = ScriptHash::from(Token::NEO_HASH);
		let neo_balance =
			retry_network("fetch NEO balance", max_attempts, DEFAULT_RETRY_DELAY, || async {
				self.client
					.invoke_function(
						&neo_hash,
						"balanceOf".to_string(),
						vec![ContractParameter::h160(&script_hash)],
						None,
					)
					.await
			})
			.await?;

		let gas_hash = ScriptHash::from(Token::GAS_HASH);
		let gas_balance =
			retry_network("fetch GAS balance", max_attempts, DEFAULT_RETRY_DELAY, || async {
				self.client
					.invoke_function(
						&gas_hash,
						"balanceOf".to_string(),
						vec![ContractParameter::h160(&script_hash)],
						None,
					)
					.await
			})
			.await?;

		let neo = neo_balance
			.stack
			.first()
			.ok_or_else(|| invalid_balance_response("NEO", "missing stack item"))?;
		let neo = parse_balance_stack_item_u64(neo, "NEO")?;

		let gas_raw = gas_balance
			.stack
			.first()
			.ok_or_else(|| invalid_balance_response("GAS", "missing stack item"))?;
		let gas_raw = parse_balance_stack_item_u64(gas_raw, "GAS")?;
		let gas = DecimalAmount::try_from_raw(gas_raw.to_string(), 8)
			.map_err(|err| invalid_balance_response("GAS", err.to_string()))?;

		let nep17 =
			retry_network("fetch NEP-17 balances", max_attempts, DEFAULT_RETRY_DELAY, || async {
				self.client.get_nep17_balances(script_hash).await
			})
			.await?;

		let mut tokens = Vec::new();
		for b in nep17.balances {
			if b.asset_hash == neo_hash || b.asset_hash == gas_hash {
				continue;
			}

			// Standard `getnep17balances` responses omit `symbol`/`decimals`;
			// fall back to querying the token contract instead of failing the
			// whole balance request for one unknown token.
			let asset_hash = b.asset_hash;
			let decimals = match b.decimals.as_deref() {
				Some(raw) => parse_nep17_decimals(raw, &asset_hash)?,
				None => self.fetch_nep17_decimals(&asset_hash).await?,
			};
			let amount = DecimalAmount::try_from_raw(b.amount, decimals)
				.map_err(|err| invalid_balance_response(&asset_hash.to_hex(), err.to_string()))?;

			let symbol = match b.symbol.or(b.name) {
				Some(symbol) => symbol,
				None => self
					.fetch_nep17_symbol(&asset_hash)
					.await
					.unwrap_or_else(|| asset_hash.to_hex()),
			};

			tokens.push(TokenBalance { contract: asset_hash, symbol, amount });
		}

		let balance = Balance { neo, gas, tokens };

		// Cache for subsequent reads (short TTL; balances change frequently).
		if let Some(cache) = &self.cache {
			if let Ok(value) = serde_json::to_value(&balance) {
				cache.cache_balance(address.to_string(), value).await;
			}
		}

		Ok(balance)
	}

	/// Query a NEP-17 token's `decimals()` from the token contract itself.
	///
	/// Used when the node's `getnep17balances` response omits token metadata.
	async fn fetch_nep17_decimals(&self, contract: &ScriptHash) -> Result<u8, NeoError> {
		let result = self
			.client
			.invoke_function(contract, "decimals".to_string(), vec![], None)
			.await
			.map_err(|e| invalid_balance_response(&contract.to_hex(), e.to_string()))?;

		let item = result
			.stack
			.first()
			.ok_or_else(|| invalid_balance_response("decimals", "missing stack item"))?;
		let decimals = item
			.as_int()
			.ok_or_else(|| invalid_balance_response("decimals", "not an integer stack item"))?;

		u8::try_from(decimals)
			.map_err(|_| invalid_balance_response("decimals", format!("out of range: {decimals}")))
	}

	/// Query a NEP-17 token's `symbol()` from the token contract itself.
	async fn fetch_nep17_symbol(&self, contract: &ScriptHash) -> Option<String> {
		let result = self
			.client
			.invoke_function(contract, "symbol".to_string(), vec![], None)
			.await
			.ok()?;
		result.stack.first().and_then(|item| item.as_string())
	}

	/// Transfer tokens from one address to another
	///
	/// Handles all the complexity of building, signing, and sending the transaction.
	///
	/// # Examples
	///
	/// ```no_run
	/// use neo3::neo_wallets::wallet::Wallet;
	/// use neo3::sdk::{Neo, Token};
	///
	/// # #[tokio::main]
	/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let neo = Neo::testnet().await?;
	/// let wallet = Wallet::new();
	/// let tx_hash = neo
	///     .transfer(
	///         &wallet,
	///         "NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc",
	///         100,
	///         Token::GAS,
	///     )
	///     .await?;
	/// println!("Transaction sent: {}", tx_hash);
	/// # Ok(())
	/// # }
	/// ```
	pub async fn transfer(
		&self,
		from: &Wallet,
		to: &str,
		amount: u64,
		token: Token,
	) -> Result<TxHash, NeoError> {
		let started = std::time::Instant::now();
		let result = self.transfer_inner(from, to, amount, token).await;
		self.record_transaction_metric("transfer", started.elapsed(), result.is_ok());
		result
	}

	async fn transfer_inner(
		&self,
		from: &Wallet,
		to: &str,
		amount: u64,
		token: Token,
	) -> Result<TxHash, NeoError> {
		use crate::neo_wallets::WalletTrait;

		let max_attempts = self.config.retries.saturating_add(1);
		let from_account = from.default_account().ok_or_else(no_default_account_error)?;

		let tx_hash = build_and_send_transfer(
			&self.client,
			from_account,
			to,
			amount,
			token,
			None,
			max_attempts,
			self.fee_policy,
			None, // No sponsor by default for backwards compatibility
		)
		.await?;

		// Sent transactions change balances; drop stale cached balances so a
		// read-after-write does not observe pre-transfer amounts.
		if let Some(cache) = &self.cache {
			cache.invalidate_by_prefix("balance:").await;
		}

		Ok(tx_hash)
	}

	/// Deploy a smart contract
	///
	/// Simplifies the contract deployment process.
	///
	/// # Examples
	///
	/// ```no_run
	/// use neo3::neo_wallets::wallet::Wallet;
	/// use neo3::sdk::Neo;
	///
	/// # #[tokio::main]
	/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let neo = Neo::testnet().await?;
	/// let wallet = Wallet::new();
	/// let nef_bytes = Vec::new();
	/// let manifest = "{}".to_string();
	///
	/// let contract_hash = neo.deploy_contract(&wallet, nef_bytes, manifest).await?;
	/// println!("Contract deployed: {}", contract_hash);
	/// # Ok(())
	/// # }
	/// ```
	pub async fn deploy_contract(
		&self,
		deployer: &Wallet,
		nef: Vec<u8>,
		manifest: String,
	) -> Result<ScriptHash, NeoError> {
		let started = std::time::Instant::now();
		let result = self.deploy_contract_inner(deployer, nef, manifest).await;
		self.record_transaction_metric("deploy", started.elapsed(), result.is_ok());
		result
	}

	async fn deploy_contract_inner(
		&self,
		deployer: &Wallet,
		nef: Vec<u8>,
		manifest: String,
	) -> Result<ScriptHash, NeoError> {
		use crate::neo_builder::{AccountSigner, CallFlags, ScriptBuilder, TransactionBuilder};
		use crate::neo_types::{ContractManifest, NefFile, ScriptHashExtension};
		use crate::neo_wallets::WalletTrait;

		let max_attempts = self.config.retries.saturating_add(1);

		let deployer_account = deployer.default_account().ok_or_else(no_default_account_error)?;

		let nef_file = NefFile::deserialize(&nef).map_err(|e| NeoError::Validation {
			message: format!("Invalid NEF file: {}", e),
			field: "nef".to_string(),
			value: None,
			recovery: ErrorRecovery::new()
				.suggest("Ensure the NEF bytes are valid")
				.suggest("Load the NEF file using std::fs::read"),
		})?;

		let manifest_bytes = manifest.as_bytes().to_vec();
		let manifest_struct: ContractManifest =
			serde_json::from_str(&manifest).map_err(|e| NeoError::Validation {
				message: format!("Invalid manifest JSON: {}", e),
				field: "manifest".to_string(),
				value: None,
				recovery: ErrorRecovery::new()
					.suggest("Ensure the manifest is valid JSON")
					.suggest("Provide the full contract manifest"),
			})?;

		let contract_name = manifest_struct.name.clone().ok_or_else(|| NeoError::Validation {
			message: "Manifest is missing contract name".to_string(),
			field: "manifest.name".to_string(),
			value: None,
			recovery: ErrorRecovery::new()
				.suggest("Include a `name` field in the manifest")
				.suggest("Check your contract compiler output"),
		})?;

		let nef_checksum = {
			if nef_file.checksum.len() != 4 {
				return Err(NeoError::Validation {
					message: "NEF checksum length is invalid".to_string(),
					field: "nef.checksum".to_string(),
					value: None,
					recovery: ErrorRecovery::new().suggest("Provide a valid NEF file"),
				});
			}
			let mut arr = [0u8; 4];
			arr.copy_from_slice(&nef_file.checksum);
			arr.reverse();
			u32::from_be_bytes(arr)
		};

		let contract_script = ScriptBuilder::build_contract_script(
			&deployer_account.get_script_hash(),
			nef_checksum,
			&contract_name,
		)
		.map_err(|e| {
			NeoError::contract("Failed to derive contract script", None, Some("deploy".into()), e)
		})?;
		let expected_hash = ScriptHash::from_script(&contract_script);

		let management_hash = ScriptHash::from(hex!("fffdc93764dbaddd97c48f252a53ea4643faa3fd"));
		let mut sb = ScriptBuilder::new();
		sb.contract_call(
			&management_hash,
			"deploy",
			&[
				ContractParameter::byte_array(nef),
				ContractParameter::byte_array(manifest_bytes),
				ContractParameter::any(),
			],
			Some(CallFlags::All),
		)
		.map_err(|e| {
			NeoError::contract(
				"Failed to build deploy script",
				Some(management_hash.to_hex()),
				Some("deploy".into()),
				e,
			)
		})?;

		let signer = AccountSigner::called_by_entry(deployer_account)
			.map_err(|e| NeoError::transaction("Failed to create signer", e))?;

		let current_height = retry_network(
			"fetch current block height",
			max_attempts,
			DEFAULT_RETRY_DELAY,
			|| async { self.client.get_block_count().await },
		)
		.await?;

		let mut tb = TransactionBuilder::with_client(self.client.as_ref());
		tb.extend_script(sb.to_bytes());
		tb.set_signers(vec![signer.into()])
			.map_err(|e| NeoError::transaction("Failed to set signer", e))?;
		tb.valid_until_block(current_height + 2400)
			.map_err(|e| NeoError::transaction("Invalid valid-until-block", e))?;

		let mut tx = tb
			.sign()
			.await
			.map_err(|e| NeoError::transaction("Failed to sign deploy transaction", e))?;

		let _ = send_tx_with_retry(
			&mut tx,
			max_attempts,
			DEFAULT_RETRY_DELAY,
			"send deploy transaction",
		)
		.await?;

		// Deploying/initializing a contract can mint or transfer tokens.
		if let Some(cache) = &self.cache {
			cache.invalidate_by_prefix("balance:").await;
		}

		Ok(expected_hash)
	}

	/// Invoke a smart contract method (read-only)
	///
	/// For contract methods that don't modify state.
	///
	/// # Examples
	///
	/// ```no_run
	/// use neo3::neo_types::{ContractParameter, ScriptHash};
	/// use neo3::sdk::Neo;
	///
	/// # #[tokio::main]
	/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let neo = Neo::testnet().await?;
	/// let contract_hash = ScriptHash::zero();
	/// let result = neo
	///     .invoke_read(&contract_hash, "balanceOf", Vec::<ContractParameter>::new())
	///     .await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn invoke_read(
		&self,
		contract: &ScriptHash,
		method: &str,
		params: Vec<ContractParameter>,
	) -> Result<serde_json::Value, NeoError> {
		use crate::neo_types::ScriptHashExtension;

		let max_attempts = self.config.retries.saturating_add(1);

		let result =
			retry_network("invoke read-only method", max_attempts, DEFAULT_RETRY_DELAY, || async {
				self.client
					.invoke_function(contract, method.to_string(), params.clone(), None)
					.await
			})
			.await?;

		if result.has_state_fault() {
			return Err(NeoError::Contract {
				message: result
					.exception
					.clone()
					.unwrap_or_else(|| "Invocation resulted in FAULT state".to_string()),
				contract: Some(contract.to_hex()),
				method: Some(method.to_string()),
				source: None,
				recovery: ErrorRecovery::new()
					.suggest("Check contract parameters")
					.suggest("Ensure the method is safe/read-only"),
			});
		}

		serde_json::to_value(result).map_err(|e| NeoError::Other {
			message: format!("Failed to serialize invocation result: {}", e),
			source: None,
			recovery: ErrorRecovery::new(),
		})
	}

	/// Invoke a smart contract method (with transaction)
	///
	/// For contract methods that modify state.
	///
	/// # Examples
	///
	/// ```no_run
	/// use neo3::neo_types::{ContractParameter, ScriptHash};
	/// use neo3::neo_wallets::wallet::Wallet;
	/// use neo3::sdk::Neo;
	///
	/// # #[tokio::main]
	/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let neo = Neo::testnet().await?;
	/// let wallet = Wallet::new();
	/// let contract_hash = ScriptHash::zero();
	/// let tx_hash = neo
	///     .invoke_write(&wallet, &contract_hash, "transfer", Vec::<ContractParameter>::new())
	///     .await?;
	/// println!("Transaction sent: {}", tx_hash);
	/// # Ok(())
	/// # }
	/// ```
	pub async fn invoke_write(
		&self,
		signer: &Wallet,
		contract: &ScriptHash,
		method: &str,
		params: Vec<ContractParameter>,
	) -> Result<TxHash, NeoError> {
		let started = std::time::Instant::now();
		let result = self.invoke_write_inner(signer, contract, method, params).await;
		self.record_transaction_metric("invoke", started.elapsed(), result.is_ok());
		result
	}

	async fn invoke_write_inner(
		&self,
		signer: &Wallet,
		contract: &ScriptHash,
		method: &str,
		params: Vec<ContractParameter>,
	) -> Result<TxHash, NeoError> {
		use crate::neo_builder::{AccountSigner, CallFlags, ScriptBuilder, TransactionBuilder};
		use crate::neo_types::ScriptHashExtension;
		use crate::neo_wallets::WalletTrait;

		let max_attempts = self.config.retries.saturating_add(1);

		let signer_account = signer.default_account().ok_or_else(no_default_account_error)?;

		let mut sb = ScriptBuilder::new();
		sb.contract_call(contract, method, params.as_slice(), Some(CallFlags::All))
			.map_err(|e| {
				NeoError::contract(
					"Failed to build invocation script",
					Some(contract.to_hex()),
					Some(method.to_string()),
					e,
				)
			})?;

		let signer_obj = AccountSigner::called_by_entry(signer_account)
			.map_err(|e| NeoError::transaction("Failed to create signer", e))?;

		let current_height = retry_network(
			"fetch current block height",
			max_attempts,
			DEFAULT_RETRY_DELAY,
			|| async { self.client.get_block_count().await },
		)
		.await?;

		let script = sb.to_bytes();
		let signers = vec![signer_obj.into()];

		// Estimate fees dynamically (priority-aware) via GasEstimator before signing.
		let adjustment = resolve_fee_adjustment(
			self.client.as_ref(),
			&script,
			&signers,
			&self.fee_policy,
		)
		.await;

		let mut tb = TransactionBuilder::with_client(self.client.as_ref());
		tb.extend_script(script);
		tb.set_signers(signers)
			.map_err(|e| NeoError::transaction("Failed to set signer", e))?;
		tb.valid_until_block(current_height + 2400)
			.map_err(|e| NeoError::transaction("Invalid valid-until-block", e))?;
		tb.set_additional_system_fee(adjustment.additional_system_fee);
		tb.set_additional_network_fee(adjustment.additional_network_fee);

		let mut tx = tb
			.sign()
			.await
			.map_err(|e| NeoError::transaction("Failed to sign invocation transaction", e))?;

		let result = send_tx_with_retry(
			&mut tx,
			max_attempts,
			DEFAULT_RETRY_DELAY,
			"send invocation transaction",
		)
		.await?;

		// Write invocations change balances; drop stale cached balances.
		if let Some(cache) = &self.cache {
			cache.invalidate_by_prefix("balance:").await;
		}

		Ok(result.hash.to_string())
	}

	/// Wait for a transaction to be confirmed
	///
	/// # Examples
	///
	/// ```no_run
	/// use neo3::sdk::Neo;
	/// use std::time::Duration;
	///
	/// # #[tokio::main]
	/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let neo = Neo::testnet().await?;
	/// let tx_hash = "0x0000000000000000000000000000000000000000000000000000000000000000";
	/// neo.wait_for_confirmation(tx_hash, Duration::from_secs(60)).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn wait_for_confirmation(
		&self,
		tx_hash: &str,
		timeout: Duration,
	) -> Result<(), NeoError> {
		use primitive_types::H256;
		use std::str::FromStr;
		use std::time::Instant;

		let tx_h256 = H256::from_str(tx_hash).map_err(|e| NeoError::Validation {
			message: format!("Invalid transaction hash: {}", e),
			field: "tx_hash".to_string(),
			value: Some(tx_hash.to_string()),
			recovery: ErrorRecovery::new().suggest("Provide a valid 0x-prefixed transaction hash"),
		})?;

		let start = Instant::now();
		while start.elapsed() < timeout {
			match self.client.get_application_log(tx_h256).await {
				Ok(log) => {
					// A reverted invocation also produces an application log;
					// only HALT executions mean the transaction succeeded.
					if let Some(execution) =
						log.executions.iter().find(|exec| exec.state != VMState::Halt)
					{
						return Err(NeoError::Transaction {
							message: format!(
								"Transaction failed with VM state {:?}{}",
								execution.state,
								execution
									.exception
									.as_deref()
									.map(|e| format!(": {e}"))
									.unwrap_or_default()
							),
							tx_hash: Some(tx_h256.to_string()),
							source: None,
							recovery: ErrorRecovery::new()
								.suggest("Inspect the application log for the failure reason")
								.suggest("Simulate the transaction before sending"),
						});
					}
					return Ok(());
				},
				Err(err) if err.is_unknown_transaction() || err.is_retryable() => {
					tokio::time::sleep(Duration::from_secs(1)).await;
				},
				Err(err) => return Err(NeoError::provider("poll transaction confirmation", err)),
			}
		}

		Err(NeoError::Timeout {
			duration: timeout,
			operation: "wait_for_confirmation".to_string(),
			recovery: ErrorRecovery::new()
				.suggest("Increase the timeout duration")
				.suggest("Check the transaction hash")
				.retryable(true),
		})
	}

	/// Get the current block height
	pub async fn get_block_height(&self) -> Result<u32, NeoError> {
		let started = std::time::Instant::now();
		let result = self.get_block_height_inner().await;
		self.record_operation("get_block_height", started.elapsed(), result.is_ok());
		result
	}

	async fn get_block_height_inner(&self) -> Result<u32, NeoError> {
		let max_attempts = self.config.retries.saturating_add(1);
		retry_network("fetch block height", max_attempts, DEFAULT_RETRY_DELAY, || async {
			self.client.get_block_count().await
		})
		.await
	}

	/// Get access to the underlying RPC client for advanced operations
	pub fn client(&self) -> &RpcClient<HttpProvider> {
		&self.client
	}

	/// Get the configured RPC endpoint URL.
	pub fn endpoint(&self) -> &str {
		&self.endpoint
	}

	/// Get the current network
	pub fn network(&self) -> &Network {
		&self.network
	}

	/// Human-readable network label for metrics and diagnostics.
	fn network_label(&self) -> String {
		match &self.network {
			Network::MainNet => "mainnet".to_string(),
			Network::TestNet => "testnet".to_string(),
			Network::Custom(endpoint) => endpoint.clone(),
		}
	}

	/// Record an SDK read operation when metrics are enabled.
	///
	/// No-op unless [`SdkConfig::metrics_enabled`] was set and the process
	/// initialized the monitoring registry (`crate::monitoring::metrics::init`).
	fn record_operation(&self, operation: &str, duration: std::time::Duration, success: bool) {
		if !self.config.metrics_enabled {
			return;
		}
		crate::monitoring::metrics::record_rpc_request(
			operation,
			self.endpoint(),
			duration.as_secs_f64(),
			success,
		);
	}

	/// Record an SDK transaction when metrics are enabled.
	fn record_transaction_metric(
		&self,
		tx_type: &str,
		duration: std::time::Duration,
		success: bool,
	) {
		if !self.config.metrics_enabled {
			return;
		}
		crate::monitoring::metrics::record_transaction(
			tx_type,
			&self.network_label(),
			duration.as_secs_f64(),
			success,
		);
	}
}

/// Builder for configuring the Neo SDK
#[derive(Debug)]
pub struct NeoBuilder {
	network: Network,
	config: SdkConfig,
	fee_policy: FeePolicy,
}

impl Default for NeoBuilder {
	fn default() -> Self {
		Self { network: Network::TestNet, config: SdkConfig::default(), fee_policy: FeePolicy::default() }
	}
}

impl NeoBuilder {
	/// Set the network to connect to
	#[must_use]
	pub fn network(mut self, network: Network) -> Self {
		self.network = network;
		self
	}

	/// Override the RPC endpoint with a custom URL.
	///
	/// Shorthand for `.network(Network::Custom(endpoint.into()))`. Useful when
	/// pointing the SDK at a private node, local sandbox, or load balancer.
	#[must_use]
	pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
		self.network = Network::Custom(endpoint.into());
		self
	}

	/// Replace the entire [`SdkConfig`] in one shot.
	///
	/// Use this when you have a pre-built `SdkConfig` (e.g. constructed from
	/// shared application settings) and want to apply it wholesale. Subsequent
	/// builder methods like [`timeout`](Self::timeout) override fields on the
	/// supplied config.
	#[must_use]
	pub fn config(mut self, config: SdkConfig) -> Self {
		self.config = config;
		self
	}

	/// Set the request timeout
	#[must_use]
	pub fn timeout(mut self, timeout: Duration) -> Self {
		self.config.timeout = timeout;
		self
	}

	/// Set the number of retries for failed requests
	#[must_use]
	pub fn retries(mut self, retries: u32) -> Self {
		self.config.retries = retries;
		self
	}

	/// Enable or disable caching
	#[must_use]
	pub fn cache(mut self, enabled: bool) -> Self {
		self.config.cache_enabled = enabled;
		self
	}

	/// Enable or disable metrics collection
	#[must_use]
	pub fn metrics(mut self, enabled: bool) -> Self {
		self.config.metrics_enabled = enabled;
		self
	}

	/// Set the fee estimation policy.
	///
	/// By default dynamic gas estimation is used with a Medium priority margin.
	/// Use this to switch to a different priority (Low/Medium/High) or to lock
	/// fixed additional fees that will be applied on top of the builder's estimates.
	#[must_use]
	pub fn fee_policy(mut self, fee_policy: FeePolicy) -> Self {
		self.fee_policy = fee_policy;
		self
	}

	/// Build the Neo SDK instance
	pub async fn build(self) -> Result<Neo, NeoError> {
		let endpoint = match &self.network {
			Network::MainNet => "https://mainnet1.neo.org:443".to_string(),
			Network::TestNet => "https://testnet1.neo.coz.io:443".to_string(),
			Network::Custom(url) => url.clone(),
		};

		let url = url::Url::parse(&endpoint).map_err(|e| NeoError::Network {
			message: format!("Invalid RPC endpoint URL: {}", e),
			source: None,
			recovery: ErrorRecovery::new()
				.suggest("Check the RPC endpoint URL")
				.suggest("Ensure it includes a valid scheme (http/https)"),
		})?;

		let http_client =
			reqwest::Client::builder().timeout(self.config.timeout).build().map_err(|e| {
				NeoError::Network {
					message: format!("Failed to build HTTP client: {}", e),
					source: None,
					recovery: ErrorRecovery::new()
						.suggest("Check your TLS configuration")
						.suggest("Verify system root certificates are available"),
				}
			})?;

		let provider = HttpProvider::new_with_client(url, http_client);
		let client = Arc::new(RpcClient::new(provider));

		let max_attempts = self.config.retries.saturating_add(1);
		retry_network("connect to Neo network", max_attempts, DEFAULT_RETRY_DELAY, || async {
			client.get_block_count().await
		})
		.await?;

		let cache = self.config.cache_enabled.then(RpcCache::new_rpc_cache);

		Ok(Neo {
			client,
			network: self.network,
			endpoint,
			cache,
			config: self.config,
			fee_policy: self.fee_policy,
		})
	}
}

/// Quick transfer builder for simplified token transfers
#[derive(Debug)]
#[allow(dead_code)]
pub struct Transfer {
	from: Wallet,
	to: String,
	amount: u64,
	token: Token,
	memo: Option<String>,
	sponsor: Option<ScriptHash>,
}

impl Transfer {
	/// Create a new transfer
	pub fn new(from: Wallet, to: impl Into<String>, amount: u64, token: Token) -> Self {
		Self { from, to: to.into(), amount, token, memo: None, sponsor: None }
	}

	/// Add an optional memo to the transfer
	#[must_use]
	pub fn with_memo(mut self, memo: impl Into<String>) -> Self {
		self.memo = Some(memo.into());
		self
	}

	/// Optionally configure a sponsor (paymaster) address for gasless transaction
	#[must_use]
	pub fn with_sponsor(mut self, sponsor: ScriptHash) -> Self {
		self.sponsor = Some(sponsor);
		self
	}

	/// Execute the transfer
	pub async fn execute(self, client: &RpcClient<HttpProvider>) -> Result<TxHash, NeoError> {
		use crate::neo_wallets::WalletTrait;

		let from_account = self.from.default_account().ok_or_else(no_default_account_error)?;

		build_and_send_transfer(
			client,
			from_account,
			&self.to,
			self.amount,
			self.token,
			self.memo.as_deref(),
			// No retry budget configured on the standalone builder; a single
			// attempt still benefits from retry-aware height fetch and
			// already-known-transaction handling in the send path.
			1,
			FeePolicy::default(), // Default policy for the Transfer builder
			self.sponsor,
		)
		.await
	}
}

fn invalid_balance_response(token: &str, detail: impl Into<String>) -> NeoError {
	let detail = detail.into();
	NeoError::Other {
		message: format!("Invalid {token} balance response: {detail}"),
		source: None,
		recovery: ErrorRecovery::new()
			.suggest("Retry against another RPC endpoint")
			.suggest("Inspect the raw balance response from the node"),
	}
}

fn parse_balance_stack_item_u64(item: &StackItem, token: &str) -> Result<u64, NeoError> {
	let bytes = item.as_bytes().ok_or_else(|| {
		invalid_balance_response(token, "balance stack item is not byte-convertible")
	})?;
	let value = BigInt::from_signed_bytes_le(&bytes);

	match value.sign() {
		Sign::Minus => Err(invalid_balance_response(token, "balance cannot be negative")),
		_ => value
			.to_u64()
			.ok_or_else(|| invalid_balance_response(token, "balance does not fit into u64")),
	}
}

fn parse_nep17_decimals(decimals: &str, asset_hash: &ScriptHash) -> Result<u8, NeoError> {
	decimals.parse::<u8>().map_err(|_| NeoError::Other {
		message: format!(
			"Invalid decimals '{}' for NEP-17 token {}",
			decimals,
			asset_hash.to_hex()
		),
		source: None,
		recovery: ErrorRecovery::new()
			.suggest("Retry against another RPC endpoint")
			.suggest("Verify the token contract returns a valid decimals value"),
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use tokio::io::{AsyncReadExt, AsyncWriteExt};

	#[test]
	fn decimal_amount_rejects_invalid_raw_values() {
		assert_eq!(
			DecimalAmount::try_from_raw("not-a-number", 8),
			Err(DecimalAmountParseError::InvalidCharacter)
		);
		assert_eq!(
			DecimalAmount::try_from_raw("-1", 8),
			Err(DecimalAmountParseError::NegativeNotAllowed)
		);
	}

	#[test]
	fn decimal_amount_deserialization_validates_raw_value() {
		let error = serde_json::from_str::<DecimalAmount>(r#"{"raw":"invalid","decimals":8}"#)
			.expect_err("invalid raw amounts must not enter the SDK through cached JSON");
		assert!(error.to_string().contains("invalid"));
	}

	#[test]
	#[allow(deprecated)]
	fn deprecated_decimal_amount_constructor_preserves_invalid_input_fallback() {
		let amount = DecimalAmount::from_raw("not-a-number", 8);

		assert_eq!(amount.raw(), "0");
		assert_eq!(amount.to_fixed_string(), "0.00000000");
	}

	async fn transaction_with_broadcast_responses(
		broadcast_responses: Vec<(&'static str, String)>,
		attempts: u32,
	) -> (Result<crate::neo_protocol::RawTransaction, NeoError>, primitive_types::H256) {
		let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
		let address = listener.local_addr().unwrap();
		let server = tokio::spawn(async move {
			let mut responses = broadcast_responses.into_iter();
			loop {
				let (mut stream, _) = listener.accept().await.unwrap();
				let mut request = [0_u8; 16 * 1024];
				let bytes_read = stream.read(&mut request).await.unwrap();
				let request = String::from_utf8_lossy(&request[..bytes_read]);

				let (status, body) = if request.contains("getblockcount") {
					("200 OK", r#"{"jsonrpc":"2.0","id":1,"result":100}"#.to_string())
				} else {
					responses.next().expect("unexpected broadcast request")
				};

				let response = format!(
					"HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
					body.len()
				);
				stream.write_all(response.as_bytes()).await.unwrap();
				if responses.len() == 0 {
					break;
				}
			}
		});

		let endpoint = url::Url::parse(&format!("http://{address}")).unwrap();
		let provider = HttpProvider::new(endpoint).unwrap();
		let client = RpcClient::new(provider);
		let mut transaction = crate::neo_builder::Transaction::<HttpProvider> {
			network: Some(&client),
			..Default::default()
		};
		let expected_hash = transaction.tx_id().unwrap();

		let result = send_tx_with_retry(
			&mut transaction,
			attempts,
			Duration::ZERO,
			"broadcast test transaction",
		)
		.await;

		server.await.unwrap();
		(result, expected_hash)
	}

	#[tokio::test]
	async fn send_retry_accepts_transactions_already_known_by_the_node() {
		for code in [-501, -503] {
			let responses = vec![
				("503 Service Unavailable", "response lost after submission".to_string()),
				(
					"200 OK",
					format!(
						r#"{{"jsonrpc":"2.0","id":4,"error":{{"code":{code},"message":"already known","data":null}}}}"#
					),
				),
			];
			let (result, expected_hash) = transaction_with_broadcast_responses(responses, 2).await;
			assert_eq!(result.unwrap().hash, expected_hash);
		}
	}

	#[tokio::test]
	async fn send_retry_classifies_node_rejection_as_transaction_error() {
		let response = r#"{"jsonrpc":"2.0","id":2,"error":{"code":-504,"message":"Insufficient network fee","data":null}}"#;
		let (result, expected_hash) =
			transaction_with_broadcast_responses(vec![("200 OK", response.to_string())], 1).await;

		match result.unwrap_err() {
			NeoError::Transaction { tx_hash, recovery, .. } => {
				assert_eq!(tx_hash.as_deref(), Some(format!("{expected_hash:#x}").as_str()));
				assert!(!recovery.retryable);
			},
			other => panic!("expected transaction rejection, got {other:?}"),
		}
	}
	use crate::neo_types::StackItem;

	#[test]
	fn test_builder_configuration() {
		let builder = Neo::builder()
			.network(Network::TestNet)
			.timeout(Duration::from_secs(60))
			.retries(5)
			.cache(true)
			.metrics(false);

		assert_eq!(builder.config.timeout, Duration::from_secs(60));
		assert_eq!(builder.config.retries, 5);
		assert!(builder.config.cache_enabled);
		assert!(!builder.config.metrics_enabled);
	}

	#[test]
	fn endpoint_shortcut_picks_custom_network() {
		let builder = Neo::builder().endpoint("https://example.com:443");
		match &builder.network {
			Network::Custom(url) => assert_eq!(url, "https://example.com:443"),
			other => panic!("expected Network::Custom, got {other:?}"),
		}
	}

	#[test]
	fn config_setter_replaces_entire_config() {
		let cfg = SdkConfig::builder()
			.timeout(Duration::from_secs(7))
			.retries(2)
			.cache_enabled(false)
			.metrics_enabled(true)
			.build();
		let builder = Neo::builder().config(cfg.clone()).network(Network::TestNet);
		assert_eq!(builder.config.timeout, Duration::from_secs(7));
		assert_eq!(builder.config.retries, 2);
		assert!(!builder.config.cache_enabled);
		assert!(builder.config.metrics_enabled);
	}

	#[test]
	fn token_contract_hash_returns_native_hashes() {
		let neo = Token::NEO.contract_hash();
		let gas = Token::GAS.contract_hash();
		assert_eq!(<[u8; 20]>::from(neo), Token::NEO_HASH);
		assert_eq!(<[u8; 20]>::from(gas), Token::GAS_HASH);
		let custom = ScriptHash::zero();
		assert_eq!(Token::Custom(custom).contract_hash(), custom);
	}

	#[tokio::test]
	async fn connect_with_malformed_url_returns_network_error() {
		// "not-a-url" cannot be parsed as a URL, so `build()` reports a Network
		// error (the SDK treats endpoint-parse failures as a network-layer issue).
		let err = Neo::connect("not-a-url").await.unwrap_err();
		assert_eq!(err.kind(), crate::neo_error::unified::NeoErrorKind::Network);
		assert!(err.message().to_lowercase().contains("invalid"));
	}

	#[tokio::test]
	async fn from_env_without_env_var_falls_back_to_testnet() {
		// Guard against environment pollution from parallel tests. We snapshot
		// the prior value, unset the variable, exercise the fallback path, and
		// restore on the way out. The fallback path only requires that the
		// builder *chooses* TestNet — we cannot verify the network call without
		// hitting the network, so we just assert it does not error on
		// configuration grounds.
		let prior = std::env::var_os("NEO_RPC_URL");
		// SAFETY: tests in this crate are not parallelised with set_var elsewhere.
		std::env::remove_var("NEO_RPC_URL");

		// The build() may succeed or fail depending on network availability;
		// either way the URL choice should NOT be a config/validation error.
		let result = Neo::from_env().await;

		// Restore env to whatever it was.
		if let Some(value) = prior {
			std::env::set_var("NEO_RPC_URL", value);
		}

		if let Err(err) = result {
			let kind = err.kind();
			assert!(
				matches!(kind, crate::neo_error::unified::NeoErrorKind::Network),
				"unexpected error kind {kind:?}: {err}"
			);
		}
	}

	#[tokio::test]
	async fn from_env_with_malformed_url_yields_network_error() {
		let prior = std::env::var_os("NEO_RPC_URL");
		std::env::set_var("NEO_RPC_URL", "::::not a url::::");

		let result = Neo::from_env().await;

		// Restore env first.
		match prior {
			Some(value) => std::env::set_var("NEO_RPC_URL", value),
			None => std::env::remove_var("NEO_RPC_URL"),
		}

		let err = result.expect_err("malformed NEO_RPC_URL must error");
		assert_eq!(err.kind(), crate::neo_error::unified::NeoErrorKind::Network);
	}

	#[test]
	fn test_parse_balance_stack_item_u64_rejects_negative_value() {
		let item = StackItem::Integer { value: -1 };
		let err = parse_balance_stack_item_u64(&item, "NEO").unwrap_err();

		match err {
			NeoError::Other { message, .. } => {
				assert!(message.contains("balance cannot be negative"));
			},
			other => panic!("expected balance parsing error, got {other:?}"),
		}
	}

	#[test]
	fn test_parse_nep17_decimals_rejects_invalid_value() {
		let asset_hash = ScriptHash::zero();
		let err = parse_nep17_decimals("not-a-u8", &asset_hash).unwrap_err();

		match err {
			NeoError::Other { message, .. } => {
				assert!(message.contains("Invalid decimals"));
			},
			other => panic!("expected decimals parsing error, got {other:?}"),
		}
	}
}
