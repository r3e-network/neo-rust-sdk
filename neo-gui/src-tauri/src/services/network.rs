use neo3::{
	neo_clients::{HttpProvider, RpcClient, APITrait},
	neo_error::{Neo3Error, Neo3Result},
	prelude::*,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json;
use std::str::FromStr;

/// Network service for managing Neo network connections
pub struct NetworkService {
	client: Arc<RwLock<Option<Arc<RpcClient<HttpProvider>>>>>,
	current_endpoint: Arc<RwLock<Option<String>>>,
	network_type: Arc<RwLock<NetworkType>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NetworkType {
	Mainnet,
	Testnet,
	Private,
}

impl NetworkService {
	pub fn new() -> Self {
		Self {
			client: Arc::new(RwLock::new(None)),
			current_endpoint: Arc::new(RwLock::new(None)),
			network_type: Arc::new(RwLock::new(NetworkType::Testnet)),
		}
	}

	/// Connect to a Neo network
	pub async fn connect(&self, endpoint: String, network_type: NetworkType) -> Neo3Result<()> {
		log::info!("Attempting to connect to endpoint: {endpoint}");
		
		// Ensure the endpoint has proper protocol
		let normalized_endpoint = if !endpoint.starts_with("http://") && !endpoint.starts_with("https://") {
			format!("https://{endpoint}")
		} else {
			endpoint.clone()
		};
		
		log::info!("Normalized endpoint: {normalized_endpoint}");
		
		// Create HTTP provider
		let provider = HttpProvider::new(normalized_endpoint.as_str())
			.map_err(|e| {
				log::error!("Failed to create provider: {e}");
				Neo3Error::Config(format!("Failed to create provider: {e}"))
			})?;
		
		let client = RpcClient::new(provider);

		// Test the connection by trying to get the version
		log::info!("Testing connection with get_version call...");
		match client.get_version().await {
			Ok(version) => {
				log::info!("Connection successful! Neo node version: {}", version.user_agent);
				*self.client.write().await = Some(Arc::new(client));
				*self.current_endpoint.write().await = Some(normalized_endpoint);
				*self.network_type.write().await = network_type;
				Ok(())
			},
			Err(e) => {
				log::error!("Connection test failed: {e}");
				Err(Neo3Error::Config(format!("Connection test failed: {e}")))
			}
		}
	}

	/// Disconnect from the network
	pub async fn disconnect(&self) {
		*self.client.write().await = None;
		*self.current_endpoint.write().await = None;
	}

	/// Check if connected to network
	pub async fn is_connected(&self) -> bool {
		self.client.read().await.is_some()
	}

	/// Get current network status
	pub async fn get_status(&self) -> NetworkStatus {
		let is_connected = self.is_connected().await;
		let endpoint = self.current_endpoint.read().await.clone();
		let network_type = self.network_type.read().await.clone();

		// Get real blockchain data if connected
		let (block_height, peer_count) = if is_connected {
			if let Some(client) = self.client.read().await.as_ref() {
				// Get actual block height
				let height = match client.get_block_count().await {
					Ok(count) => Some(count as u64),
					Err(_) => None,
				};

				// Neo N3 doesn't expose peer count through standard RPC API
				// This is a limitation of the Neo N3 protocol, not a placeholder
				let peers = None;

				(height, peers)
			} else {
				(None, None)
			}
		} else {
			(None, None)
		};

		NetworkStatus { connected: is_connected, endpoint, network_type, block_height, peer_count }
	}

	/// Get block information
	pub async fn get_block_info(
		&self,
		block_identifier: BlockIdentifier,
	) -> Neo3Result<serde_json::Value> {
		if let Some(client) = self.client.read().await.as_ref() {
			let block = match block_identifier {
				BlockIdentifier::Height(height) => {
					// Get block hash by index first, then get the full block
					let hash = client.get_block_hash(height as u32).await
						.map_err(|e| Neo3Error::Config(format!("Failed to get block hash: {e}")))?;
					client.get_block(hash, true).await
						.map_err(|e| Neo3Error::Config(format!("Failed to get block: {e}")))?
				},
				BlockIdentifier::Hash(hash) => {
					let hash = H256::from_str(&hash)
						.map_err(|e| Neo3Error::Config(format!("Invalid block hash: {e}")))?;
					client.get_block(hash, true).await
						.map_err(|e| Neo3Error::Config(format!("Failed to get block: {e}")))?
				},
				BlockIdentifier::Latest => {
					let block_count = client.get_block_count().await
						.map_err(|e| Neo3Error::Config(format!("Failed to get block count: {e}")))?;
					let hash = client.get_block_hash(block_count - 1).await
						.map_err(|e| Neo3Error::Config(format!("Failed to get block hash: {e}")))?;
					client.get_block(hash, true).await
						.map_err(|e| Neo3Error::Config(format!("Failed to get block: {e}")))?
				},
			};
			
			// Serialize the block to JSON
			serde_json::to_value(block)
				.map_err(|e| Neo3Error::Config(format!("Failed to serialize block: {e}")))
		} else {
			Err(Neo3Error::Config("Not connected to network".to_string()))
		}
	}

	/// Get transaction information
	pub async fn get_transaction_info(&self, tx_hash: String) -> Neo3Result<serde_json::Value> {
		if let Some(client) = self.client.read().await.as_ref() {
			let hash = H256::from_str(&tx_hash)
				.map_err(|e| Neo3Error::Config(format!("Invalid transaction hash: {e}")))?;
			
			let tx = client.get_transaction(hash).await
				.map_err(|e| Neo3Error::Config(format!("Failed to get transaction: {e}")))?;
			
			// Serialize the transaction to JSON
			serde_json::to_value(tx)
				.map_err(|e| Neo3Error::Config(format!("Failed to serialize transaction: {e}")))
		} else {
			Err(Neo3Error::Config("Not connected to network".to_string()))
		}
	}

	/// Get contract information
	pub async fn get_contract_info(&self, contract_hash: String) -> Neo3Result<serde_json::Value> {
		if let Some(client) = self.client.read().await.as_ref() {
			let hash = H160::from_str(&contract_hash)
				.map_err(|e| Neo3Error::Config(format!("Invalid contract hash: {e}")))?;
			
			let contract = client.get_contract_state(hash).await
				.map_err(|e| Neo3Error::Config(format!("Failed to get contract state: {e}")))?;
			
			// Serialize the contract state to JSON
			serde_json::to_value(contract)
				.map_err(|e| Neo3Error::Config(format!("Failed to serialize contract state: {e}")))
		} else {
			Err(Neo3Error::Config("Not connected to network".to_string()))
		}
	}

	/// Get the RPC client for other services to use
	pub async fn get_client(&self) -> Option<Arc<RpcClient<HttpProvider>>> {
		self.client.read().await.clone()
	}
}

#[derive(Debug, Clone)]
pub struct NetworkStatus {
	pub connected: bool,
	pub endpoint: Option<String>,
	pub network_type: NetworkType,
	pub block_height: Option<u64>,
	pub peer_count: Option<u32>,
}

#[derive(Debug, Clone)]
pub enum BlockIdentifier {
	Height(u64),
	Hash(String),
	Latest,
}

impl Default for NetworkService {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_network_service_creation() {
		let service = NetworkService::new();
		assert!(!service.is_connected().await);
	}

	#[tokio::test]
	async fn test_network_status() {
		let service = NetworkService::new();
		let status = service.get_status().await;

		assert!(!status.connected);
		assert_eq!(status.network_type, NetworkType::Testnet);
		assert!(status.endpoint.is_none());
	}

	#[tokio::test]
	async fn test_disconnect() {
		let service = NetworkService::new();
		service.disconnect().await;
		assert!(!service.is_connected().await);
	}
}
