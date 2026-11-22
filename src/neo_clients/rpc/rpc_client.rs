use async_trait::async_trait;
use futures_util::lock::Mutex;
use getset::Getters;
use primitive_types::{H160, H256};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;
use std::{
	collections::HashMap,
	fmt::{Debug, Display},
	future::Future,
	pin::Pin,
	str::FromStr,
	sync::Arc,
	time::Duration,
};
use tracing::trace;
use tracing_futures::Instrument;
use url::Url;

// Replace the generic import with specific imports
use crate::{
	neo_builder::{
		BuilderError, InteropService, ScriptBuilder, TransactionBuilder, TransactionSigner,
	},
	neo_clients::{APITrait, Http, JsonRpcProvider, ProviderError, RwClient},
};

use crate::{
	Address, ContractManifest, ContractParameter, ContractState, InvocationResult,
	NativeContractState, NefFile, StackItem, ValueExtension,
	builder::{Signer, Transaction, TransactionSendToken},
	config::NEOCONFIG,
	neo_protocol::*,
	neo_types::ScriptHashExtension,
	prelude::{Base64Encode, TryBase64Encode},
};

fn encode_hex_parameter_as_base64(value: &str, field_name: &str) -> Result<String, ProviderError> {
	value
		.try_to_base64()
		.map_err(|err| ProviderError::ParseError(format!("Invalid {field_name}: {err}")))
}

fn provider_error_from_builder(err: BuilderError) -> ProviderError {
	match err {
		BuilderError::ProviderError(err) => err,
		other => ProviderError::IllegalState(other.to_string()),
	}
}

fn try_transaction_signers<'a, I>(signers: I) -> Result<Vec<TransactionSigner>, ProviderError>
where
	I: IntoIterator<Item = &'a Signer>,
{
	signers
		.into_iter()
		.map(Signer::try_to_transaction_signer)
		.collect::<Result<Vec<_>, _>>()
		.map_err(provider_error_from_builder)
}

/// Node Clients
#[derive(Copy, Clone)]
pub enum NeoClient {
	/// RNEO
	NEO,
}

impl FromStr for NeoClient {
	type Err = ProviderError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let first_segment = s
			.split('/')
			.next()
			.ok_or(ProviderError::ParseError("Invalid client string format".to_string()))?;
		match first_segment.to_lowercase().as_str() {
			"neo" => Ok(NeoClient::NEO),
			_ => Err(ProviderError::UnsupportedNodeClient),
		}
	}
}

/// An abstract provider for interacting with the [Neo JSON RPC
/// API](https://github.com/neo/wiki/JSON-RPC). Must be instantiated
/// with a data transport which implements the `JsonRpcClient` trait
/// (e.g. HTTP, Websockets etc.)
///
/// # Example
///
/// ```no_run
/// use neo3::neo_clients::{HttpProvider, RpcClient, APITrait};
/// use neo3::neo_config::NeoConstants;
///
/// async fn foo() -> Result<(), Box<dyn std::error::Error>> {
///     let provider = HttpProvider::new(NeoConstants::SEED_1)?;
///     let client = RpcClient::new(provider);
///
///     let block = client.get_block_by_index(100u32, false).await?;
///     println!("Got block: {}", serde_json::to_string(&block)?);
///     Ok(())
/// }
/// ```
#[derive(Clone, Debug, Getters)]
pub struct RpcClient<P> {
	provider: P,
	interval: Option<Duration>,
	from: Option<Address>,
	_node_client: Arc<Mutex<Option<NeoVersion>>>,
}

impl<P> AsRef<P> for RpcClient<P> {
	fn as_ref(&self) -> &P {
		&self.provider
	}
}

// JSON RPC bindings
impl<P: JsonRpcProvider> RpcClient<P> {
	/// Instantiate a new provider with a backend.
	pub fn new(provider: P) -> Self {
		Self { provider, interval: None, from: None, _node_client: Arc::new(Mutex::new(None)) }
	}

	/// Returns the type of node we're connected to, while also caching the value for use
	/// in other node-specific API calls, such as the get_block_receipts call.
	pub async fn node_client(&self) -> Result<NeoVersion, ProviderError> {
		let mut node_client = self._node_client.lock().await;

		if let Some(ref node_client) = *node_client {
			Ok(node_client.clone())
		} else {
			let client_version = self.get_version().await?;
			*node_client = Some(client_version.clone());
			Ok(client_version)
		}
	}

	#[must_use]
	/// Set the default sender on the provider
	pub fn with_sender(mut self, address: impl Into<Address>) -> Self {
		self.from = Some(address.into());
		self
	}

	/// Make an RPC request via the internal connection, and return the result.
	pub async fn request<T, R>(&self, method: &str, params: T) -> Result<R, ProviderError>
	where
		T: Debug + Serialize + Send + Sync,
		R: Serialize + DeserializeOwned + Debug + Send,
	{
		let span =
			tracing::trace_span!("rpc", method = method, params_type = core::any::type_name::<T>());
		// https://docs.rs/tracing/0.1.22/tracing/span/struct.Span.html#in-asynchronous-code
		let res = async move {
			let fetched = self.provider.fetch(method, params).await;
			let res: R = fetched.map_err(Into::into)?;
			trace!(rx_type = core::any::type_name::<R>());
			Ok::<_, ProviderError>(res)
		}
		.instrument(span)
		.await?;
		Ok(res)
	}
}

#[cfg_attr(target_arch = "wasm32", async_trait(? Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<P: JsonRpcProvider> APITrait for RpcClient<P> {
	type Error = ProviderError;
	type Provider = P;

	fn rpc_client(&self) -> &RpcClient<Self::Provider> {
		self
	}

	async fn network(&self) -> Result<u32, ProviderError> {
		if NEOCONFIG.lock().map_err(|_| ProviderError::LockError)?.network.is_none() {
			// Use the cached node version when available to avoid redundant getversion calls.
			let version = self.node_client().await?;
			let protocol = version.protocol.ok_or(ProviderError::ProtocolNotFound)?;
			return Ok(protocol.network);
		}
		NEOCONFIG
			.lock()
			.map_err(|_| ProviderError::LockError)?
			.network
			.ok_or(ProviderError::NetworkNotFound)
	}

	//////////////////////// Neo methods////////////////////////////

	// Blockchain methods
	/// Gets the hash of the latest block in the blockchain.
	/// - Returns: The request object
	async fn get_best_block_hash(&self) -> Result<H256, ProviderError> {
		self.request("getbestblockhash", Vec::<H256>::new()).await
	}

	/// Gets the block hash of the corresponding block based on the specified block index.
	/// - Parameter blockIndex: The block index
	/// - Returns: The request object
	async fn get_block_hash(&self, block_index: u32) -> Result<H256, ProviderError> {
		self.request("getblockhash", [block_index.to_value()].to_vec()).await
	}

	/// Gets the corresponding block information according to the specified block hash.
	/// - Parameters:
	///   - blockHash: The block hash
	///   - returnFullTransactionObjects: Whether to get block information with all transaction objects or just the block header
	/// - Returns: The request object
	async fn get_block(&self, block_hash: H256, full_tx: bool) -> Result<NeoBlock, ProviderError> {
		Ok(if full_tx {
			self.request("getblock", [block_hash.to_value(), 1.to_value()]).await?
		} else {
			self.get_block_header(block_hash).await?
		})
	}

	/// Gets the block by hash string.
	/// - Parameters:
	///   - hash: The block hash as a string
	///   - full_tx: Whether to get block information with all transaction objects or just the block header
	/// - Returns: The request object
	async fn get_block_by_hash(
		&self,
		hash: &str,
		full_tx: bool,
	) -> Result<NeoBlock, ProviderError> {
		let block_hash = H256::from_str(hash)
			.map_err(|e| ProviderError::ParseError(format!("Invalid block hash: {}", e)))?;
		self.get_block(block_hash, full_tx).await
	}

	/// Gets the corresponding block information for the specified block hash.
	/// - Parameter blockHash: The block hash
	/// - Returns: The request object
	async fn get_raw_block(&self, block_hash: H256) -> Result<String, ProviderError> {
		self.request("getblock", [block_hash.to_value(), 0.to_value()]).await
	}

	// Node methods
	/// Gets the block header count of the blockchain.
	/// - Returns: The request object
	async fn get_block_header_count(&self) -> Result<u32, ProviderError> {
		self.request("getblockheadercount", Vec::<u32>::new()).await
	}

	/// Gets the block count of the blockchain.
	/// - Returns: The request object
	async fn get_block_count(&self) -> Result<u32, ProviderError> {
		self.request("getblockcount", Vec::<u32>::new()).await
	}

	/// Gets the corresponding block header information according to the specified block hash.
	/// - Parameter blockHash: The block hash
	/// - Returns: The request object
	async fn get_block_header(&self, block_hash: H256) -> Result<NeoBlock, ProviderError> {
		self.request("getblockheader", vec![block_hash.to_value(), 1.to_value()]).await
	}

	/// Gets the corresponding block header information according to the specified index.
	/// - Parameter blockIndex: The block index
	/// - Returns: The request object
	async fn get_block_header_by_index(&self, index: u32) -> Result<NeoBlock, ProviderError> {
		self.request("getblockheader", vec![index.to_value(), 1.to_value()]).await
	}

	/// Gets the corresponding block header information according to the specified block hash.
	/// - Parameter blockHash: The block hash
	/// - Returns: The request object
	async fn get_raw_block_header(&self, block_hash: H256) -> Result<String, ProviderError> {
		self.request("getblockheader", vec![block_hash.to_value(), 0.to_value()]).await
	}

	/// Gets the corresponding block header information according to the specified index.
	/// - Parameter blockIndex: The block index
	/// - Returns: The request object
	async fn get_raw_block_header_by_index(&self, index: u32) -> Result<String, ProviderError> {
		self.request("getblockheader", vec![index.to_value(), 0.to_value()]).await
	}

	/// Gets the native contracts list, which includes the basic information of native contracts and the contract descriptive file `manifest.json`.
	/// - Returns: The request object
	async fn get_native_contracts(&self) -> Result<Vec<NativeContractState>, ProviderError> {
		self.request("getnativecontracts", Vec::<NativeContractState>::new()).await
	}

	/// Gets the contract information.
	/// - Parameter contractHash: The contract script hash
	/// - Returns: The request object
	async fn get_contract_state(&self, hash: H160) -> Result<ContractState, ProviderError> {
		self.request("getcontractstate", vec![hash.to_hex()]).await
	}

	/// Gets the contract information.
	/// - Parameter contractHash: The contract script hash
	/// - Returns: The request object
	async fn get_contract_state_by_id(&self, id: i64) -> Result<ContractState, ProviderError> {
		self.request("getcontractstate", vec![id.to_value()]).await
	}

	/// Gets the native contract information by its name.
	///
	/// This RPC only works for native contracts.
	/// - Parameter contractName: The name of the native contract
	/// - Returns: The request object
	async fn get_native_contract_state(&self, name: &str) -> Result<ContractState, ProviderError> {
		self.request("getcontractstate", vec![name.to_value()]).await
	}

	/// Gets a list of unconfirmed or confirmed transactions in memory.
	/// - Returns: The request object
	async fn get_mem_pool(&self) -> Result<MemPoolDetails, ProviderError> {
		self.request("getrawmempool", vec![1.to_value()]).await
	}

	/// Gets a list of confirmed transactions in memory.
	/// - Returns: The request object
	async fn get_raw_mem_pool(&self) -> Result<Vec<H256>, ProviderError> {
		self.request("getrawmempool", Vec::<H256>::new()).await
	}

	/// Gets the corresponding transaction information based on the specified transaction hash.
	/// - Parameter txHash: The transaction hash
	/// - Returns: The request object
	async fn get_transaction(&self, hash: H256) -> Result<RTransaction, ProviderError> {
		self.request("getrawtransaction", vec![hash.to_value(), 1.to_value()]).await
	}

	/// Gets the corresponding transaction information based on the specified transaction hash.
	/// - Parameter txHash: The transaction hash
	/// - Returns: The request object
	async fn get_raw_transaction(&self, tx_hash: H256) -> Result<String, ProviderError> {
		self.request("getrawtransaction", vec![tx_hash.to_value(), 0.to_value()]).await
	}

	/// Gets the stored value according to the contract hash and the key.
	/// - Parameters:
	///   - contractHash: The contract hash
	///   - keyHexString: The key to look up in storage as a hexadecimal string
	/// - Returns: The request object
	async fn get_storage(&self, contract_hash: H160, key: &str) -> Result<String, ProviderError> {
		let params: [String; 2] =
			[contract_hash.to_hex(), encode_hex_parameter_as_base64(key, "storage key")?];
		self.request("getstorage", params.to_vec()).await
	}

	/// Finds the storage entries of a contract based on the prefix and  start index.
	/// - Parameters:
	///   - contractHash: The contract hash
	///   - prefix_hex_string: The prefix to filter the storage entries
	///   - start_index: the start index
	/// - Returns: The request object
	async fn find_storage(
		&self,
		contract_hash: H160,
		prefix_hex_string: &str,
		start_index: u64,
	) -> Result<String, ProviderError> {
		let params = json!([
			contract_hash.to_hex(),
			encode_hex_parameter_as_base64(prefix_hex_string, "storage prefix")?,
			start_index
		]);
		self.request("findstorage", params).await
	}

	/// Finds the storage entries of a contract based on the prefix and  start index.
	/// - Parameters:
	///   - contract_id: The contract id
	///   - prefix_hex_string: The prefix to filter the storage entries
	///   - start_index: the start index
	/// - Returns: The request object
	async fn find_storage_with_id(
		&self,
		contract_id: i64,
		prefix_hex_string: &str,
		start_index: u64,
	) -> Result<String, ProviderError> {
		let params = json!([
			contract_id,
			encode_hex_parameter_as_base64(prefix_hex_string, "storage prefix")?,
			start_index
		]);
		self.request("findstorage", params).await
	}

	/// Gets the transaction height with the specified transaction hash.
	/// - Parameter txHash: The transaction hash
	/// - Returns: The request object
	async fn get_transaction_height(&self, tx_hash: H256) -> Result<u32, ProviderError> {
		let params = [tx_hash.to_value()];
		self.request("gettransactionheight", params.to_vec()).await
	}

	/// Gets the validators of the next block.
	/// - Returns: The request object
	async fn get_next_block_validators(&self) -> Result<Vec<Validator>, ProviderError> {
		self.request("getnextblockvalidators", Vec::<Validator>::new()).await
	}

	/// Gets the public key list of current Neo committee members.
	/// - Returns: The request object
	async fn get_committee(&self) -> Result<Vec<String>, ProviderError> {
		self.request("getcommittee", Vec::<String>::new()).await
	}

	/// Gets the current number of connections for the node.
	/// - Returns: The request object
	async fn get_connection_count(&self) -> Result<u32, ProviderError> {
		self.request("getconnectioncount", Vec::<u32>::new()).await
	}

	/// Gets a list of nodes that the node is currently connected or disconnected from.
	/// - Returns: The request object
	async fn get_peers(&self) -> Result<Peers, ProviderError> {
		self.request("getpeers", Vec::<Peers>::new()).await
	}

	/// Gets the version information of the node.
	/// - Returns: The request object
	async fn get_version(&self) -> Result<NeoVersion, ProviderError> {
		self.request("getversion", Vec::<NeoVersion>::new()).await
	}

	/// Broadcasts a transaction over the NEO network.
	/// - Parameter rawTransactionHex: The raw transaction in hexadecimal
	/// - Returns: The request object
	async fn send_raw_transaction(&self, hex: String) -> Result<RawTransaction, ProviderError> {
		self.request(
			"sendrawtransaction",
			vec![encode_hex_parameter_as_base64(&hex, "raw transaction")?],
		)
		.await
	}

	/// Sends a transaction to the network
	///
	/// # Arguments
	///
	/// * `tx` - The transaction to send
	///
	/// # Returns
	///
	/// A `Result` containing the transaction hash or a `ProviderError`
	async fn send_transaction<'a>(&self, tx: Transaction<'a, P>) -> Result<H256, ProviderError> {
		let tx_hex = hex::encode(tx.try_to_array().map_err(|e| {
			ProviderError::ParseError(format!("Failed to serialize transaction: {}", e))
		})?);
		let result = self.send_raw_transaction(tx_hex).await?;

		// Convert the transaction hash to H256
		let tx_hash = H256::from_str(&result.hash.to_string()).map_err(|e| {
			ProviderError::ParseError(format!("Failed to parse transaction hash: {}", e))
		})?;

		Ok(tx_hash)
	}

	/// Broadcasts a new block over the NEO network.
	/// - Parameter serializedBlockAsHex: The block in hexadecimal
	/// - Returns: The request object
	async fn submit_block(&self, hex: String) -> Result<SubmitBlock, ProviderError> {
		self.request("submitblock", vec![hex.to_value()]).await
	}

	/// Broadcasts the node's address to the network
	async fn broadcast_address(&self) -> Result<bool, ProviderError> {
		self.request("broadcastaddr", Vec::<String>::new()).await
	}

	/// Broadcasts a block to the network
	async fn broadcast_block(&self, block: NeoBlock) -> Result<bool, ProviderError> {
		let block_json = serde_json::to_string(&block)
			.map_err(|e| ProviderError::ParseError(format!("Failed to serialize block: {}", e)))?;

		self.request("broadcastblock", vec![block_json.to_value()]).await
	}

	/// Broadcasts a request for blocks to the network
	///
	/// # Arguments
	///
	/// * `hash` - The hash of the block to start from
	/// * `count` - The number of blocks to request
	///
	/// # Returns
	///
	/// A `Result` containing a boolean indicating success or a `ProviderError`
	async fn broadcast_get_blocks(&self, hash: &str, count: u32) -> Result<bool, ProviderError> {
		let hash_obj = H256::from_str(hash)
			.map_err(|e| ProviderError::ParseError(format!("Invalid block hash: {}", e)))?;

		self.request("broadcastgetblocks", vec![hash_obj.to_value(), count.to_value()])
			.await
	}

	/// Broadcasts a transaction to the network
	///
	/// # Arguments
	///
	/// * `tx` - The transaction to broadcast
	///
	/// # Returns
	///
	/// A `Result` containing a boolean indicating success or a `ProviderError`
	async fn broadcast_transaction(&self, tx: RTransaction) -> Result<bool, ProviderError> {
		let tx_json = serde_json::to_string(&tx).map_err(|e| {
			ProviderError::ParseError(format!("Failed to serialize transaction: {}", e))
		})?;

		self.request("broadcasttransaction", vec![tx_json.to_value()]).await
	}

	/// Creates a contract deployment transaction
	async fn create_contract_deployment_transaction(
		&self,
		nef: NefFile,
		manifest: ContractManifest,
		_signers: Vec<Signer>,
	) -> Result<TransactionBuilder<P>, ProviderError> {
		let nef_bytes = nef
			.try_to_array()
			.map_err(|e| ProviderError::ParseError(format!("Failed to serialize NEF: {}", e)))?;
		let manifest_json = serde_json::to_string(&manifest).map_err(|e| {
			ProviderError::ParseError(format!("Failed to serialize manifest: {}", e))
		})?;

		let mut script_builder = ScriptBuilder::new();
		script_builder
			.push_data(manifest_json.as_bytes().to_vec())
			.push_data(nef_bytes)
			.sys_call(InteropService::SystemContractCall);

		let mut builder = TransactionBuilder::new();
		builder.extend_script(script_builder.to_bytes());

		// Add signers to the transaction
		// Note: Signers will be added when the transaction is built

		Ok(builder)
	}

	/// Creates a contract update transaction
	async fn create_contract_update_transaction(
		&self,
		contract_hash: H160,
		nef: NefFile,
		manifest: ContractManifest,
		_signers: Vec<Signer>,
	) -> Result<TransactionBuilder<P>, ProviderError> {
		let nef_bytes = nef
			.try_to_array()
			.map_err(|e| ProviderError::ParseError(format!("Failed to serialize NEF: {}", e)))?;
		let manifest_json = serde_json::to_string(&manifest).map_err(|e| {
			ProviderError::ParseError(format!("Failed to serialize manifest: {}", e))
		})?;

		let mut script_builder = ScriptBuilder::new();
		script_builder
			.push_data(manifest_json.as_bytes().to_vec())
			.push_data(nef_bytes)
			.push_data(contract_hash.to_vec())
			.sys_call(InteropService::SystemContractCall);

		let mut builder = TransactionBuilder::new();
		builder.extend_script(script_builder.to_bytes());

		// Add signers to the transaction
		// Note: Signers will be added when the transaction is built

		Ok(builder)
	}

	/// Creates an invocation transaction
	async fn create_invocation_transaction(
		&self,
		contract_hash: H160,
		method: &str,
		parameters: Vec<ContractParameter>,
		_signers: Vec<Signer>,
	) -> Result<TransactionBuilder<P>, ProviderError> {
		let mut script_builder = ScriptBuilder::new();
		script_builder
			.contract_call(&contract_hash, method, &parameters, None)
			.map_err(|e| {
				ProviderError::ParseError(format!("Failed to create contract call: {}", e))
			})?;

		let mut builder = TransactionBuilder::new();
		builder.extend_script(script_builder.to_bytes());

		// Add signers to the transaction
		// Note: Signers will be added when the transaction is built

		Ok(builder)
	}

	// MARK: SmartContract Methods

	/// Invokes the function with `functionName` of the smart contract with the specified contract hash.
	/// - Parameters:
	///   - contractHash: The contract hash to invoke
	///   - functionName: The function to invoke
	///   - contractParams: The parameters of the function
	///   - signers: The signers
	/// - Returns: The request object
	async fn invoke_function(
		&self,
		contract_hash: &H160,
		method: String,
		params: Vec<ContractParameter>,
		signers: Option<Vec<Signer>>,
	) -> Result<InvocationResult, ProviderError> {
		let signers = signers
			.map(|s| try_transaction_signers(s.iter()))
			.transpose()?
			.unwrap_or_default();
		self.request("invokefunction", json!([contract_hash.to_hex(), method, params, signers]))
			.await
	}

	/// Invokes a script.
	/// - Parameters:
	///   - scriptHex: The script to invoke
	///   - signers: The signers
	/// - Returns: The request object
	async fn invoke_script(
		&self,
		hex: String,
		signers: Vec<Signer>,
	) -> Result<InvocationResult, ProviderError> {
		let signers = try_transaction_signers(signers.iter())?;
		let hex_bytes = hex::decode(&hex)
			.map_err(|e| ProviderError::ParseError(format!("Failed to parse hex: {}", e)))?;
		let script_base64 = serde_json::to_value(hex_bytes.to_base64())?;
		let signers_json = serde_json::to_value(&signers)?;
		self.request("invokescript", [script_base64, signers_json]).await
	}

	/// Gets the unclaimed GAS of the account with the specified script hash.
	/// - Parameter scriptHash: The account's script hash
	/// - Returns: The request object
	async fn get_unclaimed_gas(&self, hash: H160) -> Result<UnclaimedGas, ProviderError> {
		self.request("getunclaimedgas", [hash.to_address()]).await
	}

	/// Gets a list of plugins loaded by the node.
	/// - Returns: The request object
	async fn list_plugins(&self) -> Result<Vec<Plugin>, ProviderError> {
		self.request("listplugins", Vec::<u32>::new()).await
	}

	/// Verifies whether the address is a valid NEO address.
	/// - Parameter address: The address to verify
	/// - Returns: The request object
	async fn validate_address(&self, address: &str) -> Result<ValidateAddress, ProviderError> {
		self.request("validateaddress", vec![address.to_value()]).await
	}

	/// Closes the current wallet.
	/// - Returns: The request object
	async fn close_wallet(&self) -> Result<bool, ProviderError> {
		self.request("closewallet", Vec::<u32>::new()).await
	}

	/// Exports the private key of the specified script hash.
	/// - Parameter scriptHash: The account's script hash
	/// - Returns: The request object
	async fn dump_priv_key(&self, script_hash: H160) -> Result<String, ProviderError> {
		let params = [script_hash.to_address()].to_vec();
		self.request("dumpprivkey", params).await
	}

	/// Gets the wallet balance of the corresponding token.
	/// - Parameter tokenHash: The token hash
	/// - Returns: The request object
	async fn get_wallet_balance(&self, token_hash: H160) -> Result<WalletBalance, ProviderError> {
		self.request("getwalletbalance", vec![token_hash.to_value()]).await
	}

	/// Creates a new address.
	/// - Returns: The request object
	async fn get_new_address(&self) -> Result<String, ProviderError> {
		self.request("getnewaddress", Vec::<u32>::new()).await
	}

	/// Gets the amount of unclaimed GAS in the wallet.
	/// - Returns: The request object
	async fn get_wallet_unclaimed_gas(&self) -> Result<String, ProviderError> {
		self.request("getwalletunclaimedgas", Vec::<String>::new()).await
	}

	/// Gets the current wallet height.
	/// - Returns: The request object
	async fn get_wallet_height(&self) -> Result<u32, ProviderError> {
		self.request("getwalletheight", Vec::<u32>::new()).await
	}

	/// Imports a private key to the wallet.
	/// - Parameter privateKeyInWIF: The private key in WIF-format
	/// - Returns: The request object
	async fn import_priv_key(&self, priv_key: String) -> Result<NeoAddress, ProviderError> {
		let params = [priv_key.to_value()].to_vec();
		self.request("importprivkey", params).await
	}

	/// Calculates the network fee for the specified transaction.
	/// - Parameter txBase64: The transaction in hexadecimal
	/// - Returns: The request object
	async fn calculate_network_fee(
		&self,
		tx_base64: String,
	) -> Result<NeoNetworkFee, ProviderError> {
		self.request(
			"calculatenetworkfee",
			vec![encode_hex_parameter_as_base64(&tx_base64, "transaction")?],
		)
		.await
	}

	/// Lists all the addresses in the current wallet.
	/// - Returns: The request object
	async fn list_address(&self) -> Result<Vec<NeoAddress>, ProviderError> {
		self.request("listaddress", Vec::<NeoAddress>::new()).await
	}

	/// Opens the specified wallet.
	/// - Parameters:
	///   - walletPath: The wallet file path
	///   - password: The password for the wallet
	/// - Returns: The request object
	async fn open_wallet(&self, path: String, password: String) -> Result<bool, ProviderError> {
		self.request("openwallet", vec![path.to_value(), password.to_value()]).await
	}

	/// Transfers an amount of a token from an account to another account.
	/// - Parameters:
	///   - tokenHash: The token hash of the NEP-17 contract
	///   - from: The transferring account's script hash
	///   - to: The recipient
	///   - amount: The transfer amount in token fractions
	/// - Returns: The request object
	async fn send_from(
		&self,
		token_hash: H160,
		from: H160,
		to: H160,
		amount: u32,
	) -> Result<RTransaction, ProviderError> {
		let params = json!([token_hash.to_hex(), from.to_address(), to.to_address(), amount,]);
		self.request("sendfrom", params).await
	}

	/// Initiates multiple transfers to multiple accounts from one specific account in a transaction.
	/// - Parameters:
	///   - from: The transferring account's script hash
	///   - txSendTokens: a list of ``TransactionSendToken`` objects, that each contains the token hash, the recipient and the transfer amount.
	/// - Returns: The request object
	async fn send_many(
		&self,
		from: Option<H160>,
		send_tokens: Vec<TransactionSendToken>,
	) -> Result<RTransaction, ProviderError> {
		let params = match from {
			Some(f) => json!([f.to_address(), send_tokens]),
			None => json!([send_tokens]),
		};
		self.request("sendmany", params).await
	}

	/// Transfers an amount of a token to another account.
	/// - Parameters:
	///   - tokenHash: The token hash of the NEP-17 contract
	///   - to: The recipient
	///   - amount: The transfer amount in token fractions
	/// - Returns: The request object
	async fn send_to_address(
		&self,
		token_hash: H160,
		to: H160,
		amount: u32,
	) -> Result<RTransaction, ProviderError> {
		let params = json!([token_hash.to_hex(), to.to_address(), amount]);
		self.request("sendtoaddress", params).await
	}

	async fn cancel_transaction(
		&self,
		tx_hash: H256,
		signers: Vec<H160>,
		extra_fee: Option<u64>,
	) -> Result<RTransaction, ProviderError> {
		if signers.is_empty() {
			return Err(ProviderError::CustomError("signers must not be empty".into()));
		}
		let signer_addresses: Vec<String> =
			signers.into_iter().map(|signer| signer.to_address()).collect();
		let params = json!([
			hex::encode(tx_hash.0),
			signer_addresses,
			extra_fee.map_or("".to_string(), |fee| fee.to_string())
		]);
		self.request("canceltransaction", params).await
	}

	/// Gets the application logs of the specified transaction hash.
	/// - Parameter txHash: The transaction hash
	/// - Returns: The request object
	async fn get_application_log(&self, tx_hash: H256) -> Result<ApplicationLog, ProviderError> {
		self.request("getapplicationlog", vec![hex::encode(tx_hash.0).to_value()]).await
	}

	/// Gets the balance of all NEP-17 token assets in the specified script hash.
	/// - Parameter scriptHash: The account's script hash
	/// - Returns: The request object
	async fn get_nep17_balances(&self, script_hash: H160) -> Result<Nep17Balances, ProviderError> {
		self.request("getnep17balances", [script_hash.to_address().to_value()].to_vec())
			.await
	}

	/// Gets all the NEP-17 transaction information occurred in the specified script hash.
	/// - Parameter scriptHash: The account's script hash
	/// - Returns: The request object
	async fn get_nep17_transfers(
		&self,
		script_hash: H160,
	) -> Result<Nep17Transfers, ProviderError> {
		let params = json!([script_hash.to_address()]);
		self.request("getnep17transfers", params).await
	}

	/// Gets all the NEP17 transaction information occurred in the specified script hash since the specified time.
	/// - Parameters:
	///   - scriptHash: The account's script hash
	///   - from: The timestamp transactions occurred since
	/// - Returns: The request object
	async fn get_nep17_transfers_from(
		&self,
		script_hash: H160,
		from: u64,
	) -> Result<Nep17Transfers, ProviderError> {
		self.request("getnep17transfers", json!([script_hash.to_address(), from])).await
	}

	/// Gets all the NEP17 transaction information occurred in the specified script hash in the specified time range.
	/// - Parameters:
	///   - scriptHash: The account's script hash
	///   - from: The start timestamp
	///   - to: The end timestamp
	/// - Returns: The request object
	async fn get_nep17_transfers_range(
		&self,
		script_hash: H160,
		from: u64,
		to: u64,
	) -> Result<Nep17Transfers, ProviderError> {
		let params = json!([script_hash.to_address(), from, to]);
		self.request("getnep17transfers", params).await
	}

	/// Gets all NEP-11 balances of the specified account.
	/// - Parameter scriptHash: The account's script hash
	/// - Returns: The request object
	async fn get_nep11_balances(&self, script_hash: H160) -> Result<Nep11Balances, ProviderError> {
		let params = json!([script_hash.to_address()]);
		self.request("getnep11balances", params).await
	}

	/// Gets all NEP-11 transaction of the given account.
	/// - Parameter scriptHash: The account's script hash
	/// - Returns: The request object
	async fn get_nep11_transfers(
		&self,
		script_hash: H160,
	) -> Result<Nep11Transfers, ProviderError> {
		let params = json!([script_hash.to_address()]);
		self.request("getnep11transfers", params).await
	}

	/// Gets all NEP-11 transaction of the given account since the given time.
	/// - Parameters:
	///   - scriptHash: The account's script hash
	///   - from: The date from when to report transactions
	/// - Returns: The request object
	async fn get_nep11_transfers_from(
		&self,
		script_hash: H160,
		from: u64,
	) -> Result<Nep11Transfers, ProviderError> {
		let params = json!([script_hash.to_address(), from]);
		self.request("getnep11transfers", params).await
	}

	/// Gets all NEP-11 transactions of the given account in the time span between `from` and `to`.
	/// - Parameters:
	///   - scriptHash: The account's script hash
	///   - from: The start timestamp
	///   - to: The end timestamp
	/// - Returns: The request object
	async fn get_nep11_transfers_range(
		&self,
		script_hash: H160,
		from: u64,
		to: u64,
	) -> Result<Nep11Transfers, ProviderError> {
		let params = json!([script_hash.to_address(), from, to]);
		self.request("getnep11transfers", params).await
	}

	/// Gets the properties of the token with `tokenId` from the NEP-11 contract with `scriptHash`.
	///
	/// The properties are a mapping from the property name string to the value string.
	/// The value is plain text if the key is one of the properties defined in the NEP-11 standard.
	/// Otherwise, the value is a Base64-encoded byte array.
	///
	/// To receive custom property values that consist of nested types (e.g., Maps or Arrays) use ``invokeFunction(_:_:_:)``  to directly invoke the method `properties` of the NEP-11 smart contract.
	/// - Parameters:
	///   - scriptHash: The account's script hash
	///   - tokenId: The ID of the token as a hexadecimal string
	/// - Returns: The request object
	async fn get_nep11_properties(
		&self,
		script_hash: H160,
		token_id: &str,
	) -> Result<HashMap<String, String>, ProviderError> {
		let params = json!([script_hash.to_address(), token_id]);
		self.request("getnep11properties", params).await
	}

	/// Gets the state root by the block height.
	/// - Parameter blockIndex: The block index
	/// - Returns: The request object
	async fn get_state_root(&self, block_index: u32) -> Result<StateRoot, ProviderError> {
		let params = json!([block_index]);
		self.request("getstateroot", params).await
	}

	/// Gets the proof based on the root hash, the contract hash and the storage key.
	/// - Parameters:
	///   - rootHash: The root hash
	///   - contractHash: The contract hash
	///   - storageKeyHex: The storage key
	/// - Returns: The request object
	async fn get_proof(
		&self,
		root_hash: H256,
		contract_hash: H160,
		key: &str,
	) -> Result<String, ProviderError> {
		self.request(
			"getproof",
			json!([
				hex::encode(root_hash.0),
				contract_hash.to_hex(),
				encode_hex_parameter_as_base64(key, "storage key")?
			]),
		)
		.await
	}

	/// Verifies the proof data and gets the value of the storage corresponding to the key.
	/// - Parameters:
	///   - rootHash: The root hash
	///   - proof: The proof data of the state root
	/// - Returns: The request object
	async fn verify_proof(&self, root_hash: H256, proof: &str) -> Result<String, ProviderError> {
		let params =
			json!([hex::encode(root_hash.0), encode_hex_parameter_as_base64(proof, "proof")?,]);
		self.request("verifyproof", params).await
	}

	/// Gets the state root height.
	/// - Returns: The request object
	async fn get_state_height(&self) -> Result<StateHeight, ProviderError> {
		self.request("getstateheight", Vec::<StateHeight>::new()).await
	}

	/// Gets the state.
	/// - Parameters:
	///   - rootHash: The root hash
	///   - contractHash: The contract hash
	///   - keyHex: The storage key
	/// - Returns: The request object
	async fn get_state(
		&self,
		root_hash: H256,
		contract_hash: H160,
		key: &str,
	) -> Result<String, ProviderError> {
		self.request(
			"getstate",
			json!([
				hex::encode(root_hash.0),
				contract_hash.to_hex(),
				encode_hex_parameter_as_base64(key, "storage key")?
			]), //key.to_base64()],
		)
		.await
	}

	/// Gets a list of states that match the provided key prefix.
	///
	/// Includes proofs of the first and last entry.
	/// - Parameters:
	///   - rootHash: The root hash
	///   - contractHash: The contact hash
	///   - keyPrefixHex: The key prefix
	///   - startKeyHex: The start key
	///   - countFindResultItems: The number of results. An upper limit is defined in the Neo core
	/// - Returns: The request object
	async fn find_states(
		&self,
		root_hash: H256,
		contract_hash: H160,
		key_prefix: &str,
		start_key: Option<&str>,
		count: Option<u32>,
	) -> Result<States, ProviderError> {
		let key_prefix_base64 = encode_hex_parameter_as_base64(key_prefix, "key prefix")?;
		let mut params =
			json!([hex::encode(root_hash.0), contract_hash.to_hex(), key_prefix_base64]);
		if let (Some(start_key), Some(count)) = (start_key, count) {
			params = json!([
				hex::encode(root_hash.0),
				contract_hash.to_hex(),
				key_prefix_base64.clone(),
				encode_hex_parameter_as_base64(start_key, "start key")?,
				count,
			]);
		} else if let Some(count) = count {
			params = json!([
				hex::encode(root_hash.0),
				contract_hash.to_hex(),
				key_prefix_base64.clone(),
				"".to_string(),
				count,
			]);
		} else if let Some(start_key) = start_key {
			params = json!([
				hex::encode(root_hash.0),
				contract_hash.to_hex(),
				key_prefix_base64.clone(),
				encode_hex_parameter_as_base64(start_key, "start key")?,
			]);
		}

		self.request("findstates", params).await
	}

	async fn get_block_by_index(
		&self,
		index: u32,
		full_tx: bool,
	) -> Result<NeoBlock, ProviderError> {
		Ok(if full_tx {
			self.request("getblock", vec![index.to_value(), 1.to_value()]).await?
		} else {
			self.get_block_header_by_index(index).await?
		})
	}

	async fn get_raw_block_by_index(&self, index: u32) -> Result<String, ProviderError> {
		self.request("getblock", vec![index.to_value(), 0.to_value()]).await
	}

	/// Invokes the function with `functionName` of the smart contract with the specified contract hash.
	///
	/// Includes diagnostics from the invocation.
	/// - Parameters:
	///   - contractHash: The contract hash to invoke
	///   - functionName: The function to invoke
	///   - contractParams: The parameters of the function
	///   - signers: The signers
	/// - Returns: The request object
	async fn invoke_function_diagnostics(
		&self,
		contract_hash: H160,
		function_name: String,
		params: Vec<ContractParameter>,
		signers: Vec<Signer>,
	) -> Result<InvocationResult, ProviderError> {
		let signers = try_transaction_signers(signers.iter())?;
		let params = json!([contract_hash.to_hex(), function_name, params, signers, true]);
		self.request("invokefunction", params).await
	}

	/// Invokes a script.
	///
	/// Includes diagnostics from the invocation.
	/// - Parameters:
	///   - scriptHex: The script to invoke
	///   - signers: The signers
	/// - Returns: The request object
	async fn invoke_script_diagnostics(
		&self,
		hex: String,
		signers: Vec<Signer>,
	) -> Result<InvocationResult, ProviderError> {
		let signers = try_transaction_signers(signers.iter())?;
		let hex_bytes = hex::decode(&hex)
			.map_err(|e| ProviderError::ParseError(format!("Failed to parse hex: {}", e)))?;
		let script_base64 = serde_json::to_value(hex_bytes.to_base64())?;
		let signers_json = serde_json::to_value(&signers)?;
		let params = vec![script_base64, signers_json, true.to_value()];
		self.request("invokescript", params).await
	}

	/// Returns the results from an iterator.
	///
	/// The results are limited to `count` items. If `count` is greater than `MaxIteratorResultItems` in the Neo Node's configuration file, this request fails.
	/// - Parameters:
	///   - sessionId: The session id
	///   - iteratorId: The iterator id
	///   - count: The maximal number of stack items returned
	/// - Returns: The request object
	async fn traverse_iterator(
		&self,
		session_id: String,
		iterator_id: String,
		count: u32,
	) -> Result<Vec<StackItem>, ProviderError> {
		let params = vec![session_id.to_value(), iterator_id.to_value(), count.to_value()];
		self.request("traverseiterator", params).await
	}

	async fn terminate_session(&self, session_id: &str) -> Result<bool, ProviderError> {
		self.request("terminatesession", vec![session_id.to_value()]).await
	}

	async fn invoke_contract_verify(
		&self,
		hash: H160,
		params: Vec<ContractParameter>,
		signers: Vec<Signer>,
	) -> Result<InvocationResult, ProviderError> {
		let signers = try_transaction_signers(signers.iter())?;
		let params = json!([hash.to_hex(), params, signers]);
		self.request("invokecontractverify", params).await
	}

	fn get_raw_mempool<'life0, 'async_trait>(
		&'life0 self,
	) -> Pin<Box<dyn Future<Output = Result<MemPoolDetails, Self::Error>> + Send + 'async_trait>>
	where
		'life0: 'async_trait,
		Self: 'async_trait,
	{
		Box::pin(async move { self.get_mem_pool().await })
	}

	fn import_private_key<'life0, 'async_trait>(
		&'life0 self,
		wif: String,
	) -> Pin<Box<dyn Future<Output = Result<NeoAddress, Self::Error>> + Send + 'async_trait>>
	where
		'life0: 'async_trait,
		Self: 'async_trait,
	{
		Box::pin(async move { self.import_priv_key(wif).await })
	}

	fn get_block_header_hash<'life0, 'async_trait>(
		&'life0 self,
		hash: H256,
	) -> Pin<Box<dyn Future<Output = Result<NeoBlock, Self::Error>> + Send + 'async_trait>>
	where
		'life0: 'async_trait,
		Self: 'async_trait,
	{
		Box::pin(async move { self.get_block_header(hash).await })
	}

	async fn send_to_address_send_token(
		&self,
		send_token: &TransactionSendToken,
	) -> Result<RTransaction, ProviderError> {
		let params = json!([send_token.token.to_hex(), send_token.address, send_token.value,]);
		self.request("sendtoaddress", params).await
	}

	async fn send_from_send_token(
		&self,
		send_token: &TransactionSendToken,
		from: H160,
	) -> Result<RTransaction, ProviderError> {
		let params = json!([
			send_token.token.to_hex(),
			from.to_address(),
			send_token.address,
			send_token.value,
		]);
		self.request("sendfrom", params).await
	}
}

impl<P: JsonRpcProvider> RpcClient<P> {
	/// Sets the default polling interval for event filters and pending transactions
	/// (default: 7 seconds)
	pub fn set_interval<T: Into<Duration>>(&mut self, interval: T) -> &mut Self {
		self.interval = Some(interval.into());
		self
	}

	/// Sets the default polling interval for event filters and pending transactions
	/// (default: 7 seconds)
	#[must_use]
	pub fn interval<T: Into<Duration>>(mut self, interval: T) -> Self {
		self.set_interval(interval);
		self
	}
}

#[cfg(all(feature = "ipc", any(unix, windows)))]
impl RpcClient<crate::neo_clients::Ipc> {
	#[cfg_attr(unix, doc = "Connects to the Unix socket at the provided path.")]
	#[cfg_attr(windows, doc = "Connects to the named pipe at the provided path.\n")]
	#[cfg_attr(
		windows,
		doc = r"Note: the path must be the fully qualified, like: `\\.\pipe\<name>`."
	)]
	pub async fn connect_ipc(path: impl AsRef<std::path::Path>) -> Result<Self, ProviderError> {
		let ipc = crate::neo_clients::Ipc::connect(path).await?;
		Ok(Self::new(ipc))
	}
}

impl RpcClient<Http> {
	/// The Url to which requests are made
	pub fn url(&self) -> &Url {
		self.provider.url()
	}

	/// Mutable access to the Url to which requests are made
	pub fn url_mut(&mut self) -> &mut Url {
		self.provider.url_mut()
	}
}

impl<Read, Write> RpcClient<RwClient<Read, Write>>
where
	Read: JsonRpcProvider + 'static,
	<Read as JsonRpcProvider>::Error: Sync + Send + 'static + Display,
	Write: JsonRpcProvider + 'static,
	<Write as JsonRpcProvider>::Error: Sync + Send + 'static + Display,
{
	/// Creates a new [RpcClient] with a [RwClient]
	pub fn rw(r: Read, w: Write) -> Self {
		Self::new(RwClient::new(r, w))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		neo_builder::{
			AccountSigner, OracleResponse, OracleResponseCode, TransactionAttribute, WitnessScope,
		},
		neo_clients::{APITrait, MockProvider},
		neo_protocol::Account,
	};
	fn assert_parse_error(err: ProviderError, expected_fragment: &str) {
		match err {
			ProviderError::ParseError(message) => {
				assert!(
					message.contains(expected_fragment),
					"expected parse error containing '{expected_fragment}', got '{message}'"
				);
			},
			other => panic!("expected parse error, got {other:?}"),
		}
	}

	fn assert_illegal_state(err: ProviderError, expected_fragment: &str) {
		match err {
			ProviderError::IllegalState(message) => {
				assert!(
					message.contains(expected_fragment),
					"expected illegal state containing '{expected_fragment}', got '{message}'"
				);
			},
			other => panic!("expected illegal state, got {other:?}"),
		}
	}

	fn invalid_signer() -> Signer {
		let account =
			Account::from_wif("Kzt94tAAiZSgH7Yt4i25DW6jJFprZFPSqTgLr5dWmWgKDKCjXMfZ").unwrap();
		let mut signer = AccountSigner::called_by_entry(&account).unwrap();
		signer.scopes = vec![WitnessScope::Global, WitnessScope::CalledByEntry];
		Signer::from(signer)
	}

	#[tokio::test]
	async fn get_storage_rejects_invalid_hex_key_before_request() {
		let provider = MockProvider::new();
		let client = RpcClient::new(provider.clone());

		let err = client.get_storage(H160::zero(), "not-hex").await.unwrap_err();
		assert_parse_error(err, "storage key");
		assert!(provider.take_requests().is_empty());
	}

	#[tokio::test]
	async fn send_raw_transaction_rejects_invalid_hex_before_request() {
		let provider = MockProvider::new();
		let client = RpcClient::new(provider.clone());

		let err = client.send_raw_transaction("not-hex".to_string()).await.unwrap_err();
		assert_parse_error(err, "raw transaction");
		assert!(provider.take_requests().is_empty());
	}

	#[tokio::test]
	async fn verify_proof_rejects_invalid_hex_proof_before_request() {
		let provider = MockProvider::new();
		let client = RpcClient::new(provider.clone());

		let err = client.verify_proof(H256::zero(), "xyz").await.unwrap_err();
		assert_parse_error(err, "proof");
		assert!(provider.take_requests().is_empty());
	}

	fn encodable_test_nef() -> NefFile {
		NefFile::new(Some("test-compiler".to_string()), "", vec![0x11, 0x40], vec![0; 4])
	}

	#[tokio::test]
	async fn create_contract_deployment_transaction_rejects_invalid_nef_before_request() {
		let provider = MockProvider::new();
		let client = RpcClient::new(provider.clone());
		let mut nef = encodable_test_nef();
		nef.compiler = Some("x".repeat(65));

		let err = client
			.create_contract_deployment_transaction(nef, ContractManifest::default(), vec![])
			.await
			.unwrap_err();
		assert_parse_error(err, "NEF");
		assert!(provider.take_requests().is_empty());
	}

	#[tokio::test]
	async fn create_contract_update_transaction_rejects_invalid_nef_before_request() {
		let provider = MockProvider::new();
		let client = RpcClient::new(provider.clone());
		let mut nef = encodable_test_nef();
		nef.compiler = Some("x".repeat(65));

		let err = client
			.create_contract_update_transaction(
				H160::zero(),
				nef,
				ContractManifest::default(),
				vec![],
			)
			.await
			.unwrap_err();
		assert_parse_error(err, "NEF");
		assert!(provider.take_requests().is_empty());
	}

	#[tokio::test]
	async fn send_transaction_rejects_invalid_transaction_before_request() {
		let provider = MockProvider::new();
		let client = RpcClient::new(provider.clone());
		let mut tx = Transaction::<MockProvider>::new();
		tx.attributes = vec![TransactionAttribute::OracleResponse(OracleResponse {
			id: 1,
			response_code: OracleResponseCode::Success,
			result: "not-base64".to_string(),
		})];

		let err = client.send_transaction(tx).await.unwrap_err();
		assert_parse_error(err, "transaction");
		assert!(provider.take_requests().is_empty());
	}

	#[tokio::test]
	async fn find_states_rejects_invalid_start_key_before_request() {
		let provider = MockProvider::new();
		let client = RpcClient::new(provider.clone());

		let err = client
			.find_states(H256::zero(), H160::zero(), "01", Some("xyz"), None)
			.await
			.unwrap_err();
		assert_parse_error(err, "start key");
		assert!(provider.take_requests().is_empty());
	}

	#[tokio::test]
	async fn invoke_script_rejects_invalid_signer_before_request() {
		let provider = MockProvider::new();
		let client = RpcClient::new(provider.clone());

		let err = client
			.invoke_script("0102".to_string(), vec![invalid_signer()])
			.await
			.unwrap_err();
		assert_illegal_state(err, "Global scope cannot be combined with other scopes");
		assert!(provider.take_requests().is_empty());
	}

	#[tokio::test]
	async fn invoke_function_rejects_invalid_signer_before_request() {
		let provider = MockProvider::new();
		let client = RpcClient::new(provider.clone());

		let err = client
			.invoke_function(
				&H160::zero(),
				"symbol".to_string(),
				vec![],
				Some(vec![invalid_signer()]),
			)
			.await
			.unwrap_err();
		assert_illegal_state(err, "Global scope cannot be combined with other scopes");
		assert!(provider.take_requests().is_empty());
	}
}
