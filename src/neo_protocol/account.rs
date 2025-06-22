//! # Neo Account Module (v0.1.8)
//!
//! The Account module provides functionality for managing Neo accounts, including
//! key management, signature operations, and interaction with the Neo blockchain.
//!
//! ## Overview
//!
//! This module implements account-related operations for the Neo blockchain, including:
//!
//! - **Account Creation**: Generate accounts from private keys, WIFs, or public keys
//! - **Key Management**: Encrypt/decrypt private keys, manage key pairs
//! - **Multi-signature Support**: Create and manage multi-signature accounts
//! - **Blockchain Integration**: Query balances and account state
//! - **Wallet Integration**: Connect accounts to wallet infrastructure
//!
//! ## Example
//!
//! ```rust
//! use neo3::prelude::*;
//! use neo3::neo_protocol::{Account, AccountTrait};
//! use neo3::neo_crypto::keys::{Secp256r1PrivateKey, Secp256r1PublicKey};
//! use std::str::FromStr;
//!
//! // Create a new random account
//! fn create_account_example() -> Result<(), Box<dyn std::error::Error>> {
//!     // Generate a completely new random account
//!     let account = Account::create()?;
//!     
//!     // Print the new account details
//!     println!("Address: {}", account.get_address());
//!     println!("Script Hash: {}", account.get_script_hash());
//!     
//!     // Create an account from an existing WIF (Wallet Import Format)
//!     let wif = "KwVEKk78X65fDrJ3VgqHLcpPpbQVfJLjXrkFUCozHQBJ5nT2xwP8";
//!     let account_from_wif = Account::from_wif(wif)?;
//!     println!("Imported account address: {}", account_from_wif.get_address());
//!     
//!     // Create an account from a public key (watch-only)
//!     let public_key_hex = "02f9ec1fd0a98796cf75b586772a4ddd41a0af07a1dbdf86a7238f74fb72503575";
//!     let public_key = Secp256r1PublicKey::from_hex(public_key_hex)?;
//!     let watch_only_account = Account::from_public_key(&public_key)?;
//!     println!("Watch-only account address: {}", watch_only_account.get_address());
//!     
//!     // Create a multi-signature account (2 of 3)
//!     let pub_key1 = Secp256r1PublicKey::from_hex(
//!         "02f9ec1fd0a98796cf75b586772a4ddd41a0af07a1dbdf86a7238f74fb72503575")?;
//!     let pub_key2 = Secp256r1PublicKey::from_hex(
//!         "03c6aa6e12638b36e88adc1ccdceac4db9929575c3e03576c617c49cce7114a050")?;
//!     let pub_key3 = Secp256r1PublicKey::from_hex(
//!         "03b209fd4f53a7170ea4444e0cb0a6bb6a53c2bd016926989cf85f9b0fba17a70c")?;
//!     
//!     let mut pub_keys = vec![pub_key1, pub_key2, pub_key3];
//!     let multi_sig_account = Account::multi_sig_from_public_keys(&mut pub_keys, 2)?;
//!     
//!     if multi_sig_account.is_multi_sig() {
//!         println!("Created multi-signature account:");
//!         println!("  Address: {}", multi_sig_account.get_address());
//!         println!("  Signing threshold: {}", multi_sig_account.get_signing_threshold()?);
//!         println!("  Number of participants: {}", multi_sig_account.get_nr_of_participants()?);
//!     }
//!     
//!     // Encrypt and decrypt private keys
//!     let mut account_to_encrypt = Account::create()?;
//!     account_to_encrypt.encrypt_private_key("my-secure-password")?;
//!     println!("Encrypted private key: {:?}", account_to_encrypt.encrypted_private_key());
//!     
//!     // Decrypt the private key for signing operations
//!     account_to_encrypt.decrypt_private_key("my-secure-password")?;
//!     println!("Account unlocked and ready for signing");
//!     
//!     Ok(())
//! }
//! ```

use std::{
	collections::HashMap,
	fmt::Debug,
	hash::{Hash, Hasher},
	str::FromStr,
	sync::{Arc, Weak},
};

use crate::neo_crypto::utils::ToHexString;
use primitive_types::H160;
use serde_derive::{Deserialize, Serialize};
use signature::{hazmat::PrehashSigner, Error, SignerMut};

use crate::{
	neo_builder::VerificationScript,
	neo_clients::{public_key_to_address, APITrait, JsonRpcProvider, ProviderError, RpcClient},
	neo_crypto::{private_key_from_wif, KeyPair, Secp256r1PublicKey, Secp256r1Signature},
	neo_protocol::{get_nep2_from_private_key, get_private_key_from_nep2},
	neo_types::{
		deserialize_address_or_script_hash, serialize_address_or_script_hash, Address,
		AddressOrScriptHash, ContractParameterType, ScriptHash,
	},
	neo_wallets::{NEP6Account, NEP6Contract, NEP6Parameter, Wallet},
	vec_to_array32, Base64Encode, ScriptHashExtension,
};

pub trait AccountTrait: Sized + PartialEq + Send + Sync + Debug + Clone {
	type Error: Sync + Send + Debug + Sized;

	// Methods to access the fields
	fn key_pair(&self) -> &Option<KeyPair>;
	fn address_or_scripthash(&self) -> &AddressOrScriptHash;
	fn label(&self) -> &Option<String>;
	fn verification_script(&self) -> &Option<VerificationScript>;
	fn is_locked(&self) -> bool;
	fn encrypted_private_key(&self) -> &Option<String>;
	fn signing_threshold(&self) -> &Option<u32>;
	fn nr_of_participants(&self) -> &Option<u32>;
	fn set_key_pair(&mut self, key_pair: Option<KeyPair>);
	fn set_address_or_scripthash(&mut self, address_or_scripthash: AddressOrScriptHash);
	fn set_label(&mut self, label: Option<String>);
	fn set_verification_script(&mut self, verification_script: Option<VerificationScript>);
	fn set_locked(&mut self, is_locked: bool);
	fn set_encrypted_private_key(&mut self, encrypted_private_key: Option<String>);

	fn set_signing_threshold(&mut self, signing_threshold: Option<u32>);
	fn set_nr_of_participants(&mut self, nr_of_participants: Option<u32>);

	fn new(
		address: AddressOrScriptHash,
		label: Option<String>,
		verification_script: Option<VerificationScript>,
		signing_threshold: Option<u32>,
		nr_of_participants: Option<u32>,
	) -> Self;

	fn from_key_pair(
		key_pair: KeyPair,
		signing_threshold: Option<u32>,
		nr_of_participants: Option<u32>,
	) -> Result<Self, Self::Error>;

	fn from_key_pair_opt(
		key_pair: Option<KeyPair>,
		address: AddressOrScriptHash,
		label: Option<String>,
		verification_script: Option<VerificationScript>,
		is_locked: bool,
		is_default: bool,
		encrypted_private_key: Option<String>,
		signing_threshold: Option<u32>,
		nr_of_participants: Option<u32>,
	) -> Self;

	fn from_wif(wif: &str) -> Result<Self, Self::Error>;

	fn decrypt_private_key(&mut self, password: &str) -> Result<(), Self::Error>;

	fn encrypt_private_key(&mut self, password: &str) -> Result<(), Self::Error>;

	fn get_script_hash(&self) -> ScriptHash;

	fn get_signing_threshold(&self) -> Result<u32, Self::Error>;

	fn get_nr_of_participants(&self) -> Result<u32, Self::Error>;

	fn from_verification_script(script: &VerificationScript) -> Result<Self, Self::Error>;

	fn from_public_key(public_key: &Secp256r1PublicKey) -> Result<Self, Self::Error>;

	fn set_wallet(&mut self, wallet: Option<Weak<Wallet>>);

	fn get_wallet(&self) -> Option<Arc<Wallet>>;

	fn multi_sig_from_public_keys(
		public_keys: &mut [Secp256r1PublicKey],
		signing_threshold: u32,
	) -> Result<Self, Self::Error>;
	fn multi_sig_from_addr(
		address: String,
		signing_threshold: u8,
		nr_of_participants: u8,
	) -> Result<Self, Self::Error>;

	fn from_address(address: &str) -> Result<Self, Self::Error>;

	fn from_script_hash(script_hash: &H160) -> Result<Self, Self::Error>;

	fn create() -> Result<Self, Self::Error>;

	fn is_multi_sig(&self) -> bool;
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Account {
	#[serde(skip)]
	pub key_pair: Option<KeyPair>,
	#[serde(
		serialize_with = "serialize_address_or_script_hash",
		deserialize_with = "deserialize_address_or_script_hash"
	)]
	pub address_or_scripthash: AddressOrScriptHash,
	pub label: Option<String>,
	pub verification_script: Option<VerificationScript>,
	pub is_default: bool,
	pub is_locked: bool,
	pub encrypted_private_key: Option<String>,
	pub signing_threshold: Option<u32>,
	pub nr_of_participants: Option<u32>,
	#[serde(skip)]
	pub wallet: Option<Weak<Wallet>>,
}

impl Account {
	pub fn get_address(&self) -> String {
		self.address_or_scripthash.address()
	}

	pub fn get_script_hash(&self) -> H160 {
		self.address_or_scripthash.script_hash()
	}

	pub fn get_verification_script(&self) -> Option<VerificationScript> {
		self.verification_script.clone()
	}
	pub fn get_public_key(&self) -> Option<Secp256r1PublicKey> {
		self.key_pair.as_ref().map(|k| k.public_key.clone())
	}
}

impl From<H160> for Account {
	fn from(script_hash: H160) -> Self {
		Self {
			address_or_scripthash: AddressOrScriptHash::ScriptHash(script_hash),
			..Default::default()
		}
	}
}

impl From<&H160> for Account {
	fn from(script_hash: &H160) -> Self {
		Self {
			address_or_scripthash: AddressOrScriptHash::ScriptHash(script_hash.clone()),
			..Default::default()
		}
	}
}

impl PartialEq for Account {
	fn eq(&self, other: &Self) -> bool {
		self.address_or_scripthash == other.address_or_scripthash
			&& self.label == other.label
			&& self.verification_script == other.verification_script
			&& self.is_locked == other.is_locked
			&& self.encrypted_private_key == other.encrypted_private_key
			&& self.signing_threshold == other.signing_threshold
			&& self.nr_of_participants == other.nr_of_participants
	}
}

impl Hash for Account {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.address_or_scripthash.hash(state);
		self.label.hash(state);
		self.verification_script.hash(state);
		self.is_locked.hash(state);
		self.encrypted_private_key.hash(state);
		self.signing_threshold.hash(state);
		self.nr_of_participants.hash(state);
	}
}

impl AccountTrait for Account {
	type Error = ProviderError;

	fn key_pair(&self) -> &Option<KeyPair> {
		&self.key_pair
	}

	fn address_or_scripthash(&self) -> &AddressOrScriptHash {
		&self.address_or_scripthash
	}

	fn label(&self) -> &Option<String> {
		&self.label
	}

	fn verification_script(&self) -> &Option<VerificationScript> {
		&self.verification_script
	}

	fn is_locked(&self) -> bool {
		self.is_locked
	}

	fn encrypted_private_key(&self) -> &Option<String> {
		&self.encrypted_private_key
	}

	fn signing_threshold(&self) -> &Option<u32> {
		&self.signing_threshold
	}

	fn nr_of_participants(&self) -> &Option<u32> {
		&self.nr_of_participants
	}

	fn set_key_pair(&mut self, key_pair: Option<KeyPair>) {
		self.key_pair = key_pair;
	}

	fn set_address_or_scripthash(&mut self, address_or_scripthash: AddressOrScriptHash) {
		self.address_or_scripthash = address_or_scripthash;
	}

	fn set_label(&mut self, label: Option<String>) {
		self.label = label;
	}

	fn set_verification_script(&mut self, verification_script: Option<VerificationScript>) {
		self.verification_script = verification_script;
	}

	fn set_locked(&mut self, is_locked: bool) {
		self.is_locked = is_locked;
	}

	fn set_encrypted_private_key(&mut self, encrypted_private_key: Option<String>) {
		self.encrypted_private_key = encrypted_private_key;
	}

	fn set_signing_threshold(&mut self, signing_threshold: Option<u32>) {
		self.signing_threshold = signing_threshold;
	}

	fn set_nr_of_participants(&mut self, nr_of_participants: Option<u32>) {
		self.nr_of_participants = nr_of_participants;
	}

	fn new(
		address: AddressOrScriptHash,
		label: Option<String>,
		verification_script: Option<VerificationScript>,
		signing_threshold: Option<u32>,
		nr_of_participants: Option<u32>,
	) -> Self {
		Self {
			key_pair: None,
			address_or_scripthash: address,
			label,
			verification_script,
			is_default: false,
			is_locked: false,
			encrypted_private_key: None,
			signing_threshold,
			nr_of_participants,
			wallet: None,
		}
	}

	fn from_key_pair(
		key_pair: KeyPair,
		signing_threshold: Option<u32>,
		nr_of_participants: Option<u32>,
	) -> Result<Self, Self::Error> {
		let address = public_key_to_address(&key_pair.public_key);
		Ok(Self {
			key_pair: Some(key_pair.clone()),
			address_or_scripthash: AddressOrScriptHash::Address(address.clone()),
			label: Some(address),
			verification_script: Some(VerificationScript::from_public_key(
				&key_pair.clone().public_key(),
			)),
			is_default: false,
			is_locked: false,
			encrypted_private_key: None,
			signing_threshold,
			nr_of_participants,
			wallet: None,
		})
	}

	fn from_key_pair_opt(
		key_pair: Option<KeyPair>,
		address: AddressOrScriptHash,
		label: Option<String>,
		verification_script: Option<VerificationScript>,
		is_locked: bool,
		_is_default: bool,
		encrypted_private_key: Option<String>,
		signing_threshold: Option<u32>,
		nr_of_participants: Option<u32>,
	) -> Self {
		Self {
			key_pair,
			address_or_scripthash: address,
			label,
			verification_script,
			is_default: false,
			is_locked,
			encrypted_private_key,
			signing_threshold,
			nr_of_participants,
			wallet: None,
		}
	}

	fn from_wif(wif: &str) -> Result<Self, Self::Error> {
		let key_pair = KeyPair::from_secret_key(&private_key_from_wif(wif)?);
		Self::from_key_pair(key_pair, None, None)
	}

	fn decrypt_private_key(&mut self, password: &str) -> Result<(), Self::Error> {
		if self.key_pair.is_some() {
			return Ok(());
		}

		let encrypted_private_key = self
			.encrypted_private_key
			.as_ref()
			.ok_or(Self::Error::IllegalState("No encrypted private key present".to_string()))?;

		let key_pair = get_private_key_from_nep2(encrypted_private_key, password).map_err(|e| {
			Self::Error::IllegalState(format!("Failed to decrypt private key: {e}"))
		})?;

		let key_pair_array = vec_to_array32(key_pair).map_err(|_| {
			Self::Error::IllegalState("Failed to convert private key to 32-byte array".to_string())
		})?;

		self.key_pair =
			Some(KeyPair::from_private_key(&key_pair_array).map_err(|e| {
				Self::Error::IllegalState(format!("Failed to create key pair: {e}"))
			})?);

		Ok(())
	}

	fn encrypt_private_key(&mut self, password: &str) -> Result<(), Self::Error> {
		let key_pair = self.key_pair.as_ref().ok_or(Self::Error::IllegalState(
			"The account does not hold a decrypted private key.".to_string(),
		))?;

		let encrypted_private_key = get_nep2_from_private_key(
			&key_pair.private_key.to_raw_bytes().to_hex_string(),
			password,
		)
		.map_err(|e| Self::Error::IllegalState(format!("Failed to encrypt private key: {e}")))?;

		self.encrypted_private_key = Some(encrypted_private_key);
		self.key_pair = None;
		Ok(())
	}

	fn get_script_hash(&self) -> ScriptHash {
		self.address_or_scripthash.script_hash()
	}

	fn get_signing_threshold(&self) -> Result<u32, Self::Error> {
		self.signing_threshold.ok_or_else(|| {
			Self::Error::IllegalState(format!(
				"Cannot get signing threshold from account {}",
				self.address_or_scripthash().address()
			))
		})
	}

	fn get_nr_of_participants(&self) -> Result<u32, Self::Error> {
		self.nr_of_participants.ok_or_else(|| {
			Self::Error::IllegalState(format!(
				"Cannot get signing threshold from account {}",
				self.address_or_scripthash().address()
			))
		})
	}

	fn from_verification_script(script: &VerificationScript) -> Result<Self, Self::Error> {
		let address = ScriptHash::from_script(&script.script());

		let (signing_threshold, nr_of_participants) = if script.is_multi_sig() {
			(
				Some(script.get_signing_threshold().unwrap()),
				Some(script.get_nr_of_accounts().unwrap()),
			)
		} else {
			(None, None)
		};

		Ok(Self {
			address_or_scripthash: AddressOrScriptHash::ScriptHash(address),
			label: Some(address.to_address()),
			verification_script: Some(script.clone()),
			signing_threshold: signing_threshold.map(|x| x as u32),
			nr_of_participants: nr_of_participants.map(|x| x as u32),
			..Default::default()
		})
	}

	fn from_public_key(public_key: &Secp256r1PublicKey) -> Result<Self, Self::Error> {
		let script = VerificationScript::from_public_key(public_key);
		let address = ScriptHash::from_script(&script.script());

		Ok(Self {
			address_or_scripthash: AddressOrScriptHash::ScriptHash(address),
			label: Some(address.to_address()),
			verification_script: Some(script),
			..Default::default()
		})
	}

	fn set_wallet(&mut self, wallet: Option<Weak<Wallet>>) {
		self.wallet = wallet;
	}

	fn get_wallet(&self) -> Option<Arc<Wallet>> {
		self.wallet.as_ref().and_then(|w| w.upgrade())
	}

	fn multi_sig_from_public_keys(
		public_keys: &mut [Secp256r1PublicKey],
		signing_threshold: u32,
	) -> Result<Self, Self::Error> {
		let script = VerificationScript::from_multi_sig(public_keys, signing_threshold as u8);
		let addr = ScriptHash::from_script(&script.script());

		Ok(Self {
			label: Some(addr.to_address()),
			verification_script: Some(script),
			signing_threshold: Some(signing_threshold),
			nr_of_participants: Some(public_keys.len() as u32),
			address_or_scripthash: AddressOrScriptHash::ScriptHash(addr),
			..Default::default()
		})
	}

	fn multi_sig_from_addr(
		address: String,
		signing_threshold: u8,
		nr_of_participants: u8,
	) -> Result<Self, Self::Error> {
		Ok(Self {
			label: Option::from(address.clone()),
			signing_threshold: Some(signing_threshold as u32),
			nr_of_participants: Some(nr_of_participants as u32),
			address_or_scripthash: AddressOrScriptHash::Address(address),
			..Default::default()
		})
	}

	fn from_address(address: &str) -> Result<Self, Self::Error> {
		let address = Address::from_str(address).map_err(|_| {
			Self::Error::IllegalState(format!("Invalid address format: {address}"))
		})?;

		Ok(Self {
			address_or_scripthash: AddressOrScriptHash::Address(address.clone()),
			label: Some(address),
			..Default::default()
		})
	}

	fn from_script_hash(script_hash: &H160) -> Result<Self, Self::Error> {
		let address = script_hash.to_address();
		Self::from_address(&address)
	}

	fn create() -> Result<Self, Self::Error> {
		let key_pair = KeyPair::new_random();
		Self::from_key_pair(key_pair, None, None)
	}

	fn is_multi_sig(&self) -> bool {
		self.signing_threshold.is_some() && self.nr_of_participants.is_some()
	}
}

impl PrehashSigner<Secp256r1Signature> for Account {
	fn sign_prehash(&self, _prehash: &[u8]) -> Result<Secp256r1Signature, Error> {
		let key_pair = self.key_pair.as_ref().ok_or_else(|| Error::new())?;

		let signature = key_pair.private_key.sign_prehash(_prehash).map_err(|_| Error::new())?;

		Ok(signature)
	}
}

impl Account {
	pub fn to_nep6_account(&self) -> Result<NEP6Account, ProviderError> {
		if self.key_pair.is_some() && self.encrypted_private_key.is_none() {
			return Err(ProviderError::IllegalState(
				"Account private key is available but not encrypted.".to_string(),
			));
		}

		if self.verification_script.is_none() {
			return Ok(NEP6Account::new(
				self.address_or_scripthash.address().clone(),
				self.label.clone(),
				self.is_default,
				self.is_locked,
				self.encrypted_private_key.clone(),
				None,
				None,
			));
		}

		let mut parameters = Vec::new();
		let script_data = self.verification_script.as_ref().unwrap();

		if script_data.is_multi_sig() {
			for i in 0..script_data.get_nr_of_accounts().unwrap() {
				parameters.push(NEP6Parameter {
					param_name: format!("signature{i}"),
					param_type: ContractParameterType::Signature,
				});
			}
		} else if script_data.is_single_sig() {
			parameters.push(NEP6Parameter {
				param_name: "signature".to_string(),
				param_type: ContractParameterType::Signature,
			});
		}

		let script_encoded = script_data.script().to_base64();
		let contract = NEP6Contract {
			script: Some(script_encoded),
			is_deployed: false, // Assuming a simple setup; might need actual logic
			nep6_parameters: parameters,
		};

		Ok(NEP6Account::new(
			self.address_or_scripthash.address().clone(),
			self.label.clone(),
			self.is_default,
			self.is_locked,
			self.encrypted_private_key.clone(),
			Some(contract),
			None,
		))
	}

	pub async fn get_nep17_balances<P>(
		&self,
		provider: &RpcClient<P>,
	) -> Result<HashMap<H160, u64>, ProviderError>
	where
		P: JsonRpcProvider,
	{
		let response =
			provider.get_nep17_balances(self.address_or_scripthash().script_hash()).await?;
		let mut balances = HashMap::new();
		for balance in response.balances {
			let asset_hash = balance.asset_hash;
			let amount = balance.amount.parse::<u64>().map_err(|e| {
				ProviderError::CustomError(format!("Failed to parse balance amount: {e}"))
			})?;
			balances.insert(asset_hash, amount);
		}
		Ok(balances)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		neo_clients::{BodyRegexMatcher, HttpProvider, MockClient},
		neo_config::TestConstants,
	};

	// ... rest of test module
}
