//! Treasury native contract for Neo N3.
//!
//! The Treasury contract manages treasury funds in the Neo N3 blockchain.
//! It was introduced in Neo 3.9 (HF_Faun) and implements NEP-26, NEP-27, and NEP-30 standards.
//!
//! The Treasury contract can receive NEP-17 and NEP-11 tokens and is controlled
//! by the committee.

use async_trait::async_trait;
use primitive_types::H160;
use serde::{Deserialize, Serialize};

use crate::{
	neo_clients::{JsonRpcProvider, RpcClient},
	neo_contract::{ContractError, traits::SmartContractTrait},
	neo_types::{
		ScriptHash,
		serde_with_utils::{deserialize_script_hash, serialize_script_hash},
	},
};

/// The Treasury native contract for managing treasury funds.
///
/// This contract was introduced in Neo 3.9 (HF_Faun) and supports:
/// - NEP-26 (Treasury standard)
/// - NEP-27 (Token callbacks)
/// - NEP-30 (Native contract interface)
///
/// The Treasury can receive both NEP-17 (fungible) and NEP-11 (non-fungible) tokens.
/// All operations that modify the treasury require committee approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryContract<'a, P: JsonRpcProvider> {
	#[serde(deserialize_with = "deserialize_script_hash")]
	#[serde(serialize_with = "serialize_script_hash")]
	script_hash: ScriptHash,
	#[serde(skip)]
	provider: Option<&'a RpcClient<P>>,
}

impl<'a, P: JsonRpcProvider + 'static> TreasuryContract<'a, P> {
	/// The name of the Treasury contract.
	pub const NAME: &'static str = "Treasury";

	/// Creates a new Treasury contract instance.
	pub fn new(provider: Option<&'a RpcClient<P>>) -> Self {
		Self { script_hash: Self::calc_native_contract_hash_unchecked(Self::NAME), provider }
	}

	/// Verifies that the current transaction is signed by the committee.
	///
	/// This method is used internally by the Treasury contract to validate
	/// committee signatures before processing certain operations.
	///
	/// # Returns
	///
	/// Returns `true` if the transaction is signed by the committee, `false` otherwise.
	pub async fn verify(&self) -> Result<bool, ContractError> {
		self.call_function_returning_bool("verify", vec![]).await
	}

	/// Gets the supported standards of the Treasury contract.
	///
	/// # Returns
	///
	/// A list of supported standards: NEP-26, NEP-27, NEP-30
	pub fn supported_standards() -> Vec<&'static str> {
		vec!["NEP-26", "NEP-27", "NEP-30"]
	}
}

#[async_trait]
impl<'a, P: JsonRpcProvider> SmartContractTrait<'a> for TreasuryContract<'a, P> {
	type P = P;

	fn script_hash(&self) -> H160 {
		self.script_hash
	}

	fn set_script_hash(&mut self, script_hash: H160) {
		self.script_hash = script_hash;
	}

	fn provider(&self) -> Option<&RpcClient<P>> {
		self.provider
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::neo_clients::MockProvider;

	#[test]
	fn test_treasury_contract_name() {
		assert_eq!(TreasuryContract::<MockProvider>::NAME, "Treasury");
	}

	#[test]
	fn test_treasury_supported_standards() {
		let standards = TreasuryContract::<MockProvider>::supported_standards();
		assert!(standards.contains(&"NEP-26"));
		assert!(standards.contains(&"NEP-27"));
		assert!(standards.contains(&"NEP-30"));
	}

	#[test]
	fn test_treasury_contract_hash() {
		// The Treasury contract should have a deterministic hash based on its name
		let treasury = TreasuryContract::<MockProvider>::new(None);
		assert!(!treasury.script_hash.is_zero());
	}
}
