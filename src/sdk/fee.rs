//! Dynamic, priority-aware fee estimation for the high-level [`Neo`](super::Neo) send flow.
//!
//! Historically the SDK relied entirely on [`TransactionBuilder`]'s built-in
//! fee estimation: the builder calls `invokescript` for the system fee and
//! `calculatenetworkfee` for the network fee right before signing. That path is
//! correct but offers no lever for callers who want to bias a transaction for
//! faster mempool inclusion or add an execution-safety margin.
//!
//! This module wires the dedicated [`GasEstimator`] into the SDK send flow and
//! layers a small, backward-compatible policy on top of it:
//!
//! - [`FeePriority`] — a `Low`/`Medium`/`High` selector that turns a base
//!   estimated fee into a margin multiplier.
//! - [`FeePolicy`] — chooses between *dynamic* estimation (default) and a
//!   *fixed* caller-supplied override.
//! - [`resolve_fee_adjustment`] — the glue that produces the additional
//!   system/network fees handed to the builder just before signing.
//!
//! ## Backward compatibility & graceful degradation
//!
//! The default policy is [`FeePolicy::Dynamic`] with [`FeePriority::Medium`].
//! Dynamic estimation requires a live provider that answers `invokescript`; when
//! that call is unavailable or fails, [`resolve_fee_adjustment`] degrades
//! gracefully to a zero adjustment so the builder falls back to its existing
//! estimation behaviour. This keeps every existing caller and every
//! `MockProvider`-based test working unchanged.
//!
//! [`TransactionBuilder`]: crate::neo_builder::TransactionBuilder

use crate::neo_builder::{GasEstimator, Signer};
use crate::neo_clients::{APITrait, ProviderError};

/// Default safety margin applied by [`FeePriority::Medium`] (10%).
const MEDIUM_MARGIN_PERCENT: u8 = 10;
/// Aggressive safety margin applied by [`FeePriority::High`] (25%).
const HIGH_MARGIN_PERCENT: u8 = 25;

/// Mempool/priority selector that applies a margin multiplier over a base fee.
///
/// Neo orders mempool candidates by the fee a transaction is willing to pay, so
/// biasing the fee upward increases the odds of prompt inclusion and guards
/// against small under-estimates that would otherwise cause an out-of-gas
/// `FAULT`. The margin is always rounded *up* so an estimate never under-charges.
///
/// # Examples
///
/// ```
/// use neo3::sdk::fee::FeePriority;
///
/// assert_eq!(FeePriority::Low.margin_percent(), 0);
/// assert_eq!(FeePriority::Medium.margin_percent(), 10);
/// assert_eq!(FeePriority::High.margin_percent(), 25);
///
/// // 1_000 base fee + 10% margin => 1_100 total.
/// assert_eq!(FeePriority::Medium.apply(1_000), 1_100);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeePriority {
	/// No margin: pay exactly the base estimated fee.
	Low,
	/// A moderate 10% margin — the SDK default.
	Medium,
	/// An aggressive 25% margin for time-sensitive transactions.
	High,
}

impl Default for FeePriority {
	fn default() -> Self {
		Self::Medium
	}
}

impl FeePriority {
	/// The margin, expressed as a whole percentage, applied over the base fee.
	#[must_use]
	pub const fn margin_percent(self) -> u8 {
		match self {
			Self::Low => 0,
			Self::Medium => MEDIUM_MARGIN_PERCENT,
			Self::High => HIGH_MARGIN_PERCENT,
		}
	}

	/// The absolute margin (delta) added on top of `base_fee`.
	///
	/// Integer arithmetic rounds the margin *up* (`div_ceil`) so the adjusted
	/// fee never falls below the intended percentage of the base estimate.
	#[must_use]
	pub fn margin(self, base_fee: u64) -> u64 {
		let percent = u64::from(self.margin_percent());
		if percent == 0 {
			return 0;
		}
		base_fee.saturating_mul(percent).div_ceil(100)
	}

	/// The base fee with the priority margin applied.
	#[must_use]
	pub fn apply(self, base_fee: u64) -> u64 {
		base_fee.saturating_add(self.margin(base_fee))
	}
}

/// How the SDK should determine transaction fees before signing.
///
/// # Examples
///
/// ```
/// use neo3::sdk::fee::{FeePolicy, FeePriority};
///
/// // Dynamic estimation with a high-priority margin.
/// let dynamic = FeePolicy::dynamic(FeePriority::High);
/// assert!(dynamic.is_dynamic());
///
/// // A fixed override that skips estimation entirely (offline / backward compatible).
/// let fixed = FeePolicy::fixed(1_000, 500);
/// assert!(fixed.is_fixed());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeePolicy {
	/// Estimate the fee dynamically via [`GasEstimator`] and apply a
	/// [`FeePriority`] margin. This is the SDK default.
	Dynamic {
		/// The priority/margin selector applied over the base estimate.
		priority: FeePriority,
	},
	/// Skip estimation and apply caller-supplied additional fees verbatim.
	///
	/// The values are *additional* fees layered on top of whatever the
	/// underlying [`TransactionBuilder`](crate::neo_builder::TransactionBuilder)
	/// computes, matching the builder's `set_additional_*_fee` semantics. Use
	/// this for deterministic offline behaviour or to preserve legacy fees.
	Fixed {
		/// Additional system fee (execution safety margin) in GAS base units.
		additional_system_fee: u64,
		/// Additional network fee (mempool priority) in GAS base units.
		additional_network_fee: u64,
	},
}

impl Default for FeePolicy {
	fn default() -> Self {
		Self::Dynamic { priority: FeePriority::default() }
	}
}

impl FeePolicy {
	/// Build a [`FeePolicy::Dynamic`] from a priority selector.
	#[must_use]
	pub const fn dynamic(priority: FeePriority) -> Self {
		Self::Dynamic { priority }
	}

	/// Build a [`FeePolicy::Fixed`] from additional system/network fees.
	#[must_use]
	pub const fn fixed(additional_system_fee: u64, additional_network_fee: u64) -> Self {
		Self::Fixed { additional_system_fee, additional_network_fee }
	}

	/// Returns `true` when the policy estimates fees dynamically.
	#[must_use]
	pub const fn is_dynamic(&self) -> bool {
		matches!(self, Self::Dynamic { .. })
	}

	/// Returns `true` when the policy uses fixed additional fees.
	#[must_use]
	pub const fn is_fixed(&self) -> bool {
		matches!(self, Self::Fixed { .. })
	}
}

/// The additional fees to layer onto a transaction before signing.
///
/// Produced by [`resolve_fee_adjustment`] and fed straight into
/// [`TransactionBuilder::set_additional_system_fee`](crate::neo_builder::TransactionBuilder)
/// and its network-fee counterpart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FeeAdjustment {
	/// Additional system fee in GAS base units.
	pub additional_system_fee: u64,
	/// Additional network fee in GAS base units.
	pub additional_network_fee: u64,
}

impl FeeAdjustment {
	/// A zero adjustment — the graceful-degradation fallback.
	#[must_use]
	pub const fn zero() -> Self {
		Self { additional_system_fee: 0, additional_network_fee: 0 }
	}

	/// Returns `true` when both additional fees are zero.
	#[must_use]
	pub const fn is_zero(&self) -> bool {
		self.additional_system_fee == 0 && self.additional_network_fee == 0
	}
}

/// Resolve the additional fees for a script under the given [`FeePolicy`].
///
/// For [`FeePolicy::Fixed`] the supplied fees are returned verbatim with no
/// network access. For [`FeePolicy::Dynamic`] the base execution fee is
/// estimated in real time via [`GasEstimator::estimate_gas_realtime`] and the
/// priority margin is applied to produce the additional system fee.
///
/// # Graceful degradation
///
/// Dynamic estimation needs a live provider. If the estimation call fails, a
/// non-positive value is returned, or the result overflows `u64`, this function
/// returns [`FeeAdjustment::zero`] instead of propagating the error, so the
/// caller silently falls back to the builder's existing estimation behaviour.
/// This guarantees the send flow never becomes *more* fragile than before.
pub async fn resolve_fee_adjustment<T>(
	client: &T,
	script: &[u8],
	signers: &[Signer],
	policy: &FeePolicy,
) -> FeeAdjustment
where
	T: APITrait,
	T::Error: Into<ProviderError>,
{
	match policy {
		FeePolicy::Fixed { additional_system_fee, additional_network_fee } => FeeAdjustment {
			additional_system_fee: *additional_system_fee,
			additional_network_fee: *additional_network_fee,
		},
		FeePolicy::Dynamic { priority } => {
			// An empty script cannot be estimated; fall back to zero and let the
			// builder surface the real (EmptyScript) error during signing.
			if script.is_empty() {
				return FeeAdjustment::zero();
			}

			match GasEstimator::estimate_gas_realtime(client, script, signers.to_vec()).await {
				Ok(base_fee) if base_fee > 0 => {
					let base = base_fee as u64;
					FeeAdjustment {
						additional_system_fee: priority.margin(base),
						// Priority is expressed as an execution-fee safety margin;
						// the network fee is still estimated by the builder itself.
						additional_network_fee: 0,
					}
				},
				Ok(_) => FeeAdjustment::zero(),
				Err(err) => {
					// Degrade gracefully: no live estimate => existing behaviour.
					tracing::debug!(
						error = %err,
						"dynamic fee estimation unavailable; falling back to builder defaults"
					);
					FeeAdjustment::zero()
				},
			}
		},
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::neo_builder::{AccountSigner, TransactionBuilder};
	use crate::neo_clients::{MockClient, RpcClient};
	use crate::neo_protocol::Account;
	use crate::neo_protocol::AccountTrait;
	use serde_json::json;
	use std::sync::Arc;
	use tokio::sync::Mutex;

	// ---- Pure FeePriority behaviour ---------------------------------------

	#[test]
	fn fee_priority_margin_percent_is_ordered() {
		assert_eq!(FeePriority::Low.margin_percent(), 0);
		assert_eq!(FeePriority::Medium.margin_percent(), 10);
		assert_eq!(FeePriority::High.margin_percent(), 25);
	}

	#[test]
	fn fee_priority_default_is_medium() {
		assert_eq!(FeePriority::default(), FeePriority::Medium);
		assert_eq!(FeePolicy::default(), FeePolicy::Dynamic { priority: FeePriority::Medium });
	}

	#[test]
	fn fee_priority_apply_matches_expected_margins() {
		// Low never adds a margin.
		assert_eq!(FeePriority::Low.margin(1_000), 0);
		assert_eq!(FeePriority::Low.apply(1_000), 1_000);

		// Medium adds exactly 10%.
		assert_eq!(FeePriority::Medium.margin(1_000), 100);
		assert_eq!(FeePriority::Medium.apply(1_000), 1_100);

		// High adds exactly 25%.
		assert_eq!(FeePriority::High.margin(1_000), 250);
		assert_eq!(FeePriority::High.apply(1_000), 1_250);
	}

	#[test]
	fn fee_priority_margin_rounds_up_and_handles_edge_cases() {
		// 30 * 10% = 3 exactly.
		assert_eq!(FeePriority::Medium.margin(30), 3);
		// 25 * 10% = 2.5 -> rounds up to 3 (never under-charge).
		assert_eq!(FeePriority::Medium.margin(25), 3);
		// Zero base fee yields zero margin for every priority.
		assert_eq!(FeePriority::High.margin(0), 0);
		// Saturating multiply must not panic on huge inputs.
		assert_eq!(FeePriority::High.apply(u64::MAX), u64::MAX);
	}

	#[test]
	fn fee_policy_constructors_and_predicates() {
		let dynamic = FeePolicy::dynamic(FeePriority::High);
		assert!(dynamic.is_dynamic());
		assert!(!dynamic.is_fixed());

		let fixed = FeePolicy::fixed(100, 50);
		assert!(fixed.is_fixed());
		assert!(!fixed.is_dynamic());
		assert_eq!(
			fixed,
			FeePolicy::Fixed { additional_system_fee: 100, additional_network_fee: 50 }
		);
	}

	#[test]
	fn fee_adjustment_zero_helpers() {
		assert!(FeeAdjustment::zero().is_zero());
		assert!(!FeeAdjustment { additional_system_fee: 1, additional_network_fee: 0 }.is_zero());
	}

	// ---- Fixed policy needs no provider -----------------------------------

	#[tokio::test]
	async fn fixed_policy_returns_supplied_fees_without_estimation() {
		// No invokescript mock is registered: a Fixed policy must not touch the
		// network at all, so this succeeds purely from the supplied values.
		let mock = Arc::new(Mutex::new(MockClient::new().await));
		let client = {
			let guard = mock.lock().await;
			Arc::new(guard.into_client())
		};

		let account = Account::create().unwrap();
		let signer: Signer = AccountSigner::none(&account).unwrap().into();

		let adjustment = resolve_fee_adjustment(
			client.as_ref(),
			&[1, 2, 3],
			std::slice::from_ref(&signer),
			&FeePolicy::fixed(1_234, 567),
		)
		.await;

		assert_eq!(adjustment.additional_system_fee, 1_234);
		assert_eq!(adjustment.additional_network_fee, 567);
	}

	// ---- Dynamic policy uses GasEstimator ---------------------------------

	async fn mock_client_with_gas(gas_consumed: &str) -> Arc<RpcClient<crate::neo_clients::MockProvider>>
	{
		let mock = Arc::new(Mutex::new(MockClient::new().await));
		{
			let mut guard = mock.lock().await;
			guard
				.mock_response_ignore_param(
					"invokescript",
					json!({
						"script": "AQID",
						"state": "HALT",
						"gasconsumed": gas_consumed,
						"exception": null,
						"stack": []
					}),
				)
				.await;
			guard
				.mock_response_with_file_ignore_param("getblockcount", "getblockcount_1000.json")
				.await;
			guard
				.mock_response_with_file_ignore_param("calculatenetworkfee", "calculatenetworkfee.json")
				.await;
			guard.mount_mocks().await;
		}
		let guard = mock.lock().await;
		Arc::new(guard.into_client())
	}

	#[tokio::test]
	async fn dynamic_policy_applies_priority_margin_over_estimated_fee() {
		// invokescript reports gas_consumed = "30" (base execution fee).
		let client = mock_client_with_gas("30").await;
		let account = Account::create().unwrap();
		let signer: Signer = AccountSigner::none(&account).unwrap().into();
		let signers = std::slice::from_ref(&signer);

		let low =
			resolve_fee_adjustment(client.as_ref(), &[1, 2, 3], signers, &FeePolicy::dynamic(FeePriority::Low))
				.await;
		assert_eq!(low.additional_system_fee, 0);

		let medium = resolve_fee_adjustment(
			client.as_ref(),
			&[1, 2, 3],
			signers,
			&FeePolicy::dynamic(FeePriority::Medium),
		)
		.await;
		// 10% of 30 = 3.
		assert_eq!(medium.additional_system_fee, 3);

		let high =
			resolve_fee_adjustment(client.as_ref(), &[1, 2, 3], signers, &FeePolicy::dynamic(FeePriority::High))
				.await;
		// 25% of 30 = 7.5 -> rounds up to 8.
		assert_eq!(high.additional_system_fee, 8);
	}

	#[tokio::test]
	async fn dynamic_policy_degrades_gracefully_when_estimation_fails() {
		// No invokescript mock registered => the estimation RPC errors, and the
		// helper must return a zero adjustment instead of propagating the error.
		let mock = Arc::new(Mutex::new(MockClient::new().await));
		let client = {
			let guard = mock.lock().await;
			Arc::new(guard.into_client())
		};

		let account = Account::create().unwrap();
		let signer: Signer = AccountSigner::none(&account).unwrap().into();

		let adjustment = resolve_fee_adjustment(
			client.as_ref(),
			&[1, 2, 3],
			std::slice::from_ref(&signer),
			&FeePolicy::dynamic(FeePriority::High),
		)
		.await;

		assert!(adjustment.is_zero());
	}

	#[tokio::test]
	async fn dynamic_policy_returns_zero_for_empty_script() {
		let client = mock_client_with_gas("30").await;
		let account = Account::create().unwrap();
		let signer: Signer = AccountSigner::none(&account).unwrap().into();

		let adjustment = resolve_fee_adjustment(
			client.as_ref(),
			&[],
			std::slice::from_ref(&signer),
			&FeePolicy::dynamic(FeePriority::High),
		)
		.await;

		assert!(adjustment.is_zero());
	}

	// ---- End-to-end: the adjustment flows into the built transaction ------

	#[tokio::test]
	async fn dynamic_fee_is_applied_to_the_transaction_send_flow() {
		// Mirror the SDK send flow: estimate via resolve_fee_adjustment, feed the
		// result into the TransactionBuilder, and confirm the signed transaction's
		// system fee carries the priority margin on top of the base estimate.
		let client = mock_client_with_gas("30").await;
		let account = Account::create().unwrap();
		let signer: Signer = AccountSigner::none(&account).unwrap().into();
		let signers = vec![signer.clone()];
		let script = vec![1u8, 2, 3];

		// Base system fee without any priority adjustment.
		let mut base_builder = TransactionBuilder::with_client(client.as_ref());
		base_builder.extend_script(script.clone());
		base_builder.set_signers(signers.clone()).unwrap();
		base_builder.valid_until_block(2_000).unwrap();
		let base_tx = base_builder.get_unsigned_tx().await.unwrap();
		let base_sys_fee = base_tx.sys_fee;
		assert_eq!(base_sys_fee, 30);

		// Now apply a Medium dynamic policy through the same wiring the SDK uses.
		let adjustment = resolve_fee_adjustment(
			client.as_ref(),
			&script,
			&signers,
			&FeePolicy::dynamic(FeePriority::Medium),
		)
		.await;
		assert_eq!(adjustment.additional_system_fee, 3);

		let mut builder = TransactionBuilder::with_client(client.as_ref());
		builder.extend_script(script);
		builder.set_signers(signers).unwrap();
		builder.valid_until_block(2_000).unwrap();
		builder.set_additional_system_fee(adjustment.additional_system_fee);
		builder.set_additional_network_fee(adjustment.additional_network_fee);
		let tx = builder.get_unsigned_tx().await.unwrap();

		// The dynamic priority fee is layered onto the base estimate.
		assert_eq!(tx.sys_fee, base_sys_fee + 3);
	}
}
