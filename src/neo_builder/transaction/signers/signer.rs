use crate::{
	builder::{
		AccountSigner, BuilderError, ContractSigner, TransactionError, TransactionSigner,
		WitnessCondition, WitnessRule, WitnessScope,
	},
	codec::{Decoder, Encoder, NeoSerializable},
	config::NeoConstants,
	crypto::Secp256r1PublicKey,
	neo_protocol::AccountTrait,
};
use elliptic_curve::pkcs8::der::Encode;
use primitive_types::H160;
use serde::{Deserialize, Serialize, Serializer};
use std::{
	collections::HashSet,
	hash::{Hash, Hasher},
	ops::Deref,
};

use lazy_static::lazy_static;

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
		match self {
			Signer::AccountSigner(account_signer) => match other {
				Signer::AccountSigner(other_account_signer) => {
					account_signer.get_signer_hash() == other_account_signer.get_signer_hash()
				},
				_ => false,
			},
			Signer::ContractSigner(contract_signer) => match other {
				Signer::ContractSigner(other_contract_signer) => {
					contract_signer.get_signer_hash() == other_contract_signer.get_signer_hash()
				},
				_ => false,
			},
			Signer::TransactionSigner(transaction_signer) => match other {
				Signer::TransactionSigner(other_transaction_signer) => {
					transaction_signer.get_signer_hash()
						== other_transaction_signer.get_signer_hash()
				},
				_ => false,
			},
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
	pub fn to_account_signer(self) -> Result<AccountSigner, String> {
		match self {
			Signer::AccountSigner(account_signer) => Ok(account_signer),
			_ => {
				Err("Cannot convert ContractSigner or TransactionSigner into AccountSigner"
					.to_string())
			},
		}
	}

	/// Safely converts to ContractSigner
	pub fn to_contract_signer(self) -> Result<ContractSigner, String> {
		match self {
			Signer::AccountSigner(_) => {
				Err("Cannot convert AccountSigner into ContractSigner".to_string())
			},
			Signer::ContractSigner(contract_signer) => Ok(contract_signer),
			Signer::TransactionSigner(_) => {
				Err("Cannot convert TransactionSigner into ContractSigner".to_string())
			},
		}
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

// Keep the existing Into implementations for backward compatibility
impl Into<AccountSigner> for Signer {
	fn into(self) -> AccountSigner {
		match self {
			Signer::AccountSigner(account_signer) => account_signer,
			_ => {
				eprintln!("Warning: Cannot convert ContractSigner or TransactionSigner into AccountSigner, returning default");
				// Return a default AccountSigner as fallback
				AccountSigner::none_hash160(primitive_types::H160::zero()).unwrap_or_else(|_| {
					// If even the default creation fails, create one with a valid account
					let account = crate::neo_protocol::Account::from_wif(
						"L1WMhxazScMhUrdv34JqQb1HFSQmWeN2Kpc1R9JGKwL7CDNP21uR",
					)
					.unwrap();
					AccountSigner::new(&account, crate::builder::WitnessScope::None)
				})
			},
		}
	}
}

impl Into<TransactionSigner> for Signer {
	fn into(self) -> TransactionSigner {
		match self {
			Signer::AccountSigner(account_signer) => TransactionSigner::new_full(
				account_signer.account.get_script_hash(),
				account_signer.get_scopes().to_vec(),
				account_signer.get_allowed_contracts().to_vec(),
				account_signer.get_allowed_groups().to_vec(),
				account_signer.get_rules().to_vec(),
			),
			Signer::ContractSigner(contract_signer) => TransactionSigner::new_full(
				*contract_signer.get_signer_hash(),
				contract_signer.get_scopes().to_vec(),
				contract_signer.get_allowed_contracts().to_vec(),
				contract_signer.get_allowed_groups().to_vec(),
				contract_signer.get_rules().to_vec(),
			),
			Signer::TransactionSigner(transaction_signer) => transaction_signer,
		}
	}
}

impl Into<TransactionSigner> for &Signer {
	fn into(self) -> TransactionSigner {
		match self {
			Signer::AccountSigner(account_signer) => TransactionSigner::new_full(
				account_signer.account.get_script_hash(),
				account_signer.get_scopes().to_vec(),
				account_signer.get_allowed_contracts().to_vec(),
				account_signer.get_allowed_groups().to_vec(),
				account_signer.get_rules().to_vec(),
			),
			Signer::ContractSigner(contract_signer) => TransactionSigner::new_full(
				*contract_signer.get_signer_hash(),
				contract_signer.get_scopes().to_vec(),
				contract_signer.get_allowed_contracts().to_vec(),
				contract_signer.get_allowed_groups().to_vec(),
				contract_signer.get_rules().to_vec(),
			),
			// Signer::Account(_account_signer) =>
			// 	panic!("Cannot convert AccountSigner into TransactionSigner"),
			// Signer::Contract(_contract_signer) =>
			// 	panic!("Cannot convert ContractSigner into AccountSigner"),
			Signer::TransactionSigner(transaction_signer) => transaction_signer.clone(),
		}
	}
}

impl Into<TransactionSigner> for &mut Signer {
	fn into(self) -> TransactionSigner {
		match self {
			Signer::AccountSigner(account_signer) => TransactionSigner::new_full(
				account_signer.account.get_script_hash(),
				account_signer.get_scopes().to_vec(),
				account_signer.get_allowed_contracts().to_vec(),
				account_signer.get_allowed_groups().to_vec(),
				account_signer.get_rules().to_vec(),
			),
			Signer::ContractSigner(contract_signer) => TransactionSigner::new_full(
				*contract_signer.get_signer_hash(),
				contract_signer.get_scopes().to_vec(),
				contract_signer.get_allowed_contracts().to_vec(),
				contract_signer.get_allowed_groups().to_vec(),
				contract_signer.get_rules().to_vec(),
			),
			Signer::TransactionSigner(transaction_signer) => transaction_signer.clone(),
		}
	}
}

impl Into<ContractSigner> for &mut Signer {
	fn into(self) -> ContractSigner {
		match self {
			Signer::ContractSigner(contract_signer) => contract_signer.clone(),
			_ => {
				// Log the error and return a minimal ContractSigner to avoid panic
				eprintln!("Warning: Cannot convert signer type to ContractSigner, returning minimal fallback");
				ContractSigner::called_by_entry(H160::zero(), &[])
			},
		}
	}
}

impl Into<ContractSigner> for Signer {
	fn into(self) -> ContractSigner {
		match self {
			Signer::ContractSigner(contract_signer) => contract_signer,
			_ => {
				// Log the error and return a minimal ContractSigner to avoid panic
				eprintln!("Warning: Cannot convert signer type to ContractSigner, returning minimal fallback");
				ContractSigner::called_by_entry(H160::zero(), &[])
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
		match reader.read_u8() {
			0 => Ok(Signer::AccountSigner(AccountSigner::decode(reader)?)),
			1 => Ok(Signer::ContractSigner(ContractSigner::decode(reader)?)),
			//_ => Ok(Signer::Transaction(TransactionSigner::decode(reader)?)),
			_ => Err(TransactionError::InvalidTransaction),
		}
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
		builder::{
			AccountSigner, BuilderError, ContractSigner, SignerTrait, WitnessAction,
			WitnessCondition, WitnessRule, WitnessScope,
		},
		codec::{Encoder, NeoSerializable},
		config::NeoConstants,
		crypto::Secp256r1PublicKey,
		neo_protocol::{Account, AccountTrait},
		ScriptHash, ScriptHashExtension,
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
		let mut data = hex::decode(&serialized).unwrap();
		data.insert(0, 1);

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
		let mut data = hex::decode(&serialized).unwrap();
		data.insert(0, 1);

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
		let mut serialized = hex::decode(&data_str).unwrap();
		serialized.insert(0, 1);

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
