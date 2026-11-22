//! Policy native contract for Neo N3.
//!
//! The Policy contract manages system policies including fees, blocked accounts,
//! and various network parameters.
//!
//! Updated for Neo 3.9 with support for:
//! - Milliseconds per block (HF_Echidna)
//! - Max valid until block increment (HF_Echidna)
//! - Max traceable blocks (HF_Echidna)
//! - Attribute fees (HF_Echidna)
//! - Execution fee factor with decimals (HF_Faun)
//! - Whitelist fee contracts (HF_Faun)
//! - Fund recovery from blocked accounts (HF_Faun)
//!
//! Updated for Neo 3.11.2 (HF_Huyao groundwork) with support for:
//! - On-chain hardfork activation query (`getHardforkActivationHeight`)
//! - Committee hardfork activation via `activateHardfork` (HF_Huyao and later)
//! - Temporary storage TTL policy (`getTemporaryStorageMaxTTL`)

use async_trait::async_trait;
use primitive_types::H160;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
	ScriptHashExtension,
	neo_builder::TransactionBuilder,
	neo_clients::{JsonRpcProvider, RpcClient},
	neo_contract::{ContractError, NeoIterator, checked_vm_integer, traits::SmartContractTrait},
	neo_types::{
		ScriptHash, StackItem, WhitelistedContract,
		serde_with_utils::{deserialize_script_hash, serialize_script_hash},
	},
};

/// The Policy native contract for managing system policies.
///
/// This contract controls network parameters such as fees, blocked accounts,
/// and various protocol settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyContract<'a, P: JsonRpcProvider> {
	#[serde(deserialize_with = "deserialize_script_hash")]
	#[serde(serialize_with = "serialize_script_hash")]
	script_hash: ScriptHash,
	#[serde(skip)]
	provider: Option<&'a RpcClient<P>>,
}

impl<'a, P: JsonRpcProvider + 'static> PolicyContract<'a, P> {
	/// The name of the Policy contract.
	pub const NAME: &'static str = "PolicyContract";

	/// The default execution fee factor.
	pub const DEFAULT_EXEC_FEE_FACTOR: u32 = 30;

	/// The default storage price.
	pub const DEFAULT_STORAGE_PRICE: u32 = 100000;

	/// The default network fee per byte (in datoshi, 1 datoshi = 1e-8 GAS).
	pub const DEFAULT_FEE_PER_BYTE: u32 = 1000;

	/// The default fee for transaction attributes.
	pub const DEFAULT_ATTRIBUTE_FEE: u32 = 0;

	/// The default fee for NotaryAssisted attribute (in datoshi).
	pub const DEFAULT_NOTARY_ASSISTED_ATTRIBUTE_FEE: u32 = 10_000_000;

	/// The maximum execution fee factor.
	pub const MAX_EXEC_FEE_FACTOR: u64 = 100;

	/// The maximum fee for transaction attributes.
	pub const MAX_ATTRIBUTE_FEE: u32 = 10_0000_0000;

	/// The maximum storage price.
	pub const MAX_STORAGE_PRICE: u32 = 10_000_000;

	/// The maximum block generation time in milliseconds.
	pub const MAX_MILLISECONDS_PER_BLOCK: u32 = 30_000;

	/// The maximum MaxValidUntilBlockIncrement value (1 day of 1-second blocks).
	pub const MAX_MAX_VALID_UNTIL_BLOCK_INCREMENT: u32 = 86400;

	/// The maximum MaxTraceableBlocks value (1 year of 15-second blocks).
	pub const MAX_MAX_TRACEABLE_BLOCKS: u32 = 2_102_400;

	/// The maximum TemporaryStorageMaxTTL value the committee can set
	/// (365 days in milliseconds). Introduced with HF_Huyao (Neo v3.11.x).
	pub const MAX_TEMPORARY_STORAGE_MAX_TTL: u64 = 365 * 24 * 60 * 60 * 1_000;

	/// Creates a new PolicyContract instance.
	pub fn new(provider: Option<&'a RpcClient<P>>) -> Self {
		Self { script_hash: Self::calc_native_contract_hash_unchecked(Self::NAME), provider }
	}

	// ========== Basic Policy Methods ==========

	/// Gets the network fee per transaction byte (in datoshi).
	pub async fn get_fee_per_byte(&self) -> Result<i64, ContractError> {
		self.call_function_returning_int("getFeePerByte", vec![]).await
	}

	/// Gets the execution fee factor.
	///
	/// This is a multiplier used to calculate system fees for transactions.
	/// After HF_Faun, this returns the factor without decimals (use `get_exec_pico_fee_factor`
	/// for the full precision value).
	pub async fn get_exec_fee_factor(&self) -> Result<u32, ContractError> {
		checked_vm_integer(
			self.call_function_returning_int("getExecFeeFactor", vec![]).await?,
			"execution fee factor",
		)
	}

	/// Gets the execution fee factor in pico-GAS (1 pico-GAS = 1e-12 GAS).
	///
	/// This method is available after HF_Faun and returns the fee factor with full decimal precision.
	pub async fn get_exec_pico_fee_factor(&self) -> Result<i64, ContractError> {
		self.call_function_returning_int("getExecPicoFeeFactor", vec![]).await
	}

	/// Gets the storage price (cost per byte of storage).
	pub async fn get_storage_price(&self) -> Result<u32, ContractError> {
		checked_vm_integer(
			self.call_function_returning_int("getStoragePrice", vec![]).await?,
			"storage price",
		)
	}

	// ========== Block Time and Limits (HF_Echidna) ==========

	/// Gets the block generation time in milliseconds.
	///
	/// This method is available after HF_Echidna.
	pub async fn get_milliseconds_per_block(&self) -> Result<u32, ContractError> {
		checked_vm_integer(
			self.call_function_returning_int("getMillisecondsPerBlock", vec![]).await?,
			"milliseconds per block",
		)
	}

	/// Sets the block generation time in milliseconds.
	///
	/// This method requires committee approval.
	/// Value must be between 1 and MAX_MILLISECONDS_PER_BLOCK (30,000).
	///
	/// Available after HF_Echidna.
	pub async fn set_milliseconds_per_block(
		&self,
		value: u32,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		self.invoke_function("setMillisecondsPerBlock", vec![value.into()]).await
	}

	/// Gets the maximum ValidUntilBlock increment for transactions.
	///
	/// This method is available after HF_Echidna.
	pub async fn get_max_valid_until_block_increment(&self) -> Result<u32, ContractError> {
		checked_vm_integer(
			self.call_function_returning_int("getMaxValidUntilBlockIncrement", vec![])
				.await?,
			"maximum valid-until-block increment",
		)
	}

	/// Sets the maximum ValidUntilBlock increment.
	///
	/// This method requires committee approval.
	/// Value must be between 1 and MAX_MAX_VALID_UNTIL_BLOCK_INCREMENT (86,400)
	/// and less than MaxTraceableBlocks.
	///
	/// Available after HF_Echidna.
	pub async fn set_max_valid_until_block_increment(
		&self,
		value: u32,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		self.invoke_function("setMaxValidUntilBlockIncrement", vec![value.into()]).await
	}

	/// Gets the maximum number of blocks accessible to smart contracts.
	///
	/// This method is available after HF_Echidna.
	pub async fn get_max_traceable_blocks(&self) -> Result<u32, ContractError> {
		checked_vm_integer(
			self.call_function_returning_int("getMaxTraceableBlocks", vec![]).await?,
			"maximum traceable blocks",
		)
	}

	/// Sets the maximum number of traceable blocks.
	///
	/// This method requires committee approval.
	/// Value must be between 1 and MAX_MAX_TRACEABLE_BLOCKS (2,102,400),
	/// cannot be increased, and must be greater than MaxValidUntilBlockIncrement.
	///
	/// Available after HF_Echidna.
	pub async fn set_max_traceable_blocks(
		&self,
		value: u32,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		self.invoke_function("setMaxTraceableBlocks", vec![value.into()]).await
	}

	// ========== Attribute Fees (HF_Echidna) ==========

	/// Gets the fee for a specific transaction attribute type.
	///
	/// Available after HF_Echidna.
	///
	/// # Arguments
	///
	/// * `attribute_type` - The transaction attribute type (as a byte value).
	pub async fn get_attribute_fee(&self, attribute_type: u8) -> Result<u32, ContractError> {
		checked_vm_integer(
			self.call_function_returning_int("getAttributeFee", vec![attribute_type.into()])
				.await?,
			"attribute fee",
		)
	}

	/// Sets the fee for a specific transaction attribute type.
	///
	/// This method requires committee approval.
	///
	/// Available after HF_Echidna.
	///
	/// # Arguments
	///
	/// * `attribute_type` - The transaction attribute type (as a byte value).
	/// * `value` - The fee value (must be <= MAX_ATTRIBUTE_FEE).
	pub async fn set_attribute_fee(
		&self,
		attribute_type: u8,
		value: u32,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		self.invoke_function("setAttributeFee", vec![attribute_type.into(), value.into()])
			.await
	}

	// ========== Account Blocking ==========

	/// Checks if the specified account is blocked.
	pub async fn is_blocked(&self, script_hash: &H160) -> Result<bool, ContractError> {
		self.call_function_returning_bool("isBlocked", vec![script_hash.into()]).await
	}

	/// Gets blocked accounts as an iterator.
	///
	/// Available after HF_Faun.
	pub async fn get_blocked_accounts(&self) -> Result<NeoIterator<'_, H160, P>, ContractError> {
		self.call_function_returning_iterator(
			"getBlockedAccounts",
			vec![],
			Arc::new(|item: StackItem| {
				item.as_hash160()
					.ok_or_else(|| ContractError::UnexpectedReturnType("Hash160".to_string()))
			}),
		)
		.await
	}

	/// Gets all blocked accounts by fully unwrapping the iterator.
	///
	/// Available after HF_Faun.
	pub async fn get_blocked_accounts_all(&self) -> Result<Vec<H160>, ContractError> {
		self.get_blocked_accounts_all_with_batch(
			<Self as SmartContractTrait>::DEFAULT_ITERATOR_COUNT,
		)
		.await
	}

	/// Gets all blocked accounts using a custom batch size.
	///
	/// Available after HF_Faun.
	pub async fn get_blocked_accounts_all_with_batch(
		&self,
		batch_size: usize,
	) -> Result<Vec<H160>, ContractError> {
		let batch_size = Self::checked_iterator_batch_size(batch_size)?;
		let iterator = self.get_blocked_accounts().await?;
		self.collect_all(iterator, batch_size).await
	}

	/// Blocks an account.
	///
	/// This method requires committee approval.
	/// After HF_Faun, blocking an account will also clear its votes.
	pub async fn block_account(
		&self,
		account: &H160,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		self.invoke_function("blockAccount", vec![account.into()]).await
	}

	/// Blocks an account by address.
	///
	/// This method requires committee approval.
	pub async fn block_account_address(
		&self,
		address: &str,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		let account = ScriptHash::from_address(address)
			.map_err(|_| ContractError::InvalidAccount("Invalid address".to_string()))?;
		self.block_account(&account).await
	}

	/// Unblocks an account.
	///
	/// This method requires committee approval.
	pub async fn unblock_account(
		&self,
		account: &H160,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		self.invoke_function("unblockAccount", vec![account.into()]).await
	}

	/// Unblocks an account by address.
	///
	/// This method requires committee approval.
	pub async fn unblock_account_address(
		&self,
		address: &str,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		let account = ScriptHash::from_address(address)
			.map_err(|_| ContractError::InvalidAccount("Invalid address".to_string()))?;
		self.unblock_account(&account).await
	}

	// ========== Fee Factor and Price Setters ==========

	/// Sets the network fee per byte.
	///
	/// This method requires committee approval.
	/// Value must be between 0 and 100,000,000.
	pub async fn set_fee_per_byte(
		&self,
		fee: i64,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		self.invoke_function("setFeePerByte", vec![fee.into()]).await
	}

	/// Sets the execution fee factor.
	///
	/// This method requires committee approval.
	/// After HF_Faun, the maximum value is increased to account for decimals.
	pub async fn set_exec_fee_factor(
		&self,
		fee: u64,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		self.invoke_function("setExecFeeFactor", vec![fee.into()]).await
	}

	/// Sets the storage price.
	///
	/// This method requires committee approval.
	/// Value must be between 1 and MAX_STORAGE_PRICE (10,000,000).
	pub async fn set_storage_price(
		&self,
		price: u32,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		self.invoke_function("setStoragePrice", vec![price.into()]).await
	}

	// ========== Whitelist Fee Contracts (HF_Faun) ==========

	/// Gets whitelisted fee contracts as an iterator.
	///
	/// Available after HF_Faun.
	pub async fn get_whitelist_fee_contracts(
		&self,
	) -> Result<NeoIterator<'_, WhitelistedContract, P>, ContractError> {
		self.call_function_returning_iterator(
			"getWhitelistFeeContracts",
			vec![],
			Arc::new(|item: StackItem| {
				WhitelistedContract::from_stack_item(&item).map_err(|err| {
					ContractError::UnexpectedReturnType(format!("WhitelistedContract: {err}"))
				})
			}),
		)
		.await
	}

	/// Gets all whitelisted fee contracts by fully unwrapping the iterator.
	///
	/// Available after HF_Faun.
	pub async fn get_whitelist_fee_contracts_all(
		&self,
	) -> Result<Vec<WhitelistedContract>, ContractError> {
		self.get_whitelist_fee_contracts_all_with_batch(
			<Self as SmartContractTrait>::DEFAULT_ITERATOR_COUNT,
		)
		.await
	}

	/// Gets all whitelisted fee contracts using a custom batch size.
	///
	/// Available after HF_Faun.
	pub async fn get_whitelist_fee_contracts_all_with_batch(
		&self,
		batch_size: usize,
	) -> Result<Vec<WhitelistedContract>, ContractError> {
		let batch_size = Self::checked_iterator_batch_size(batch_size)?;
		let iterator = self.get_whitelist_fee_contracts().await?;
		self.collect_all(iterator, batch_size).await
	}

	fn checked_iterator_batch_size(batch_size: usize) -> Result<i32, ContractError> {
		if batch_size == 0 {
			return Err(ContractError::InvalidArgError(
				"Batch size must be greater than zero".to_string(),
			));
		}

		i32::try_from(batch_size).map_err(|_| {
			ContractError::InvalidArgError(format!("Batch size must not exceed {}", i32::MAX))
		})
	}

	async fn collect_all<T>(
		&self,
		iterator: NeoIterator<'_, T, P>,
		batch_size: i32,
	) -> Result<Vec<T>, ContractError> {
		let mut all_items = Vec::new();
		let mut traverse_error: Option<ContractError> = None;

		loop {
			match iterator.traverse(batch_size).await {
				Ok(batch) => {
					if batch.is_empty() {
						break;
					}
					all_items.extend(batch);
				},
				Err(err) => {
					traverse_error = Some(err);
					break;
				},
			}
		}

		if let Some(err) = traverse_error {
			if let Err(term_err) = iterator.terminate_session().await {
				tracing::warn!(error = %term_err, "Failed to terminate iterator session during error cleanup");
			}
			return Err(err);
		}

		iterator.terminate_session().await?;
		Ok(all_items)
	}

	/// Sets a whitelisted contract method for fee exemption.
	///
	/// When a contract method is whitelisted, invocations use the specified fixed fee
	/// instead of the calculated execution fee.
	///
	/// This method requires committee approval.
	/// Available after HF_Faun.
	///
	/// # Arguments
	///
	/// * `contract_hash` - The contract to whitelist.
	/// * `method` - The method name.
	/// * `arg_count` - The number of arguments the method takes.
	/// * `fixed_fee` - The fixed execution fee for this method.
	pub async fn set_whitelist_fee_contract(
		&self,
		contract_hash: &H160,
		method: &str,
		arg_count: i32,
		fixed_fee: i64,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		self.invoke_function(
			"setWhitelistFeeContract",
			vec![contract_hash.into(), method.into(), arg_count.into(), fixed_fee.into()],
		)
		.await
	}

	/// Removes a whitelisted contract method.
	///
	/// This method requires committee approval.
	/// Available after HF_Faun.
	///
	/// # Arguments
	///
	/// * `contract_hash` - The contract to remove from whitelist.
	/// * `method` - The method name.
	/// * `arg_count` - The number of arguments the method takes.
	pub async fn remove_whitelist_fee_contract(
		&self,
		contract_hash: &H160,
		method: &str,
		arg_count: i32,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		self.invoke_function(
			"removeWhitelistFeeContract",
			vec![contract_hash.into(), method.into(), arg_count.into()],
		)
		.await
	}

	// ========== Fund Recovery (HF_Faun) ==========

	/// Recovers funds from a blocked account.
	///
	/// This method requires almost full committee approval and can only be called
	/// after the account has been blocked for at least 1 year.
	///
	/// Available after HF_Faun.
	///
	/// # Arguments
	///
	/// * `account` - The blocked account to recover funds from.
	/// * `token` - The token contract to recover (must implement NEP-17).
	pub async fn recover_fund(
		&self,
		account: &H160,
		token: &H160,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		self.invoke_function("recoverFund", vec![account.into(), token.into()]).await
	}

	// ========== Hardfork Activation (HF_Huyao, Neo v3.11.x) ==========

	/// Gets the block height at which a hardfork was activated via a committee
	/// transaction, or `None` if the hardfork is unknown or not scheduled on-chain.
	///
	/// The hardfork name must be the raw Policy name (case-sensitive), e.g.
	/// `"Iara"`, `"Jackalope"` — not `"HF_Iara"`. Use
	/// [`crate::neo_types::Hardfork::policy_name`] to convert an enum value.
	///
	/// Available after HF_Huyao (introduced in Neo v3.11.x).
	///
	/// # Arguments
	///
	/// * `hardfork` - The raw hardfork name (e.g. `"Iara"`).
	pub async fn get_hardfork_activation_height(
		&self,
		hardfork: &str,
	) -> Result<Option<u32>, ContractError> {
		let output = self
			.call_invoke_function("getHardforkActivationHeight", vec![hardfork.into()], vec![])
			.await?;
		self.throw_if_fault_state(&output)?;

		let item = output
			.get_first_stack_item()
			.map_err(|e| ContractError::InvalidResponse(e.to_string()))?;
		match item {
			StackItem::Any => Ok(None),
			_ => match item.as_int() {
				Some(i) => Ok(Some(checked_vm_integer(i, "hardfork activation height")?)),
				None => Err(ContractError::UnexpectedReturnType("Int".to_string())),
			},
		}
	}

	/// Builds a committee invocation that activates a hardfork on-chain.
	///
	/// Activation takes effect `activation_delay` blocks after the persisting
	/// block that includes this call. Only hardforks after HF_Huyao (e.g.
	/// `"Iara"`, `"Jackalope"`) may be activated this way; config-managed
	/// hardforks are rejected by the contract.
	///
	/// This method requires committee approval.
	/// Available after HF_Huyao (introduced in Neo v3.11.x).
	///
	/// # Arguments
	///
	/// * `hardfork` - The raw hardfork name to activate (e.g. `"Iara"`).
	/// * `activation_delay` - The number of blocks to pass before activation (must be positive).
	pub async fn activate_hardfork(
		&self,
		hardfork: &str,
		activation_delay: u32,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		if activation_delay == 0 {
			return Err(ContractError::InvalidArgError(
				"Hardfork activation delay should be positive".to_string(),
			));
		}
		self.invoke_function("activateHardfork", vec![hardfork.into(), activation_delay.into()])
			.await
	}

	// ========== Temporary Storage Policy (HF_Huyao, Neo v3.11.x) ==========

	/// Gets the maximum allowed TTL value (in milliseconds) for key-value
	/// records in the native TemporaryStorage contract.
	///
	/// Available after HF_Huyao (introduced in Neo v3.11.x).
	pub async fn get_temporary_storage_max_ttl(&self) -> Result<u64, ContractError> {
		let value = self.call_function_returning_int("getTemporaryStorageMaxTTL", vec![]).await?;
		u64::try_from(value).map_err(|_| {
			ContractError::UnexpectedReturnType(
				"TemporaryStorageMaxTTL must be a non-negative integer".to_string(),
			)
		})
	}

	/// Builds a committee invocation that sets the maximum allowed TTL value
	/// (in milliseconds) for key-value records in the native TemporaryStorage
	/// contract. The value must not exceed `MAX_TEMPORARY_STORAGE_MAX_TTL`.
	///
	/// This method requires committee approval.
	/// Available after HF_Huyao (introduced in Neo v3.11.x).
	///
	/// # Arguments
	///
	/// * `value` - The TemporaryStorageMaxTTL value in milliseconds.
	pub async fn set_temporary_storage_max_ttl(
		&self,
		value: u64,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		if value > Self::MAX_TEMPORARY_STORAGE_MAX_TTL {
			return Err(ContractError::InvalidArgError(format!(
				"TemporaryStorageMaxTTL must be between [0, {}], got {}",
				Self::MAX_TEMPORARY_STORAGE_MAX_TTL,
				value
			)));
		}
		let value = i64::try_from(value).map_err(|_| {
			ContractError::InvalidArgError("TemporaryStorageMaxTTL out of range".to_string())
		})?;
		self.invoke_function("setTemporaryStorageMaxTTL", vec![value.into()]).await
	}
}

#[async_trait]
impl<'a, P: JsonRpcProvider> SmartContractTrait<'a> for PolicyContract<'a, P> {
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
	use crate::{
		neo_clients::{MockProvider, RpcClient},
		neo_types::{InvocationResult, NeoVMStateType, StackItem},
	};
	use serde_json::json;

	fn iterator_invocation_result(session_id: &str, iterator_id: &str) -> InvocationResult {
		InvocationResult::new(
			String::new(),
			NeoVMStateType::Halt,
			"0".to_string(),
			None,
			None,
			None,
			vec![StackItem::InteropInterface {
				id: iterator_id.to_string(),
				interface: "IIterator".to_string(),
			}],
			None,
			None,
			Some(session_id.to_string()),
		)
	}

	#[test]
	fn test_policy_contract_constants() {
		assert_eq!(PolicyContract::<MockProvider>::DEFAULT_EXEC_FEE_FACTOR, 30);
		assert_eq!(PolicyContract::<MockProvider>::DEFAULT_STORAGE_PRICE, 100000);
		assert_eq!(PolicyContract::<MockProvider>::DEFAULT_FEE_PER_BYTE, 1000);
		assert_eq!(PolicyContract::<MockProvider>::MAX_MILLISECONDS_PER_BLOCK, 30_000);
		assert_eq!(PolicyContract::<MockProvider>::MAX_MAX_VALID_UNTIL_BLOCK_INCREMENT, 86400);
		assert_eq!(PolicyContract::<MockProvider>::MAX_MAX_TRACEABLE_BLOCKS, 2_102_400);
		// Neo v3.11.2 (HF_Huyao): 365 days in milliseconds.
		assert_eq!(PolicyContract::<MockProvider>::MAX_TEMPORARY_STORAGE_MAX_TTL, 31_536_000_000);
	}

	fn simple_invocation_result(stack: Vec<StackItem>) -> InvocationResult {
		InvocationResult::new(
			String::new(),
			NeoVMStateType::Halt,
			"0".to_string(),
			None,
			None,
			None,
			stack,
			None,
			None,
			None,
		)
	}

	#[tokio::test]
	async fn get_hardfork_activation_height_returns_none_for_unscheduled() {
		let provider = MockProvider::new();
		let client = RpcClient::new(provider.clone());
		let contract = PolicyContract::new(Some(&client));

		provider.push_result(
			"invokefunction",
			serde_json::to_value(simple_invocation_result(vec![StackItem::Any])).unwrap(),
		);

		let result = contract.get_hardfork_activation_height("Iara").await.unwrap();
		assert_eq!(result, None);
	}

	#[tokio::test]
	async fn get_hardfork_activation_height_returns_scheduled_height() {
		let provider = MockProvider::new();
		let client = RpcClient::new(provider.clone());
		let contract = PolicyContract::new(Some(&client));

		provider.push_result(
			"invokefunction",
			serde_json::to_value(simple_invocation_result(vec![StackItem::Integer {
				value: 13_000_000,
			}]))
			.unwrap(),
		);

		let result = contract.get_hardfork_activation_height("Iara").await.unwrap();
		assert_eq!(result, Some(13_000_000));
	}

	#[tokio::test]
	async fn activate_hardfork_rejects_zero_delay_without_rpc() {
		let provider = MockProvider::new();
		let client = RpcClient::new(provider.clone());
		let contract = PolicyContract::new(Some(&client));

		let result = contract.activate_hardfork("Iara", 0).await;

		assert!(matches!(result, Err(ContractError::InvalidArgError(_))));
		assert!(provider.take_requests().is_empty());
	}

	#[tokio::test]
	async fn activate_hardfork_builds_invocation_script() {
		let provider = MockProvider::new();
		let client = RpcClient::new(provider.clone());
		let contract = PolicyContract::new(Some(&client));

		let builder = contract.activate_hardfork("Iara", 1_000).await.unwrap();
		// Sanity: a builder was produced for the caller to sign and relay.
		let _ = builder;
		// Local script construction only: no RPC round-trip should have happened.
		assert!(provider.take_requests().is_empty());
	}

	#[tokio::test]
	async fn set_temporary_storage_max_ttl_rejects_out_of_range_values() {
		let provider = MockProvider::new();
		let client = RpcClient::new(provider.clone());
		let contract = PolicyContract::new(Some(&client));

		let result = contract
			.set_temporary_storage_max_ttl(
				PolicyContract::<MockProvider>::MAX_TEMPORARY_STORAGE_MAX_TTL + 1,
			)
			.await;

		assert!(matches!(result, Err(ContractError::InvalidArgError(_))));
		assert!(provider.take_requests().is_empty());
	}

	#[test]
	fn test_policy_contract_name() {
		assert_eq!(PolicyContract::<MockProvider>::NAME, "PolicyContract");
	}

	#[tokio::test]
	async fn test_get_blocked_accounts_iterator_rejects_invalid_items() {
		let provider = MockProvider::new();
		let client = RpcClient::new(provider.clone());
		let contract = PolicyContract::new(Some(&client));
		let hash = contract.script_hash();

		provider.push_result_with_params(
			"invokefunction",
			json!([hash.to_hex(), "getBlockedAccounts", [], []]),
			serde_json::to_value(iterator_invocation_result("session-1", "iter-1")).unwrap(),
		);
		provider.push_result_with_params(
			"traverseiterator",
			json!(["session-1", "iter-1", 1]),
			serde_json::to_value(vec![StackItem::Integer { value: 7 }]).unwrap(),
		);

		let iterator = contract.get_blocked_accounts().await.unwrap();
		let result = iterator.traverse(1).await;
		assert!(matches!(
			result,
			Err(ContractError::UnexpectedReturnType(message))
				if message.contains("Hash160")
		));
	}

	#[tokio::test]
	async fn test_get_whitelist_fee_contracts_iterator_rejects_invalid_items() {
		let provider = MockProvider::new();
		let client = RpcClient::new(provider.clone());
		let contract = PolicyContract::new(Some(&client));
		let hash = contract.script_hash();

		provider.push_result_with_params(
			"invokefunction",
			json!([hash.to_hex(), "getWhitelistFeeContracts", [], []]),
			serde_json::to_value(iterator_invocation_result("session-2", "iter-2")).unwrap(),
		);
		provider.push_result_with_params(
			"traverseiterator",
			json!(["session-2", "iter-2", 1]),
			serde_json::to_value(vec![StackItem::Integer { value: 11 }]).unwrap(),
		);

		let iterator = contract.get_whitelist_fee_contracts().await.unwrap();
		let result = iterator.traverse(1).await;
		assert!(matches!(
			result,
			Err(ContractError::UnexpectedReturnType(message))
				if message.contains("WhitelistedContract")
		));
	}

	#[tokio::test]
	async fn blocked_accounts_reject_invalid_batch_sizes_before_opening_iterator() {
		for batch_size in [0, i32::MAX as usize + 1] {
			let provider = MockProvider::new();
			let client = RpcClient::new(provider.clone());
			let contract = PolicyContract::new(Some(&client));

			let result = contract.get_blocked_accounts_all_with_batch(batch_size).await;

			assert!(matches!(result, Err(ContractError::InvalidArgError(_))));
			assert!(provider.take_requests().is_empty());
		}
	}

	#[tokio::test]
	async fn whitelist_rejects_invalid_batch_sizes_before_opening_iterator() {
		for batch_size in [0, i32::MAX as usize + 1] {
			let provider = MockProvider::new();
			let client = RpcClient::new(provider.clone());
			let contract = PolicyContract::new(Some(&client));

			let result = contract.get_whitelist_fee_contracts_all_with_batch(batch_size).await;

			assert!(matches!(result, Err(ContractError::InvalidArgError(_))));
			assert!(provider.take_requests().is_empty());
		}
	}
}
