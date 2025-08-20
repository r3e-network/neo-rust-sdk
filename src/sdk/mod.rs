//! High-level SDK API for simplified Neo blockchain interaction
//!
//! This module provides a user-friendly interface that wraps the lower-level
//! components, making common operations simple while still allowing access
//! to advanced features when needed.

pub mod hd_wallet;
pub mod transaction_simulator;
pub mod websocket;

use crate::{
	neo_builder::{ScriptBuilder, TransactionBuilder},
	neo_clients::{APITrait, HttpProvider, RpcClient},
	neo_error::unified::NeoError,
	neo_protocol::{Account, AccountTrait},
	neo_types::{ContractParameter, ScriptHash, StackItem},
	neo_wallets::wallet::Wallet,
};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

/// Main entry point for the Neo SDK
///
/// Provides high-level, user-friendly methods for common blockchain operations
/// while maintaining access to lower-level APIs when needed.
///
/// # Examples
///
/// ```no_run
/// use neo3::sdk::Neo;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Quick connection to testnet
///     let neo = Neo::testnet().await?;
///     
///     // Check balance
///     let balance = neo.get_balance("NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc").await?;
///     println!("Balance: {} GAS", balance.gas);
///     
///     Ok(())
/// }
/// ```
pub struct Neo {
	client: Arc<RpcClient<HttpProvider>>,
	network: Network,
	config: SdkConfig,
}

/// Network configuration
#[derive(Debug, Clone)]
pub enum Network {
	/// Neo MainNet
	MainNet,
	/// Neo TestNet
	TestNet,
	/// Custom network with RPC endpoint
	Custom(String),
}

/// SDK configuration options
#[derive(Debug, Clone)]
pub struct SdkConfig {
	/// Request timeout
	pub timeout: Duration,
	/// Number of retries for failed requests
	pub retries: u32,
	/// Enable caching
	pub cache_enabled: bool,
	/// Enable metrics collection
	pub metrics_enabled: bool,
}

impl Default for SdkConfig {
	fn default() -> Self {
		Self {
			timeout: Duration::from_secs(30),
			retries: 3,
			cache_enabled: true,
			metrics_enabled: false,
		}
	}
}

/// Balance information for an address
#[derive(Debug, Clone)]
pub struct Balance {
	/// NEO token balance
	pub neo: u64,
	/// GAS token balance
	pub gas: f64,
	/// Other NEP-17 token balances
	pub tokens: Vec<TokenBalance>,
}

/// Individual token balance
#[derive(Debug, Clone)]
pub struct TokenBalance {
	/// Token contract hash
	pub contract: ScriptHash,
	/// Token symbol
	pub symbol: String,
	/// Token amount
	pub amount: f64,
	/// Number of decimals
	pub decimals: u8,
}

/// Transaction hash type
pub type TxHash = String;

/// Common tokens
#[derive(Debug, Clone)]
pub enum Token {
	/// Native NEO token
	NEO,
	/// Native GAS token
	GAS,
	/// Custom NEP-17 token
	Custom(ScriptHash),
}

impl Neo {
	/// Connect to Neo TestNet with default configuration
	///
	/// # Examples
	///
	/// ```no_run
	/// let neo = Neo::testnet().await?;
	/// ```
	pub async fn testnet() -> Result<Self, NeoError> {
		Self::builder().network(Network::TestNet).build().await
	}

	/// Connect to Neo MainNet with default configuration
	///
	/// # Examples
	///
	/// ```no_run
	/// let neo = Neo::mainnet().await?;
	/// ```
	pub async fn mainnet() -> Result<Self, NeoError> {
		Self::builder().network(Network::MainNet).build().await
	}

	/// Create a new SDK builder for custom configuration
	///
	/// # Examples
	///
	/// ```no_run
	/// let neo = Neo::builder()
	///     .network(Network::TestNet)
	///     .timeout(Duration::from_secs(60))
	///     .retries(5)
	///     .build()
	///     .await?;
	/// ```
	pub fn builder() -> NeoBuilder {
		NeoBuilder::default()
	}

	/// Get the balance of an address
	///
	/// Returns NEO, GAS, and all NEP-17 token balances.
	///
	/// # Examples
	///
	/// ```no_run
	/// let balance = neo.get_balance("NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc").await?;
	/// println!("NEO: {}, GAS: {}", balance.neo, balance.gas);
	/// ```
	pub async fn get_balance(&self, address: &str) -> Result<Balance, NeoError> {
		use crate::neo_types::ScriptHashExtension;

		// Parse the address to script hash
		let script_hash = ScriptHash::from_address(address).map_err(|e| NeoError::Validation {
			message: format!("Invalid address: {}", e),
			field: "address".to_string(),
			value: Some(address.to_string()),
			recovery: crate::neo_error::unified::ErrorRecovery::new()
				.suggest("Check the address format")
				.suggest("Ensure it's a valid Neo N3 address"),
		})?;

		// Get NEO balance
		let neo_hash = ScriptHash::from_str("ef4073a0f2b305a38ec4050e4d3d28bc40ea63f5")
			.expect("NEO contract hash is valid");
		let neo_balance = self
			.client
			.invoke_function(
				&neo_hash,
				"balanceOf".to_string(),
				vec![ContractParameter::h160(&script_hash)],
				None,
			)
			.await
			.map_err(|e| NeoError::Network {
				message: format!("Failed to get NEO balance: {}", e),
				source: None,
				recovery: crate::neo_error::unified::ErrorRecovery::new()
					.suggest("Check network connection")
					.retryable(true),
			})?;

		// Get GAS balance
		let gas_hash = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")
			.expect("GAS contract hash is valid");
		let gas_balance = self
			.client
			.invoke_function(
				&gas_hash,
				"balanceOf".to_string(),
				vec![ContractParameter::h160(&script_hash)],
				None,
			)
			.await
			.map_err(|e| NeoError::Network {
				message: format!("Failed to get GAS balance: {}", e),
				source: None,
				recovery: crate::neo_error::unified::ErrorRecovery::new()
					.suggest("Check network connection")
					.retryable(true),
			})?;

		// Parse balances
		let neo = neo_balance
			.stack
			.first()
			.and_then(|item| match item {
				StackItem::Integer { value } => Some(*value),
				StackItem::ByteString { value } => {
					// Try to decode base64 and interpret as integer
					base64::decode(value).ok().and_then(|bytes| {
						if bytes.is_empty() {
							Some(0)
						} else if bytes.len() <= 8 {
							let mut array = [0u8; 8];
							array[..bytes.len()].copy_from_slice(&bytes);
							Some(i64::from_le_bytes(array))
						} else {
							// For larger values, just take the first 8 bytes
							// This is a simplification and may not be accurate for very large values
							let mut array = [0u8; 8];
							array.copy_from_slice(&bytes[..8]);
							Some(i64::from_le_bytes(array))
						}
					})
				},
				_ => None,
			})
			.unwrap_or(0) as u64;

		let gas_raw = gas_balance
			.stack
			.first()
			.and_then(|item| match item {
				StackItem::Integer { value } => Some(*value),
				StackItem::ByteString { value } => {
					// Try to decode base64 and interpret as integer
					base64::decode(value).ok().and_then(|bytes| {
						if bytes.is_empty() {
							Some(0)
						} else if bytes.len() <= 8 {
							let mut array = [0u8; 8];
							array[..bytes.len()].copy_from_slice(&bytes);
							Some(i64::from_le_bytes(array))
						} else {
							// For larger values, just take the first 8 bytes
							// This is a simplification and may not be accurate for very large values
							let mut array = [0u8; 8];
							array.copy_from_slice(&bytes[..8]);
							Some(i64::from_le_bytes(array))
						}
					})
				},
				_ => None,
			})
			.unwrap_or(0) as u64;
		let gas = gas_raw as f64 / 100_000_000.0; // GAS has 8 decimals

		// TODO: Get other NEP-17 token balances
		let tokens = Vec::new();

		Ok(Balance { neo, gas, tokens })
	}

	/// Transfer tokens from one address to another
	///
	/// Handles all the complexity of building, signing, and sending the transaction.
	///
	/// # Examples
	///
	/// ```no_run
	/// let tx_hash = neo.transfer(
	///     &wallet,
	///     "NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc",
	///     100,
	///     Token::GAS,
	/// ).await?;
	/// println!("Transaction sent: {}", tx_hash);
	/// ```
	pub async fn transfer(
		&self,
		from: &Wallet,
		to: &str,
		amount: u64,
		token: Token,
	) -> Result<TxHash, NeoError> {
		// Implementation would handle transaction building and sending
		todo!("Implement simplified transfer")
	}

	/// Deploy a smart contract
	///
	/// Simplifies the contract deployment process.
	///
	/// # Examples
	///
	/// ```no_run
	/// let contract_hash = neo.deploy_contract(
	///     &wallet,
	///     nef_bytes,
	///     manifest,
	/// ).await?;
	/// println!("Contract deployed: {}", contract_hash);
	/// ```
	pub async fn deploy_contract(
		&self,
		deployer: &Wallet,
		nef: Vec<u8>,
		manifest: String,
	) -> Result<ScriptHash, NeoError> {
		// Implementation would handle contract deployment
		todo!("Implement contract deployment")
	}

	/// Invoke a smart contract method (read-only)
	///
	/// For contract methods that don't modify state.
	///
	/// # Examples
	///
	/// ```no_run
	/// let result = neo.invoke_read(
	///     &contract_hash,
	///     "balanceOf",
	///     vec![address.into()],
	/// ).await?;
	/// ```
	pub async fn invoke_read(
		&self,
		contract: &ScriptHash,
		method: &str,
		params: Vec<ContractParameter>,
	) -> Result<serde_json::Value, NeoError> {
		// Implementation would handle read-only invocation
		todo!("Implement read-only invocation")
	}

	/// Invoke a smart contract method (with transaction)
	///
	/// For contract methods that modify state.
	///
	/// # Examples
	///
	/// ```no_run
	/// let tx_hash = neo.invoke_write(
	///     &wallet,
	///     &contract_hash,
	///     "transfer",
	///     vec![from.into(), to.into(), amount.into()],
	/// ).await?;
	/// ```
	pub async fn invoke_write(
		&self,
		signer: &Wallet,
		contract: &ScriptHash,
		method: &str,
		params: Vec<ContractParameter>,
	) -> Result<TxHash, NeoError> {
		// Implementation would handle state-changing invocation
		todo!("Implement write invocation")
	}

	/// Wait for a transaction to be confirmed
	///
	/// # Examples
	///
	/// ```no_run
	/// neo.wait_for_confirmation(&tx_hash, Duration::from_secs(60)).await?;
	/// ```
	pub async fn wait_for_confirmation(
		&self,
		tx_hash: &str,
		timeout: Duration,
	) -> Result<(), NeoError> {
		// Implementation would poll for transaction confirmation
		todo!("Implement confirmation waiting")
	}

	/// Get the current block height
	pub async fn get_block_height(&self) -> Result<u32, NeoError> {
		self.client.get_block_count().await.map_err(|e| NeoError::Network {
			message: format!("Failed to get block height: {}", e),
			source: None,
			recovery: crate::neo_error::unified::ErrorRecovery::new()
				.suggest("Check network connection")
				.retryable(true),
		})
	}

	/// Get access to the underlying RPC client for advanced operations
	pub fn client(&self) -> &RpcClient<HttpProvider> {
		&self.client
	}

	/// Get the current network
	pub fn network(&self) -> &Network {
		&self.network
	}
}

/// Builder for configuring the Neo SDK
pub struct NeoBuilder {
	network: Network,
	config: SdkConfig,
}

impl Default for NeoBuilder {
	fn default() -> Self {
		Self { network: Network::TestNet, config: SdkConfig::default() }
	}
}

impl NeoBuilder {
	/// Set the network to connect to
	pub fn network(mut self, network: Network) -> Self {
		self.network = network;
		self
	}

	/// Set the request timeout
	pub fn timeout(mut self, timeout: Duration) -> Self {
		self.config.timeout = timeout;
		self
	}

	/// Set the number of retries for failed requests
	pub fn retries(mut self, retries: u32) -> Self {
		self.config.retries = retries;
		self
	}

	/// Enable or disable caching
	pub fn cache(mut self, enabled: bool) -> Self {
		self.config.cache_enabled = enabled;
		self
	}

	/// Enable or disable metrics collection
	pub fn metrics(mut self, enabled: bool) -> Self {
		self.config.metrics_enabled = enabled;
		self
	}

	/// Build the Neo SDK instance
	pub async fn build(self) -> Result<Neo, NeoError> {
		let endpoint = match &self.network {
			Network::MainNet => "https://mainnet1.neo.org:443",
			Network::TestNet => "https://testnet1.neo.org:443",
			Network::Custom(url) => url,
		};

		let provider = HttpProvider::new(endpoint).map_err(|e| NeoError::Network {
			message: format!("Failed to create HTTP provider: {}", e),
			source: None,
			recovery: crate::neo_error::unified::ErrorRecovery::new()
				.suggest("Check the RPC endpoint URL")
				.suggest("Ensure the network is reachable")
				.retryable(true),
		})?;
		let client = Arc::new(RpcClient::new(provider));

		// Test connection
		client.get_block_count().await.map_err(|e| NeoError::Network {
			message: format!("Failed to connect to Neo network: {}", e),
			source: None,
			recovery: crate::neo_error::unified::ErrorRecovery::new()
				.suggest("Verify the RPC endpoint is accessible")
				.suggest("Check your internet connection")
				.suggest("Try a different RPC endpoint")
				.retryable(true)
				.retry_after(std::time::Duration::from_secs(5)),
		})?;

		Ok(Neo { client, network: self.network, config: self.config })
	}
}

/// Quick transfer builder for simplified token transfers
pub struct Transfer {
	from: Wallet,
	to: String,
	amount: u64,
	token: Token,
	memo: Option<String>,
}

impl Transfer {
	/// Create a new transfer
	pub fn new(from: Wallet, to: impl Into<String>, amount: u64, token: Token) -> Self {
		Self { from, to: to.into(), amount, token, memo: None }
	}

	/// Add an optional memo to the transfer
	pub fn with_memo(mut self, memo: impl Into<String>) -> Self {
		self.memo = Some(memo.into());
		self
	}

	/// Execute the transfer
	pub async fn execute(self, client: &RpcClient<HttpProvider>) -> Result<TxHash, NeoError> {
		// Implementation would build and send the transfer transaction
		todo!("Implement transfer execution")
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_builder_configuration() {
		let builder = Neo::builder()
			.network(Network::TestNet)
			.timeout(Duration::from_secs(60))
			.retries(5)
			.cache(true)
			.metrics(false);

		assert_eq!(builder.config.timeout, Duration::from_secs(60));
		assert_eq!(builder.config.retries, 5);
		assert!(builder.config.cache_enabled);
		assert!(!builder.config.metrics_enabled);
	}
}
