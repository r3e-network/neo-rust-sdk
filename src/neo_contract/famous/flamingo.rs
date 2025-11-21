use async_trait::async_trait;
use primitive_types::H160;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::{
	builder::{AccountSigner, TransactionBuilder},
	neo_clients::{JsonRpcProvider, RpcClient},
	neo_contract::{ContractError, SmartContractTrait},
	neo_protocol::Account,
};
use neo3::prelude::*;

/// Flamingo Finance contract interface for Neo N3
///
/// Flamingo Finance is a DeFi platform on Neo N3 offering trading, earning, and borrowing services.
/// This contract interface provides methods to interact with the Flamingo Finance smart contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlamingoContract<'a, P: JsonRpcProvider> {
	#[serde(deserialize_with = "deserialize_script_hash")]
	#[serde(serialize_with = "serialize_script_hash")]
	script_hash: ScriptHash,
	#[serde(skip)]
	provider: Option<&'a RpcClient<P>>,
}

impl<'a, P: JsonRpcProvider + 'static> FlamingoContract<'a, P> {
	/// The script hash of the Flamingo Finance contract on Neo N3 MainNet
	pub const CONTRACT_HASH: &'static str = "f970f4cddcd087ab5d8a5697a32b3cfd32c8b465";

	// Method constants
	/// Method name for swapping tokens
	pub const SWAP: &'static str = "swap";
	/// Method name for adding liquidity
	pub const ADD_LIQUIDITY: &'static str = "addLiquidity";
	/// Method name for removing liquidity
	pub const REMOVE_LIQUIDITY: &'static str = "removeLiquidity";
	/// Method name for staking tokens
	pub const STAKE: &'static str = "stake";
	/// Method name for claiming rewards
	pub const CLAIM_REWARDS: &'static str = "claimRewards";

	/// Creates a new FlamingoContract instance with the default contract hash
	///
	/// # Arguments
	///
	/// * `provider` - An optional reference to an RPC client
	///
	/// # Returns
	///
	/// A new FlamingoContract instance
	pub fn new(provider: Option<&'a RpcClient<P>>) -> Self {
		Self { script_hash: ScriptHash::from_str(Self::CONTRACT_HASH).unwrap(), provider }
	}

	/// Creates a new FlamingoContract instance with a custom script hash
	///
	/// # Arguments
	///
	/// * `script_hash` - The script hash of the Flamingo Finance contract
	/// * `provider` - An optional reference to an RPC client
	///
	/// # Returns
	///
	/// A new FlamingoContract instance
	pub fn with_script_hash(script_hash: ScriptHash, provider: Option<&'a RpcClient<P>>) -> Self {
		Self { script_hash, provider }
	}

	/// Swaps tokens on Flamingo Finance
	///
	/// # Arguments
	///
	/// * `from_token` - The script hash of the token to swap from
	/// * `to_token` - The script hash of the token to swap to
	/// * `amount` - The amount of tokens to swap
	/// * `min_return` - The minimum amount of tokens to receive
	/// * `account` - The account that will sign the transaction
	///
	/// # Returns
	///
	/// A transaction builder that can be used to build and sign the transaction
	pub async fn swap(
		&self,
		from_token: &ScriptHash,
		to_token: &ScriptHash,
		amount: i64,
		min_return: i64,
		account: &Account,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		let params = vec![
			from_token.into(),
			to_token.into(),
			ContractParameter::integer(amount),
			ContractParameter::integer(min_return),
		];

		let mut builder = self.invoke_function(Self::SWAP, params).await?;
		builder
			.set_signers(vec![AccountSigner::called_by_entry(account).unwrap().into()])
			.map_err(|err| ContractError::RuntimeError(err.to_string()))?;

		Ok(builder)
	}

	/// Adds liquidity to a Flamingo Finance liquidity pool
	///
	/// # Arguments
	///
	/// * `token_a` - The script hash of the first token
	/// * `token_b` - The script hash of the second token
	/// * `amount_a` - The amount of the first token to add
	/// * `amount_b` - The amount of the second token to add
	/// * `account` - The account that will sign the transaction
	///
	/// # Returns
	///
	/// A transaction builder that can be used to build and sign the transaction
	pub async fn add_liquidity(
		&self,
		token_a: &ScriptHash,
		token_b: &ScriptHash,
		amount_a: i64,
		amount_b: i64,
		account: &Account,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		let params = vec![
			token_a.into(),
			token_b.into(),
			ContractParameter::integer(amount_a),
			ContractParameter::integer(amount_b),
		];

		let mut builder = self.invoke_function(Self::ADD_LIQUIDITY, params).await?;
		builder
			.set_signers(vec![AccountSigner::called_by_entry(account).unwrap().into()])
			.map_err(|err| ContractError::RuntimeError(err.to_string()))?;

		Ok(builder)
	}

	/// Removes liquidity from a Flamingo Finance liquidity pool
	///
	/// # Arguments
	///
	/// * `token_a` - The script hash of the first token
	/// * `token_b` - The script hash of the second token
	/// * `liquidity` - The amount of liquidity tokens to remove
	/// * `account` - The account that will sign the transaction
	///
	/// # Returns
	///
	/// A transaction builder that can be used to build and sign the transaction
	pub async fn remove_liquidity(
		&self,
		token_a: &ScriptHash,
		token_b: &ScriptHash,
		liquidity: i64,
		account: &Account,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		let params = vec![token_a.into(), token_b.into(), ContractParameter::integer(liquidity)];

		let mut builder = self.invoke_function(Self::REMOVE_LIQUIDITY, params).await?;
		builder
			.set_signers(vec![AccountSigner::called_by_entry(account).unwrap().into()])
			.map_err(|err| ContractError::RuntimeError(err.to_string()))?;

		Ok(builder)
	}

	/// Stakes tokens on Flamingo Finance
	///
	/// # Arguments
	///
	/// * `token` - The script hash of the token to stake
	/// * `amount` - The amount of tokens to stake
	/// * `account` - The account that will sign the transaction
	///
	/// # Returns
	///
	/// A transaction builder that can be used to build and sign the transaction
	pub async fn stake(
		&self,
		token: &ScriptHash,
		amount: i64,
		account: &Account,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		let params = vec![token.into(), ContractParameter::integer(amount)];

		let mut builder = self.invoke_function(Self::STAKE, params).await?;
		builder
			.set_signers(vec![AccountSigner::called_by_entry(account).unwrap().into()])
			.map_err(|err| ContractError::RuntimeError(err.to_string()))?;

		Ok(builder)
	}

	/// Claims rewards from Flamingo Finance
	///
	/// # Arguments
	///
	/// * `account` - The account that will sign the transaction
	///
	/// # Returns
	///
	/// A transaction builder that can be used to build and sign the transaction
	pub async fn claim_rewards(
		&self,
		account: &Account,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		let params = vec![];

		let mut builder = self.invoke_function(Self::CLAIM_REWARDS, params).await?;
		builder
			.set_signers(vec![AccountSigner::called_by_entry(account).unwrap().into()])
			.map_err(|err| ContractError::RuntimeError(err.to_string()))?;

		Ok(builder)
	}
}

#[async_trait]
impl<'a, P: JsonRpcProvider> SmartContractTrait<'a> for FlamingoContract<'a, P> {
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
