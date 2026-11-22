#![allow(
	clippy::from_over_into,
	clippy::to_string_in_format_args,
	clippy::unnecessary_to_owned,
	clippy::clone_on_copy
)]

use crate::{
	builder::{
		AccountSigner, BuilderError, ContractSigner, TransactionError, TransactionSigner,
		WitnessCondition, WitnessRule, WitnessScope,
	},
	codec::{Decoder, Encoder, NeoSerializable},
	config::NeoConstants,
	crypto::Secp256r1PublicKey,
};
use primitive_types::H160;
use serde::{Deserialize, Serialize, Serializer};
use std::hash::{Hash, Hasher};

/// Represents the type of signer in the NEO blockchain.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SignerType {
	AccountSigner,
	ContractSigner,
	TransactionSigner,
}

/// A trait for common signer operations in the NEO blockchain.
pub trait SignerTrait {
	fn get_type(&self) -> SignerType;

	fn get_signer_hash(&self) -> &H160;

	fn set_signer_hash(&mut self, signer_hash: H160);

	fn get_scopes(&self) -> &Vec<WitnessScope>;
	fn get_scopes_mut(&mut self) -> &mut Vec<WitnessScope>;

	fn set_scopes(&mut self, scopes: Vec<WitnessScope>);

	fn get_allowed_contracts(&self) -> &Vec<H160>;

	fn get_allowed_contracts_mut(&mut self) -> &mut Vec<H160>;

	// fn set_allowed_contracts(&mut self, allowed_contracts: Vec<H160>);

	fn get_allowed_groups(&self) -> &Vec<Secp256r1PublicKey>;
	fn get_allowed_groups_mut(&mut self) -> &mut Vec<Secp256r1PublicKey>;

	fn get_rules(&self) -> &Vec<WitnessRule>;
	fn get_rules_mut(&mut self) -> &mut Vec<WitnessRule>;

	// Set allowed contracts
	fn set_allowed_contracts(&mut self, contracts: Vec<H160>) -> Result<(), BuilderError> {
		// Validate
		if self.get_scopes().contains(&WitnessScope::Global) {
			return Err(BuilderError::SignerConfiguration(
				"Trying to set allowed contracts on a Signer with global scope.".to_string(),
			));
		}

		if self.get_allowed_contracts().len() + contracts.len()
			> NeoConstants::MAX_SIGNER_SUBITEMS as usize
		{
			return Err(BuilderError::SignerConfiguration(format!(
				"Trying to set more than {} allowed contracts on a signer.",
				NeoConstants::MAX_SIGNER_SUBITEMS
			)));
		}

		// Update state
		// if !self.get_scopes().contains(&WitnessScope::CustomContracts) {
		// 	if self.get_scopes().contains(&WitnessScope::None) {
		// 		self.set_scopes(vec![WitnessScope::CustomContracts]);
		// 	} else {
		// 		self.get_scopes_mut().push(WitnessScope::CustomContracts);
		// 	}
		// }
		// Remove WitnessScope::None if it is present
		self.get_scopes_mut().retain(|scope| *scope != WitnessScope::None);

		// Add WitnessScope::CustomContracts if it is not already present
		if !self.get_scopes().contains(&WitnessScope::CustomContracts) {
			self.get_scopes_mut().push(WitnessScope::CustomContracts);
		}

		self.get_allowed_contracts_mut().extend(contracts);

		Ok(())
	}

	// Set allowed groups
	fn set_allowed_groups(&mut self, groups: Vec<Secp256r1PublicKey>) -> Result<(), BuilderError> {
		if self.get_scopes().contains(&WitnessScope::Global) {
			return Err(BuilderError::SignerConfiguration(
				"Trying to set allowed contract groups on a Signer with global scope.".to_string(),
			));
		}

		if self.get_allowed_groups().len() + groups.len()
			> NeoConstants::MAX_SIGNER_SUBITEMS as usize
		{
			return Err(BuilderError::SignerConfiguration(format!(
				"Trying to set more than {} allowed contract groups on a signer.",
				NeoConstants::MAX_SIGNER_SUBITEMS
			)));
		}

		self.get_scopes_mut().retain(|scope| *scope != WitnessScope::None);

		if !self.get_scopes().contains(&WitnessScope::CustomGroups) {
			self.get_scopes_mut().push(WitnessScope::CustomGroups);
		}

		self.get_allowed_groups_mut().extend(groups);

		Ok(())
	}

	fn set_rules(&mut self, mut rules: Vec<WitnessRule>) -> Result<&mut Self, BuilderError> {
		if !rules.is_empty() {
			if self.get_scopes().contains(&WitnessScope::Global) {
				return Err(BuilderError::SignerConfiguration(
					"Trying to set witness rules on a Signer with global scope.".to_string(),
				));
			}

			if self.get_rules().len() + rules.len() > NeoConstants::MAX_SIGNER_SUBITEMS as usize {
				return Err(BuilderError::SignerConfiguration(format!(
					"Trying to set more than {} allowed witness rules on a signer.",
					NeoConstants::MAX_SIGNER_SUBITEMS
				)));
			}

			for rule in &rules {
				self.check_depth(&rule.condition, WitnessCondition::MAX_NESTING_DEPTH as i8)?;
			}

			if !self.get_scopes().contains(&WitnessScope::WitnessRules) {
				self.get_scopes_mut().push(WitnessScope::WitnessRules);
			}

			self.get_rules_mut().append(&mut rules);
		}

		Ok(self)
	}

	fn check_depth(&self, condition: &WitnessCondition, depth: i8) -> Result<(), BuilderError> {
		if depth < 0 {
			return Err(BuilderError::IllegalState(format!(
				"A maximum nesting depth of {} is allowed for witness conditions",
				WitnessCondition::MAX_NESTING_DEPTH
			))); // ::)
		}

		match condition {
			WitnessCondition::And(conditions) | WitnessCondition::Or(conditions) => {
				for c in conditions {
					self.check_depth(c, depth - 1)?
				}
			},
			_ => (),
		};

		Ok(())
	}

	fn validate_subitems(&self, count: usize, _name: &str) -> Result<(), BuilderError> {
		if count > NeoConstants::MAX_SIGNER_SUBITEMS as usize {
			return Err(BuilderError::TooManySigners("".to_string()));
		}
		Ok(())
	}
}

/// Represents a signer in the NEO blockchain.
///
/// This enum can be either an `AccountSigner`, `ContractSigner`, or `TransactionSigner`.
#[derive(Debug, Clone, Deserialize)]
pub enum Signer {
	AccountSigner(AccountSigner),
	ContractSigner(ContractSigner),
	TransactionSigner(TransactionSigner),
}

impl PartialEq for Signer {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Signer::AccountSigner(left), Signer::AccountSigner(right)) => left == right,
			(Signer::ContractSigner(left), Signer::ContractSigner(right)) => left == right,
			(Signer::TransactionSigner(left), Signer::TransactionSigner(right)) => left == right,
			_ => false,
		}
	}
}

impl SignerTrait for Signer {
	fn get_type(&self) -> SignerType {
		match self {
			Signer::AccountSigner(account_signer) => account_signer.get_type(),
			Signer::ContractSigner(contract_signer) => contract_signer.get_type(),
			Signer::TransactionSigner(transaction_signer) => transaction_signer.get_type(),
		}
	}

	fn get_signer_hash(&self) -> &H160 {
		match self {
			Signer::AccountSigner(account_signer) => account_signer.get_signer_hash(),
			Signer::ContractSigner(contract_signer) => contract_signer.get_signer_hash(),
			Signer::TransactionSigner(transaction_signer) => transaction_signer.get_signer_hash(),
		}
	}

	fn set_signer_hash(&mut self, signer_hash: H160) {
		match self {
			Signer::AccountSigner(account_signer) => account_signer.set_signer_hash(signer_hash),
			Signer::ContractSigner(contract_signer) => contract_signer.set_signer_hash(signer_hash),
			Signer::TransactionSigner(transaction_signer) => {
				transaction_signer.set_signer_hash(signer_hash)
			},
		}
	}

	fn get_scopes(&self) -> &Vec<WitnessScope> {
		match self {
			Signer::AccountSigner(account_signer) => account_signer.get_scopes(),
			Signer::ContractSigner(contract_signer) => contract_signer.get_scopes(),
			Signer::TransactionSigner(transaction_signer) => transaction_signer.get_scopes(),
		}
	}

	fn get_scopes_mut(&mut self) -> &mut Vec<WitnessScope> {
		match self {
			Signer::AccountSigner(account_signer) => account_signer.get_scopes_mut(),
			Signer::ContractSigner(contract_signer) => contract_signer.get_scopes_mut(),
			Signer::TransactionSigner(transaction_signer) => transaction_signer.get_scopes_mut(),
		}
	}

	fn set_scopes(&mut self, scopes: Vec<WitnessScope>) {
		match self {
			Signer::AccountSigner(account_signer) => account_signer.set_scopes(scopes),
			Signer::ContractSigner(contract_signer) => contract_signer.set_scopes(scopes),
			Signer::TransactionSigner(transaction_signer) => transaction_signer.set_scopes(scopes),
		}
	}

	fn get_allowed_contracts(&self) -> &Vec<H160> {
		match self {
			Signer::AccountSigner(account_signer) => account_signer.get_allowed_contracts(),
			Signer::ContractSigner(contract_signer) => contract_signer.get_allowed_contracts(),
			Signer::TransactionSigner(transaction_signer) => {
				transaction_signer.get_allowed_contracts()
			},
		}
	}

	fn get_allowed_contracts_mut(&mut self) -> &mut Vec<H160> {
		match self {
			Signer::AccountSigner(account_signer) => account_signer.get_allowed_contracts_mut(),
			Signer::ContractSigner(contract_signer) => contract_signer.get_allowed_contracts_mut(),
			Signer::TransactionSigner(transaction_signer) => {
				transaction_signer.get_allowed_contracts_mut()
			},
		}
	}

	fn get_allowed_groups(&self) -> &Vec<Secp256r1PublicKey> {
		match self {
			Signer::AccountSigner(account_signer) => account_signer.get_allowed_groups(),
			Signer::ContractSigner(contract_signer) => contract_signer.get_allowed_groups(),
			Signer::TransactionSigner(transaction_signer) => {
				transaction_signer.get_allowed_groups()
			},
		}
	}

	fn get_allowed_groups_mut(&mut self) -> &mut Vec<Secp256r1PublicKey> {
		match self {
			Signer::AccountSigner(account_signer) => account_signer.get_allowed_groups_mut(),
			Signer::ContractSigner(contract_signer) => contract_signer.get_allowed_groups_mut(),
			Signer::TransactionSigner(transaction_signer) => {
				transaction_signer.get_allowed_groups_mut()
			},
		}
	}

	fn get_rules(&self) -> &Vec<WitnessRule> {
		match self {
			Signer::AccountSigner(account_signer) => account_signer.get_rules(),
			Signer::ContractSigner(contract_signer) => contract_signer.get_rules(),
			Signer::TransactionSigner(transaction_signer) => transaction_signer.get_rules(),
		}
	}

	fn get_rules_mut(&mut self) -> &mut Vec<WitnessRule> {
		match self {
			Signer::AccountSigner(account_signer) => account_signer.get_rules_mut(),
			Signer::ContractSigner(contract_signer) => contract_signer.get_rules_mut(),
			Signer::TransactionSigner(transaction_signer) => transaction_signer.get_rules_mut(),
		}
	}
}

fn validate_serializable_subitems(count: usize, item_name: &str) -> Result<(), TransactionError> {
	if count > NeoConstants::MAX_SIGNER_SUBITEMS as usize {
		return Err(TransactionError::TransactionConfiguration(format!(
			"A signer's scope can only contain {} {}. The signer contains {} {}.",
			NeoConstants::MAX_SIGNER_SUBITEMS,
			item_name,
			count,
			item_name
		)));
	}

	Ok(())
}

pub(crate) fn validate_signer_serialization<S: SignerTrait>(
	signer: &S,
) -> Result<(), TransactionError> {
	if signer.get_scopes().contains(&WitnessScope::CustomContracts) {
		validate_serializable_subitems(signer.get_allowed_contracts().len(), "allowed contracts")?;
	}

	if signer.get_scopes().contains(&WitnessScope::CustomGroups) {
		validate_serializable_subitems(
			signer.get_allowed_groups().len(),
			"allowed contract groups",
		)?;
	}

	if signer.get_scopes().contains(&WitnessScope::WitnessRules) {
		validate_serializable_subitems(signer.get_rules().len(), "rules")?;
		for rule in signer.get_rules() {
			signer
				.check_depth(&rule.condition, WitnessCondition::MAX_NESTING_DEPTH as i8)
				.map_err(|err| TransactionError::TransactionConfiguration(err.to_string()))?;
			rule.condition.validate_serialization().map_err(|err| {
				TransactionError::TransactionConfiguration(format!(
					"Invalid witness condition: {}",
					err
				))
			})?;
		}
	}

	Ok(())
}

impl Signer {
	/// Creates a `Signer` from a byte array.
	///
	/// # Arguments
	///
	/// * `data` - The byte array containing the serialized signer data.
	///
	/// # Returns
	///
	/// A `Result` containing the deserialized `Signer` or a `TransactionError`.
	pub fn from_bytes(data: &[u8]) -> Result<Signer, TransactionError> {
		let mut reader = Decoder::new(data);
		Signer::decode(&mut reader)
	}

	/// Returns the type of the signer.
	pub fn get_type(&self) -> SignerType {
		match self {
			Signer::AccountSigner(account_signer) => account_signer.get_type(),
			Signer::ContractSigner(contract_signer) => contract_signer.get_type(),
			Signer::TransactionSigner(transaction_signer) => transaction_signer.get_type(),
		}
	}
	/// Returns a reference to the signer's script hash.
	pub fn get_signer_hash(&self) -> &H160 {
		match self {
			Signer::AccountSigner(account_signer) => account_signer.get_signer_hash(),
			Signer::ContractSigner(contract_signer) => contract_signer.get_signer_hash(),
			Signer::TransactionSigner(transaction_signer) => transaction_signer.get_signer_hash(),
		}
	}

	pub fn as_account_signer(&self) -> Option<&AccountSigner> {
		match self {
			Signer::AccountSigner(account_signer) => Some(account_signer),
			_ => None,
		}
	}

	pub fn as_contract_signer(&self) -> Option<&ContractSigner> {
		match self {
			Signer::ContractSigner(contract_signer) => Some(contract_signer),
			_ => None,
		}
	}

	pub fn as_transaction_signer(&self) -> Option<&TransactionSigner> {
		match self {
			Signer::TransactionSigner(transaction_signer) => Some(transaction_signer),
			_ => None,
		}
	}

	/// Safely converts to AccountSigner
	///
	/// # Errors
	///
	/// Returns `BuilderError::IllegalState` if the signer is not an AccountSigner.
	pub fn to_account_signer(self) -> Result<AccountSigner, BuilderError> {
		match self {
			Signer::AccountSigner(account_signer) => Ok(account_signer),
			Signer::ContractSigner(_) => Err(BuilderError::IllegalState(
				"Cannot convert ContractSigner into AccountSigner".to_string(),
			)),
			Signer::TransactionSigner(_) => Err(BuilderError::IllegalState(
				"Cannot convert TransactionSigner into AccountSigner".to_string(),
			)),
		}
	}

	/// Safely converts to ContractSigner
	///
	/// # Errors
	///
	/// Returns `BuilderError::IllegalState` if the signer is not a ContractSigner.
	pub fn to_contract_signer(self) -> Result<ContractSigner, BuilderError> {
		match self {
			Signer::AccountSigner(_) => Err(BuilderError::IllegalState(
				"Cannot convert AccountSigner into ContractSigner".to_string(),
			)),
			Signer::ContractSigner(contract_signer) => Ok(contract_signer),
			Signer::TransactionSigner(_) => Err(BuilderError::IllegalState(
				"Cannot convert TransactionSigner into ContractSigner".to_string(),
			)),
		}
	}

	/// Safely converts any signer variant into a transaction signer.
	pub fn try_to_transaction_signer(&self) -> Result<TransactionSigner, BuilderError> {
		match self {
			Signer::AccountSigner(account_signer) => try_build_transaction_signer(
				account_signer.account.get_script_hash(),
				account_signer.get_scopes(),
				account_signer.get_allowed_contracts(),
				account_signer.get_allowed_groups(),
				account_signer.get_rules(),
			),
			Signer::ContractSigner(contract_signer) => try_build_transaction_signer(
				*contract_signer.get_signer_hash(),
				contract_signer.get_scopes(),
				contract_signer.get_allowed_contracts(),
				contract_signer.get_allowed_groups(),
				contract_signer.get_rules(),
			),
			Signer::TransactionSigner(transaction_signer) => Ok(transaction_signer.clone()),
		}
	}

	pub fn try_encode(&self, writer: &mut Encoder) -> Result<(), TransactionError> {
		validate_signer_serialization(self)?;
		match self {
			Signer::AccountSigner(account_signer) => account_signer.encode(writer),
			Signer::ContractSigner(contract_signer) => contract_signer.encode(writer),
			Signer::TransactionSigner(transaction_signer) => transaction_signer.encode(writer),
		}
		Ok(())
	}

	pub fn try_to_array(&self) -> Result<Vec<u8>, TransactionError> {
		let mut writer = Encoder::new();
		self.try_encode(&mut writer)?;
		Ok(writer.to_bytes())
	}
}

impl Hash for Signer {
	fn hash<H: Hasher>(&self, state: &mut H) {
		match self {
			Signer::AccountSigner(account_signer) => account_signer.hash(state),
			Signer::ContractSigner(contract_signer) => contract_signer.hash(state),
			Signer::TransactionSigner(transaction_signer) => transaction_signer.hash(state),
		}
	}
}

impl From<AccountSigner> for Signer {
	fn from(account_signer: AccountSigner) -> Self {
		Signer::AccountSigner(account_signer)
	}
}

impl From<ContractSigner> for Signer {
	fn from(contract_signer: ContractSigner) -> Self {
		Signer::ContractSigner(contract_signer)
	}
}

fn try_build_transaction_signer(
	signer_hash: H160,
	scopes: &[WitnessScope],
	allowed_contracts: &[H160],
	allowed_groups: &[Secp256r1PublicKey],
	rules: &[WitnessRule],
) -> Result<TransactionSigner, BuilderError> {
	TransactionSigner::new_full(
		signer_hash,
		scopes.to_vec(),
		allowed_contracts.to_vec(),
		allowed_groups.to_vec(),
		rules.to_vec(),
	)
}

// Keep the existing Into implementations for backward compatibility.
// Note: These will panic if called with the wrong variant. Use the fallible helpers for safe conversion.
impl Into<AccountSigner> for Signer {
	#[track_caller]
	fn into(self) -> AccountSigner {
		match self {
			Signer::AccountSigner(account_signer) => account_signer,
			Signer::ContractSigner(_) => {
				panic!(
					"Cannot convert ContractSigner into AccountSigner. Use Signer::to_account_signer() instead."
				)
			},
			Signer::TransactionSigner(_) => {
				panic!(
					"Cannot convert TransactionSigner into AccountSigner. Use Signer::to_account_signer() instead."
				)
			},
		}
	}
}

impl Into<TransactionSigner> for Signer {
	fn into(self) -> TransactionSigner {
		match self {
			Signer::AccountSigner(account_signer) => try_build_transaction_signer(
				account_signer.account.get_script_hash(),
				account_signer.get_scopes(),
				account_signer.get_allowed_contracts(),
				account_signer.get_allowed_groups(),
				account_signer.get_rules(),
			)
			.expect("Signer already has valid scopes"),
			Signer::ContractSigner(contract_signer) => try_build_transaction_signer(
				*contract_signer.get_signer_hash(),
				contract_signer.get_scopes(),
				contract_signer.get_allowed_contracts(),
				contract_signer.get_allowed_groups(),
				contract_signer.get_rules(),
			)
			.expect("Signer already has valid scopes"),
			Signer::TransactionSigner(transaction_signer) => transaction_signer,
		}
	}
}

impl Into<TransactionSigner> for &Signer {
	fn into(self) -> TransactionSigner {
		self.try_to_transaction_signer().expect("Signer already has valid scopes")
	}
}

impl Into<TransactionSigner> for &mut Signer {
	fn into(self) -> TransactionSigner {
		self.try_to_transaction_signer().expect("Signer already has valid scopes")
	}
}

impl Into<ContractSigner> for &mut Signer {
	#[track_caller]
	fn into(self) -> ContractSigner {
		match self {
			Signer::ContractSigner(contract_signer) => contract_signer.clone(),
			Signer::AccountSigner(_) => {
				panic!(
					"Cannot convert AccountSigner into ContractSigner. Use Signer::to_contract_signer() instead."
				)
			},
			Signer::TransactionSigner(_) => {
				panic!(
					"Cannot convert TransactionSigner into ContractSigner. Use Signer::to_contract_signer() instead."
				)
			},
		}
	}
}

impl Into<ContractSigner> for Signer {
	#[track_caller]
	fn into(self) -> ContractSigner {
		match self {
			Signer::ContractSigner(contract_signer) => contract_signer,
			Signer::AccountSigner(_) => {
				panic!(
					"Cannot convert AccountSigner into ContractSigner. Use Signer::to_contract_signer() instead."
				)
			},
			Signer::TransactionSigner(_) => {
				panic!(
					"Cannot convert TransactionSigner into ContractSigner. Use Signer::to_contract_signer() instead."
				)
			},
		}
	}
}

impl Serialize for Signer {
	fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
	where
		S: Serializer,
	{
		match self {
			Signer::AccountSigner(account_signer) => account_signer.serialize(serializer),
			Signer::ContractSigner(contract_signer) => contract_signer.serialize(serializer),
			Signer::TransactionSigner(transaction_signer) => {
				transaction_signer.serialize(serializer)
			},
		}
	}
}

impl NeoSerializable for Signer {
	type Error = TransactionError;

	fn size(&self) -> usize {
		match self {
			Signer::AccountSigner(account_signer) => account_signer.size(),
			Signer::ContractSigner(contract_signer) => contract_signer.size(),
			Signer::TransactionSigner(transaction_signer) => transaction_signer.size(),
		}
	}

	fn encode(&self, writer: &mut Encoder) {
		match self {
			Signer::AccountSigner(account_signer) => account_signer.encode(writer),
			Signer::ContractSigner(contract_signer) => contract_signer.encode(writer),
			Signer::TransactionSigner(transaction_signer) => transaction_signer.encode(writer),
		}
	}

	fn decode(reader: &mut Decoder) -> Result<Self, Self::Error>
	where
		Self: Sized,
	{
		Ok(Signer::TransactionSigner(TransactionSigner::decode(reader)?))
	}

	fn to_array(&self) -> Vec<u8> {
		match self {
			Signer::AccountSigner(account_signer) => account_signer.to_array(),
			Signer::ContractSigner(contract_signer) => contract_signer.to_array(),
			Signer::TransactionSigner(transaction_signer) => transaction_signer.to_array(),
		}
	}
}

#[cfg(test)]
mod tests {
	use std::{collections::HashSet, ops::Deref};

	use lazy_static::lazy_static;
	use primitive_types::H160;

	use crate::{
		ScriptHash, ScriptHashExtension,
		builder::{
			AccountSigner, BuilderError, ContractSigner, SignerTrait, TransactionError,
			TransactionSigner, WitnessAction, WitnessCondition, WitnessRule, WitnessScope,
		},
		codec::{Encoder, NeoSerializable},
		config::NeoConstants,
		crypto::Secp256r1PublicKey,
		neo_protocol::{Account, AccountTrait},
	};
	use neo3::builder::Signer;

	lazy_static! {
		pub static ref SCRIPT_HASH: ScriptHash = {
			Account::from_wif("Kzt94tAAiZSgH7Yt4i25DW6jJFprZFPSqTgLr5dWmWgKDKCjXMfZ")
				.unwrap()
				.get_script_hash()
		};
		pub static ref SCRIPT_HASH1: H160 = H160::from_script(&hex::decode("d802a401").unwrap());
		pub static ref SCRIPT_HASH2: H160 = H160::from_script(&hex::decode("c503b112").unwrap());
		pub static ref GROUP_PUB_KEY1: Secp256r1PublicKey = Secp256r1PublicKey::from_encoded(
			"0306d3e7f18e6dd477d34ce3cfeca172a877f3c907cc6c2b66c295d1fcc76ff8f7",
		)
		.unwrap();
		pub static ref GROUP_PUB_KEY2: Secp256r1PublicKey = Secp256r1PublicKey::from_encoded(
			"02958ab88e4cea7ae1848047daeb8883daf5fdf5c1301dbbfe973f0a29fe75de60",
		)
		.unwrap();
	}

	#[test]
	fn test_create_signer_with_call_by_entry_scope() {
		let signer = AccountSigner::called_by_entry(&SCRIPT_HASH.deref().into()).unwrap();

		assert_eq!(signer.signer_hash, *SCRIPT_HASH);
		assert_eq!(signer.scopes, vec![WitnessScope::CalledByEntry]);
		assert!(signer.get_allowed_contracts().is_empty());
		assert!(signer.get_allowed_groups().is_empty());
	}

	#[test]
	fn test_create_signer_with_global_scope() {
		let signer = AccountSigner::global(&SCRIPT_HASH.deref().into()).unwrap();

		assert_eq!(signer.signer_hash, *SCRIPT_HASH);
		assert_eq!(signer.scopes, vec![WitnessScope::Global]);
		assert!(signer.get_allowed_contracts().is_empty());
		assert!(signer.get_allowed_groups().is_empty());
	}

	#[test]
	fn test_build_valid_signer1() {
		let mut signer = AccountSigner::called_by_entry(&SCRIPT_HASH.deref().into())
			.expect("Should be able to create AccountSigner with called_by_entry scope in test");
		signer
			.set_allowed_contracts(vec![*SCRIPT_HASH1, *SCRIPT_HASH2])
			.expect("Should be able to set allowed contracts in test");

		assert_eq!(signer.signer_hash, *SCRIPT_HASH);
		assert_eq!(
			signer.get_scopes().iter().cloned().collect::<HashSet<_>>(),
			vec![WitnessScope::CalledByEntry, WitnessScope::CustomContracts]
				.into_iter()
				.collect()
		);
		assert_eq!(
			signer.get_allowed_contracts().iter().cloned().collect::<HashSet<_>>(),
			vec![*SCRIPT_HASH1, *SCRIPT_HASH2].into_iter().collect()
		);
		assert!(signer.get_allowed_groups().is_empty());
	}

	#[test]
	fn test_build_valid_signer2() {
		let mut signer = AccountSigner::none(&SCRIPT_HASH.deref().into())
			.expect("Should be able to create AccountSigner with none scope in test");
		signer
			.set_allowed_contracts(vec![*SCRIPT_HASH1, *SCRIPT_HASH2])
			.expect("Should be able to set allowed contracts in test");

		assert_eq!(signer.signer_hash, *SCRIPT_HASH);
		assert_eq!(signer.get_scopes(), &vec![WitnessScope::CustomContracts]);
		assert_eq!(
			signer.get_allowed_contracts().iter().cloned().collect::<HashSet<_>>(),
			vec![*SCRIPT_HASH1, *SCRIPT_HASH2].into_iter().collect()
		);
		assert!(signer.get_allowed_groups().is_empty());
	}

	#[test]
	fn test_build_valid_signer3() {
		let mut signer = AccountSigner::none(&SCRIPT_HASH.deref().into())
			.expect("Should be able to create AccountSigner with none scope in test");
		signer
			.set_allowed_groups(vec![GROUP_PUB_KEY1.clone(), GROUP_PUB_KEY2.clone()])
			.expect("Should be able to set allowed groups in test");

		assert_eq!(signer.signer_hash, *SCRIPT_HASH);
		assert_eq!(signer.get_scopes(), &vec![WitnessScope::CustomGroups]);
		assert_eq!(
			signer.get_allowed_groups().iter().cloned().collect::<HashSet<_>>(),
			vec![GROUP_PUB_KEY1.clone(), GROUP_PUB_KEY2.clone()].into_iter().collect()
		);
		assert!(signer.get_allowed_contracts().is_empty());
	}

	#[test]
	fn test_fail_building_signer_with_global_scope_and_custom_contracts() {
		let mut signer = AccountSigner::global(&SCRIPT_HASH.deref().into()).unwrap();
		let err = signer.set_allowed_contracts(vec![*SCRIPT_HASH1, *SCRIPT_HASH2]).unwrap_err();

		assert_eq!(
			err,
			BuilderError::SignerConfiguration(
				"Trying to set allowed contracts on a Signer with global scope.".to_string()
			)
		);
	}

	#[test]
	fn test_fail_building_signer_with_global_scope_and_custom_groups() {
		let mut signer = AccountSigner::global(&SCRIPT_HASH.deref().into()).unwrap();
		let err = signer
			.set_allowed_groups(vec![GROUP_PUB_KEY1.clone(), GROUP_PUB_KEY2.clone()])
			.unwrap_err();

		assert_eq!(
			err,
			BuilderError::SignerConfiguration(
				"Trying to set allowed contract groups on a Signer with global scope.".to_string()
			)
		);
	}

	#[test]
	fn test_fail_building_signer_too_many_contracts() {
		let script = H160::from_hex("3ab0be8672e25cf475219d018ded961ec684ca88").unwrap();
		let contracts = (0..=16).map(|_| script.clone()).collect::<Vec<_>>();

		let err = AccountSigner::called_by_entry(&SCRIPT_HASH.deref().into())
			.unwrap()
			.set_allowed_contracts(contracts)
			.unwrap_err();

		assert_eq!(
			err,
			BuilderError::SignerConfiguration(format!(
				"Trying to set more than {} allowed contracts on a signer.",
				NeoConstants::MAX_SIGNER_SUBITEMS
			))
		);
	}

	#[test]
	fn test_fail_building_signer_too_many_contracts_added_separately() {
		let script = H160::from_hex("3ab0be8672e25cf475219d018ded961ec684ca88").unwrap();
		let contracts = (0..=15).map(|_| script.clone()).collect::<Vec<_>>();

		let mut signer = AccountSigner::none(&SCRIPT_HASH.deref().into()).unwrap();
		signer.set_allowed_contracts(vec![script]).expect("");

		let err = signer.set_allowed_contracts(contracts).unwrap_err();

		assert_eq!(
			err,
			BuilderError::SignerConfiguration(format!(
				"Trying to set more than {} allowed contracts on a signer.",
				NeoConstants::MAX_SIGNER_SUBITEMS
			))
		);
	}

	#[test]
	fn test_fail_building_signer_too_many_groups() {
		let public_key = Secp256r1PublicKey::from_encoded(
			"0306d3e7f18e6dd477d34ce3cfeca172a877f3c907cc6c2b66c295d1fcc76ff8f7",
		)
		.unwrap();
		let groups = (0..=16).map(|_| public_key.clone()).collect::<Vec<_>>();

		let err = AccountSigner::called_by_entry(&SCRIPT_HASH.deref().into())
			.unwrap()
			.set_allowed_groups(groups)
			.unwrap_err();

		assert_eq!(
			err,
			BuilderError::SignerConfiguration(format!(
				"Trying to set more than {} allowed contract groups on a signer.",
				NeoConstants::MAX_SIGNER_SUBITEMS
			))
		);
	}

	#[test]
	fn test_fail_building_signer_too_many_groups_added_separately() {
		let public_key = Secp256r1PublicKey::from_encoded(
			"0306d3e7f18e6dd477d34ce3cfeca172a877f3c907cc6c2b66c295d1fcc76ff8f7",
		)
		.unwrap();
		let groups = (0..=15).map(|_| public_key.clone()).collect::<Vec<_>>();

		let mut signer = AccountSigner::none(&SCRIPT_HASH.deref().into()).unwrap();
		signer.set_allowed_groups(vec![public_key]).expect("");

		let err = signer.set_allowed_groups(groups).unwrap_err();

		assert_eq!(
			err,
			BuilderError::SignerConfiguration(format!(
				"Trying to set more than {} allowed contract groups on a signer.",
				NeoConstants::MAX_SIGNER_SUBITEMS
			))
		);
	}

	#[test]
	fn test_account_signer_try_to_array_rejects_too_many_allowed_contracts() {
		let mut signer = AccountSigner::none(&SCRIPT_HASH.deref().into()).unwrap();
		signer.set_scopes(vec![WitnessScope::CustomContracts]);
		signer
			.get_allowed_contracts_mut()
			.extend((0..=NeoConstants::MAX_SIGNER_SUBITEMS).map(|_| H160::zero()));

		assert!(matches!(
			signer.try_to_array(),
			Err(TransactionError::TransactionConfiguration(message))
				if message.contains("allowed contracts")
		));
	}

	#[test]
	fn test_contract_signer_try_to_array_rejects_too_many_allowed_groups() {
		let mut signer = ContractSigner::called_by_entry(*SCRIPT_HASH, &[]);
		signer.set_scopes(vec![WitnessScope::CustomGroups]);
		signer
			.get_allowed_groups_mut()
			.extend((0..=NeoConstants::MAX_SIGNER_SUBITEMS).map(|_| GROUP_PUB_KEY1.clone()));

		assert!(matches!(
			signer.try_to_array(),
			Err(TransactionError::TransactionConfiguration(message))
				if message.contains("allowed contract groups")
		));
	}

	#[test]
	fn test_transaction_signer_try_to_array_rejects_rule_with_too_many_expressions() {
		let mut signer =
			TransactionSigner::new(*SCRIPT_HASH, vec![WitnessScope::WitnessRules]).unwrap();
		signer.get_rules_mut().push(WitnessRule::new(
			WitnessAction::Allow,
			WitnessCondition::And(
				(0..=NeoConstants::MAX_SIGNER_SUBITEMS)
					.map(|_| WitnessCondition::Boolean(true))
					.collect(),
			),
		));

		assert!(matches!(
			signer.try_to_array(),
			Err(TransactionError::TransactionConfiguration(message))
				if message.contains("witness condition")
		));
	}

	#[test]
	fn test_transaction_signer_try_to_array_rejects_too_many_rules() {
		let mut signer =
			TransactionSigner::new(*SCRIPT_HASH, vec![WitnessScope::WitnessRules]).unwrap();
		let rule =
			WitnessRule::new(WitnessAction::Allow, WitnessCondition::ScriptHash(*SCRIPT_HASH));
		signer
			.get_rules_mut()
			.extend((0..=NeoConstants::MAX_SIGNER_SUBITEMS).map(|_| rule.clone()));

		assert!(matches!(
			signer.try_to_array(),
			Err(TransactionError::TransactionConfiguration(message))
				if message.contains("rules")
		));
	}

	#[test]
	fn test_serialize_global_scope() {
		let mut buffer = Encoder::new();

		AccountSigner::global(&SCRIPT_HASH.deref().into()).unwrap().encode(&mut buffer);

		let expected = format!(
			"{}{:02x}",
			hex::encode(SCRIPT_HASH.as_bytes()),
			WitnessScope::Global.byte_repr()
		);
		assert_eq!(hex::encode(buffer.to_bytes()), expected);
	}

	#[test]
	fn test_serialize_custom_contracts_scope_produces_correct_byte_array() {
		let mut signer = AccountSigner::none(&SCRIPT_HASH.deref().into()).unwrap();
		signer.set_allowed_contracts(vec![*SCRIPT_HASH1, *SCRIPT_HASH2]).unwrap();

		let expected = format!(
			"{}{:02x}02{}{}",
			hex::encode(SCRIPT_HASH.as_bytes()),
			WitnessScope::CustomContracts.byte_repr(),
			hex::encode(SCRIPT_HASH1.as_bytes()),
			hex::encode(SCRIPT_HASH2.as_bytes())
		);

		assert_eq!(signer.to_array(), hex::decode(&expected).unwrap());
	}

	#[test]
	fn test_serialize_custom_group_scope() {
		let mut signer = AccountSigner::none(&SCRIPT_HASH.deref().into()).unwrap();
		signer
			.set_allowed_groups(vec![GROUP_PUB_KEY1.clone(), GROUP_PUB_KEY2.clone()])
			.expect("");

		let expected = format!(
			"{}{:02x}02{}{}",
			hex::encode(SCRIPT_HASH.as_bytes()),
			WitnessScope::CustomGroups.byte_repr(),
			GROUP_PUB_KEY1.get_encoded_compressed_hex().trim_start_matches("0x").to_string(),
			GROUP_PUB_KEY2.get_encoded_compressed_hex().trim_start_matches("0x").to_string(),
		);

		assert_eq!(signer.to_array(), hex::decode(&expected).unwrap());
	}

	#[test]
	fn test_serialize_multiple_scopes_contracts_groups_and_rules() {
		let rule = WitnessRule::new(
			WitnessAction::Allow,
			WitnessCondition::CalledByContract(*SCRIPT_HASH1),
		);
		let mut signer = AccountSigner::called_by_entry(&SCRIPT_HASH.deref().into()).unwrap();
		signer
			.set_allowed_groups(vec![GROUP_PUB_KEY1.clone(), GROUP_PUB_KEY2.clone()])
			.expect("");
		signer.set_allowed_contracts(vec![*SCRIPT_HASH1, *SCRIPT_HASH2]).expect("");
		signer.set_rules(vec![rule]).expect("");

		let expected = format!(
			"{}{}{}{}{}{}{}{}{}{}{}{}",
			hex::encode(SCRIPT_HASH.as_bytes()),
			"71",
			"02",
			hex::encode(SCRIPT_HASH1.as_bytes()),
			hex::encode(SCRIPT_HASH2.as_bytes()),
			"02",
			GROUP_PUB_KEY1.get_encoded_compressed_hex().trim_start_matches("0x").to_string(),
			GROUP_PUB_KEY2.get_encoded_compressed_hex().trim_start_matches("0x").to_string(),
			"01",
			"01",
			"28",
			hex::encode(SCRIPT_HASH1.as_bytes())
		);

		assert_eq!(signer.to_array(), hex::decode(&expected).unwrap());
	}

	#[test]
	fn test_fail_deserialize_too_many_contracts() {
		let mut serialized = format!("{}1111", hex::encode(SCRIPT_HASH.as_bytes()));
		for _ in 0..=17 {
			serialized.push_str(&hex::encode(SCRIPT_HASH1.as_bytes()));
		}
		let mut data = hex::decode(&serialized).unwrap();
		data.insert(0, 1);

		let err = Signer::from_bytes(&data).unwrap_err();

		assert!(err.to_string().contains(&format!(
			"A signer's scope can only contain {} allowed contracts.",
			NeoConstants::MAX_SIGNER_SUBITEMS
		)));
	}

	#[test]
	fn test_fail_deserialize_too_many_contract_groups() {
		let mut serialized = format!("{}2111", hex::encode(SCRIPT_HASH.as_bytes()));
		for _ in 0..=17 {
			serialized.push_str(
				&GROUP_PUB_KEY1.get_encoded_compressed_hex().trim_start_matches("0x").to_string(),
			);
		}
		let data = hex::decode(&serialized).unwrap();

		let err = Signer::from_bytes(&data).unwrap_err();

		assert!(err.to_string().contains(&format!(
			"A signer's scope can only contain {} allowed contract groups.",
			NeoConstants::MAX_SIGNER_SUBITEMS
		)));
	}

	#[test]
	fn test_fail_deserialize_too_many_rules() {
		let mut serialized = format!("{}4111", hex::encode(SCRIPT_HASH.as_bytes()));
		for _ in 0..=17 {
			serialized.push_str("01");
			serialized.push_str("28");
			serialized.push_str(&hex::encode(SCRIPT_HASH1.as_bytes()));
		}
		let data = hex::decode(&serialized).unwrap();

		let err = Signer::from_bytes(&data).unwrap_err();

		assert!(err.to_string().contains(&format!(
			"A signer's scope can only contain {} rules.",
			NeoConstants::MAX_SIGNER_SUBITEMS
		)));
	}

	#[test]
	fn test_get_size() {
		let rule = WitnessRule::new(
			WitnessAction::Allow,
			WitnessCondition::And(vec![
				WitnessCondition::Boolean(true),
				WitnessCondition::Boolean(false),
			]),
		);
		let mut signer = AccountSigner::called_by_entry(&SCRIPT_HASH.deref().into()).unwrap();
		signer
			.set_allowed_groups(vec![GROUP_PUB_KEY1.clone(), GROUP_PUB_KEY2.clone()])
			.expect("");
		signer.set_allowed_contracts(vec![*SCRIPT_HASH1, *SCRIPT_HASH2]).expect("");
		signer.set_rules(vec![rule.clone(), rule]).expect("");

		let expected_size = 20 // Account script hash
            + 1 // Scope byte
            + 1 // length byte of allowed contracts list
            + 20 + 20 // Script hashes of two allowed contracts
            + 1 // length byte of allowed groups list
            + 33 + 33 // Public keys of two allowed groups
            + 1 // length byte of rules list
            + 1 // byte for WitnessRuleAction Allow
            + 1 // byte for WitnessCondition type (AndCondition)
            + 1 // length of AND condition list
            + 1 // byte for WitnessCondition type (BooleanCondition)
            + 1 // byte for value of BooleanCondition
            + 1 // byte for WitnessCondition type (BooleanCondition)
            + 1 // byte for value of BooleanCondition
            + 1 // byte for WitnessRuleAction Allow
            + 1 // byte for WitnessCondition type (AndCondition)
            + 1 // length of AND condition list
            + 1 // byte for WitnessCondition type (BooleanCondition)
            + 1 // byte for value of BooleanCondition
            + 1 // byte for WitnessCondition type (BooleanCondition)
            + 1; // byte for value of BooleanCondition

		assert_eq!(signer.size(), expected_size);
	}

	#[test]
	fn test_serialize_deserialize_max_nested_rules() {
		let rule = WitnessRule::new(
			WitnessAction::Allow,
			WitnessCondition::And(vec![WitnessCondition::And(vec![WitnessCondition::Boolean(
				true,
			)])]),
		);

		let mut buffer = Encoder::new();
		let mut account_signer = AccountSigner::none(&ScriptHash::zero().into()).unwrap();

		account_signer.set_rules(vec![rule]).unwrap();
		account_signer.encode(&mut buffer);

		let expected =
			hex::decode("0000000000000000000000000000000000000000400101020102010001").unwrap();
		assert_eq!(buffer.to_bytes(), expected);
	}

	#[test]
	fn test_fail_adding_rules_to_global_signer() {
		let rule =
			WitnessRule::new(WitnessAction::Allow, WitnessCondition::ScriptHash(*SCRIPT_HASH));

		let mut signer = AccountSigner::global(&SCRIPT_HASH.deref().into()).unwrap();

		let err = signer.set_rules(vec![rule]).unwrap_err();

		assert_eq!(
			err,
			BuilderError::SignerConfiguration(
				"Trying to set witness rules on a Signer with global scope.".to_string()
			)
		);
	}

	#[test]
	fn test_fail_adding_too_many_rules() {
		let rule =
			WitnessRule::new(WitnessAction::Allow, WitnessCondition::ScriptHash(*SCRIPT_HASH));

		let mut signer = AccountSigner::none(&SCRIPT_HASH.deref().into()).unwrap();

		for _ in 0..NeoConstants::MAX_SIGNER_SUBITEMS {
			signer.set_rules(vec![rule.clone()]).unwrap();
		}

		let err = signer.set_rules(vec![rule]).unwrap_err();

		assert_eq!(
			err,
			BuilderError::SignerConfiguration(format!(
				"Trying to set more than {} allowed witness rules on a signer.",
				NeoConstants::MAX_SIGNER_SUBITEMS
			))
		);
	}

	#[test]
	fn test_signer_equals() {
		let signer1 = AccountSigner::global(&SCRIPT_HASH.deref().into()).unwrap();
		let signer2 = AccountSigner::global(&SCRIPT_HASH.deref().into()).unwrap();

		assert_eq!(signer1, signer2);

		let signer3 = ContractSigner::called_by_entry(*SCRIPT_HASH, &[]);
		let signer4 = ContractSigner::called_by_entry(*SCRIPT_HASH, &[]);

		assert_eq!(signer3, signer4);

		let mut signer5 = AccountSigner::called_by_entry(&SCRIPT_HASH.deref().into()).unwrap();
		signer5
			.set_allowed_groups(vec![GROUP_PUB_KEY1.clone(), GROUP_PUB_KEY2.clone()])
			.expect("");
		signer5.set_allowed_contracts(vec![*SCRIPT_HASH1, *SCRIPT_HASH2]).expect("");

		let mut signer6 = AccountSigner::called_by_entry(&SCRIPT_HASH.deref().into()).unwrap();
		signer6
			.set_allowed_groups(vec![GROUP_PUB_KEY1.clone(), GROUP_PUB_KEY2.clone()])
			.expect("");
		signer6.set_allowed_contracts(vec![*SCRIPT_HASH1, *SCRIPT_HASH2]).expect("");

		assert_eq!(signer5, signer6);
	}

	#[test]
	fn test_to_account_signer_accepts_account_variant() {
		let account_signer = AccountSigner::called_by_entry(&SCRIPT_HASH.deref().into()).unwrap();
		let signer = Signer::from(account_signer.clone());

		let converted = signer.to_account_signer().unwrap();

		assert_eq!(converted, account_signer);
	}

	#[test]
	fn test_to_account_signer_rejects_other_variants() {
		let contract_signer = Signer::from(ContractSigner::called_by_entry(*SCRIPT_HASH, &[]));

		let err = contract_signer.to_account_signer().unwrap_err();

		assert_eq!(
			err,
			BuilderError::IllegalState(
				"Cannot convert ContractSigner into AccountSigner".to_string()
			)
		);
	}

	#[test]
	fn test_to_contract_signer_accepts_contract_variant() {
		let contract_signer = ContractSigner::called_by_entry(*SCRIPT_HASH, &[]);
		let signer = Signer::from(contract_signer.clone());

		let converted = signer.to_contract_signer().unwrap();

		assert_eq!(converted, contract_signer);
	}

	#[test]
	fn test_to_contract_signer_rejects_other_variants() {
		let account_signer =
			Signer::from(AccountSigner::called_by_entry(&SCRIPT_HASH.deref().into()).unwrap());

		let err = account_signer.to_contract_signer().unwrap_err();

		assert_eq!(
			err,
			BuilderError::IllegalState(
				"Cannot convert AccountSigner into ContractSigner".to_string()
			)
		);
	}

	#[test]
	fn test_try_to_transaction_signer_preserves_signer_data() {
		let mut signer = AccountSigner::called_by_entry(&SCRIPT_HASH.deref().into()).unwrap();
		signer.set_allowed_contracts(vec![*SCRIPT_HASH1]).unwrap();
		let expected_hash = signer.get_script_hash();
		let expected_scopes = signer.get_scopes().clone();
		let expected_contracts = signer.get_allowed_contracts().clone();
		let signer = Signer::from(signer);

		let converted = signer.try_to_transaction_signer().unwrap();

		assert_eq!(converted.account, expected_hash);
		assert_eq!(converted.scopes, expected_scopes);
		assert_eq!(converted.allowed_contracts, Some(expected_contracts));
		assert_eq!(converted.allowed_groups, Some(Vec::new()));
		assert_eq!(converted.rules, Some(Vec::new()));
	}

	#[test]
	fn test_try_to_transaction_signer_rejects_invalid_scopes() {
		let account =
			Account::from_wif("Kzt94tAAiZSgH7Yt4i25DW6jJFprZFPSqTgLr5dWmWgKDKCjXMfZ").unwrap();
		let mut account_signer = AccountSigner::called_by_entry(&account).unwrap();
		account_signer.scopes = vec![WitnessScope::Global, WitnessScope::CalledByEntry];
		let signer = Signer::from(account_signer);

		let err = signer.try_to_transaction_signer().unwrap_err();

		assert_eq!(
			err,
			BuilderError::SignerConfiguration(
				"Global scope cannot be combined with other scopes".to_string()
			)
		);
	}

	#[test]
	fn test_serialize_with_multiple_scopes_contracts_groups_and_rules() {
		let rule = WitnessRule::new(
			WitnessAction::Allow,
			WitnessCondition::CalledByContract(*SCRIPT_HASH1),
		);
		let mut signer = AccountSigner::called_by_entry(&SCRIPT_HASH.deref().into()).unwrap();
		signer
			.set_allowed_groups(vec![GROUP_PUB_KEY1.clone(), GROUP_PUB_KEY2.clone()])
			.expect("");
		signer.set_allowed_contracts(vec![*SCRIPT_HASH1, *SCRIPT_HASH2]).expect("");
		signer.set_rules(vec![rule]).expect("");

		let expected = format!(
			"{}{}{}{}{}{}{}{}{}{}{}{}",
			hex::encode(SCRIPT_HASH.as_bytes()),
			"71",
			"02",
			hex::encode(SCRIPT_HASH1.as_bytes()),
			hex::encode(SCRIPT_HASH2.as_bytes()),
			"02",
			GROUP_PUB_KEY1.get_encoded_compressed_hex().trim_start_matches("0x").to_string(),
			GROUP_PUB_KEY2.get_encoded_compressed_hex().trim_start_matches("0x").to_string(),
			"01",
			"01",
			"28",
			hex::encode(SCRIPT_HASH1.as_bytes())
		);

		assert_eq!(signer.to_array(), hex::decode(&expected).unwrap());
	}

	#[test]
	fn test_deserialize() {
		let data_str = format!(
			"{}{}{}{}{}{}{}{}{}{}{}{}",
			hex::encode(SCRIPT_HASH.as_bytes()),
			"71",
			"02",
			hex::encode(SCRIPT_HASH1.as_bytes()),
			hex::encode(SCRIPT_HASH2.as_bytes()),
			"02",
			GROUP_PUB_KEY1.get_encoded_compressed_hex().trim_start_matches("0x").to_string(),
			GROUP_PUB_KEY2.get_encoded_compressed_hex().trim_start_matches("0x").to_string(),
			"01",
			"01",
			"28",
			hex::encode(SCRIPT_HASH1.as_bytes())
		);
		let serialized = hex::decode(&data_str).unwrap();

		let signer = Signer::from_bytes(&serialized).unwrap();

		assert_eq!(signer.get_signer_hash(), SCRIPT_HASH.deref());

		let expected_scopes: HashSet<WitnessScope> = vec![
			WitnessScope::CalledByEntry,
			WitnessScope::CustomContracts,
			WitnessScope::CustomGroups,
			WitnessScope::WitnessRules,
		]
		.into_iter()
		.collect();
		assert_eq!(signer.get_scopes().iter().cloned().collect::<HashSet<_>>(), expected_scopes);

		assert_eq!(
			signer.get_allowed_contracts().iter().cloned().collect::<HashSet<_>>(),
			vec![*SCRIPT_HASH1, *SCRIPT_HASH2].into_iter().collect()
		);

		assert_eq!(
			signer.get_allowed_groups().iter().cloned().collect::<HashSet<_>>(),
			vec![GROUP_PUB_KEY1.clone(), GROUP_PUB_KEY2.clone()].into_iter().collect()
		);

		let rule = &signer.get_rules()[0];
		assert_eq!(rule.action, WitnessAction::Allow);
		assert_eq!(rule.condition, WitnessCondition::CalledByContract(*SCRIPT_HASH1));
	}
}
