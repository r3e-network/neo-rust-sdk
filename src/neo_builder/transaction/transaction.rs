use std::hash::{Hash, Hasher};

use base64::Engine;
use getset::{CopyGetters, Getters, MutGetters, Setters};
use primitive_types::U256;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use serde_with::__private__::DeError;
use tracing::info;

use crate::{
	Bytes,
	builder::{BuilderError, Signer, TransactionAttribute, TransactionError, Witness},
	codec::{Decoder, Encoder, NeoSerializable, VarSizeTrait},
	config::NeoConstants,
	crypto::HashableForVec,
	neo_clients::{APITrait, HttpProvider, JsonRpcProvider, RpcClient},
	neo_protocol::{ApplicationLog, RawTransaction},
	neo_types::NameOrAddress,
};

/// A Neo N3 blockchain transaction.
///
/// The `Transaction` struct represents a transaction on the Neo N3 blockchain. It contains
/// all the necessary information for a transaction, including version, nonce, validity period,
/// signers, fees, attributes, script, and witnesses.
///
/// # Fields
///
/// * `network` - An optional reference to an RPC client for network operations.
/// * `version` - The transaction version.
/// * `nonce` - A random number to prevent transaction duplication.
/// * `valid_until_block` - The block height until which the transaction is valid.
/// * `signers` - A list of transaction signers.
/// * `size` - The size of the transaction in bytes.
/// * `sys_fee` - The system fee for the transaction.
/// * `net_fee` - The network fee for the transaction.
/// * `attributes` - Transaction attributes.
/// * `script` - The transaction script.
/// * `witnesses` - Transaction witnesses (signatures).
/// * `block_count_when_sent` - The block count when the transaction was sent.
///
/// # Examples
///
/// ```rust,no_run
/// use neo3::neo_builder::{Transaction, TransactionBuilder};
/// use neo3::neo_clients::{HttpProvider, RpcClient};
///
/// fn example() -> Result<(), Box<dyn std::error::Error>> {
///     // Create a new transaction using TransactionBuilder
///     // let tx = tx_builder.build()?;
///
///     // Transactions are typically created using the TransactionBuilder
///     let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
///     let client = RpcClient::new(provider);
///     // let mut tx_builder = TransactionBuilder::with_client(&client);
///     Ok(())
/// }
/// ```
#[derive(Serialize, Getters, Setters, MutGetters, CopyGetters, Debug, Clone)]
pub struct Transaction<'a, P: JsonRpcProvider + 'static> {
	#[serde(skip)]
	#[getset(get = "pub", set = "pub")]
	pub network: Option<&'a RpcClient<P>>,

	#[serde(rename = "version")]
	#[getset(get = "pub", set = "pub")]
	pub version: u8,

	#[serde(rename = "nonce")]
	#[getset(get = "pub", set = "pub")]
	pub nonce: u32,

	#[serde(rename = "validuntilblock")]
	#[getset(get = "pub", set = "pub")]
	pub valid_until_block: u32,

	#[serde(rename = "signers")]
	#[getset(get = "pub", set = "pub")]
	pub signers: Vec<Signer>,

	#[serde(rename = "size")]
	#[getset(get = "pub", set = "pub")]
	pub size: i32,

	#[serde(rename = "sysfee")]
	#[getset(get = "pub", set = "pub")]
	pub sys_fee: i64,

	#[serde(rename = "netfee")]
	#[getset(get = "pub", set = "pub")]
	pub net_fee: i64,

	#[serde(rename = "attributes")]
	#[getset(get = "pub", set = "pub")]
	pub attributes: Vec<TransactionAttribute>,

	#[serde(rename = "script")]
	#[getset(get = "pub", set = "pub")]
	pub script: Bytes,

	#[serde(rename = "witnesses")]
	#[getset(get = "pub", set = "pub")]
	pub witnesses: Vec<Witness>,

	// #[serde(rename = "blocktime")]
	// #[getset(get = "pub", set = "pub")]
	// pub block_time: Option<i32>,
	#[serde(skip)]
	pub(crate) block_count_when_sent: Option<u32>,
}

impl<'a, P: JsonRpcProvider + 'static> Default for Transaction<'a, P> {
	fn default() -> Self {
		Transaction {
			network: None,
			version: Default::default(),
			nonce: Default::default(),
			valid_until_block: Default::default(),
			signers: Default::default(),
			size: Default::default(),
			sys_fee: Default::default(),
			net_fee: Default::default(),
			attributes: Default::default(),
			script: Default::default(),
			witnesses: Default::default(),
			// block_time: Default::default(),
			block_count_when_sent: None,
		}
	}
}

impl<'de, 'a, P: JsonRpcProvider + 'static> Deserialize<'de> for Transaction<'a, P> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let value = Value::deserialize(deserializer)?;

		// Example for version, apply similar logic for other fields
		let version = value
			.get("version")
			.ok_or(DeError::missing_field("version"))?
			.as_u64()
			.ok_or(DeError::custom("invalid type for version"))? as u8;

		// Production-ready field deserialization with proper error handling
		let nonce = value
			.get("nonce")
			.and_then(|v| v.as_i64())
			.ok_or(DeError::custom("Missing or invalid nonce field"))? as u32;

		let valid_until_block = value
			.get("validuntilblock")
			.and_then(|v| v.as_i64())
			.ok_or(DeError::custom("Missing or invalid validuntilblock field"))?
			as u32;
		// Continue for other fields...

		// For Vec<T> fields like signers, attributes, witnesses, you might deserialize them like this:
		// This assumes that Signer, TransactionAttribute, Witness can be deserialized directly from serde_json::Value
		let signers: Vec<Signer> =
			serde_json::from_value(value["signers"].clone()).map_err(DeError::custom)?;
		let attributes: Vec<TransactionAttribute> =
			serde_json::from_value(value["attributes"].clone()).map_err(DeError::custom)?;
		let witnesses: Vec<Witness> =
			serde_json::from_value(value["witnesses"].clone()).map_err(DeError::custom)?;

		let script_value = value
			.get("script")
			.ok_or_else(|| DeError::missing_field("script"))?
			.as_str()
			.ok_or_else(|| DeError::custom("Missing or invalid script field"))?;
		let script: Bytes = base64::engine::general_purpose::STANDARD
			.decode(script_value)
			.map_err(DeError::custom)?;

		// Complete field deserialization with comprehensive error handling
		let size = value
			.get("size")
			.and_then(|v| v.as_i64())
			.ok_or(DeError::custom("Missing or invalid size field"))? as i32;

		// Neo N3 RPC returns fees as strings (e.g. "9007810"), not JSON integers
		let sys_fee = value
			.get("sysfee")
			.and_then(|v| v.as_str().and_then(|s| s.parse::<i64>().ok()).or_else(|| v.as_i64()))
			.ok_or(DeError::custom("Missing or invalid sysfee field"))?;

		let net_fee = value
			.get("netfee")
			.and_then(|v| v.as_str().and_then(|s| s.parse::<i64>().ok()).or_else(|| v.as_i64()))
			.ok_or(DeError::custom("Missing or invalid netfee field"))?;

		Ok(Transaction {
			network: None,
			version,
			nonce,
			valid_until_block,
			signers,
			size,
			sys_fee,
			net_fee,
			attributes,
			script,
			witnesses,
			block_count_when_sent: None,
		})
	}
}

// impl<P: JsonRpcClient + 'static> DeserializeOwned for Transaction<P> {}

impl<'a, P: JsonRpcProvider + 'static> Hash for Transaction<'a, P> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.version.hash(state);
		self.nonce.hash(state);
		self.valid_until_block.hash(state);
		self.signers.hash(state);
		self.sys_fee.hash(state);
		self.net_fee.hash(state);
		self.attributes.hash(state);
		self.script.hash(state);
		self.witnesses.hash(state);
	}
}

impl<'a, T: JsonRpcProvider + 'static> Transaction<'a, T> {
	const HEADER_SIZE: usize = 25;
	pub fn new() -> Self {
		Self::default()
	}

	/// Convenience function for sending a new payment transaction to the receiver.
	pub fn pay<K: Into<NameOrAddress>, V: Into<U256>>(_to: K, _value: V) -> Self {
		Transaction { ..Default::default() }
	}

	pub fn add_witness(&mut self, witness: Witness) {
		self.witnesses.push(witness);
	}

	pub async fn get_hash_data(&self) -> Result<Bytes, TransactionError> {
		let network = self.network.ok_or_else(|| {
			TransactionError::TransactionConfiguration(
				"Transaction network magic is not set".to_string(),
			)
		})?;

		let mut encoder = Encoder::new();
		self.try_serialize_without_witnesses(&mut encoder)?;
		let mut data = encoder.to_bytes().double_sha256();
		let network_value = network.network().await?;
		data.splice(0..0, network_value.to_le_bytes());

		Ok(data)
	}

	fn get_tx_id(&self) -> Result<primitive_types::H256, TransactionError> {
		let mut encoder = Encoder::new();
		self.try_serialize_without_witnesses(&mut encoder)?;
		let data = encoder.to_bytes().double_sha256();
		let reversed_data = data.iter().rev().cloned().collect::<Vec<u8>>();
		Ok(primitive_types::H256::from_slice(&reversed_data))
	}

	/// Returns the transaction identifier (hash) as used by Neo nodes.
	///
	/// This is derived from the unsigned transaction payload (witnesses are not included).
	pub fn tx_id(&self) -> Result<primitive_types::H256, TransactionError> {
		self.get_tx_id()
	}

	fn serialize_without_witnesses(&self, writer: &mut Encoder) {
		writer.write_u8(self.version);
		writer.write_u32(self.nonce);
		writer.write_i64(self.sys_fee);
		writer.write_i64(self.net_fee);
		writer.write_u32(self.valid_until_block);
		if let Err(e) = writer.write_serializable_variable_list(&self.signers) {
			tracing::warn!(error = %e, "Failed to encode transaction signers");
		}
		if let Err(e) = writer.write_serializable_variable_list(&self.attributes) {
			tracing::warn!(error = %e, "Failed to encode transaction attributes");
		}
		if let Err(e) = writer.write_var_bytes(&self.script) {
			tracing::warn!(error = %e, "Failed to encode transaction script");
		}
	}

	fn try_serialize_without_witnesses(
		&self,
		writer: &mut Encoder,
	) -> Result<(), TransactionError> {
		writer.write_u8(self.version);
		writer.write_u32(self.nonce);
		writer.write_i64(self.sys_fee);
		writer.write_i64(self.net_fee);
		writer.write_u32(self.valid_until_block);
		writer.write_var_int(self.signers.len() as i64).map_err(|e| {
			TransactionError::TransactionConfiguration(format!(
				"Failed to encode transaction signers length: {}",
				e
			))
		})?;
		for signer in &self.signers {
			signer.try_encode(writer)?;
		}
		writer.write_var_int(self.attributes.len() as i64).map_err(|e| {
			TransactionError::TransactionConfiguration(format!(
				"Failed to encode transaction attributes length: {}",
				e
			))
		})?;
		for attribute in &self.attributes {
			attribute.try_encode(writer)?;
		}
		writer.write_var_bytes(&self.script).map_err(|e| {
			TransactionError::TransactionConfiguration(format!(
				"Failed to encode transaction script: {}",
				e
			))
		})?;
		Ok(())
	}

	pub fn try_encode(&self, writer: &mut Encoder) -> Result<(), TransactionError> {
		self.try_serialize_without_witnesses(writer)?;
		writer.write_var_int(self.witnesses.len() as i64).map_err(|e| {
			TransactionError::TransactionConfiguration(format!(
				"Failed to encode transaction witnesses length: {}",
				e
			))
		})?;
		for witness in &self.witnesses {
			witness.try_encode(writer).map_err(|err| {
				let message = match err {
					BuilderError::InvalidScript(message) => message,
					other => other.to_string(),
				};
				TransactionError::TransactionConfiguration(format!(
					"Failed to encode transaction witness: {}",
					message
				))
			})?;
		}
		Ok(())
	}

	pub fn try_to_array(&self) -> Result<Vec<u8>, TransactionError> {
		let mut writer = Encoder::new();
		self.try_encode(&mut writer)?;
		Ok(writer.to_bytes())
	}

	/// Sends the transaction to the Neo N3 network.
	///
	/// This method validates the transaction, converts it to a hexadecimal string,
	/// and sends it to the network using the RPC client. It also records the current
	/// block count for transaction tracking purposes.
	///
	/// # Returns
	///
	/// A `Result` containing the `RawTransaction` response if successful,
	/// or a `TransactionError` if an error occurs.
	///
	/// # Errors
	///
	/// Returns an error if:
	/// * The number of signers does not match the number of witnesses
	/// * The transaction exceeds the maximum transaction size
	/// * The network client encounters an error when sending the transaction
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use neo3::neo_builder::{ScriptBuilder, AccountSigner, TransactionBuilder, CallFlags};
	/// use neo3::neo_clients::{HttpProvider, RpcClient, APITrait};
	/// use neo3::neo_protocol::{Account, AccountTrait};
	/// use neo3::neo_types::{Address, ContractParameter, ScriptHash, AddressExtension};
	/// use std::str::FromStr;
	///
	/// #[tokio::main]
	/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
	///     // Connect to Neo N3 TestNet
	///     let provider = HttpProvider::new("https://testnet1.neo.org:443").unwrap();
	///     let client = RpcClient::new(provider);
	///     
	///     // Load your account from WIF or keystore
	///     let private_key = "your_private_key_here";
	///     let account = Account::from_wif(private_key)?;
	///     
	///     // Create a token transfer transaction
	///     let from_address = account.get_address();
	///     let to_address = "NdUL5oDPD159KeFpD5A9zw5xNF1xLX6nLT";
	///     let amount = 1_000_000; // 1 GAS (with 8 decimals)
	///     
	///     let mut script_builder = ScriptBuilder::new();
	///     let script = script_builder.contract_call(
	///         &ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?, // GAS token
	///         "transfer",
	///         &[
	///             ContractParameter::from(account.get_script_hash()),
	///             ContractParameter::from(Address::from_str(to_address)?.address_to_script_hash()?),
	///             ContractParameter::integer(amount),
	///             ContractParameter::any()
	///         ],
	///         Some(CallFlags::All)
	///     )?;
	///     
	///     // Build the transaction with proper fee calculation
	///     let mut tx_builder = TransactionBuilder::with_client(&client);
	///     tx_builder.extend_script(script.to_bytes());
	///     let account_signer = AccountSigner::called_by_entry(&account)?;
	///     tx_builder.set_signers(vec![account_signer.into()])?;
	///     tx_builder.valid_until_block(client.get_block_count().await? + 2400)?; // ~1 hour validity
	///     
	///     // Sign the transaction and get signed transaction
	///     let signed_transaction = tx_builder.sign().await?;
	///     
	///     // Send the transaction to the network
	///     let mut transaction = signed_transaction;
	///     let response = transaction.send_tx().await?;
	///     println!("✅ Transaction sent successfully!");
	///     println!("Transaction ID: {}", response.hash);
	///     println!("Transferred {} GAS to {}", amount as f64 / 100_000_000.0, to_address);
	///     
	///     Ok(())
	/// }
	/// ```
	pub async fn send_tx(&mut self) -> Result<RawTransaction, TransactionError>
// where
	// 	P: APITrait,
	{
		let network = self.network.ok_or_else(|| {
			TransactionError::IllegalState("Transaction network is not set".to_string())
		})?;

		if self.signers.len() != self.witnesses.len() {
			return Err(TransactionError::TransactionConfiguration(
				"The transaction does not have the same number of signers and witnesses."
					.to_string(),
			));
		}
		if self.size() > &(NeoConstants::MAX_TRANSACTION_SIZE as i32) {
			return Err(TransactionError::TransactionConfiguration(
				"The transaction exceeds the maximum transaction size.".to_string(),
			));
		}
		let hex = hex::encode(self.try_to_array().map_err(|err| {
			TransactionError::TransactionConfiguration(format!(
				"Failed to serialize transaction for sending: {}",
				err
			))
		})?);
		// self.throw()?;
		self.block_count_when_sent = Some(network.get_block_count().await?);
		network.send_raw_transaction(hex).await.map_err(TransactionError::ProviderError)
	}

	/// Tracks a transaction until it appears in a block.
	///
	/// This method waits for the transaction to be included in a block by monitoring new blocks
	/// as they are added to the blockchain. It returns when the transaction is found in a block.
	///
	/// # Arguments
	///
	/// * `max_blocks` - The maximum number of blocks to wait for the transaction to appear
	///
	/// # Returns
	///
	/// * `Ok(())` - If the transaction is found in a block
	/// * `Err(TransactionError)` - If the transaction is not found after waiting for `max_blocks` blocks
	///
	/// # Errors
	///
	/// Returns an error if:
	/// * The transaction has not been sent yet
	/// * The maximum number of blocks is reached without finding the transaction
	/// * There is an error communicating with the blockchain
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use neo3::neo_builder::{ScriptBuilder, AccountSigner, TransactionBuilder, CallFlags};
	/// use neo3::neo_clients::{HttpProvider, RpcClient, APITrait};
	/// use neo3::neo_protocol::{Account, AccountTrait};
	/// use neo3::neo_types::{ContractParameter, ScriptHash};
	/// use std::str::FromStr;
	///
	/// #[tokio::main]
	/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
	///     // Initialize provider and client
	///     let provider = HttpProvider::new("https://testnet1.neo.org:443").unwrap();
	///     let client = RpcClient::new(provider);
	///     
	///     // Load account and create smart contract deployment transaction
	///     let account = Account::from_wif("your_private_key_here")?;
	///     
	///     // Load contract NEF and manifest
	///     let nef_bytes = std::fs::read("path/to/contract.nef")?;
	///     let manifest_bytes = std::fs::read("path/to/contract.manifest.json")?;
	///     
	///     let mut script_builder = ScriptBuilder::new();
	///     let deploy_script = script_builder.contract_call(
	///         &ScriptHash::from_str("0xfffdc93764dbaddd97c48f252a53ea4643faa3fd")?, // ContractManagement
	///         "deploy",
	///         &[
	///             ContractParameter::byte_array(nef_bytes),
	///             ContractParameter::byte_array(manifest_bytes),
	///             ContractParameter::any() // Optional data parameter
	///         ],
	///         Some(CallFlags::All)
	///     )?;
	///     
	///     // Build transaction with appropriate settings for contract deployment
	///     let mut tx_builder = TransactionBuilder::with_client(&client);
	///     tx_builder.extend_script(deploy_script.to_bytes());
	///     let account_signer = AccountSigner::called_by_entry(&account)?;
	///     tx_builder.set_signers(vec![account_signer.into()])?;
	///     tx_builder.valid_until_block(client.get_block_count().await? + 2400)?;
	///     
	///     // Sign the transaction and get signed transaction
	///     let signed_transaction = tx_builder.sign().await?;
	///     
	///     // Send the transaction
	///     let mut transaction = signed_transaction;
	///     let response = transaction.send_tx().await?;
	///     println!("✅ Contract deployment transaction sent!");
	///     println!("Transaction ID: {}", response.hash);
	///     
	///     // Track the transaction until confirmation
	///     println!("⏳ Waiting for transaction confirmation...");
	///     transaction.track_tx(15).await?; // Wait up to 15 blocks (~15 seconds)
	///     println!("🎉 Contract deployment confirmed!");
	///     
	///     Ok(())
	/// }
	/// ```
	pub async fn track_tx(&self, max_blocks: u32) -> Result<(), TransactionError> {
		if max_blocks == 0 {
			return Err(TransactionError::IllegalState(
				"max_blocks must be greater than 0".to_string(),
			));
		}

		let network = self.network.ok_or_else(|| {
			TransactionError::IllegalState("Transaction network is not set".to_string())
		})?;

		let block_count_when_sent =
			self.block_count_when_sent.ok_or(TransactionError::IllegalState(
				"Cannot track transaction before it has been sent.".to_string(),
			))?;

		let tx_id = self.get_tx_id()?;
		let mut next_block = block_count_when_sent;
		let end_block = block_count_when_sent.saturating_add(max_blocks);

		while next_block < end_block {
			let latest_block = network.get_block_count().await?;
			let scan_up_to = std::cmp::min(latest_block, end_block);

			while next_block < scan_up_to {
				let block_hash = network.get_block_hash(next_block).await?;
				let block = network.get_block(block_hash, true).await?;

				if let Some(transactions) = &block.transactions
					&& transactions.iter().any(|tx| tx.hash == tx_id)
				{
					return Ok(());
				}

				next_block += 1;
			}

			if next_block >= end_block {
				break;
			}

			// Wait a bit before checking again
			tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
		}

		Err(TransactionError::IllegalState(format!(
			"Transaction {} not found after waiting for {} blocks",
			tx_id, max_blocks
		)))
	}

	/// Retrieves the application log for this transaction.
	///
	/// Application logs contain detailed information about the execution of a transaction,
	/// including notifications, stack items, and any exceptions that occurred during execution.
	///
	/// # Arguments
	///
	/// * `provider` - A provider implementing the `APITrait` to make the RPC call.
	///
	/// # Returns
	///
	/// A `Result` containing the `ApplicationLog` if successful,
	/// or a `TransactionError` if an error occurs.
	///
	/// # Errors
	///
	/// Returns an error if:
	/// * The transaction has not been sent yet
	/// * The transaction ID cannot be calculated
	/// * The provider encounters an error when retrieving the application log
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use neo3::neo_builder::{ScriptBuilder, AccountSigner, TransactionBuilder, CallFlags};
	/// use neo3::neo_clients::{HttpProvider, RpcClient, APITrait};
	/// use neo3::neo_protocol::{Account, AccountTrait};
	/// use neo3::neo_types::{ContractParameter, ScriptHash};
	/// use std::str::FromStr;
	///
	/// #[tokio::main]
	/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
	///     // Setup client connection
	///     let provider = HttpProvider::new("https://testnet1.neo.org:443").unwrap();
	///     let client = RpcClient::new(provider);
	///     
	///     // Load account for contract interaction
	///     let account = Account::from_wif("your_private_key_here")?;
	///     let contract_hash = ScriptHash::from_str("your_contract_hash_here")?;
	///     
	///     // Create a contract invocation transaction
	///     let mut script_builder = ScriptBuilder::new();
	///     let invoke_script = script_builder.contract_call(
	///         &contract_hash,
	///         "setValue", // Contract method name
	///         &[
	///             ContractParameter::string("myKey".to_string()),
	///             ContractParameter::string("myValue".to_string()),
	///             ContractParameter::integer(42)
	///         ],
	///         Some(CallFlags::All)
	///     )?;
	///     
	///     // Build and configure the transaction
	///     let mut tx_builder = TransactionBuilder::with_client(&client);
	///     tx_builder.extend_script(invoke_script.to_bytes());
	///     let account_signer = AccountSigner::called_by_entry(&account)?;
	///     tx_builder.set_signers(vec![account_signer.into()])?;
	///     tx_builder.valid_until_block(client.get_block_count().await? + 1200)?; // 20 minutes validity
	///     
	///     // Sign the transaction and get signed transaction
	///     let signed_transaction = tx_builder.sign().await?;
	///     
	///     // Send the transaction
	///     let mut transaction = signed_transaction;
	///     let response = transaction.send_tx().await?;
	///     println!("📤 Smart contract invocation sent!");
	///     println!("Transaction ID: {}", response.hash);
	///     
	///     // Wait for confirmation and get detailed execution results
	///     println!("⏳ Waiting for transaction confirmation...");
	///     transaction.track_tx(12).await?;
	///     println!("✅ Transaction confirmed!");
	///     
	///     // Analyze the execution results
	///     let app_log = transaction.get_application_log(&client).await?;
	///     println!("🔍 Transaction execution analysis:");
	///     if let Ok(execution) = app_log.get_first_execution() {
	///         println!("  Execution state: {:?}", execution.state);
	///         println!("  GAS consumed: {}", execution.gas_consumed);
	///         
	///         // Process contract notifications and events
	///         if !execution.notifications.is_empty() {
	///             println!("📋 Contract notifications:");
	///             for (i, notification) in execution.notifications.iter().enumerate() {
	///                 println!("  {}. Event: {} from contract {}",
	///                     i + 1, notification.event_name, notification.contract);
	///                 println!("     State: {:?}", notification.state);
	///             }
	///         }
	///         
	///         // Check execution stack for return values
	///         if !execution.stack.is_empty() {
	///             println!("📊 Return values: {:?}", execution.stack);
	///         }
	///         
	///         if execution.state.to_string() == "HALT" {
	///             println!("🎉 Smart contract executed successfully!");
	///         } else {
	///             println!("❌ Smart contract execution failed");
	///             if let Some(exception) = &execution.exception {
	///                 println!("   Exception: {}", exception);
	///             }
	///         }
	///     }
	///     
	///     Ok(())
	/// }
	/// ```
	pub async fn get_application_log<P>(
		&self,
		provider: &P,
	) -> Result<ApplicationLog, TransactionError>
	where
		P: APITrait,
	{
		if self.block_count_when_sent.is_none() {
			return Err(TransactionError::IllegalState(
				"Cannot get the application log before transaction has been sent.".to_string(),
			));
		}

		let hash = self.get_tx_id()?;
		info!("hash: {:?}", hash);

		// self.thro
		provider
			.get_application_log(hash)
			.await
			.map_err(|e| TransactionError::IllegalState(e.to_string()))
	}
}

// This commented-out code has been replaced by the send_tx method above

impl<'a, P: JsonRpcProvider + 'static> Eq for Transaction<'a, P> {}

impl<'a, P: JsonRpcProvider + 'static> PartialEq for Transaction<'a, P> {
	fn eq(&self, other: &Self) -> bool {
		self.version == other.version
			&& self.nonce == other.nonce
			&& self.valid_until_block == other.valid_until_block
			&& self.signers == other.signers
			&& self.sys_fee == other.sys_fee
			&& self.net_fee == other.net_fee
			&& self.attributes == other.attributes
			&& self.script == other.script
			&& self.witnesses == other.witnesses
	}
}

impl<'a, P: JsonRpcProvider + 'static> NeoSerializable for Transaction<'a, P> {
	type Error = TransactionError;

	fn size(&self) -> usize {
		Transaction::<HttpProvider>::HEADER_SIZE
			+ self.signers.var_size()
			+ self.attributes.var_size()
			+ self.script.var_size()
			+ self.witnesses.var_size()
	}

	fn encode(&self, writer: &mut Encoder) {
		self.serialize_without_witnesses(writer);
		if let Err(e) = writer.write_serializable_variable_list(&self.witnesses) {
			tracing::warn!(error = %e, "Failed to encode transaction witnesses");
		}
	}

	fn decode(reader: &mut Decoder) -> Result<Self, Self::Error>
	where
		Self: Sized,
	{
		let start = *reader.pointer();

		let version = reader.read_u8_safe()?;
		let nonce = reader.read_u32().map_err(|e| {
			TransactionError::TransactionConfiguration(format!("Failed to read nonce: {}", e))
		})?;
		let system_fee = reader.read_i64().map_err(|e| {
			TransactionError::TransactionConfiguration(format!("Failed to read system fee: {}", e))
		})?;
		let network_fee = reader.read_i64().map_err(|e| {
			TransactionError::TransactionConfiguration(format!("Failed to read network fee: {}", e))
		})?;
		let valid_until_block = reader.read_u32().map_err(|e| {
			TransactionError::TransactionConfiguration(format!(
				"Failed to read valid until block: {}",
				e
			))
		})?;

		// Read signers
		let signers: Vec<Signer> = reader
			.read_serializable_list_bounded::<Signer>(NeoConstants::MAX_SIGNER_SUBITEMS as usize)?;

		// Read attributes
		let attributes: Vec<TransactionAttribute> = reader
			.read_serializable_list_bounded::<TransactionAttribute>(
				NeoConstants::MAX_TRANSACTION_ATTRIBUTES as usize,
			)?;

		let script = reader.read_var_bytes_bounded(NeoConstants::MAX_TRANSACTION_SIZE as usize)?;

		let mut witnesses = vec![];
		if reader.available() > 0 {
			witnesses.append(&mut reader.read_serializable_list_bounded::<Witness>(signers.len())?);
		}

		let end = *reader.pointer();
		let tx_len = end.checked_sub(start).ok_or(TransactionError::InvalidTransaction)?;
		if tx_len > NeoConstants::MAX_TRANSACTION_SIZE as usize {
			return Err(TransactionError::TxTooLarge);
		}

		Ok(Self {
			network: None,
			version,
			nonce,
			valid_until_block,
			size: 0,
			sys_fee: system_fee,
			net_fee: network_fee,
			signers,
			attributes,
			script,
			witnesses,
			// block_time: None,
			block_count_when_sent: None,
		})
	}

	fn to_array(&self) -> Vec<u8> {
		self.try_to_array().unwrap_or_else(|err| {
			panic!(
				"failed to serialize transaction; use try_to_array for fallible handling: {}",
				err
			)
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		builder::{AccountSigner, SignerTrait, Witness, WitnessScope},
		config::NeoConstants,
		neo_builder::{OracleResponse, OracleResponseCode},
		neo_clients::{MockClient, MockProvider},
		neo_protocol::{Account, RawTransaction},
		prelude::Base64Encode,
	};
	use primitive_types::{H160, H256};
	use std::sync::Arc;
	use tokio::sync::Mutex;

	fn transaction_with_attribute(
		attribute: TransactionAttribute,
	) -> Transaction<'static, MockProvider> {
		let mut tx = Transaction::<MockProvider>::new();
		tx.attributes = vec![attribute];
		tx
	}

	fn transaction_with_signer(signer: Signer) -> Transaction<'static, MockProvider> {
		let mut tx = Transaction::<MockProvider>::new();
		tx.signers = vec![signer];
		tx
	}

	fn transaction_with_witness(witness: Witness) -> Transaction<'static, MockProvider> {
		let mut tx = Transaction::<MockProvider>::new();
		tx.witnesses = vec![witness];
		tx
	}

	#[test]
	fn test_transaction_equality_distinguishes_hidden_signer_state() {
		let signer = AccountSigner::none(&Account::from(H160::zero())).unwrap();
		let tx1 = transaction_with_signer(signer.clone().into());
		let mut tx2 = transaction_with_signer(signer.into());

		if let Signer::AccountSigner(account_signer) = &mut tx2.signers[0] {
			account_signer.get_allowed_contracts_mut().push(H160::zero());
		}

		assert_ne!(tx1, tx2);

		use std::collections::hash_map::DefaultHasher;

		let mut tx1_hasher = DefaultHasher::new();
		tx1.hash(&mut tx1_hasher);
		let mut tx2_hasher = DefaultHasher::new();
		tx2.hash(&mut tx2_hasher);

		assert_ne!(tx1_hasher.finish(), tx2_hasher.finish());
	}

	#[test]
	fn test_try_to_array_rejects_oversized_witness_invocation_script() {
		let tx = transaction_with_witness(Witness::from_scripts(
			vec![0_u8; NeoConstants::MAX_TRANSACTION_SIZE as usize + 1],
			vec![],
		));

		assert!(matches!(
			tx.try_to_array(),
			Err(TransactionError::TransactionConfiguration(message))
				if message.contains("invocation script")
		));
	}

	#[test]
	fn test_try_to_array_rejects_invalid_oracle_response_attribute() {
		let tx = transaction_with_attribute(TransactionAttribute::OracleResponse(OracleResponse {
			id: 1,
			response_code: OracleResponseCode::Success,
			result: "not-base64".to_string(),
		}));

		assert!(matches!(
			tx.try_to_array(),
			Err(TransactionError::TransactionConfiguration(message))
				if message.contains("valid base64")
		));
	}

	#[test]
	#[should_panic(
		expected = "failed to serialize transaction; use try_to_array for fallible handling"
	)]
	fn test_to_array_panics_on_invalid_oracle_response_attribute() {
		let tx = transaction_with_attribute(TransactionAttribute::OracleResponse(OracleResponse {
			id: 1,
			response_code: OracleResponseCode::Success,
			result: "not-base64".to_string(),
		}));

		let _ = tx.to_array();
	}

	#[test]
	fn test_try_to_array_rejects_signer_with_too_many_allowed_contracts() {
		let mut signer = AccountSigner::none(&Account::from(H160::zero())).unwrap();
		signer.set_scopes(vec![WitnessScope::CustomContracts]);
		signer
			.get_allowed_contracts_mut()
			.extend((0..=NeoConstants::MAX_SIGNER_SUBITEMS).map(|_| H160::zero()));

		let tx = transaction_with_signer(signer.into());

		assert!(matches!(
			tx.try_to_array(),
			Err(TransactionError::TransactionConfiguration(message))
				if message.contains("allowed contracts")
		));
	}

	#[test]
	fn test_deserialize_rejects_missing_script_field() {
		let value = serde_json::json!({
			"version": 0,
			"nonce": 1,
			"validuntilblock": 1,
			"signers": [],
			"size": 0,
			"sysfee": "0",
			"netfee": "0",
			"attributes": [],
			"witnesses": []
		});

		let result = serde_json::from_value::<Transaction<'static, MockProvider>>(value);
		assert!(result.is_err());
	}

	#[test]
	fn test_deserialize_rejects_non_string_script_field() {
		let value = serde_json::json!({
			"version": 0,
			"nonce": 1,
			"validuntilblock": 1,
			"signers": [],
			"size": 0,
			"sysfee": "0",
			"netfee": "0",
			"attributes": [],
			"script": 123,
			"witnesses": []
		});

		let result = serde_json::from_value::<Transaction<'static, MockProvider>>(value);
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_send_tx_rejects_signer_with_too_many_allowed_contracts() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		let client: &'static _ = {
			let mut mock_provider = mock_provider.lock().await;
			mock_provider
				.mock_get_block_count(1000)
				.await
				.mock_send_raw_transaction(RawTransaction { hash: H256::zero() })
				.await
				.mount_mocks()
				.await;
			Box::leak(Box::new(mock_provider.into_client()))
		};

		let mut signer = AccountSigner::none(&Account::from(H160::zero())).unwrap();
		signer.set_scopes(vec![WitnessScope::CustomContracts]);
		signer
			.get_allowed_contracts_mut()
			.extend((0..=NeoConstants::MAX_SIGNER_SUBITEMS).map(|_| H160::zero()));

		let mut tx = transaction_with_signer(signer.into());
		tx.network = Some(client);
		tx.witnesses = vec![Witness::default()];

		assert!(matches!(
			tx.send_tx().await,
			Err(TransactionError::TransactionConfiguration(message))
				if message.contains("allowed contracts")
		));
	}

	#[test]
	fn test_tx_id_rejects_invalid_oracle_response_attribute() {
		let tx = transaction_with_attribute(TransactionAttribute::OracleResponse(OracleResponse {
			id: 1,
			response_code: OracleResponseCode::Success,
			result: "not-base64".to_string(),
		}));

		assert!(matches!(
			tx.tx_id(),
			Err(TransactionError::TransactionConfiguration(message))
				if message.contains("valid base64")
		));
	}

	#[test]
	fn test_try_to_array_matches_legacy_for_valid_oracle_response_attribute() {
		let tx = transaction_with_attribute(TransactionAttribute::OracleResponse(OracleResponse {
			id: 7,
			response_code: OracleResponseCode::Success,
			result: vec![1_u8, 2, 3].to_base64(),
		}));

		assert_eq!(tx.try_to_array().unwrap(), tx.to_array());
	}
}
