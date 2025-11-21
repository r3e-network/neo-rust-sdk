use std::{collections::HashMap, error::Error, fmt::Debug};

use crate::{
	builder::{Signer, Transaction, TransactionBuilder, TransactionSendToken},
	config::NEOCONFIG,
	neo_clients::{JsonRpcProvider, ProviderError, RpcClient},
	neo_protocol::{
		ApplicationLog, MemPoolDetails, NeoAddress, NeoBlock, NeoNetworkFee, NeoVersion,
		Nep11Balances, Nep11Transfers, Nep17Balances, Nep17Transfers, Peers, Plugin, RTransaction,
		RawTransaction, StateHeight, StateRoot, States, SubmitBlock, UnclaimedGas, ValidateAddress,
		Validator, WalletBalance,
	},
	ContractManifest, ContractParameter, ContractState, InvocationResult, NativeContractState,
	NefFile, StackItem,
};
use async_trait::async_trait;
use auto_impl::auto_impl;
use primitive_types::{H160, H256};

#[async_trait]
#[auto_impl(&, Box, Arc)]
pub trait APITrait: Sync + Send + Debug {
	/// Error type returned by most operations
	type Error: Error + Send + Sync + 'static;

	/// The JSON-RPC client type at the bottom of the stack
	type Provider: JsonRpcProvider;

	/// The HTTP or Websocket provider.
	fn rpc_client(&self) -> &RpcClient<Self::Provider>;

	async fn network(&self) -> Result<u32, ProviderError>;

	fn nns_resolver(&self) -> H160 {
		NEOCONFIG.lock().unwrap().nns_resolver
	}

	fn block_interval(&self) -> u32 {
		NEOCONFIG.lock().unwrap().milliseconds_per_block
	}

	fn polling_interval(&self) -> u32 {
		NEOCONFIG.lock().unwrap().milliseconds_per_block
	}

	fn max_valid_until_block_increment(&self) -> u32 {
		NEOCONFIG.lock().unwrap().get_max_valid_until_block_increment()
	}

	// Blockchain methods
	async fn get_best_block_hash(&self) -> Result<H256, Self::Error>;

	async fn get_block_hash(&self, block_index: u32) -> Result<H256, Self::Error>;

	async fn get_block(&self, block_hash: H256, full_tx: bool) -> Result<NeoBlock, Self::Error>;

	async fn get_raw_block(&self, block_hash: H256) -> Result<String, Self::Error>;

	// Node methods
	async fn get_block_header_count(&self) -> Result<u32, Self::Error>;

	async fn get_block_count(&self) -> Result<u32, Self::Error>;

	async fn get_block_header(&self, block_hash: H256) -> Result<NeoBlock, Self::Error>;

	async fn get_block_header_by_index(&self, index: u32) -> Result<NeoBlock, Self::Error>;

	// Smart contract methods

	async fn get_raw_block_header(&self, block_hash: H256) -> Result<String, Self::Error>;

	async fn get_raw_block_header_by_index(&self, index: u32) -> Result<String, Self::Error>;

	// Utility methods

	async fn get_native_contracts(&self) -> Result<Vec<NativeContractState>, Self::Error>;

	// Wallet methods

	async fn get_contract_state(&self, hash: H160) -> Result<ContractState, Self::Error>;

	async fn get_contract_state_by_id(&self, id: i64) -> Result<ContractState, Self::Error>;

	async fn get_native_contract_state(&self, name: &str) -> Result<ContractState, Self::Error>;

	async fn get_mem_pool(&self) -> Result<MemPoolDetails, Self::Error>;

	async fn get_raw_mem_pool(&self) -> Result<Vec<H256>, Self::Error>;

	// Application logs

	async fn get_transaction(&self, hash: H256) -> Result<RTransaction, Self::Error>;

	// State service

	async fn get_raw_transaction(&self, tx_hash: H256) -> Result<String, Self::Error>;

	async fn get_storage(&self, contract_hash: H160, key: &str) -> Result<String, Self::Error>;

	async fn find_storage(
		&self,
		contract_hash: H160,
		prefix_hex_string: &str,
		start_index: u64,
	) -> Result<String, ProviderError>;

	async fn find_storage_with_id(
		&self,
		contract_id: i64,
		prefix_hex_string: &str,
		start_index: u64,
	) -> Result<String, ProviderError>;

	// Blockchain methods

	async fn get_transaction_height(&self, tx_hash: H256) -> Result<u32, Self::Error>;

	async fn get_next_block_validators(&self) -> Result<Vec<Validator>, Self::Error>;

	async fn get_committee(&self) -> Result<Vec<String>, Self::Error>;

	async fn get_connection_count(&self) -> Result<u32, Self::Error>;

	async fn get_peers(&self) -> Result<Peers, Self::Error>;

	// Smart contract method
	async fn get_version(&self) -> Result<NeoVersion, Self::Error>;

	async fn send_raw_transaction(&self, hex: String) -> Result<RawTransaction, Self::Error>;

	/// Sends a transaction to the network
	///
	/// # Arguments
	///
	/// * `tx` - The transaction to send
	///
	/// # Returns
	///
	/// A `Result` containing the transaction hash or a `ProviderError`
	async fn send_transaction<'a>(
		&self,
		tx: Transaction<'a, Self::Provider>,
	) -> Result<H256, Self::Error>;

	async fn submit_block(&self, hex: String) -> Result<SubmitBlock, Self::Error>;

	// Blockchain methods
	async fn invoke_function(
		&self,
		contract_hash: &H160,
		method: String,
		params: Vec<ContractParameter>,
		signers: Option<Vec<Signer>>,
	) -> Result<InvocationResult, Self::Error>;

	async fn invoke_script(
		&self,
		hex: String,
		signers: Vec<Signer>,
	) -> Result<InvocationResult, Self::Error>;

	// More smart contract methods

	async fn get_unclaimed_gas(&self, hash: H160) -> Result<UnclaimedGas, Self::Error>;

	async fn list_plugins(&self) -> Result<Vec<Plugin>, Self::Error>;

	async fn validate_address(&self, address: &str) -> Result<ValidateAddress, Self::Error>;

	// Wallet methods
	async fn close_wallet(&self) -> Result<bool, Self::Error>;

	async fn dump_priv_key(&self, script_hash: H160) -> Result<String, Self::Error>;

	async fn get_wallet_balance(&self, token_hash: H160) -> Result<WalletBalance, Self::Error>;

	async fn get_new_address(&self) -> Result<String, Self::Error>;

	async fn get_wallet_unclaimed_gas(&self) -> Result<String, Self::Error>;

	async fn import_priv_key(&self, priv_key: String) -> Result<NeoAddress, Self::Error>;

	async fn calculate_network_fee(&self, hex: String) -> Result<NeoNetworkFee, Self::Error>;

	async fn list_address(&self) -> Result<Vec<NeoAddress>, Self::Error>;

	async fn open_wallet(&self, path: String, password: String) -> Result<bool, Self::Error>;

	async fn send_from(
		&self,
		token_hash: H160,
		from: H160,
		to: H160,
		amount: u32,
	) -> Result<RTransaction, Self::Error>;

	// Transaction methods

	async fn send_many(
		&self,
		from: Option<H160>,
		send_tokens: Vec<TransactionSendToken>,
	) -> Result<RTransaction, Self::Error>;

	async fn send_to_address(
		&self,
		token_hash: H160,
		to: H160,
		amount: u32,
	) -> Result<RTransaction, Self::Error>;

	async fn cancel_transaction(
		&self,
		tx_hash: H256,
		signers: Vec<H160>,
		extra_fee: Option<u64>,
	) -> Result<RTransaction, ProviderError>;

	async fn get_application_log(&self, tx_hash: H256) -> Result<ApplicationLog, Self::Error>;

	async fn get_nep17_balances(&self, script_hash: H160) -> Result<Nep17Balances, Self::Error>;

	async fn get_nep17_transfers(&self, script_hash: H160) -> Result<Nep17Transfers, Self::Error>;

	// NEP-17 methods

	async fn get_nep17_transfers_from(
		&self,
		script_hash: H160,
		from: u64,
	) -> Result<Nep17Transfers, Self::Error>;

	async fn get_nep17_transfers_range(
		&self,
		script_hash: H160,
		from: u64,
		to: u64,
	) -> Result<Nep17Transfers, Self::Error>;

	async fn get_nep11_balances(&self, script_hash: H160) -> Result<Nep11Balances, Self::Error>;

	// NEP-11 methods

	async fn get_nep11_transfers(&self, script_hash: H160) -> Result<Nep11Transfers, Self::Error>;

	async fn get_nep11_transfers_from(
		&self,
		script_hash: H160,
		from: u64,
	) -> Result<Nep11Transfers, Self::Error>;

	async fn get_nep11_transfers_range(
		&self,
		script_hash: H160,
		from: u64,
		to: u64,
	) -> Result<Nep11Transfers, Self::Error>;

	async fn get_nep11_properties(
		&self,
		script_hash: H160,
		token_id: &str,
	) -> Result<HashMap<String, String>, Self::Error>;

	async fn get_state_root(&self, block_index: u32) -> Result<StateRoot, Self::Error>;

	// State service methods
	async fn get_proof(
		&self,
		root_hash: H256,
		contract_hash: H160,
		key: &str,
	) -> Result<String, Self::Error>;

	async fn verify_proof(&self, root_hash: H256, proof: &str) -> Result<String, Self::Error>;

	async fn get_state_height(&self) -> Result<StateHeight, Self::Error>;

	async fn get_state(
		&self,
		root_hash: H256,
		contract_hash: H160,
		key: &str,
	) -> Result<String, Self::Error>;

	async fn find_states(
		&self,
		root_hash: H256,
		contract_hash: H160,
		key_prefix: &str,
		start_key: Option<&str>,
		count: Option<u32>,
	) -> Result<States, Self::Error>;

	async fn get_block_by_hash(&self, hash: &str, full_tx: bool) -> Result<NeoBlock, Self::Error>;

	async fn broadcast_address(&self) -> Result<bool, Self::Error>;

	async fn broadcast_block(&self, block: NeoBlock) -> Result<bool, Self::Error>;

	async fn broadcast_get_blocks(&self, hash: &str, count: u32) -> Result<bool, Self::Error>;

	async fn broadcast_transaction(&self, tx: RTransaction) -> Result<bool, Self::Error>;

	async fn create_contract_deployment_transaction(
		&self,
		nef: NefFile,
		manifest: ContractManifest,
		signers: Vec<Signer>,
	) -> Result<TransactionBuilder<'life0, Self::Provider>, Self::Error>;

	async fn create_contract_update_transaction(
		&self,
		contract_hash: H160,
		nef: NefFile,
		manifest: ContractManifest,
		signers: Vec<Signer>,
	) -> Result<TransactionBuilder<'life0, Self::Provider>, Self::Error>;

	async fn create_invocation_transaction(
		&self,
		contract_hash: H160,
		method: &str,
		params: Vec<ContractParameter>,
		signers: Vec<Signer>,
	) -> Result<TransactionBuilder<'life0, Self::Provider>, Self::Error>;

	async fn get_block_by_index(&self, index: u32, full_tx: bool) -> Result<NeoBlock, Self::Error>;

	async fn get_raw_block_by_index(&self, index: u32) -> Result<String, Self::Error>;

	async fn invoke_function_diagnostics(
		&self,
		contract_hash: H160,
		name: String,
		params: Vec<ContractParameter>,
		signers: Vec<Signer>,
	) -> Result<InvocationResult, Self::Error>;

	async fn invoke_script_diagnostics(
		&self,
		hex: String,
		signers: Vec<Signer>,
	) -> Result<InvocationResult, Self::Error>;

	async fn traverse_iterator(
		&self,
		session_id: String,
		iterator_id: String,
		count: u32,
	) -> Result<Vec<StackItem>, Self::Error>;

	async fn terminate_session(&self, session_id: &str) -> Result<bool, Self::Error>;

	async fn invoke_contract_verify(
		&self,
		hash: H160,
		params: Vec<ContractParameter>,
		signers: Vec<Signer>,
	) -> Result<InvocationResult, Self::Error>;

	async fn get_raw_mempool(&self) -> Result<MemPoolDetails, Self::Error>;

	async fn import_private_key(&self, wif: String) -> Result<NeoAddress, Self::Error>;

	async fn get_block_header_hash(&self, hash: H256) -> Result<NeoBlock, Self::Error>;

	async fn send_to_address_send_token(
		&self,
		send_token: &TransactionSendToken,
	) -> Result<RTransaction, Self::Error>;

	async fn send_from_send_token(
		&self,
		send_token: &TransactionSendToken,
		from: H160,
	) -> Result<RTransaction, Self::Error>;
}
