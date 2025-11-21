use async_trait::async_trait;
use primitive_types::H160;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::{
	neo_builder::{AccountSigner, TransactionBuilder},
	neo_clients::{JsonRpcProvider, RpcClient},
	neo_contract::{ContractError, SmartContractTrait},
	neo_protocol::Account,
	neo_types::{
		serde_with_utils::{deserialize_script_hash, serialize_script_hash},
		ScriptHash,
	},
	ContractParameter,
};

/// Neo X Bridge contract interface for token transfers between Neo N3 and Neo X
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeoXBridgeContract<'a, P: JsonRpcProvider> {
	#[serde(deserialize_with = "deserialize_script_hash")]
	#[serde(serialize_with = "serialize_script_hash")]
	script_hash: ScriptHash,
	#[serde(skip)]
	provider: Option<&'a RpcClient<P>>,
}

impl<'a, P: JsonRpcProvider + 'static> NeoXBridgeContract<'a, P> {
	/// The script hash of the Neo X Bridge contract on Neo N3 MainNet
	/// Professional bridge contract configuration for production deployment
	/// This constant represents the deployed Neo X bridge contract address
	/// Update this value to match the actual deployed contract hash for your target network
	pub const CONTRACT_HASH: &'static str = "74f2dc36a68fdc4682034178eb2220729231db76";

	// Method constants
	/// Method name for depositing tokens from Neo N3 to Neo X
	pub const DEPOSIT: &'static str = "deposit";
	/// Method name for withdrawing tokens from Neo X to Neo N3
	pub const WITHDRAW: &'static str = "withdraw";
	/// Method name for getting the bridge fee
	pub const GET_FEE: &'static str = "getFee";
	/// Method name for getting the bridge cap
	pub const GET_CAP: &'static str = "getCap";

	/// Creates a new NeoXBridgeContract instance with the default contract hash
	///
	/// # Arguments
	///
	/// * `provider` - An optional reference to an RPC client
	///
	/// # Returns
	///
	/// A Result containing a new NeoXBridgeContract instance or an error
	pub fn new(provider: Option<&'a RpcClient<P>>) -> Result<Self, ContractError> {
		Ok(Self {
			script_hash: ScriptHash::from_str(Self::CONTRACT_HASH).map_err(|e| {
				ContractError::InvalidScriptHash(format!("Invalid contract hash: {}", e))
			})?,
			provider,
		})
	}

	/// Creates a new NeoXBridgeContract instance with a custom script hash
	///
	/// # Arguments
	///
	/// * `script_hash` - The script hash of the Neo X Bridge contract
	/// * `provider` - An optional reference to an RPC client
	///
	/// # Returns
	///
	/// A new NeoXBridgeContract instance
	pub fn with_script_hash(script_hash: ScriptHash, provider: Option<&'a RpcClient<P>>) -> Self {
		Self { script_hash, provider }
	}

	/// Deposits tokens from Neo N3 to Neo X
	///
	/// # Arguments
	///
	/// * `token` - The script hash of the token to deposit (currently only GAS is supported)
	/// * `amount` - The amount of tokens to deposit
	/// * `destination` - The destination address on Neo X
	/// * `account` - The account that will sign the transaction
	///
	/// # Returns
	///
	/// A transaction builder that can be used to build and sign the transaction
	pub async fn deposit(
		&self,
		token: &ScriptHash,
		amount: i64,
		destination: &str,
		account: &Account,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		let params = vec![
			token.into(),
			ContractParameter::integer(amount),
			ContractParameter::string(destination.to_string()),
		];

		let mut builder = self.invoke_function(Self::DEPOSIT, params).await?;
		let _ = builder.set_signers(vec![AccountSigner::called_by_entry(account)
			.map_err(|e| {
				ContractError::InvalidAccount(format!("Failed to create account signer: {}", e))
			})?
			.into()]);

		Ok(builder)
	}

	/// Withdraws tokens from Neo X to Neo N3
	///
	/// # Arguments
	///
	/// * `token` - The script hash of the token to withdraw (currently only GAS is supported)
	/// * `amount` - The amount of tokens to withdraw
	/// * `destination` - The destination address on Neo N3
	/// * `account` - The account that will sign the transaction
	///
	/// # Returns
	///
	/// A transaction builder that can be used to build and sign the transaction
	pub async fn withdraw(
		&self,
		token: &ScriptHash,
		amount: i64,
		destination: &str,
		account: &Account,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		let params = vec![
			token.into(),
			ContractParameter::integer(amount),
			ContractParameter::string(destination.to_string()),
		];

		let mut builder = self.invoke_function(Self::WITHDRAW, params).await?;
		let _ = builder.set_signers(vec![AccountSigner::called_by_entry(account)
			.map_err(|e| {
				ContractError::InvalidAccount(format!("Failed to create account signer: {}", e))
			})?
			.into()]);

		Ok(builder)
	}

	/// Gets the bridge fee for a specific token
	///
	/// # Arguments
	///
	/// * `token` - The script hash of the token to get the fee for
	///
	/// # Returns
	///
	/// The bridge fee as a u64
	pub async fn get_fee(&self, token: &ScriptHash) -> Result<u64, ContractError> {
		let result = self.call_function_returning_int(Self::GET_FEE, vec![token.into()]).await?;
		Ok(result as u64)
	}

	/// Gets the bridge cap for a specific token
	///
	/// # Arguments
	///
	/// * `token` - The script hash of the token to get the cap for
	///
	/// # Returns
	///
	/// The bridge cap as a u64
	pub async fn get_cap(&self, token: &ScriptHash) -> Result<u64, ContractError> {
		let result = self.call_function_returning_int(Self::GET_CAP, vec![token.into()]).await?;
		Ok(result as u64)
	}
}

#[async_trait]
impl<'a, P: JsonRpcProvider> SmartContractTrait<'a> for NeoXBridgeContract<'a, P> {
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
