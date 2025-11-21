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

/// NeoCompound contract interface for Neo N3
///
/// NeoCompound is an automated interest compounding service for Neo ecosystem tokens.
/// This contract interface provides methods to interact with the NeoCompound smart contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeoCompoundContract<'a, P: JsonRpcProvider> {
	#[serde(deserialize_with = "deserialize_script_hash")]
	#[serde(serialize_with = "serialize_script_hash")]
	script_hash: ScriptHash,
	#[serde(skip)]
	provider: Option<&'a RpcClient<P>>,
}

impl<'a, P: JsonRpcProvider + 'static> NeoCompoundContract<'a, P> {
	/// The script hash of the NeoCompound contract on Neo N3 MainNet
	pub const CONTRACT_HASH: &'static str = "f0151f528127558851b39c2cd8aa47da7418ab28";

	// Method constants
	/// Method name for depositing tokens
	pub const DEPOSIT: &'static str = "deposit";
	/// Method name for withdrawing tokens
	pub const WITHDRAW: &'static str = "withdraw";
	/// Method name for compounding interest
	pub const COMPOUND: &'static str = "compound";
	/// Method name for getting the APY
	pub const GET_APY: &'static str = "getAPY";

	/// Creates a new NeoCompoundContract instance with the default contract hash
	///
	/// # Arguments
	///
	/// * `provider` - An optional reference to an RPC client
	///
	/// # Returns
	///
	/// A new NeoCompoundContract instance
	pub fn new(provider: Option<&'a RpcClient<P>>) -> Self {
		Self { script_hash: ScriptHash::from_str(Self::CONTRACT_HASH).unwrap(), provider }
	}

	/// Creates a new NeoCompoundContract instance with a custom script hash
	///
	/// # Arguments
	///
	/// * `script_hash` - The script hash of the NeoCompound contract
	/// * `provider` - An optional reference to an RPC client
	///
	/// # Returns
	///
	/// A new NeoCompoundContract instance
	pub fn with_script_hash(script_hash: ScriptHash, provider: Option<&'a RpcClient<P>>) -> Self {
		Self { script_hash, provider }
	}

	/// Deposits tokens into NeoCompound
	///
	/// # Arguments
	///
	/// * `token` - The script hash of the token to deposit
	/// * `amount` - The amount of tokens to deposit
	/// * `account` - The account that will sign the transaction
	///
	/// # Returns
	///
	/// A transaction builder that can be used to build and sign the transaction
	pub async fn deposit(
		&self,
		token: &ScriptHash,
		amount: i64,
		account: &Account,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		let params = vec![token.into(), ContractParameter::integer(amount)];

		let mut builder = self.invoke_function(Self::DEPOSIT, params).await?;
		builder
			.set_signers(vec![AccountSigner::called_by_entry(account).unwrap().into()])
			.map_err(|err| ContractError::RuntimeError(err.to_string()))?;

		Ok(builder)
	}

	/// Withdraws tokens from NeoCompound
	///
	/// # Arguments
	///
	/// * `token` - The script hash of the token to withdraw
	/// * `amount` - The amount of tokens to withdraw
	/// * `account` - The account that will sign the transaction
	///
	/// # Returns
	///
	/// A transaction builder that can be used to build and sign the transaction
	pub async fn withdraw(
		&self,
		token: &ScriptHash,
		amount: i64,
		account: &Account,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		let params = vec![token.into(), ContractParameter::integer(amount)];

		let mut builder = self.invoke_function(Self::WITHDRAW, params).await?;
		builder
			.set_signers(vec![AccountSigner::called_by_entry(account).unwrap().into()])
			.map_err(|err| ContractError::RuntimeError(err.to_string()))?;

		Ok(builder)
	}

	/// Compounds interest for a specific token
	///
	/// # Arguments
	///
	/// * `token` - The script hash of the token to compound interest for
	/// * `account` - The account that will sign the transaction
	///
	/// # Returns
	///
	/// A transaction builder that can be used to build and sign the transaction
	pub async fn compound(
		&self,
		token: &ScriptHash,
		account: &Account,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		let params = vec![token.into()];

		let mut builder = self.invoke_function(Self::COMPOUND, params).await?;
		builder
			.set_signers(vec![AccountSigner::called_by_entry(account).unwrap().into()])
			.map_err(|err| ContractError::RuntimeError(err.to_string()))?;

		Ok(builder)
	}

	/// Gets the current APY for a specific token
	///
	/// # Arguments
	///
	/// * `token` - The script hash of the token to get the APY for
	///
	/// # Returns
	///
	/// The APY as a floating-point percentage
	pub async fn get_apy(&self, token: &ScriptHash) -> Result<f64, ContractError> {
		let result = self.call_function_returning_int(Self::GET_APY, vec![token.into()]).await?;
		// Convert the integer result to a floating-point percentage (assuming APY is stored as an integer with a fixed decimal point)
		Ok(result as f64 / 100.0) // Assuming 2 decimal places for percentage
	}
}

#[async_trait]
impl<'a, P: JsonRpcProvider> SmartContractTrait<'a> for NeoCompoundContract<'a, P> {
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
