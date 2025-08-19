use std::hash::{Hash, Hasher};

use futures_util::TryFutureExt;
use getset::{CopyGetters, Getters, MutGetters, Setters};
use primitive_types::U256;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use serde_with::__private__::DeError;
use tracing::info;

use crate::{
	builder::{init_logger, Signer, TransactionAttribute, TransactionError, Witness},
	codec::{Decoder, Encoder, NeoSerializable, VarSizeTrait},
	config::NeoConstants,
	crypto::HashableForVec,
	neo_clients::{APITrait, HttpProvider, JsonRpcProvider, RpcClient},
	neo_protocol::{ApplicationLog, RawTransaction},
	neo_types::NameOrAddress,
	Bytes,
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

		// For bytes, assuming it's a Vec<u8> and stored as a base64 string in JSON
		let script: Bytes = base64::decode(value["script"].as_str().unwrap_or_default())
			.map_err(DeError::custom)?;

		// For optional fields
		let block_time = value["blocktime"].as_i64().map(|v| v as i32);

		// Complete field deserialization with comprehensive error handling
		let size = value
			.get("size")
			.and_then(|v| v.as_i64())
			.ok_or(DeError::custom("Missing or invalid size field"))? as i32;

		let sys_fee = value
			.get("sysfee")
			.and_then(|v| v.as_i64())
			.ok_or(DeError::custom("Missing or invalid sysfee field"))?;

		let net_fee = value
			.get("netfee")
			.and_then(|v| v.as_i64())
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
		self.to_array().hash(state);
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
		if self.network.is_none() {
			return Err(TransactionError::TransactionConfiguration(
				"Transaction network magic is not set".to_string(),
			));
		}
		let mut encoder = Encoder::new();
		self.serialize_without_witnesses(&mut encoder);
		let mut data = encoder.to_bytes().hash256();
		let network_value = self.network.as_ref().unwrap().network().await?;
		data.splice(0..0, network_value.to_be_bytes());

		Ok(data)
	}

	fn get_tx_id(&self) -> Result<primitive_types::H256, TransactionError> {
		let mut encoder = Encoder::new();
		self.serialize_without_witnesses(&mut encoder);
		let data = encoder.to_bytes().hash256();
		let reversed_data = data.iter().rev().cloned().collect::<Vec<u8>>();
		Ok(primitive_types::H256::from_slice(&reversed_data))
	}

	fn serialize_without_witnesses(&self, writer: &mut Encoder) {
		writer.write_u8(self.version);
		writer.write_u32(self.nonce);
		writer.write_i64(self.sys_fee);
		writer.write_i64(self.net_fee);
		writer.write_u32(self.valid_until_block);
		writer.write_serializable_variable_list(&self.signers);
		writer.write_serializable_variable_list(&self.attributes);
		writer.write_var_bytes(&self.script);
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
		let hex = hex::encode(self.to_array());
		// self.throw()?;
		self.block_count_when_sent = Some(self.network().unwrap().get_block_count().await?);
		self.network()
			.unwrap()
			.send_raw_transaction(hex)
			.await
			.map_err(|e| TransactionError::IllegalState(e.to_string()))
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
		let block_count_when_sent =
			self.block_count_when_sent.ok_or(TransactionError::IllegalState(
				"Cannot track transaction before it has been sent.".to_string(),
			))?;

		let tx_id = self.get_tx_id()?;
		let mut current_block = block_count_when_sent;
		let max_block = block_count_when_sent + max_blocks;

		while current_block <= max_block {
			// Get the current block count
			let latest_block = self.network().unwrap().get_block_count().await?;

			// If there are new blocks, check them for our transaction
			if latest_block > current_block {
				for block_index in current_block..latest_block {
					// Get the block hash for this index
					let block_hash = self.network().unwrap().get_block_hash(block_index).await?;

					// Get the block with full transaction details
					let block = self.network().unwrap().get_block(block_hash, true).await?;

					// Check if our transaction is in this block
					if let Some(transactions) = &block.transactions {
						for tx in transactions.iter() {
							if tx.hash == tx_id {
								return Ok(());
							}
						}
					}

					current_block = block_index + 1;
				}
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
		init_logger();
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
		self.to_array() == other.to_array()
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
		writer.write_serializable_variable_list(&self.witnesses);
	}

	fn decode(reader: &mut Decoder) -> Result<Self, Self::Error>
	where
		Self: Sized,
	{
		let version = reader.read_u8();
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
		let signers: Vec<Signer> = reader.read_serializable_list::<Signer>()?;

		// Read attributes
		let attributes: Vec<TransactionAttribute> =
			reader.read_serializable_list::<TransactionAttribute>()?;

		let script = reader.read_var_bytes()?.to_vec();

		let mut witnesses = vec![];
		if reader.available() > 0 {
			witnesses.append(&mut reader.read_serializable_list::<Witness>()?);
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
		let mut writer = Encoder::new();
		self.encode(&mut writer);
		writer.to_bytes()
	}
}
