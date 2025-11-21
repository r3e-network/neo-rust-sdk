use async_trait::async_trait;
use primitive_types::H160;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::{
	builder::{AccountSigner, TransactionBuilder},
	neo_clients::{JsonRpcProvider, RpcClient},
	neo_contract::{ContractError, SmartContractTrait, TokenTrait},
	neo_protocol::Account,
};
use neo3::prelude::*;

/// NeoburgerNeo contract interface for Neo N3
///
/// NeoburgerNeo (bNEO) is a wrapped NEO token that allows users to earn GAS while using their NEO in DeFi.
/// This contract interface provides methods to interact with the NeoburgerNeo smart contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeoburgerContract<'a, P: JsonRpcProvider> {
	#[serde(deserialize_with = "deserialize_script_hash")]
	#[serde(serialize_with = "serialize_script_hash")]
	script_hash: ScriptHash,
	#[serde(skip_serializing_if = "Option::is_none")]
	total_supply: Option<u64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	decimals: Option<u8>,
	#[serde(skip_serializing_if = "Option::is_none")]
	symbol: Option<String>,
	#[serde(skip)]
	provider: Option<&'a RpcClient<P>>,
}

impl<'a, P: JsonRpcProvider + 'static> NeoburgerContract<'a, P> {
	/// The script hash of the NeoburgerNeo contract on Neo N3 MainNet
	pub const CONTRACT_HASH: &'static str = "48c40d4666f93408be1bef038b6722404f5c4a5a";
	/// The symbol of the NeoburgerNeo token
	pub const SYMBOL: &'static str = "bNEO";
	/// The number of decimals for the NeoburgerNeo token
	pub const DECIMALS: u8 = 8;

	// Method constants
	/// Method name for wrapping NEO to bNEO
	pub const WRAP: &'static str = "wrap";
	/// Method name for unwrapping bNEO to NEO
	pub const UNWRAP: &'static str = "unwrap";
	/// Method name for claiming GAS
	pub const CLAIM_GAS: &'static str = "claimGas";
	/// Method name for getting the exchange rate
	pub const GET_RATE: &'static str = "getRate";

	/// Creates a new NeoburgerContract instance with the default contract hash
	///
	/// # Arguments
	///
	/// * `provider` - An optional reference to an RPC client
	///
	/// # Returns
	///
	/// A new NeoburgerContract instance
	pub fn new(provider: Option<&'a RpcClient<P>>) -> Self {
		Self {
			script_hash: ScriptHash::from_str(Self::CONTRACT_HASH).unwrap(),
			total_supply: None,
			decimals: Some(Self::DECIMALS),
			symbol: Some(Self::SYMBOL.to_string()),
			provider,
		}
	}

	/// Creates a new NeoburgerContract instance with a custom script hash
	///
	/// # Arguments
	///
	/// * `script_hash` - The script hash of the NeoburgerNeo contract
	/// * `provider` - An optional reference to an RPC client
	///
	/// # Returns
	///
	/// A new NeoburgerContract instance
	pub fn with_script_hash(script_hash: ScriptHash, provider: Option<&'a RpcClient<P>>) -> Self {
		Self {
			script_hash,
			total_supply: None,
			decimals: Some(Self::DECIMALS),
			symbol: Some(Self::SYMBOL.to_string()),
			provider,
		}
	}

	/// Wraps NEO to bNEO
	///
	/// # Arguments
	///
	/// * `amount` - The amount of NEO to wrap
	/// * `account` - The account that will sign the transaction
	///
	/// # Returns
	///
	/// A transaction builder that can be used to build and sign the transaction
	pub async fn wrap(
		&self,
		amount: i64,
		account: &Account,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		let params = vec![ContractParameter::integer(amount)];

		let mut builder = self.invoke_function(Self::WRAP, params).await?;
		builder
			.set_signers(vec![AccountSigner::called_by_entry(account).unwrap().into()])
			.map_err(|err| ContractError::RuntimeError(err.to_string()))?;

		Ok(builder)
	}

	/// Unwraps bNEO to NEO
	///
	/// # Arguments
	///
	/// * `amount` - The amount of bNEO to unwrap
	/// * `account` - The account that will sign the transaction
	///
	/// # Returns
	///
	/// A transaction builder that can be used to build and sign the transaction
	pub async fn unwrap(
		&self,
		amount: i64,
		account: &Account,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		let params = vec![ContractParameter::integer(amount)];

		let mut builder = self.invoke_function(Self::UNWRAP, params).await?;
		builder
			.set_signers(vec![AccountSigner::called_by_entry(account).unwrap().into()])
			.map_err(|err| ContractError::RuntimeError(err.to_string()))?;

		Ok(builder)
	}

	/// Claims GAS rewards from holding bNEO
	///
	/// # Arguments
	///
	/// * `account` - The account that will sign the transaction
	///
	/// # Returns
	///
	/// A transaction builder that can be used to build and sign the transaction
	pub async fn claim_gas(
		&self,
		account: &Account,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		let params = vec![];

		let mut builder = self.invoke_function(Self::CLAIM_GAS, params).await?;
		builder
			.set_signers(vec![AccountSigner::called_by_entry(account).unwrap().into()])
			.map_err(|err| ContractError::RuntimeError(err.to_string()))?;

		Ok(builder)
	}

	/// Gets the current exchange rate between NEO and bNEO
	///
	/// # Returns
	///
	/// The exchange rate as a floating-point number
	pub async fn get_rate(&self) -> Result<f64, ContractError> {
		let result = self.call_function_returning_int(Self::GET_RATE, vec![]).await?;
		// Convert the integer result to a floating-point rate (assuming rate is stored as an integer with a fixed decimal point)
		Ok(result as f64 / 100_000_000.0) // Assuming 8 decimal places
	}
}

#[async_trait]
impl<'a, P: JsonRpcProvider> SmartContractTrait<'a> for NeoburgerContract<'a, P> {
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

#[async_trait]
impl<'a, P: JsonRpcProvider> TokenTrait<'a, P> for NeoburgerContract<'a, P> {
	fn total_supply(&self) -> Option<u64> {
		self.total_supply
	}

	fn set_total_supply(&mut self, total_supply: u64) {
		self.total_supply = Some(total_supply);
	}

	fn decimals(&self) -> Option<u8> {
		self.decimals
	}

	fn set_decimals(&mut self, decimals: u8) {
		self.decimals = Some(decimals);
	}

	fn symbol(&self) -> Option<String> {
		self.symbol.clone()
	}

	fn set_symbol(&mut self, symbol: String) {
		self.symbol = Some(symbol);
	}

	async fn resolve_nns_text_record(&self, _name: &NNSName) -> Result<H160, ContractError> {
		Err(ContractError::InvalidNeoName(
			"NeoburgerNeo does not support NNS resolution".to_string(),
		))
	}
}
