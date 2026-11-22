use async_trait::async_trait;
use primitive_types::H160;
use serde::{Deserialize, Serialize};

use crate::{
	neo_builder::TransactionBuilder,
	neo_clients::{JsonRpcProvider, RpcClient},
	neo_contract::{
		ContractError,
		traits::{FungibleTokenTrait, SmartContractTrait, TokenTrait},
	},
	neo_crypto::Secp256r1PublicKey,
	neo_protocol::Account,
	neo_types::{
		ContractParameter, ContractParameterType, NNSName, ScriptHash, StackItem,
		serde_with_utils::{deserialize_script_hash, serialize_script_hash},
	},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeoToken<'a, P: JsonRpcProvider> {
	#[serde(deserialize_with = "deserialize_script_hash")]
	#[serde(serialize_with = "serialize_script_hash")]
	script_hash: ScriptHash,
	#[serde(skip_serializing_if = "Option::is_none")]
	total_supply: Option<u64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	decimals: Option<u8>,
	symbol: Option<String>,
	#[serde(skip)]
	provider: Option<&'a RpcClient<P>>,
}

impl<'a, P: JsonRpcProvider + 'static> NeoToken<'a, P> {
	pub const NAME: &'static str = "NeoToken";
	// pub const SCRIPT_HASH: H160 = Self::calc_native_contract_hash(Self::NAME).unwrap();
	pub const DECIMALS: u8 = 0;
	pub const SYMBOL: &'static str = "NEO";
	pub const TOTAL_SUPPLY: u64 = 100_000_000;

	pub(crate) fn new(provider: Option<&'a RpcClient<P>>) -> Self {
		Self {
			script_hash: Self::calc_native_contract_hash_unchecked(Self::NAME),
			total_supply: Some(Self::TOTAL_SUPPLY),
			decimals: Some(Self::DECIMALS),
			symbol: Some(Self::SYMBOL.to_string()),
			provider,
		}
	}

	// Unclaimed Gas

	pub async fn unclaimed_gas(
		&self,
		account: &Account,
		block_height: u32,
	) -> Result<i64, ContractError> {
		self.unclaimed_gas_contract(&account.get_script_hash(), block_height).await
	}

	pub async fn unclaimed_gas_contract(
		&self,
		script_hash: &H160,
		block_height: u32,
	) -> Result<i64, ContractError> {
		Ok(self
			.call_function_returning_int(
				"unclaimedGas",
				vec![script_hash.into(), block_height.into()],
			)
			.await? as i64)
	}

	// Candidate Registration

	pub async fn register_candidate(
		&self,
		candidate_key: &Secp256r1PublicKey,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		self.invoke_function("registerCandidate", vec![candidate_key.into()]).await
	}

	pub async fn unregister_candidate(
		&self,
		candidate_key: &Secp256r1PublicKey,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		self.invoke_function("unregisterCandidate", vec![candidate_key.into()]).await
	}

	// Committee and Candidates Information

	pub async fn get_committee(&self) -> Result<Vec<Secp256r1PublicKey>, ContractError> {
		self.call_function_returning_list_of_public_keys("getCommittee").await
	}

	pub async fn get_candidates(&self) -> Result<Vec<Candidate>, ContractError> {
		let candidates = self.call_invoke_function("getCandidates", vec![], vec![]).await?;
		self.throw_if_fault_state(&candidates)?;

		let item = candidates
			.get_first_stack_item()
			.map_err(|e| ContractError::InvalidResponse(e.to_string()))?;

		let StackItem::Array { value: array } = item else {
			return Err(ContractError::UnexpectedReturnType("Candidates".to_string()));
		};

		array
			.chunks(2)
			.filter_map(
				|chunk| {
					if chunk.len() == 2 { Some(Candidate::from(chunk.to_vec())) } else { None }
				},
			)
			.collect::<Result<Vec<Candidate>, ContractError>>()
	}

	pub async fn is_candidate(
		&self,
		public_key: &Secp256r1PublicKey,
	) -> Result<bool, ContractError> {
		Ok(self.get_candidates().await?.into_iter().any(|c| c.public_key == *public_key))
	}

	// Voting

	pub async fn vote(
		&self,
		voter: &H160,
		candidate: Option<&Secp256r1PublicKey>,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		let params = match candidate {
			Some(key) => vec![voter.into(), key.into()],
			None => vec![voter.into(), ContractParameter::new(ContractParameterType::Any)],
		};

		self.invoke_function("vote", params).await
	}

	pub async fn cancel_vote(
		&self,
		voter: &H160,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		self.vote(voter, None).await
	}

	pub async fn build_vote_script(
		&self,
		voter: &H160,
		candidate: Option<&Secp256r1PublicKey>,
	) -> Result<Vec<u8>, ContractError> {
		let params = match candidate {
			Some(key) => vec![voter.into(), key.into()],
			None => vec![voter.into(), ContractParameter::new(ContractParameterType::Any)],
		};

		self.build_invoke_function_script("vote", params).await
	}

	// Network Settings

	pub async fn get_gas_per_block(&self) -> Result<i64, ContractError> {
		self.call_function_returning_int("getGasPerBlock", vec![]).await
	}

	pub async fn set_gas_per_block(
		&self,
		gas_per_block: i64,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		self.invoke_function("setGasPerBlock", vec![gas_per_block.into()]).await
	}

	pub async fn get_register_price(&self) -> Result<i64, ContractError> {
		self.call_function_returning_int("getRegisterPrice", vec![]).await
	}

	pub async fn set_register_price(
		&self,
		register_price: i64,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		self.invoke_function("setRegisterPrice", vec![register_price.into()]).await
	}

	/// Gets the committee multi-sig address.
	///
	/// This method returns the script hash of the committee's multi-signature account.
	/// Available after HF_Cockatrice.
	///
	/// # Returns
	///
	/// The script hash of the committee's multi-signature address.
	pub async fn get_committee_address(&self) -> Result<H160, ContractError> {
		let result = self.call_invoke_function("getCommitteeAddress", vec![], vec![]).await?;
		self.throw_if_fault_state(&result)?;

		let item = result
			.get_first_stack_item()
			.map_err(|e| ContractError::InvalidResponse(e.to_string()))?;

		item.as_bytes()
			.filter(|bytes| bytes.len() == 20)
			.map(|bytes| H160::from_slice(&bytes))
			.ok_or_else(|| ContractError::InvalidResponse("Invalid committee address".to_string()))
	}

	/// Gets the votes for a specific candidate.
	///
	/// # Arguments
	///
	/// * `pubkey` - The public key of the candidate.
	///
	/// # Returns
	///
	/// The number of votes for the candidate, or -1 if not found.
	pub async fn get_candidate_vote(
		&self,
		pubkey: &Secp256r1PublicKey,
	) -> Result<i64, ContractError> {
		Ok(self
			.call_function_returning_int("getCandidateVote", vec![pubkey.into()])
			.await? as i64)
	}

	pub async fn get_account_state(&self, account: &H160) -> Result<AccountState, ContractError> {
		let result = self
			.call_invoke_function("getAccountState", vec![account.into()], vec![])
			.await?;
		self.throw_if_fault_state(&result)?;

		let result = result
			.get_first_stack_item()
			.map_err(|e| ContractError::InvalidResponse(e.to_string()))?;

		match result {
			StackItem::Any => Ok(AccountState::with_no_balance()),
			StackItem::Array { value: items } if items.len() >= 3 => {
				let balance = items[0].as_int().ok_or_else(|| {
					ContractError::InvalidResponse(
						"Account state balance is not an integer".to_string(),
					)
				})?;
				let update_height = items[1].as_int();
				// Neo 3.9+ includes last_gas_per_vote as the 4th element
				let last_gas_per_vote = if items.len() >= 4 { items[3].as_int() } else { None };

				if matches!(items[2], StackItem::Any) {
					Ok(AccountState {
						balance,
						balance_height: update_height,
						public_key: None,
						last_gas_per_vote,
					})
				} else {
					let bytes = items[2].as_bytes().ok_or_else(|| {
						ContractError::InvalidResponse(
							"Account state public key is not a byte array".to_string(),
						)
					})?;
					let pubkey = Secp256r1PublicKey::from_bytes(&bytes).map_err(|_| {
						ContractError::InvalidResponse(
							"Account state public key is invalid".to_string(),
						)
					})?;
					Ok(AccountState {
						balance,
						balance_height: update_height,
						public_key: Some(pubkey),
						last_gas_per_vote,
					})
				}
			},
			_ => Err(ContractError::InvalidNeoName("Account state malformed".to_string())),
		}
	}

	pub async fn call_function_returning_list_of_public_keys(
		&self,
		function: &str,
	) -> Result<Vec<Secp256r1PublicKey>, ContractError> {
		let result = self.call_invoke_function(function, vec![], vec![]).await?;
		self.throw_if_fault_state(&result)?;

		let stack_item = result
			.get_first_stack_item()
			.map_err(|e| ContractError::InvalidResponse(e.to_string()))?;

		let StackItem::Array { value: array } = stack_item else {
			return Err(ContractError::UnexpectedReturnType("Expected Array".to_string()));
		};

		array
			.iter()
			.map(|item| {
				item.as_public_key().ok_or_else(|| {
					ContractError::InvalidResponse(format!(
						"Invalid public key stack item: {item:?}"
					))
				})
			})
			.collect::<Result<Vec<Secp256r1PublicKey>, ContractError>>()
	}

	#[allow(dead_code)]
	async fn resolve_nns_text_record(&self, _name: &NNSName) -> Result<H160, ContractError> {
		// NEO token doesn't support NNS text record resolution
		// Return an error indicating this operation is not supported
		Err(ContractError::UnsupportedOperation(
			"NNS text record resolution is not supported for NEO token".to_string(),
		))
	}
}

#[async_trait]
impl<'a, P: JsonRpcProvider> TokenTrait<'a, P> for NeoToken<'a, P> {
	fn total_supply(&self) -> Option<u64> {
		self.total_supply
	}

	fn set_total_supply(&mut self, total_supply: u64) {
		self.total_supply = Some(total_supply)
	}

	fn decimals(&self) -> Option<u8> {
		self.decimals
	}

	fn set_decimals(&mut self, decimals: u8) {
		self.decimals = Some(decimals)
	}

	fn symbol(&self) -> Option<String> {
		self.symbol.clone()
	}

	fn set_symbol(&mut self, symbol: String) {
		self.symbol = Some(symbol)
	}

	async fn resolve_nns_text_record(&self, _name: &NNSName) -> Result<H160, ContractError> {
		// NEO token doesn't support NNS text record resolution
		// Return an error indicating this operation is not supported
		Err(ContractError::UnsupportedOperation(
			"NNS text record resolution is not supported for NEO token".to_string(),
		))
	}
}

#[async_trait]
impl<'a, P: JsonRpcProvider> SmartContractTrait<'a> for NeoToken<'a, P> {
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
impl<'a, P: JsonRpcProvider> FungibleTokenTrait<'a, P> for NeoToken<'a, P> {}

pub struct Candidate {
	pub public_key: Secp256r1PublicKey,
	pub votes: i64,
}

impl Candidate {
	fn from(items: Vec<StackItem>) -> Result<Self, ContractError> {
		let key = items.first().and_then(StackItem::as_public_key).ok_or_else(|| {
			ContractError::InvalidResponse("Candidate public key is missing or invalid".to_string())
		})?;
		let votes = items.get(1).and_then(StackItem::as_int).ok_or_else(|| {
			ContractError::InvalidResponse("Candidate votes is missing or invalid".to_string())
		})?;
		Ok(Self { public_key: key, votes })
	}
}

/// Represents the account state for NEO token holders.
///
/// The account state tracks the NEO balance, the block height when the balance was last updated,
/// the voting target (if any), and the last gas per vote value for reward calculations.
pub struct AccountState {
	/// The NEO balance of the account.
	pub balance: i64,
	/// The block height when the balance was last changed.
	pub balance_height: Option<i64>,
	/// The public key of the candidate the account is voting for (if any).
	pub public_key: Option<Secp256r1PublicKey>,
	/// The last calculated gas per vote value (used for reward calculations).
	/// This field was added in Neo 3.9 for improved voting reward tracking.
	pub last_gas_per_vote: Option<i64>,
}

impl AccountState {
	pub fn with_no_balance() -> Self {
		Self { balance: 0, balance_height: None, public_key: None, last_gas_per_vote: None }
	}
}
