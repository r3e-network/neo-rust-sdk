//! Notary native contract for Neo N3.
//!
//! The Notary contract provides assistance for forming multisignature transactions.
//! It was introduced in Neo 3.9 (HF_Echidna) and allows users to deposit GAS as collateral
//! for notary services.
//!
//! This contract supports NEP-27 (Token callbacks) and NEP-30 (Native contract interface) standards.

use async_trait::async_trait;
use primitive_types::H160;
use serde::{Deserialize, Serialize};

use crate::{
	neo_builder::TransactionBuilder,
	neo_clients::{JsonRpcProvider, RpcClient},
	neo_contract::{ContractError, checked_vm_integer, traits::SmartContractTrait},
	neo_types::{
		ScriptHash, StackItem,
		serde_with_utils::{deserialize_script_hash, serialize_script_hash},
	},
};

/// Represents a notary deposit.
///
/// A deposit consists of an amount of GAS locked until a specified block height.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotaryDeposit {
	/// The amount of GAS deposited (in smallest units, 1 GAS = 10^8).
	pub amount: i64,
	/// The block height until which the deposit is locked.
	pub till: u32,
}

impl NotaryDeposit {
	/// Creates a new NotaryDeposit from a stack item.
	pub fn from_stack_item(item: &StackItem) -> Result<Self, String> {
		match item {
			StackItem::Struct { value } | StackItem::Array { value } if value.len() >= 2 => {
				let amount = value[0].as_int().ok_or_else(|| "Invalid amount".to_string())?;

				let till = value[1]
					.as_int()
					.ok_or_else(|| "Invalid till".to_string())?
					.try_into()
					.map_err(|_| "Invalid till: value is outside the u32 range".to_string())?;

				Ok(Self { amount, till })
			},
			_ => Err("Expected Struct or Array with 2 elements".to_string()),
		}
	}
}

/// The Notary native contract for multisignature transaction assistance.
///
/// The Notary contract provides the following functionality:
/// - Accepting GAS deposits as collateral for notary services
/// - Managing deposit lock periods
/// - Verifying notary signatures on transactions
/// - Distributing rewards to notary nodes
///
/// This contract was introduced in Neo 3.9 (HF_Echidna).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotaryContract<'a, P: JsonRpcProvider> {
	#[serde(deserialize_with = "deserialize_script_hash")]
	#[serde(serialize_with = "serialize_script_hash")]
	script_hash: ScriptHash,
	#[serde(skip)]
	provider: Option<&'a RpcClient<P>>,
}

impl<'a, P: JsonRpcProvider + 'static> NotaryContract<'a, P> {
	/// The name of the Notary contract.
	pub const NAME: &'static str = "Notary";

	/// Default value for maximum allowed NotValidBeforeDelta.
	/// It is set to be 20 rounds for 7 validators, a little more than half an hour for 15-seconds blocks.
	pub const DEFAULT_MAX_NOT_VALID_BEFORE_DELTA: u32 = 140;

	/// Default value for deposit lock period in blocks.
	pub const DEFAULT_DEPOSIT_DELTA_TILL: u32 = 5760;

	/// Creates a new Notary contract instance.
	pub fn new(provider: Option<&'a RpcClient<P>>) -> Self {
		Self { script_hash: Self::calc_native_contract_hash_unchecked(Self::NAME), provider }
	}

	/// Gets the deposited GAS balance for the specified account.
	///
	/// # Arguments
	///
	/// * `account` - The account to check the balance for.
	///
	/// # Returns
	///
	/// The amount of GAS deposited by the account (in smallest units, 1 GAS = 10^8).
	pub async fn balance_of(&self, account: &H160) -> Result<i64, ContractError> {
		self.call_function_returning_int("balanceOf", vec![account.into()]).await
	}

	/// Gets the deposit lock expiration height for the specified account.
	///
	/// # Arguments
	///
	/// * `account` - The account to check the expiration for.
	///
	/// # Returns
	///
	/// The block height at which the deposit can be withdrawn.
	/// Returns 0 if no deposit exists.
	pub async fn expiration_of(&self, account: &H160) -> Result<u32, ContractError> {
		checked_vm_integer(
			self.call_function_returning_int("expirationOf", vec![account.into()]).await?,
			"notary deposit expiration",
		)
	}

	/// Gets the maximum NotValidBefore delta value.
	///
	/// This value determines the maximum difference allowed between the current block height
	/// and the NotValidBefore attribute of a notary-assisted transaction.
	///
	/// # Returns
	///
	/// The maximum NotValidBefore delta in blocks.
	pub async fn get_max_not_valid_before_delta(&self) -> Result<u32, ContractError> {
		checked_vm_integer(
			self.call_function_returning_int("getMaxNotValidBeforeDelta", vec![]).await?,
			"maximum not-valid-before delta",
		)
	}

	/// Locks the deposit until the specified block height.
	///
	/// This method extends the lock period of an existing deposit.
	/// The new lock height must be greater than or equal to the current lock height.
	///
	/// # Arguments
	///
	/// * `account` - The account whose deposit to lock.
	/// * `till` - The block height until which to lock the deposit.
	///
	/// # Returns
	///
	/// A transaction builder for the lock operation.
	pub async fn lock_deposit_until(
		&self,
		account: &H160,
		till: u32,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		self.invoke_function("lockDepositUntil", vec![account.into(), till.into()])
			.await
	}

	/// Withdraws the deposit from the specified account.
	///
	/// The deposit can only be withdrawn after the lock period has expired.
	///
	/// # Arguments
	///
	/// * `from` - The account to withdraw from.
	/// * `to` - The account to send the withdrawn GAS to. If None, sends to `from`.
	///
	/// # Returns
	///
	/// A transaction builder for the withdrawal operation.
	pub async fn withdraw(
		&self,
		from: &H160,
		to: Option<&H160>,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		let params = match to {
			Some(to_addr) => vec![from.into(), to_addr.into()],
			None => vec![from.into(), crate::neo_types::ContractParameter::any()],
		};
		self.invoke_function("withdraw", params).await
	}

	/// Sets the maximum NotValidBefore delta value.
	///
	/// This method can only be called by the committee.
	///
	/// # Arguments
	///
	/// * `value` - The new maximum NotValidBefore delta value.
	///
	/// # Returns
	///
	/// A transaction builder for the set operation.
	pub async fn set_max_not_valid_before_delta(
		&self,
		value: u32,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		self.invoke_function("setMaxNotValidBeforeDelta", vec![value.into()]).await
	}

	/// Gets the supported standards of the Notary contract.
	///
	/// # Returns
	///
	/// A list of supported standards: NEP-27, NEP-30 (after HF_Faun)
	pub fn supported_standards() -> Vec<&'static str> {
		vec!["NEP-27", "NEP-30"]
	}
}

#[async_trait]
impl<'a, P: JsonRpcProvider> SmartContractTrait<'a> for NotaryContract<'a, P> {
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
	fn test_notary_contract_name() {
		assert_eq!(NotaryContract::<MockProvider>::NAME, "Notary");
	}

	#[test]
	fn test_notary_default_constants() {
		assert_eq!(NotaryContract::<MockProvider>::DEFAULT_MAX_NOT_VALID_BEFORE_DELTA, 140);
		assert_eq!(NotaryContract::<MockProvider>::DEFAULT_DEPOSIT_DELTA_TILL, 5760);
	}

	#[test]
	fn test_notary_supported_standards() {
		let standards = NotaryContract::<MockProvider>::supported_standards();
		assert!(standards.contains(&"NEP-27"));
		assert!(standards.contains(&"NEP-30"));
	}

	#[test]
	fn test_notary_contract_hash() {
		let notary = NotaryContract::<MockProvider>::new(None);
		assert!(!notary.script_hash.is_zero());
	}

	#[test]
	fn test_notary_deposit_from_stack_item() {
		let item = StackItem::Struct {
			value: vec![
				StackItem::Integer { value: 1000000000 }, // 10 GAS
				StackItem::Integer { value: 100000 },     // block height
			],
		};

		let deposit = NotaryDeposit::from_stack_item(&item).unwrap();
		assert_eq!(deposit.amount, 1000000000i64);
		assert_eq!(deposit.till, 100000);
	}

	#[test]
	fn notary_deposit_rejects_invalid_block_height() {
		for till in [-1_i64, i64::from(u32::MAX) + 1] {
			let item = StackItem::Struct {
				value: vec![StackItem::Integer { value: 1 }, StackItem::Integer { value: till }],
			};

			assert!(NotaryDeposit::from_stack_item(&item).is_err());
		}
	}
}
