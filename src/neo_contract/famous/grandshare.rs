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

/// GrandShare contract interface for Neo N3
///
/// GrandShare is a governance and funding platform for Neo ecosystem projects.
/// This contract interface provides methods to interact with the GrandShare smart contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrandShareContract<'a, P: JsonRpcProvider> {
	#[serde(deserialize_with = "deserialize_script_hash")]
	#[serde(serialize_with = "serialize_script_hash")]
	script_hash: ScriptHash,
	#[serde(skip)]
	provider: Option<&'a RpcClient<P>>,
}

impl<'a, P: JsonRpcProvider + 'static> GrandShareContract<'a, P> {
	/// The script hash of the GrandShare contract on Neo N3 MainNet
	pub const CONTRACT_HASH: &'static str = "74f2dc36a68fdc4682034178eb2220729231db76";

	// Method constants
	/// Method name for submitting a proposal
	pub const SUBMIT_PROPOSAL: &'static str = "submitProposal";
	/// Method name for voting on a proposal
	pub const VOTE: &'static str = "vote";
	/// Method name for funding a project
	pub const FUND_PROJECT: &'static str = "fundProject";
	/// Method name for claiming funds
	pub const CLAIM_FUNDS: &'static str = "claimFunds";

	/// Creates a new GrandShareContract instance with the default contract hash
	///
	/// # Arguments
	///
	/// * `provider` - An optional reference to an RPC client
	///
	/// # Returns
	///
	/// A new GrandShareContract instance
	pub fn new(provider: Option<&'a RpcClient<P>>) -> Self {
		Self { script_hash: ScriptHash::from_str(Self::CONTRACT_HASH).unwrap(), provider }
	}

	/// Creates a new GrandShareContract instance with a custom script hash
	///
	/// # Arguments
	///
	/// * `script_hash` - The script hash of the GrandShare contract
	/// * `provider` - An optional reference to an RPC client
	///
	/// # Returns
	///
	/// A new GrandShareContract instance
	pub fn with_script_hash(script_hash: ScriptHash, provider: Option<&'a RpcClient<P>>) -> Self {
		Self { script_hash, provider }
	}

	/// Submits a proposal to GrandShare
	///
	/// # Arguments
	///
	/// * `title` - The title of the proposal
	/// * `description` - The description of the proposal
	/// * `requested_amount` - The amount of funds requested
	/// * `account` - The account that will sign the transaction
	///
	/// # Returns
	///
	/// A transaction builder that can be used to build and sign the transaction
	pub async fn submit_proposal(
		&self,
		title: &str,
		description: &str,
		requested_amount: i64,
		account: &Account,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		let params =
			vec![title.into(), description.into(), ContractParameter::integer(requested_amount)];

		let mut builder = self.invoke_function(Self::SUBMIT_PROPOSAL, params).await?;
		builder
			.set_signers(vec![AccountSigner::called_by_entry(account).unwrap().into()])
			.map_err(|err| ContractError::RuntimeError(err.to_string()))?;

		Ok(builder)
	}

	/// Votes on a proposal in GrandShare
	///
	/// # Arguments
	///
	/// * `proposal_id` - The ID of the proposal to vote on
	/// * `vote_type` - The type of vote (true for yes, false for no)
	/// * `account` - The account that will sign the transaction
	///
	/// # Returns
	///
	/// A transaction builder that can be used to build and sign the transaction
	pub async fn vote(
		&self,
		proposal_id: i32,
		vote_type: bool,
		account: &Account,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		let params = vec![
			ContractParameter::integer(proposal_id.into()),
			ContractParameter::bool(vote_type),
		];

		let mut builder = self.invoke_function(Self::VOTE, params).await?;
		builder
			.set_signers(vec![AccountSigner::called_by_entry(account).unwrap().into()])
			.map_err(|err| ContractError::RuntimeError(err.to_string()))?;

		Ok(builder)
	}

	/// Funds a project in GrandShare
	///
	/// # Arguments
	///
	/// * `project_id` - The ID of the project to fund
	/// * `amount` - The amount of funds to provide
	/// * `account` - The account that will sign the transaction
	///
	/// # Returns
	///
	/// A transaction builder that can be used to build and sign the transaction
	pub async fn fund_project(
		&self,
		project_id: i32,
		amount: i64,
		account: &Account,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		let params =
			vec![ContractParameter::integer(project_id.into()), ContractParameter::integer(amount)];

		let mut builder = self.invoke_function(Self::FUND_PROJECT, params).await?;
		builder
			.set_signers(vec![AccountSigner::called_by_entry(account).unwrap().into()])
			.map_err(|err| ContractError::RuntimeError(err.to_string()))?;

		Ok(builder)
	}

	/// Claims funds from a funded project in GrandShare
	///
	/// # Arguments
	///
	/// * `project_id` - The ID of the project to claim funds from
	/// * `account` - The account that will sign the transaction
	///
	/// # Returns
	///
	/// A transaction builder that can be used to build and sign the transaction
	pub async fn claim_funds(
		&self,
		project_id: i32,
		account: &Account,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		let params = vec![ContractParameter::integer(project_id.into())];

		let mut builder = self.invoke_function(Self::CLAIM_FUNDS, params).await?;
		builder
			.set_signers(vec![AccountSigner::called_by_entry(account).unwrap().into()])
			.map_err(|err| ContractError::RuntimeError(err.to_string()))?;

		Ok(builder)
	}
}

#[async_trait]
impl<'a, P: JsonRpcProvider> SmartContractTrait<'a> for GrandShareContract<'a, P> {
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
