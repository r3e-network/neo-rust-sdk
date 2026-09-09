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
use std::{
	cell::RefCell,
	collections::{HashMap, HashSet},
	fmt::Debug,
	hash::{Hash, Hasher},
	iter::Iterator,
	str::FromStr,
};

use crate::builder::SignerTrait;
use getset::{CopyGetters, Getters, MutGetters, Setters};
use hex_literal::hex;
use once_cell::sync::Lazy;
use primitive_types::H160;
// Import from neo_types
use crate::neo_types::{Bytes, ContractParameter, InvocationResult, ScriptHash};

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
	neo_config::{NeoConstants, NEOCONFIG},
	neo_crypto::{utils::ToHexString, Secp256r1PublicKey, Secp256r1Signature},
	neo_protocol::AccountTrait,
};

// Helper functions
use crate::neo_clients::public_key_to_script_hash;

// Import Account from neo_protocol
use crate::neo_protocol::Account;

#[derive(Getters, Setters, MutGetters, CopyGetters)]
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
	allows_transmission_on_fault: Option<bool>,
	multi_sig_signatures: HashMap<H160, Vec<(Secp256r1PublicKey, Secp256r1Signature)>>,
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
			.field("allows_transmission_on_fault", &self.allows_transmission_on_fault)
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
			allows_transmission_on_fault: self.allows_transmission_on_fault,
			multi_sig_signatures: self.multi_sig_signatures.clone(),
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
			&& self.allows_transmission_on_fault == other.allows_transmission_on_fault
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
		self.allows_transmission_on_fault.hash(state);
	}
}

impl<'a, P: JsonRpcProvider + 'static> Default for TransactionBuilder<'a, P> {
	fn default() -> Self {
		Self::new()
	}
}

pub static GAS_TOKEN_HASH: Lazy<ScriptHash> = Lazy::new(|| {
	// Compile-time validated hex string (avoids runtime parsing and panics).
	ScriptHash::from(hex!("d2a4cff31913016155e38e474a2c06d08be276cf"))
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
			nonce: rand::random::<u32>(),
			valid_until_block: None,
			signers: Vec::new(),
			additional_network_fee: 0,
			additional_system_fee: 0,
			attributes: Vec::new(),
			script: None,
			fee_consumer: None,
			fee_error: None,
			allows_transmission_on_fault: None,
			multi_sig_signatures: HashMap::new(),
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
			nonce: rand::random::<u32>(),
			valid_until_block: None,
			signers: Vec::new(),
			additional_network_fee: 0,
			additional_system_fee: 0,
			attributes: Vec::new(),
			script: None,
			fee_consumer: None,
			fee_error: None,
			allows_transmission_on_fault: None,
			multi_sig_signatures: HashMap::new(),
		}
	}

	/// Allows building transactions even when script invocation ends in `FAULT`.
	///
	/// By default, the SDK refuses to build a transaction if `invokescript` returns a fault state.
	/// This method overrides that behavior for this builder instance.
	pub fn allow_transmission_on_fault(&mut self) -> &mut Self {
		self.allows_transmission_on_fault = Some(true);
		self
	}

	/// Forces the builder to reject `FAULT` results from script invocation.
	///
	/// This overrides any global `NEOCONFIG` setting for this builder instance.
	pub fn disallow_transmission_on_fault(&mut self) -> &mut Self {
		self.allows_transmission_on_fault = Some(false);
		self
	}

	/// Registers a collected signature from one participant of a multi-sig signer.
	///
	/// Call this method once for each signature collected from different participants
	/// before calling `sign()`. The `signer_hash` must match the hash of the multisig account.
	///
	/// # Arguments
	///
	/// * `signer_hash` - The script hash of the multisig account
	/// * `public_key` - The public key that produced the signature
	/// * `signature` - The signature over the transaction hash data
	///
	/// # Returns
	///
	/// A mutable reference to the `TransactionBuilder` for method chaining.
	pub fn add_multi_sig_signature(
		&mut self,
		signer_hash: &H160,
		public_key: Secp256r1PublicKey,
		signature: Secp256r1Signature,
	) -> &mut Self {
		self.multi_sig_signatures
			.entry(*signer_hash)
			.or_default()
			.push((public_key, signature));
		self
	}

	/// Registers all collected signatures for a multi-sig signer at once, replacing any previously stored.
	///
	/// # Arguments
	///
	/// * `signer_hash` - The script hash of the multisig account
	/// * `signatures` - Vector of (public_key, signature) pairs collected from participants
	///
	/// # Returns
	///
	/// A mutable reference to the `TransactionBuilder` for method chaining.
	pub fn set_multi_sig_signatures(
		&mut self,
		signer_hash: &H160,
		signatures: Vec<(Secp256r1PublicKey, Secp256r1Signature)>,
	) -> &mut Self {
		self.multi_sig_signatures.insert(*signer_hash, signatures);
		self
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
			Err(TransactionError::ScriptFormat(format!("Could not find a signer with script hash {}. Make sure to add the signer before calling this method.", sender)))
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
		let script = self.script.as_ref().ok_or(TransactionError::NoScript)?;
		if script.is_empty() {
			return Err(TransactionError::EmptyScript);
		}

		let client = self
			.client
			.ok_or_else(|| TransactionError::IllegalState("Client is not set".to_string()))?;

		let result = client
			.rpc_client()
			.invoke_script(script.to_hex_string(), self.signers.clone())
			.await
			.map_err(TransactionError::ProviderError)?;
		Ok(result)
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
	pub async fn build(&mut self) -> Result<Transaction<'_, P>, TransactionError> {
		self.get_unsigned_tx().await
	}

	// Get unsigned transaction
	/// Validates the transaction builder configuration before building.
	///
	/// This method performs pre-flight validation to catch configuration errors
	/// early, before attempting to build or sign the transaction.
	///
	/// # Validation Checks
	///
	/// - At least one signer is present
	/// - Script is set and non-empty
	/// - No duplicate signers
	/// - Signer count does not exceed maximum
	/// - Client is set (required for network operations)
	///
	/// # Returns
	///
	/// `Ok(())` if validation passes, or a `TransactionError` describing the issue.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use neo3::neo_builder::TransactionBuilder;
	/// use neo3::neo_clients::{HttpProvider, RpcClient};
	///
	/// let provider = HttpProvider::new("https://testnet1.neo.org:443").unwrap();
	/// let client = RpcClient::new(provider);
	/// let mut tx_builder = TransactionBuilder::with_client(&client);
	///
	/// // Validate before building
	/// match tx_builder.validate() {
	///     Ok(()) => println!("Configuration is valid"),
	///     Err(e) => println!("Validation failed: {}", e),
	/// }
	/// ```
	pub fn validate(&self) -> Result<(), TransactionError> {
		// Check signers
		if self.signers.is_empty() {
			return Err(TransactionError::NoSigners);
		}

		// Check for duplicate signers
		let mut seen_signers = std::collections::HashSet::new();
		for signer in &self.signers {
			let signer_hash = signer.get_signer_hash();
			if !seen_signers.insert(signer_hash) {
				return Err(TransactionError::DuplicateSigner);
			}
		}

		// Check signer limits
		if self.signers.len() > NeoConstants::MAX_SIGNER_SUBITEMS as usize {
			return Err(TransactionError::TooManySigners);
		}

		// Check script
		match &self.script {
			None => return Err(TransactionError::NoScript),
			Some(script) if script.is_empty() => return Err(TransactionError::EmptyScript),
			Some(_) => {},
		}

		// Check client is set
		if self.client.is_none() {
			return Err(TransactionError::IllegalState(
				"Client is not set. Use with_client() to set an RPC client.".to_string(),
			));
		}

		Ok(())
	}

	/// Checks if the transaction builder is ready to build a transaction.
	///
	/// This is a convenience method that returns `true` if `validate()` would succeed.
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
	///
	/// if tx_builder.is_ready() {
	///     println!("Ready to build transaction");
	/// }
	/// ```
	pub fn is_ready(&self) -> bool {
		self.validate().is_ok()
	}

	pub async fn get_unsigned_tx(&mut self) -> Result<Transaction<'_, P>, TransactionError> {
		// Perform pre-flight validation (checks signers, script, client, etc.)
		self.validate()?;

		// Dedup signers — remove all duplicates, not just consecutive ones
		let mut seen = Vec::with_capacity(self.signers.len());
		self.signers.retain(|s| {
			if seen.iter().any(|prev| prev == s) {
				false
			} else {
				seen.push(s.clone());
				true
			}
		});

		// Client is guaranteed to be Some after validate()
		let client = self.client.ok_or_else(|| {
			TransactionError::IllegalState("Client validated in validate()".to_string())
		})?;

		if self.valid_until_block.is_none() {
			self.valid_until_block = Some(
				self.fetch_current_block_count().await? + client.max_valid_until_block_increment()
					- 1,
			)
		}

		// Check committe member
		if self.is_high_priority() && !self.is_allowed_for_high_priority().await {
			return Err(TransactionError::IllegalState("This transaction does not have a committee member as signer. Only committee members can send transactions with high priority.".to_string()));
		}

		let system_fee = self.get_system_fee().await? + self.additional_system_fee as i64;

		let network_fee = self.get_network_fee().await? + self.additional_network_fee as i64;

		// Check sender balance if needed
		let tx = Transaction {
			network: Some(client),
			version: self.version,
			nonce: self.nonce,
			valid_until_block: self.valid_until_block.unwrap_or(100),
			size: 0,
			sys_fee: system_fee,
			net_fee: network_fee,
			signers: self.signers.clone(),
			attributes: self.attributes.clone(),
			script: self
				.script
				.clone()
				.ok_or_else(|| TransactionError::IllegalState("Script is not set".to_string()))?,
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
			let sender_balance_u64 = self.get_sender_balance().await?;
			let sender_balance = i64::try_from(sender_balance_u64).unwrap_or_else(|_| {
				tracing::warn!(
					balance = sender_balance_u64,
					"Sender balance exceeds i64::MAX; saturating for fee checks"
				);
				i64::MAX
			});
			let total_fee = network_fee + system_fee;
			if total_fee > sender_balance {
				fee_consumer(total_fee, sender_balance);
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
			.map_err(TransactionError::ProviderError)?;

		// Check if the VM execution resulted in a fault
		if response.has_state_fault() {
			// Get the current configuration for allowing transmission on fault.
			//
			// Prefer a per-builder override when set; otherwise fall back to the global `NEOCONFIG`.
			let allows_fault = match self.allows_transmission_on_fault {
				Some(allows_fault) => allows_fault,
				None => {
					NEOCONFIG
						.lock()
						.map_err(|_| {
							TransactionError::IllegalState("Failed to lock NEOCONFIG".to_string())
						})?
						.allows_transmission_on_fault
				},
			};

			// If transmission on fault is not allowed, return an error
			if !allows_fault {
				return Err(TransactionError::TransactionConfiguration(format!(
					"The vm exited due to the following exception: {}",
					response.exception.unwrap_or_else(|| "Unknown exception".to_string())
				)));
			}
			// Otherwise, we continue with the transaction despite the fault
		}

		i64::from_str(&response.gas_consumed)
			.map_err(|_| TransactionError::IllegalState("Failed to parse gas consumed".to_string()))
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

					// Use the actual verification script of the account if available
					let verification_script = if let Some(vs) = account.verification_script() {
						vs.clone()
					} else if account.is_multi_sig() {
						self.create_estimated_multi_sig_verification_script(account).map_err(
							|e| {
								TransactionError::IllegalState(format!(
									"Failed to create multi-sig verification script: {}",
									e
								))
							},
						)?
					} else {
						// For single sig, if we don't have a verification script, try to derive from public key
						if let Some(key_pair) = account.key_pair() {
							VerificationScript::from_public_key(key_pair.public_key_ref())
						} else {
							// Use a deterministic verification script with the correct encoded size.
							self.create_estimated_single_sig_verification_script().map_err(|e| {
								TransactionError::IllegalState(format!(
									"Failed to create single-sig verification script: {}",
									e
								))
							})?
						}
					};

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
		if !has_atleast_one_signing_account {
			return Err(TransactionError::TransactionConfiguration("A transaction requires at least one signing account (i.e. an AccountSigner). None was provided.".to_string()));
		}

		let tx_hex = tx.try_to_array().map(|bytes| bytes.to_hex_string()).map_err(|err| {
			TransactionError::TransactionConfiguration(format!(
				"Failed to serialize transaction for network fee calculation: {}",
				err
			))
		})?;
		let fee = client.calculate_network_fee(tx_hex).await?;
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
				.map_err(TransactionError::ProviderError)?
				.stack[0]
				.clone();

			return Ok(balance.as_int().ok_or_else(|| {
				TransactionError::IllegalState("Failed to parse balance as integer".to_string())
			})? as u64);
		}
		Err(TransactionError::InvalidSender)
	}

	fn create_estimated_single_sig_verification_script(
		&self,
	) -> Result<VerificationScript, TransactionError> {
		let estimated_public_key = Secp256r1PublicKey::from_encoded(Self::DUMMY_PUB_KEY)
			.ok_or_else(|| {
				TransactionError::IllegalState("Failed to create estimated public key".to_string())
			})?;
		Ok(VerificationScript::from_public_key(&estimated_public_key))
	}

	fn create_estimated_multi_sig_verification_script(
		&self,
		account: &Account,
	) -> Result<VerificationScript, TransactionError> {
		// Get the number of participants first to allocate capacity
		let nr_of_participants = account.get_nr_of_participants().map_err(|e| {
			TransactionError::IllegalState(format!("Failed to get number of participants: {}", e))
		})?;

		let mut pub_keys: Vec<Secp256r1PublicKey> = Vec::with_capacity(nr_of_participants as usize);

		for _ in 0..nr_of_participants {
			let estimated_public_key = Secp256r1PublicKey::from_encoded(Self::DUMMY_PUB_KEY)
				.ok_or_else(|| {
					TransactionError::IllegalState(
						"Failed to create estimated public key".to_string(),
					)
				})?;
			pub_keys.push(estimated_public_key);
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
		signer.get_type() == SignerType::AccountSigner
	}

	/// Creates a multi-sig witness from collected signatures.
	///
	/// This helper validates that enough valid participant signatures are present
	/// to meet the threshold requirement, then constructs the witness.
	fn create_multi_sig_witness_for_account(
		account: &Account,
		collected: &[(Secp256r1PublicKey, Secp256r1Signature)],
	) -> Result<Witness, BuilderError> {
		let threshold_u32 = account.get_signing_threshold().map_err(|e| {
			BuilderError::SignerConfiguration(format!(
				"Cannot derive multi-sig signing threshold: {}",
				e
			))
		})?;
		let threshold = u8::try_from(threshold_u32).map_err(|_| {
			BuilderError::SignerConfiguration(
				"Multi-sig signing threshold out of range for u8".to_string(),
			)
		})?;

		let verification_script = account.verification_script().as_ref().ok_or_else(|| {
			BuilderError::SignerConfiguration(
				"Multi-sig account has no verification script; cannot derive participant public keys."
					.to_string(),
			)
		})?;
		let public_keys = verification_script.get_public_keys().map_err(|e| {
			BuilderError::SignerConfiguration(format!(
				"Failed to read multi-sig participant public keys: {}",
				e
			))
		})?;

		// Only signatures produced by an actual participant count toward the threshold.
		let signatures: Vec<(Secp256r1PublicKey, Secp256r1Signature)> = collected
			.iter()
			.filter(|(pk, _)| public_keys.iter().any(|participant| participant == pk))
			.cloned()
			.collect();

		if signatures.len() < threshold as usize {
			return Err(BuilderError::SignerConfiguration(format!(
				"Multi-sig witness requires {} signatures but only {} valid participant signatures were collected.",
				threshold,
				signatures.len()
			)));
		}

		Witness::create_multi_sig_witness(threshold, signatures, public_keys)
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
	/// ```no_run
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
	///     let mut signed_tx = tx_builder.sign().await?;
	///     
	///     // Send the transaction to the network
	///     let tx_response = signed_tx.send_tx().await?;
	///     println!("Transaction sent: {:?}", tx_response);
	///     
	///     Ok(())
	/// }
	/// ```
	pub async fn sign(&mut self) -> Result<Transaction<'_, P>, BuilderError> {
		// Collect multi-sig signatures upfront to avoid borrow conflicts
		let multi_sig_signatures_snapshot = self.multi_sig_signatures.clone();
		
		let mut unsigned_tx = self.get_unsigned_tx().await?;
		let tx_bytes = unsigned_tx.get_hash_data().await?;

		let mut witnesses_to_add = Vec::with_capacity(unsigned_tx.signers.len());
		for signer in &mut unsigned_tx.signers {
			if Self::is_account_signer(signer) {
				let account_signer = signer.as_account_signer().ok_or_else(|| {
					BuilderError::IllegalState("Failed to get account signer".to_string())
				})?;
				let acc = &account_signer.account;
				if acc.is_multi_sig() {
					let signer_hash = *signer.get_signer_hash();
					let collected = multi_sig_signatures_snapshot.get(&signer_hash).cloned().unwrap_or_default();
					witnesses_to_add.push(Self::create_multi_sig_witness_for_account(acc, &collected)?);
					continue;
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
				TransactionAttribute::NotValidBefore { height: _ } => {
					self.add_not_valid_before_attribute(attr)?;
				},
				TransactionAttribute::Conflicts { hash: _ } => {
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
		self.attributes.iter().any(|attr| {
			matches!(
				(attr, &attr_type),
				(
					TransactionAttribute::NotValidBefore { .. },
					TransactionAttribute::NotValidBefore { .. },
				) | (TransactionAttribute::HighPriority, TransactionAttribute::HighPriority)
			)
		})
	}

	fn has_attribute(&self, attr: &TransactionAttribute) -> bool {
		self.attributes.iter().any(|a| a == attr)
	}

	// Check specifically for the HighPriority attribute
	fn is_high_priority(&self) -> bool {
		self.has_attribute_of_type(TransactionAttribute::HighPriority)
	}

	fn contains_duplicate_signers(&self, signers: &[Signer]) -> bool {
		let signer_list: Vec<H160> = signers.iter().map(|s| *s.get_signer_hash()).collect();
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

		let response = match client.get_committee().await.map_err(TransactionError::ProviderError) {
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
			.any(|script_hash| committee.contains(script_hash));

		if signers_contain_committee_member {
			return true;
		}

		self.signers_contain_multi_sig_with_committee_member(&committee)
	}

	/// Checks if the sender account of this transaction can cover the network and system fees.
	/// If not, executes the given consumer supplying it with the required fee and the sender's GAS balance.
	///
	/// The check and potential execution of the consumer is only performed when the transaction is built, i.e., when calling `TransactionBuilder::sign` or `TransactionBuilder::get_unsigned_transaction`.
	/// - Parameter consumer: The consumer
	/// - Returns: This transaction builder (self)
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
		consumer: F,
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

#[cfg(test)]
mod multisig_tests {
	use super::*;
	use crate::neo_crypto::KeyPair;

	#[test]
	fn test_create_multi_sig_witness_for_account_success() {
		// Create 3 key pairs
		let key_pair1 = KeyPair::new_random();
		let key_pair2 = KeyPair::new_random();
		let key_pair3 = KeyPair::new_random();

		let mut public_keys = vec![
			key_pair1.public_key(),
			key_pair2.public_key(),
			key_pair3.public_key(),
		];

		// Create a 2-of-3 multisig account
		let threshold = 2u32;
		let account = Account::multi_sig_from_public_keys(&mut public_keys, threshold).unwrap();

		// Sign a test message with 2 key pairs (meeting threshold)
		let message = vec![0u8; 32];
		let sig1 = key_pair1.private_key_ref().unwrap().sign_tx(&message).unwrap();
		let sig2 = key_pair2.private_key_ref().unwrap().sign_tx(&message).unwrap();

		let collected = vec![
			(key_pair1.public_key(), sig1),
			(key_pair2.public_key(), sig2),
		];

		// This should succeed
		let result = TransactionBuilder::<crate::neo_clients::HttpProvider>::create_multi_sig_witness_for_account(
			&account,
			&collected,
		);
		assert!(result.is_ok(), "Multi-sig witness creation should succeed with enough signatures");

		let witness = result.unwrap();
		// Verify the witness has a verification script
		assert!(!witness.verification.script().is_empty());
	}

	#[test]
	fn test_create_multi_sig_witness_for_account_below_threshold() {
		// Create 3 key pairs
		let key_pair1 = KeyPair::new_random();
		let key_pair2 = KeyPair::new_random();
		let key_pair3 = KeyPair::new_random();

		let mut public_keys = vec![
			key_pair1.public_key(),
			key_pair2.public_key(),
			key_pair3.public_key(),
		];

		// Create a 2-of-3 multisig account
		let threshold = 2u32;
		let account = Account::multi_sig_from_public_keys(&mut public_keys, threshold).unwrap();

		// Sign with only 1 key pair (below threshold)
		let message = vec![0u8; 32];
		let sig1 = key_pair1.private_key_ref().unwrap().sign_tx(&message).unwrap();

		let collected = vec![(key_pair1.public_key(), sig1)];

		// This should fail
		let result = TransactionBuilder::<crate::neo_clients::HttpProvider>::create_multi_sig_witness_for_account(
			&account,
			&collected,
		);
		assert!(result.is_err(), "Multi-sig witness creation should fail below threshold");

		let err = result.unwrap_err();
		assert!(matches!(err, BuilderError::SignerConfiguration(_)));
	}

	#[test]
	fn test_create_multi_sig_witness_ignores_non_participant_signatures() {
		// Create 3 participant key pairs
		let key_pair1 = KeyPair::new_random();
		let key_pair2 = KeyPair::new_random();
		let key_pair3 = KeyPair::new_random();

		// Create an outsider key pair (not a participant)
		let outsider_key_pair = KeyPair::new_random();

		let mut public_keys = vec![
			key_pair1.public_key(),
			key_pair2.public_key(),
			key_pair3.public_key(),
		];

		// Create a 2-of-3 multisig account
		let threshold = 2u32;
		let account = Account::multi_sig_from_public_keys(&mut public_keys, threshold).unwrap();

		// Sign with 1 participant and 1 outsider
		let message = vec![0u8; 32];
		let sig1 = key_pair1.private_key_ref().unwrap().sign_tx(&message).unwrap();
		let outsider_sig = outsider_key_pair.private_key_ref().unwrap().sign_tx(&message).unwrap();

		let collected = vec![
			(key_pair1.public_key(), sig1),
			(outsider_key_pair.public_key(), outsider_sig),
		];

		// This should fail because only 1 valid participant signature (outsider doesn't count)
		let result = TransactionBuilder::<crate::neo_clients::HttpProvider>::create_multi_sig_witness_for_account(
			&account,
			&collected,
		);
		assert!(result.is_err(), "Should reject non-participant signatures");
	}

	#[test]
	fn test_add_multi_sig_signature_and_set_multi_sig_signatures() {
		let mut builder: TransactionBuilder<crate::neo_clients::HttpProvider> = TransactionBuilder::new();
		
		let key_pair = KeyPair::new_random();
		let message = vec![0u8; 32];
		let signature = key_pair.private_key_ref().unwrap().sign_tx(&message).unwrap();
		
		let signer_hash = H160::random();
		
		// Test add_multi_sig_signature
		builder.add_multi_sig_signature(&signer_hash, key_pair.public_key(), signature.clone());
		assert_eq!(builder.multi_sig_signatures.get(&signer_hash).unwrap().len(), 1);
		
		// Add another signature
		let key_pair2 = KeyPair::new_random();
		let signature2 = key_pair2.private_key_ref().unwrap().sign_tx(&message).unwrap();
		builder.add_multi_sig_signature(&signer_hash, key_pair2.public_key(), signature2);
		assert_eq!(builder.multi_sig_signatures.get(&signer_hash).unwrap().len(), 2);
		
		// Test set_multi_sig_signatures (should replace)
		let new_signatures = vec![(key_pair.public_key(), signature)];
		builder.set_multi_sig_signatures(&signer_hash, new_signatures);
		assert_eq!(builder.multi_sig_signatures.get(&signer_hash).unwrap().len(), 1);
	}
}
