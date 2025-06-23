use ethereum_types::H256;
/// A builder for constructing and configuring NEO blockchain transactions.
///
/// The `TransactionBuilder` provides a fluent interface for setting various transaction parameters
/// such as version, nonce, validity period, signers, fees, and script. Once configured, it can
/// generate an unsigned transaction.
///
/// # Fields
///
/// - `client`: An optional reference to an RPC client for network operations.
/// - `version`: The transaction version.
/// - `nonce`: A random number to prevent transaction duplication.
/// - `valid_until_block`: The block height until which the transaction is valid.
/// - `signers`: A list of transaction signers.
/// - `additional_network_fee`: Additional network fee for the transaction.
/// - `additional_system_fee`: Additional system fee for the transaction.
/// - `attributes`: Transaction attributes.
/// - `script`: The transaction script.
/// - `fee_consumer`: An optional closure for fee-related operations.
/// - `fee_error`: An optional error related to fee calculations.
///
/// # Example
///
/// ```rust,no_run
/// use neo3::neo_builder::TransactionBuilder;
/// use neo3::neo_clients::{HttpProvider, RpcClient};
///
/// #[tokio::main]
/// async fn main() {
///     let provider = HttpProvider::new("https://testnet1.neo.org:443").unwrap();
///     let client = RpcClient::new(provider);
///     let mut tx_builder = TransactionBuilder::with_client(&client);
///     tx_builder.version(0);
///     tx_builder.nonce(1).unwrap();
///     tx_builder.valid_until_block(100).unwrap();
///     tx_builder.extend_script(vec![0x01, 0x02, 0x03]);
///
///     let unsigned_tx = tx_builder.get_unsigned_tx().await.unwrap();
/// }
/// ```
///
/// # Note
///
/// This builder implements `Debug`, `Clone`, `Eq`, `PartialEq`, and `Hash` traits.
/// It uses generics to allow for different types of JSON-RPC providers.
use futures_util::TryFutureExt;
use std::{
	cell::RefCell,
	collections::HashSet,
	default,
	fmt::Debug,
	hash::{Hash, Hasher},
	iter::Iterator,
	str::FromStr,
};

use crate::builder::SignerTrait;
use getset::{CopyGetters, Getters, MutGetters, Setters};
use once_cell::sync::Lazy;
use primitive_types::H160;
use serde::{Deserialize, Serialize};
use serde_json::Value;
// Import from neo_types
use crate::neo_types::{
	Bytes, ContractParameter, InvocationResult, NameOrAddress, ScriptHash, ScriptHashExtension,
};

// Import transaction types from neo_builder
use crate::neo_builder::{
	transaction::{
		Signer, SignerType, Transaction, TransactionAttribute, TransactionError,
		VerificationScript, Witness, WitnessScope,
	},
	BuilderError,
};

// Import other modules
use crate::{
	neo_clients::{APITrait, JsonRpcProvider, RpcClient},
	neo_codec::{Decoder, Encoder, NeoSerializable, VarSizeTrait},
	neo_config::{NeoConstants, NEOCONFIG},
	neo_crypto::{utils::ToHexString, HashableForVec, Secp256r1PublicKey},
	neo_protocol::{AccountTrait, NeoNetworkFee},
};

// Helper functions
use crate::neo_clients::public_key_to_script_hash;

// Import Account from neo_protocol
use crate::neo_protocol::Account;

// Special module for initialization - conditionally include
#[cfg(feature = "init")]
use crate::prelude::init_logger;
// Define a local replacement for when init feature is not enabled
#[cfg(not(feature = "init"))]
fn init_logger() {
	// No-op when feature is not enabled
}

#[derive(Getters, Setters, MutGetters, CopyGetters, Default)]
pub struct TransactionBuilder<'a, P: JsonRpcProvider + 'static> {
	pub(crate) client: Option<&'a RpcClient<P>>,
	version: u8,
	nonce: u32,
	valid_until_block: Option<u32>,
	// setter and getter
	#[getset(get = "pub")]
	pub(crate) signers: Vec<Signer>,
	#[getset(get = "pub", set = "pub")]
	additional_network_fee: u64,
	#[getset(get = "pub", set = "pub")]
	additional_system_fee: u64,
	#[getset(get = "pub")]
	attributes: Vec<TransactionAttribute>,
	#[getset(get = "pub", set = "pub")]
	script: Option<Bytes>,
	fee_consumer: Option<Box<dyn Fn(i64, i64)>>,
	fee_error: Option<TransactionError>,
}

impl<'a, P: JsonRpcProvider + 'static> Debug for TransactionBuilder<'a, P> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("TransactionBuilder")
			.field("version", &self.version)
			.field("nonce", &self.nonce)
			.field("valid_until_block", &self.valid_until_block)
			.field("signers", &self.signers)
			.field("additional_network_fee", &self.additional_network_fee)
			.field("additional_system_fee", &self.additional_system_fee)
			.field("attributes", &self.attributes)
			.field("script", &self.script)
			// .field("fee_consumer", &self.fee_consumer)
			.field("fee_error", &self.fee_error)
			.finish()
	}
}

impl<'a, P: JsonRpcProvider + 'static> Clone for TransactionBuilder<'a, P> {
	fn clone(&self) -> Self {
		Self {
			client: self.client,
			version: self.version,
			nonce: self.nonce,
			valid_until_block: self.valid_until_block,
			signers: self.signers.clone(),
			additional_network_fee: self.additional_network_fee,
			additional_system_fee: self.additional_system_fee,
			attributes: self.attributes.clone(),
			script: self.script.clone(),
			// fee_consumer: self.fee_consumer.clone(),
			fee_consumer: None,
			fee_error: None,
		}
	}
}

impl<'a, P: JsonRpcProvider + 'static> Eq for TransactionBuilder<'a, P> {}

impl<'a, P: JsonRpcProvider + 'static> PartialEq for TransactionBuilder<'a, P> {
	fn eq(&self, other: &Self) -> bool {
		self.version == other.version
			&& self.nonce == other.nonce
			&& self.valid_until_block == other.valid_until_block
			&& self.signers == other.signers
			&& self.additional_network_fee == other.additional_network_fee
			&& self.additional_system_fee == other.additional_system_fee
			&& self.attributes == other.attributes
			&& self.script == other.script
	}
}

impl<'a, P: JsonRpcProvider + 'static> Hash for TransactionBuilder<'a, P> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.version.hash(state);
		self.nonce.hash(state);
		self.valid_until_block.hash(state);
		self.signers.hash(state);
		self.additional_network_fee.hash(state);
		self.additional_system_fee.hash(state);
		self.attributes.hash(state);
		self.script.hash(state);
	}
}

pub static GAS_TOKEN_HASH: Lazy<ScriptHash> = Lazy::new(|| {
	ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")
		.expect("GAS token hash is a valid script hash")
});

impl<'a, P: JsonRpcProvider + 'static> TransactionBuilder<'a, P> {
	// const GAS_TOKEN_HASH: ScriptHash = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf").unwrap();
	pub const BALANCE_OF_FUNCTION: &'static str = "balanceOf";
	pub const DUMMY_PUB_KEY: &'static str =
		"02ec143f00b88524caf36a0121c2de09eef0519ddbe1c710a00f0e2663201ee4c0";

	/// Creates a new `TransactionBuilder` instance with default values.
	///
	/// # Returns
	///
	/// A new `TransactionBuilder` instance with default values.
	///
	/// # Examples
	///
	/// ```rust
	/// use neo3::neo_builder::TransactionBuilder;
	/// use neo3::neo_clients::HttpProvider;
	///
	/// let tx_builder: TransactionBuilder<'_, HttpProvider> = TransactionBuilder::default();
	/// ```
	pub fn new() -> Self {
		Self {
			client: None,
			version: 0,
			nonce: 0,
			valid_until_block: None,
			signers: Vec::new(),
			additional_network_fee: 0,
			additional_system_fee: 0,
			attributes: Vec::new(),
			script: None,
			fee_consumer: None,
			fee_error: None,
		}
	}

	/// Creates a new `TransactionBuilder` instance with a client reference.
	///
	/// # Arguments
	///
	/// * `client` - A reference to an RPC client for network operations.
	///
	/// # Returns
	///
	/// A new `TransactionBuilder` instance with the specified client.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use neo3::neo_builder::TransactionBuilder;
	/// use neo3::neo_clients::{HttpProvider, RpcClient};
	///
	/// let provider = HttpProvider::new("https://testnet1.neo.org:443").unwrap();
	/// let client = RpcClient::new(provider);
	/// let tx_builder = TransactionBuilder::with_client(&client);
	/// ```
	pub fn with_client(client: &'a RpcClient<P>) -> Self {
		Self {
			client: Some(client),
			version: 0,
			nonce: 0,
			valid_until_block: None,
			signers: Vec::new(),
			additional_network_fee: 0,
			additional_system_fee: 0,
			attributes: Vec::new(),
			script: None,
			fee_consumer: None,
			fee_error: None,
		}
	}

	/// Sets the version of the transaction.
	///
	/// # Arguments
	///
	/// * `version` - The transaction version (typically 0 for Neo N3).
	///
	/// # Returns
	///
	/// A mutable reference to the `TransactionBuilder` for method chaining.
	///
	/// # Examples
	///
	/// ```rust
	/// use neo3::neo_builder::TransactionBuilder;
	/// use neo3::neo_clients::{HttpProvider, RpcClient};
	///
	/// let provider = HttpProvider::new("https://testnet1.neo.org:443").unwrap();
	/// let client = RpcClient::new(provider);
	/// let mut tx_builder = TransactionBuilder::with_client(&client);
	/// tx_builder.version(0);
	/// ```
	pub fn version(&mut self, version: u8) -> &mut Self {
		self.version = version;
		self
	}

	/// Sets the nonce of the transaction.
	///
	/// The nonce is a random number used to prevent transaction duplication.
	///
	/// # Arguments
	///
	/// * `nonce` - A random number to prevent transaction duplication.
	///
	/// # Returns
	///
	/// A `Result` containing a mutable reference to the `TransactionBuilder` for method chaining,
	/// or a `TransactionError` if the nonce is invalid.
	///
	/// # Examples
	///
	/// ```rust
	/// use neo3::neo_builder::TransactionBuilder;
	/// use neo3::neo_clients::{HttpProvider, RpcClient};
	///
	/// let provider = HttpProvider::new("https://testnet1.neo.org:443").unwrap();
	/// let client = RpcClient::new(provider);
	/// let mut tx_builder = TransactionBuilder::with_client(&client);
	/// tx_builder.nonce(1234567890).unwrap();
	/// ```
	pub fn nonce(&mut self, nonce: u32) -> Result<&mut Self, TransactionError> {
		// u32 can't exceed u32::MAX, so this check is redundant
		// Keeping the function signature for API compatibility
		self.nonce = nonce;
		Ok(self)
	}

	/// Sets the block height until which the transaction is valid.
	///
	/// In Neo N3, transactions have a limited validity period defined by block height.
	/// This helps prevent transaction replay attacks and cleans up the memory pool.
	///
	/// # Arguments
	///
	/// * `block` - The block height until which the transaction is valid.
	///
	/// # Returns
	///
	/// A `Result` containing a mutable reference to the `TransactionBuilder` for method chaining,
	/// or a `TransactionError` if the block height is invalid.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use neo3::neo_builder::TransactionBuilder;
	/// use neo3::neo_clients::{HttpProvider, RpcClient};
	/// use neo3::neo_clients::APITrait;
	///
	/// #[tokio::main]
	/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
	///     let provider = HttpProvider::new("https://testnet1.neo.org:443").unwrap();
	///     let client = RpcClient::new(provider);
	///     
	///     let current_height = client.get_block_count().await?;
	///     
	///     let mut tx_builder = TransactionBuilder::with_client(&client);
	///     tx_builder.valid_until_block(current_height + 5760)?; // Valid for ~1 day
	///     
	///     Ok(())
	/// }
	/// ```
	pub fn valid_until_block(&mut self, block: u32) -> Result<&mut Self, TransactionError> {
		if block == 0 {
			return Err(TransactionError::InvalidBlock);
		}

		self.valid_until_block = Some(block);
		Ok(self)
	}

	// Set script
	// pub fn set_script(&mut self, script: Vec<u8>) -> &mut Self {
	// 	self.script = Some(script);
	// 	self
	// }

	pub fn first_signer(&mut self, sender: &Account) -> Result<&mut Self, TransactionError> {
		self.first_signer_by_hash(&sender.get_script_hash())
	}

	pub fn first_signer_by_hash(&mut self, sender: &H160) -> Result<&mut Self, TransactionError> {
		if self.signers.iter().any(|s| s.get_scopes().contains(&WitnessScope::None)) {
			return Err(TransactionError::ScriptFormat("This transaction contains a signer with fee-only witness scope that will cover the fees. Hence, the order of the signers does not affect the payment of the fees.".to_string()));
		}
		if let Some(pos) = self.signers.iter().position(|s| s.get_signer_hash() == sender) {
			let s = self.signers.remove(pos);
			self.signers.insert(0, s);
			Ok(self)
		} else {
			Err(TransactionError::ScriptFormat(format!("Could not find a signer with script hash {}. Make sure to add the signer before calling this method.", sender.to_string()).into()))
		}
	}

	pub fn extend_script(&mut self, script: Vec<u8>) -> &mut Self {
		if let Some(ref mut existing_script) = self.script {
			existing_script.extend(script);
		} else {
			self.script = Some(script);
		}
		self
	}

	pub async fn call_invoke_script(&self) -> Result<InvocationResult, TransactionError> {
		if self.script.is_none() || self.script.as_ref().unwrap().is_empty() {
			return Err((TransactionError::NoScript));
		}
		let result = self
			.client
			.unwrap()
			.rpc_client()
			.invoke_script(self.script.clone().unwrap().to_hex_string(), self.signers.clone())
			.await
			.map_err(|e| TransactionError::ProviderError(e))?;
		Ok((result))
	}

	/// Builds a transaction from the current builder configuration
	///
	/// # Returns
	///
	/// A `Result` containing the built transaction or a `TransactionError`
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use neo3::neo_builder::TransactionBuilder;
	/// use neo3::neo_clients::{HttpProvider, RpcClient};
	///
	/// #[tokio::main]
	/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
	///     let provider = HttpProvider::new("https://testnet1.neo.org:443").unwrap();
	///     let client = RpcClient::new(provider);
	///     
	///     let mut tx_builder = TransactionBuilder::with_client(&client);
	///     tx_builder.version(0)
	///               .nonce(1234567890)?
	///               .valid_until_block(100)?;
	///     
	///     let tx = tx_builder.build().await?;
	///     
	///     Ok(())
	/// }
	/// ```
	pub async fn build(&mut self) -> Result<Transaction<P>, TransactionError> {
		self.get_unsigned_tx().await
	}

	// Get unsigned transaction
	pub async fn get_unsigned_tx(&mut self) -> Result<Transaction<P>, TransactionError> {
		// Validate configuration
		if self.signers.is_empty() {
			return Err(TransactionError::NoSigners);
		}

		if self.script.is_none() {
			return Err(TransactionError::NoScript);
		}
		let len = self.signers.len();
		self.signers.dedup();

		// Validate no duplicate signers
		if len != self.signers.len() {
			return Err(TransactionError::DuplicateSigner);
		}

		// Check signer limits
		if self.signers.len() > NeoConstants::MAX_SIGNER_SUBITEMS as usize {
			return Err(TransactionError::TooManySigners);
		}

		// Validate script
		if let Some(script) = &self.script {
			if script.is_empty() {
				return Err(TransactionError::EmptyScript);
			}
		} else {
			return Err(TransactionError::NoScript);
		}

		if self.valid_until_block.is_none() {
			self.valid_until_block = Some(
				self.fetch_current_block_count().await?
					+ self.client.unwrap().max_valid_until_block_increment()
					- 1,
			)
		}

		// Check committe member
		if self.is_high_priority() && !self.is_allowed_for_high_priority().await {
			return Err(TransactionError::IllegalState("This transaction does not have a committee member as signer. Only committee members can send transactions with high priority.".to_string()));
		}

		// if self.fee_consumer.is_some() {

		// }

		// Get fees
		// let script = self.script.as_ref().unwrap();
		// let response = self
		// 	.client
		// 	.unwrap()
		// 	.invoke_script(script.to_hex(), vec![self.signers[0].clone()])
		// 	.await
		// 	.map_err(|e| TransactionError::ProviderError(e))?;

		let system_fee = self.get_system_fee().await? + self.additional_system_fee as i64;

		let network_fee = self.get_network_fee().await? + self.additional_network_fee as i64;

		// Check sender balance if needed
		let mut tx = Transaction {
			network: Some(self.client.unwrap()),
			version: self.version,
			nonce: self.nonce,
			valid_until_block: self.valid_until_block.unwrap_or(100),
			size: 0,
			sys_fee: system_fee,
			net_fee: network_fee,
			signers: self.signers.clone(),
			attributes: self.attributes.clone(),
			script: self.script.clone().unwrap(), // We've already checked for None case above
			witnesses: vec![],
			// block_time: None,
			block_count_when_sent: None,
		};

		// It's impossible to calculate network fee when the tx is unsigned, because there is no witness
		// let network_fee = Box::pin(self.client.unwrap().calculate_network_fee(base64::encode(tx.to_array()))).await?;
		if self.fee_error.is_some()
			&& !self.can_send_cover_fees(system_fee as u64 + network_fee as u64).await?
		{
			if let Some(supplier) = &self.fee_error {
				return Err(supplier.clone());
			}
		} else if let Some(fee_consumer) = &self.fee_consumer {
			let sender_balance = self.get_sender_balance().await?.try_into().unwrap(); // self.get_sender_balance().await.unwrap();
			if network_fee + system_fee > sender_balance {
				fee_consumer(network_fee + system_fee, sender_balance);
			}
		}
		// tx.set_net_fee(network_fee);

		Ok(tx)
	}

	async fn get_system_fee(&self) -> Result<i64, TransactionError> {
		let script = self.script.as_ref().ok_or_else(|| TransactionError::NoScript)?;

		let client = self
			.client
			.ok_or_else(|| TransactionError::IllegalState("Client is not set".to_string()))?;

		let response = client
			.invoke_script(script.to_hex_string(), vec![self.signers[0].clone()])
			.await
			.map_err(|e| TransactionError::ProviderError(e))?;

		// Check if the VM execution resulted in a fault
		if response.has_state_fault() {
			// Get the current configuration for allowing transmission on fault
			let allows_fault = NEOCONFIG
				.lock()
				.map_err(|_| {
					TransactionError::IllegalState("Failed to lock NEOCONFIG".to_string())
				})?
				.allows_transmission_on_fault;

			// If transmission on fault is not allowed, return an error
			if !allows_fault {
				return Err(TransactionError::TransactionConfiguration(format!(
					"The vm exited due to the following exception: {}",
					response.exception.unwrap_or_else(|| "Unknown exception".to_string())
				)));
			}
			// Otherwise, we continue with the transaction despite the fault
		}

		Ok(i64::from_str(&response.gas_consumed).map_err(|_| {
			TransactionError::IllegalState("Failed to parse gas consumed".to_string())
		})?)
	}

	async fn get_network_fee(&mut self) -> Result<i64, TransactionError> {
		// Check sender balance if needed
		let client = self
			.client
			.ok_or_else(|| TransactionError::IllegalState("Client is not set".to_string()))?;

		let script = self.script.clone().unwrap_or_default(); // Use default if None

		let valid_until_block = self.valid_until_block.unwrap_or(100);

		let mut tx = Transaction {
			network: Some(client),
			version: self.version,
			nonce: self.nonce,
			valid_until_block,
			size: 0,
			sys_fee: 0,
			net_fee: 0,
			signers: self.signers.clone(),
			attributes: self.attributes.clone(),
			script,
			witnesses: vec![],
			block_count_when_sent: None,
		};
		let mut has_atleast_one_signing_account = false;

		for signer in self.signers.iter() {
			match signer {
				Signer::ContractSigner(contract_signer) => {
					// Create contract witness and add it to the transaction
					let witness =
						Witness::create_contract_witness(contract_signer.verify_params().to_vec())
							.map_err(|e| {
								TransactionError::IllegalState(format!(
									"Failed to create contract witness: {}",
									e
								))
							})?;
					tx.add_witness(witness);
				},
				Signer::AccountSigner(account_signer) => {
					// Get the account from AccountSigner
					let account = account_signer.account();
					let verification_script;

					// Check if the account is multi-signature or single-signature
					if account.is_multi_sig() {
						// Create a fake multi-signature verification script
						verification_script = self
							.create_fake_multi_sig_verification_script(account)
							.map_err(|e| {
								TransactionError::IllegalState(format!(
									"Failed to create multi-sig verification script: {}",
									e
								))
							})?;
					} else {
						// Create a fake single-signature verification script
						verification_script =
							self.create_fake_single_sig_verification_script().map_err(|e| {
								TransactionError::IllegalState(format!(
									"Failed to create single-sig verification script: {}",
									e
								))
							})?;
					}

					// Add a witness with an empty signature and the verification script
					tx.add_witness(Witness::from_scripts(
						vec![],
						verification_script.script().to_vec(),
					));
					has_atleast_one_signing_account = true;
				},
				// If there's a case for TransactionSigner, it can be handled here if necessary.
				_ => {
					// Handle any other cases, if necessary (like TransactionSigner)
				},
			}
		}
		if (!has_atleast_one_signing_account) {
			return Err(TransactionError::TransactionConfiguration("A transaction requires at least one signing account (i.e. an AccountSigner). None was provided.".to_string()))
		}

		let fee = client.calculate_network_fee(tx.to_array().to_hex_string()).await?;
		Ok(fee.network_fee)
	}

	async fn fetch_current_block_count(&mut self) -> Result<u32, TransactionError> {
		let client = self
			.client
			.ok_or_else(|| TransactionError::IllegalState("Client is not set".to_string()))?;
		let count = client.get_block_count().await?;
		Ok(count)
	}

	async fn get_sender_balance(&self) -> Result<u64, TransactionError> {
		// Call network
		let sender = &self.signers[0];

		if Self::is_account_signer(sender) {
			let client = self
				.client
				.ok_or_else(|| TransactionError::IllegalState("Client is not set".to_string()))?;

			let balance = client
				.invoke_function(
					&GAS_TOKEN_HASH,
					Self::BALANCE_OF_FUNCTION.to_string(),
					vec![ContractParameter::from(sender.get_signer_hash())],
					None,
				)
				.await
				.map_err(|e| TransactionError::ProviderError(e))?
				.stack[0]
				.clone();

			return Ok(balance.as_int().ok_or_else(|| {
				TransactionError::IllegalState("Failed to parse balance as integer".to_string())
			})? as u64);
		}
		Err(TransactionError::InvalidSender)
	}

	fn create_fake_single_sig_verification_script(
		&self,
	) -> Result<VerificationScript, TransactionError> {
		// Vector to store dummy public keys
		let dummy_public_key =
			Secp256r1PublicKey::from_encoded(Self::DUMMY_PUB_KEY).ok_or_else(|| {
				TransactionError::IllegalState("Failed to create dummy public key".to_string())
			})?;
		// Create and return the VerificationScript with the pub_keys and signing threshold
		Ok(VerificationScript::from_public_key(&dummy_public_key))
	}

	fn create_fake_multi_sig_verification_script(
		&self,
		account: &Account,
	) -> Result<VerificationScript, TransactionError> {
		// Vector to store dummy public keys
		let mut pub_keys: Vec<Secp256r1PublicKey> = Vec::new();

		// Get the number of participants
		let nr_of_participants = account.get_nr_of_participants().map_err(|e| {
			TransactionError::IllegalState(format!("Failed to get number of participants: {}", e))
		})?;

		// Loop to add dummy public keys based on the number of participants
		for _ in 0..nr_of_participants {
			// Create a dummy public key
			let dummy_public_key = Secp256r1PublicKey::from_encoded(Self::DUMMY_PUB_KEY)
				.ok_or_else(|| {
					TransactionError::IllegalState("Failed to create dummy public key".to_string())
				})?;
			pub_keys.push(dummy_public_key);
		}

		// Get the signing threshold
		let threshold_value = account.get_signing_threshold().map_err(|e| {
			TransactionError::IllegalState(format!("Failed to get signing threshold: {}", e))
		})?;
		let signing_threshold = u8::try_from(threshold_value).map_err(|_| {
			TransactionError::IllegalState(
				"Signing threshold value out of range for u8".to_string(),
			)
		})?;

		// Create and return the VerificationScript with the pub_keys and signing threshold
		// This method returns a VerificationScript directly, not a Result
		let script = VerificationScript::from_multi_sig(&mut pub_keys[..], signing_threshold);

		Ok(script)
	}

	fn is_account_signer(signer: &Signer) -> bool {
		if signer.get_type() == SignerType::AccountSigner {
			return true;
		}
		return false;
	}

	/// Signs the transaction with the provided signers.
	///
	/// This method creates an unsigned transaction, signs it with the appropriate signers,
	/// and returns the signed transaction. For account signers, it uses the account's private key
	/// to create a signature. For contract signers, it creates a contract witness.
	///
	/// # Returns
	///
	/// A `Result` containing the signed `Transaction` if successful,
	/// or a `BuilderError` if an error occurs during signing.
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The transaction cannot be built (see `get_unsigned_tx`)
	/// - A multi-signature account is used (these require manual signing)
	/// - An account does not have a private key
	/// - Witness creation fails
	///
	/// # Examples
	///
	/// ```ignore
	/// use neo3::neo_builder::{TransactionBuilder, ScriptBuilder, AccountSigner};
	/// use neo3::neo_clients::{HttpProvider, RpcClient};
	/// use neo3::neo_protocol::{Account, AccountTrait};
	/// use neo3::neo_types::ContractParameter;
	/// use std::str::FromStr;
	/// use neo3::neo_clients::APITrait;
	///
	/// #[tokio::main]
	/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
	///     let provider = HttpProvider::new("https://testnet1.neo.org:443").unwrap();
	///     let client = RpcClient::new(provider);
	///     
	///     // Create an account for signing
	///     let account = Account::from_wif("YOUR_WIF_HERE")?;
	///     
	///     // Create a proper contract invocation script
	///     let contract_hash = neo3::neo_types::ScriptHash::from_str("0xd2a4cff31913016155e38e474a2c06d08be276cf")?; // GAS
	///     let from_address = account.get_script_hash();
	///     let to_address = neo3::neo_types::ScriptHash::from_str("0x0000000000000000000000000000000000000000")?;
	///     let amount = 1_000_000i64;
	///     let mut script_builder = ScriptBuilder::new();
	///     script_builder.contract_call(
	///         &contract_hash,
	///         "transfer",
	///         &[
	///             ContractParameter::from(from_address),
	///             ContractParameter::from(to_address),
	///             ContractParameter::integer(amount),
	///             ContractParameter::any()
	///         ],
	///         None
	///     )?;
	///     let script = script_builder.to_bytes();
	///     
	///     // Create and configure the transaction
	///     let mut tx_builder = TransactionBuilder::with_client(&client);
	///     tx_builder.extend_script(script);
	///     let account_signer = AccountSigner::called_by_entry(&account)?;
	///     tx_builder.set_signers(vec![account_signer.into()])?;
	///     tx_builder.valid_until_block(client.get_block_count().await? + 5760)?; // Valid for ~1 day
	///
	///     // Sign the transaction
	///     let signed_tx = tx_builder.sign().await?;
	///     
	///     // Send the transaction to the network
	///     let tx_response = signed_tx.send_tx().await?;
	///     println!("Transaction sent: {:?}", tx_response);
	///     
	///     Ok(())
	/// }
	/// ```
	pub async fn sign(&mut self) -> Result<Transaction<P>, BuilderError> {
		init_logger();
		let mut unsigned_tx = self.get_unsigned_tx().await?;
		let tx_bytes = unsigned_tx.get_hash_data().await?;

		let mut witnesses_to_add = Vec::new();
		for signer in &mut unsigned_tx.signers {
			if Self::is_account_signer(signer) {
				let account_signer = signer.as_account_signer().ok_or_else(|| {
					BuilderError::IllegalState("Failed to get account signer".to_string())
				})?;
				let acc = &account_signer.account;
				if acc.is_multi_sig() {
					return Err(BuilderError::IllegalState(
						"Transactions with multi-sig signers cannot be signed automatically."
							.to_string(),
					));
				}
				let key_pair = acc.key_pair().as_ref().ok_or_else(|| {
                    BuilderError::InvalidConfiguration(
                        format!("Cannot create transaction signature because account {} does not hold a private key.", acc.get_address()),
                    )
                })?;
				witnesses_to_add.push(Witness::create(tx_bytes.clone(), key_pair)?);
			} else {
				let contract_signer = signer.as_contract_signer().ok_or_else(|| {
					BuilderError::IllegalState(
						"Expected contract signer but found another type".to_string(),
					)
				})?;
				witnesses_to_add.push(Witness::create_contract_witness(
					contract_signer.verify_params().clone(),
				)?);
			}
		}
		for witness in witnesses_to_add {
			unsigned_tx.add_witness(witness);
		}

		Ok(unsigned_tx)
	}

	fn signers_contain_multi_sig_with_committee_member(&self, committee: &HashSet<H160>) -> bool {
		for signer in &self.signers {
			if let Some(account_signer) = signer.as_account_signer() {
				if account_signer.is_multi_sig() {
					if let Some(script) = &account_signer.account().verification_script() {
						// Get public keys, returning false if there's an error instead of unwrapping
						if let Ok(public_keys) = script.get_public_keys() {
							for pubkey in public_keys {
								let hash = public_key_to_script_hash(&pubkey);
								if committee.contains(&hash) {
									return true;
								}
							}
						}
					}
				}
			}
		}

		false
	}

	/// Sets the signers for the transaction.
	///
	/// Signers are entities that authorize the transaction. They can be accounts, contracts,
	/// or other entities that can provide a signature or verification method.
	///
	/// # Arguments
	///
	/// * `signers` - A vector of `Signer` objects representing the transaction signers.
	///
	/// # Returns
	///
	/// A `Result` containing a mutable reference to the `TransactionBuilder` for method chaining,
	/// or a `TransactionError` if there are duplicate signers or if adding the signers would
	/// exceed the maximum allowed number of attributes.
	///
	/// # Examples
	///
	/// ```rust
	/// use neo3::neo_builder::{TransactionBuilder, Signer, AccountSigner};
	/// use neo3::neo_protocol::{Account, AccountTrait};
	///
	/// let account = Account::create().unwrap();
	/// let account_signer = AccountSigner::called_by_entry(&account).unwrap();
	/// let signer: Signer = account_signer.into();
	///
	/// # use neo3::neo_clients::HttpProvider;
	/// let mut tx_builder: TransactionBuilder<'_, HttpProvider> = TransactionBuilder::default();
	/// tx_builder.set_signers(vec![signer]).unwrap();
	/// ```
	pub fn set_signers(&mut self, signers: Vec<Signer>) -> Result<&mut Self, TransactionError> {
		if self.contains_duplicate_signers(&signers) {
			return Err(TransactionError::TransactionConfiguration(
				"Cannot add multiple signers concerning the same account.".to_string(),
			));
		}

		self.check_and_throw_if_max_attributes_exceeded(signers.len(), self.attributes.len())?;

		self.signers = signers;
		Ok(self)
	}

	/// Adds transaction attributes to the transaction.
	///
	/// Transaction attributes provide additional metadata or functionality to the transaction.
	/// This method checks for duplicate attribute types and ensures the total number of attributes
	/// does not exceed the maximum allowed.
	///
	/// # Arguments
	///
	/// * `attributes` - A vector of `TransactionAttribute` objects to add to the transaction.
	///
	/// # Returns
	///
	/// A `Result` containing a mutable reference to the `TransactionBuilder` for method chaining,
	/// or a `TransactionError` if adding the attributes would exceed the maximum allowed number
	/// of attributes or if an attribute of the same type already exists.
	///
	/// # Examples
	///
	/// ```rust
	/// use neo3::neo_builder::{TransactionBuilder, TransactionAttribute};
	/// use neo3::neo_clients::{HttpProvider, RpcClient};
	///
	/// let provider = HttpProvider::new("https://testnet1.neo.org:443").unwrap();
	/// let client = RpcClient::new(provider);
	/// let mut tx_builder = TransactionBuilder::with_client(&client);
	///
	/// // Add a high-priority attribute
	/// let high_priority_attr = TransactionAttribute::HighPriority;
	///
	/// // Add a not-valid-before attribute
	/// let not_valid_before_attr = TransactionAttribute::NotValidBefore { height: 1000 };
	///
	/// tx_builder.add_attributes(vec![high_priority_attr, not_valid_before_attr]).unwrap();
	/// ```
	pub fn add_attributes(
		&mut self,
		attributes: Vec<TransactionAttribute>,
	) -> Result<&mut Self, TransactionError> {
		self.check_and_throw_if_max_attributes_exceeded(
			self.signers.len(),
			self.attributes.len() + attributes.len(),
		)?;
		for attr in attributes {
			match attr {
				TransactionAttribute::HighPriority => {
					self.add_high_priority_attribute(attr)?;
				},
				TransactionAttribute::NotValidBefore { height } => {
					self.add_not_valid_before_attribute(attr)?;
				},
				TransactionAttribute::Conflicts { hash } => {
					self.add_conflicts_attribute(attr)?;
				},
				// TransactionAttribute::OracleResponse(oracle_response) => {
				//     self.add_oracle_response_attribute(oracle_response);
				// },
				_ => {
					// For other cases or any default, just add the attribute directly to the Vec
					self.attributes.push(attr);
				},
			}
		}
		Ok(self)
	}

	fn add_high_priority_attribute(
		&mut self,
		attr: TransactionAttribute,
	) -> Result<(), TransactionError> {
		if self.is_high_priority() {
			return Err(TransactionError::TransactionConfiguration(
				"A transaction can only have one HighPriority attribute.".to_string(),
			));
		}
		// Add the attribute to the attributes vector
		self.attributes.push(attr);
		Ok(())
	}

	fn add_not_valid_before_attribute(
		&mut self,
		attr: TransactionAttribute,
	) -> Result<(), TransactionError> {
		if self.has_attribute_of_type(TransactionAttribute::NotValidBefore { height: 0 }) {
			return Err(TransactionError::TransactionConfiguration(
				"A transaction can only have one NotValidBefore attribute.".to_string(),
			));
		}
		// Add the attribute to the attributes vector
		self.attributes.push(attr);
		Ok(())
	}

	fn add_conflicts_attribute(
		&mut self,
		attr: TransactionAttribute,
	) -> Result<(), TransactionError> {
		if self.has_attribute(&attr) {
			let hash = attr.get_hash().ok_or_else(|| {
				TransactionError::IllegalState(
					"Expected Conflicts attribute to have a hash".to_string(),
				)
			})?;

			return Err(TransactionError::TransactionConfiguration(format!(
				"There already exists a conflicts attribute for the hash {} in this transaction.",
				hash
			)));
		}
		// Add the attribute to the attributes vector
		self.attributes.push(attr);
		Ok(())
	}

	// Check if the attributes vector has an attribute of the specified type
	fn has_attribute_of_type(&self, attr_type: TransactionAttribute) -> bool {
		self.attributes.iter().any(|attr| match (attr, &attr_type) {
			(
				TransactionAttribute::NotValidBefore { .. },
				TransactionAttribute::NotValidBefore { .. },
			) => true,
			(TransactionAttribute::HighPriority, TransactionAttribute::HighPriority) => true,
			_ => false,
		})
	}

	fn has_attribute(&self, attr: &TransactionAttribute) -> bool {
		self.attributes.iter().any(|a| a == attr)
	}

	// Check specifically for the HighPriority attribute
	fn is_high_priority(&self) -> bool {
		self.has_attribute_of_type(TransactionAttribute::HighPriority)
	}

	fn contains_duplicate_signers(&self, signers: &Vec<Signer>) -> bool {
		let signer_list: Vec<H160> = signers.iter().map(|s| s.get_signer_hash().clone()).collect();
		let signer_set: HashSet<_> = signer_list.iter().collect();
		signer_list.len() != signer_set.len()
	}

	fn check_and_throw_if_max_attributes_exceeded(
		&self,
		total_signers: usize,
		total_attributes: usize,
	) -> Result<(), TransactionError> {
		let max_attributes = NeoConstants::MAX_TRANSACTION_ATTRIBUTES.try_into().map_err(|e| {
			TransactionError::IllegalState(format!(
				"Failed to convert MAX_TRANSACTION_ATTRIBUTES to usize: {}",
				e
			))
		})?;

		if total_signers + total_attributes > max_attributes {
			return Err(TransactionError::TransactionConfiguration(format!(
				"A transaction cannot have more than {} attributes (including signers).",
				NeoConstants::MAX_TRANSACTION_ATTRIBUTES
			)));
		}
		Ok(())
	}

	// pub fn is_high_priority(&self) -> bool {
	// 	self.attributes
	// 		.iter()
	// 		.any(|attr| matches!(attr, TransactionAttribute::HighPriority))
	// }

	async fn is_allowed_for_high_priority(&self) -> bool {
		let client = match self.client {
			Some(client) => client,
			None => return false, // If no client is available, we can't verify committee membership
		};

		let response =
			match client.get_committee().await.map_err(|e| TransactionError::ProviderError(e)) {
				Ok(response) => response,
				Err(_) => return false, // If we can't get committee info, assume not allowed
			};

		// Map the Vec<String> response to Vec<Hash160>
		let committee: HashSet<H160> = response
			.iter()
			.filter_map(|key_str| {
				// Convert the String to Hash160
				let public_key = Secp256r1PublicKey::from_encoded(key_str)?;
				Some(public_key_to_script_hash(&public_key)) // Handle potential parsing errors gracefully
			})
			.collect();

		let signers_contain_committee_member = self
			.signers
			.iter()
			.map(|signer| signer.get_signer_hash())
			.any(|script_hash| committee.contains(&script_hash));

		if signers_contain_committee_member {
			return true;
		}

		return self.signers_contain_multi_sig_with_committee_member(&committee);
	}

	/// Checks if the sender account of this transaction can cover the network and system fees.
	/// If not, executes the given consumer supplying it with the required fee and the sender's GAS balance.
	///
	/// The check and potential execution of the consumer is only performed when the transaction is built, i.e., when calling `TransactionBuilder::sign` or `TransactionBuilder::get_unsigned_transaction`.
	/// - Parameter consumer: The consumer
	/// - Returns: This transaction builder (self)
	/// Checks if the sender account can cover the transaction fees and executes a callback if not.
	///
	/// This method allows you to provide a callback function that will be executed if the sender
	/// account does not have enough GAS to cover the network and system fees. The callback
	/// receives the required fee amount and the sender's current balance.
	///
	/// # Arguments
	///
	/// * `consumer` - A callback function that takes two `i64` parameters: the required fee and
	///   the sender's current balance.
	///
	/// # Returns
	///
	/// A `Result` containing a mutable reference to the `TransactionBuilder` for method chaining,
	/// or a `TransactionError` if a fee error handler is already set.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use neo3::neo_builder::TransactionBuilder;
	/// use neo3::neo_clients::{HttpProvider, RpcClient};
	///
	/// #[tokio::main]
	/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
	///     let provider = HttpProvider::new("https://testnet1.neo.org:443").unwrap();
	///     let client = RpcClient::new(provider);
	///     
	///     let mut tx_builder = TransactionBuilder::with_client(&client);
	///     
	///     // Add a callback for insufficient funds
	///     tx_builder.do_if_sender_cannot_cover_fees(|required_fee, balance| {
	///         println!("Insufficient funds: Required {} GAS, but only have {} GAS",
	///             required_fee as f64 / 100_000_000.0,
	///             balance as f64 / 100_000_000.0);
	///     })?;
	///     
	///     Ok(())
	/// }
	/// ```
	pub fn do_if_sender_cannot_cover_fees<F>(
		&mut self,
		mut consumer: F,
	) -> Result<&mut Self, TransactionError>
	where
		F: FnMut(i64, i64) + Send + Sync + 'static,
	{
		if self.fee_error.is_some() {
			return Err(TransactionError::IllegalState(
                "Cannot handle a consumer for this case, since an exception will be thrown if the sender cannot cover the fees.".to_string(),
            ));
		}
		let consumer = RefCell::new(consumer);
		self.fee_consumer = Some(Box::new(move |fee, balance| {
			let mut consumer = consumer.borrow_mut();
			consumer(fee, balance);
		}));
		Ok(self)
	}

	/// Checks if the sender account of this transaction can cover the network and system fees.
	/// If not, otherwise throw an error created by the provided supplier.
	///
	/// The check and potential throwing of the exception is only performed when the transaction is built, i.e., when calling `TransactionBuilder::sign` or `TransactionBuilder::get_unsigned_transaction`.
	/// - Parameter error: The error to throw
	/// - Returns: This transaction builder (self)
	pub fn throw_if_sender_cannot_cover_fees(
		&mut self,
		error: TransactionError,
	) -> Result<&mut Self, TransactionError> {
		if self.fee_consumer.is_some() {
			return Err(TransactionError::IllegalState(
                "Cannot handle a supplier for this case, since a consumer will be executed if the sender cannot cover the fees.".to_string(),
            ));
		}
		self.fee_error = Some(error);
		Ok(self)
	}

	async fn can_send_cover_fees(&self, fees: u64) -> Result<bool, BuilderError> {
		let balance = self.get_sender_balance().await?;
		Ok(balance >= fees)
	}

	// async fn get_sender_gas_balance(&self) -> Result<u64, BuilderError> {
	// 	let sender_hash = self.signers[0].get_signer_hash();
	// 	let result = NEO_INSTANCE
	// 		.read()
	// 		.unwrap()
	// 		.invoke_function(
	// 			&H160::from(Self::GAS_TOKEN_HASH),
	// 			Self::BALANCE_OF_FUNCTION.to_string(),
	// 			vec![sender_hash.into()],
	// 			vec![],
	// 		)
	// 		.request()
	// 		.await?;
	//
	// 	Ok(result.stack[0].as_int().unwrap() as u64)
	// }
}
