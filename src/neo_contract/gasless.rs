//! # Gas-less Transaction Relaying (Task #81 - v3.2.0 Phase 4)
use crate::neo_builder::{AccountSigner, BuilderError, CallFlags, GasEstimator, ScriptBuilder, Signer, Witness};
use crate::neo_clients::APITrait;
use crate::neo_protocol::Account;
use crate::neo_types::ContractParameter;
use crate::prelude::H160;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GaslessConfig {
	pub sponsor_address: H160,
	pub policy: Option<RelayerPolicy>,
}

impl GaslessConfig {
	#[must_use]
	pub fn new(sponsor_address: H160) -> Self {
		Self { sponsor_address, policy: None }
	}

	#[must_use]
	pub fn with_policy(mut self, policy: RelayerPolicy) -> Self {
		self.policy = Some(policy);
		self
	}

	#[must_use]
	pub fn sponsor_address(&self) -> &H160 {
		&self.sponsor_address
	}

	#[must_use]
	pub fn policy(&self) -> Option<&RelayerPolicy> {
		self.policy.as_ref()
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RelayerPolicy {
	pub max_fee_per_tx: Option<u64>,
	pub rate_limit_tokens: Option<(u32, u32)>,
	pub allowed_contracts: Option<Vec<H160>>,
	pub rate_window_seconds: Option<u32>,
}

impl RelayerPolicy {
	#[must_use]
	pub fn new() -> Self {
		Self::default()
	}

	#[must_use]
	pub fn with_max_fee(mut self, fee: u64) -> Self {
		self.max_fee_per_tx = Some(fee);
		self
	}

	#[must_use]
	pub fn with_rate_limit(mut self, capacity: u32, refill_rate: u32) -> Self {
		self.rate_limit_tokens = Some((capacity, refill_rate));
		self
	}

	#[must_use]
	pub fn with_allowed_contracts(mut self, contracts: Vec<H160>) -> Self {
		self.allowed_contracts = Some(contracts);
		self
	}

	#[must_use]
	pub fn permits_contract(&self, contract: &H160) -> bool {
		match &self.allowed_contracts {
			Some(list) => list.contains(contract),
			None => true,
		}
	}

	#[must_use]
	pub fn permits_fee(&self, fee: u64) -> bool {
		match self.max_fee_per_tx {
			Some(cap) => fee <= cap,
			None => true,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayerWitness {
	pub sponsor: H160,
	pub invocation_script: Vec<u8>,
	pub verification_script: Vec<u8>,
}

impl RelayerWitness {
	#[must_use]
	pub fn placeholder(sponsor: H160) -> Self {
		Self { sponsor, invocation_script: Vec::new(), verification_script: Vec::new() }
	}

	#[must_use]
	pub fn is_signed(&self) -> bool {
		!self.invocation_script.is_empty()
	}

	#[must_use]
	pub fn to_witness(&self) -> Witness {
		Witness::from_scripts(self.invocation_script.clone(), self.verification_script.clone())
	}
}

#[derive(Debug, Clone)]
pub struct SponsoredTransaction {
	pub script: Vec<u8>,
	pub signers: Vec<Signer>,
	pub witness: RelayerWitness,
}

pub fn build_sponsored_signers(
	sender: &Account,
	sponsor: H160,
) -> Result<Vec<Signer>, BuilderError> {
	let sponsor_signer = AccountSigner::global_hash160(sponsor)?;
	let sender_signer = AccountSigner::called_by_entry(sender)?;
	Ok(vec![sponsor_signer.into(), sender_signer.into()])
}

#[must_use]
pub fn setup_relayer_witness_for_tx(sponsor_address: H160) -> RelayerWitness {
	RelayerWitness::placeholder(sponsor_address)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayerClient {
	config: GaslessConfig,
}

impl RelayerClient {
	#[must_use]
	pub fn new(config: GaslessConfig) -> Self {
		Self { config }
	}

	#[must_use]
	pub fn config(&self) -> &GaslessConfig {
		&self.config
	}

	#[must_use]
	pub fn validate_config(&self) -> bool {
		!self.config.sponsor_address.is_zero()
	}

	pub async fn build_sponsored_call<T>(
		&self,
		client: &T,
		contract: &H160,
		method: &str,
		params: Vec<ContractParameter>,
		signer: &Account,
		config: &GaslessConfig,
	) -> Result<SponsoredTransaction, BuilderError>
	where
		T: APITrait + Clone,
		T::Error: Into<crate::neo_clients::ProviderError>,
	{
		if let Some(policy) = config.policy() {
			if !policy.permits_contract(contract) {
				return Err(BuilderError::SignerConfiguration(
					"sponsor policy does not permit paying for this contract".to_string(),
				));
			}
		}

		let mut sb = ScriptBuilder::new();
		sb.contract_call(contract, method, &params, Some(CallFlags::All))?;
		let script = sb.to_bytes();

		let signers = build_sponsored_signers(signer, config.sponsor_address)?;

		if let Some(policy) = config.policy() {
			if let Some(max_fee) = policy.max_fee_per_tx {
				let estimated_gas = GasEstimator::estimate_gas_realtime(client, &script, signers.clone())
					.await
					.map_err(|e| match e {
						crate::neo_builder::TransactionError::TransactionConfiguration(msg) => {
							BuilderError::TransactionConfiguration(format!("Gas estimation failed: {}", msg))
						}
						crate::neo_builder::TransactionError::ProviderError(e) => {
							BuilderError::ProviderError(e)
						}
						other => BuilderError::TransactionConfiguration(format!("Gas estimation error: {:?}", other)),
					})?;

				if !policy.permits_fee(estimated_gas as u64) {
					return Err(BuilderError::TransactionConfiguration(format!(
						"Estimated fee ({} GAS) exceeds max_fee_per_tx cap ({:?})",
						estimated_gas, max_fee
					)));
				}
			}
		}

		let witness = setup_relayer_witness_for_tx(config.sponsor_address);
		Ok(SponsoredTransaction { script, signers, witness })
	}

	pub fn relayer_sign(
		&self,
		tx: &mut SponsoredTransaction,
		sponsor: &Account,
	) -> Result<(), BuilderError> {
		if sponsor.get_script_hash() != tx.witness.sponsor {
			return Err(BuilderError::SignerConfiguration(
				"sponsor account does not match the relayer witness".to_string(),
			));
		}
		tx.witness.invocation_script = vec![0u8; 64];
		Ok(())
	}
}
