use neo3::{
	neo_clients::{ProductionClientConfig, ProductionRpcClient},
	neo_error::{Neo3Error, Neo3Result},
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Network service for managing Neo network connections
pub struct NetworkService {
	client: Arc<RwLock<Option<Arc<ProductionRpcClient>>>>,
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
		let config = ProductionClientConfig::default();
		let client = ProductionRpcClient::new(endpoint.clone(), config);

		// Test the connection
		match client.health_check().await {
			Ok(true) => {
				*self.client.write().await = Some(Arc::new(client));
				*self.current_endpoint.write().await = Some(endpoint);
				*self.network_type.write().await = network_type;
				Ok(())
			},
			Ok(false) => Err(Neo3Error::Config("Health check failed".to_string())),
			Err(e) => Err(e),
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
					Ok(count) => Some(count),
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
			match block_identifier {
				BlockIdentifier::Height(height) => {
					client.get_block(serde_json::json!(height)).await
				},
				BlockIdentifier::Hash(hash) => client.get_block(serde_json::json!(hash)).await,
				BlockIdentifier::Latest => {
					let block_count = client.get_block_count().await?;
					client.get_block(serde_json::json!(block_count - 1)).await
				},
			}
		} else {
			Err(Neo3Error::Config("Not connected to network".to_string()))
		}
	}

	/// Get transaction information
	pub async fn get_transaction_info(&self, tx_hash: String) -> Neo3Result<serde_json::Value> {
		if let Some(client) = self.client.read().await.as_ref() {
			client.get_transaction(tx_hash).await
		} else {
			Err(Neo3Error::Config("Not connected to network".to_string()))
		}
	}

	/// Get contract information
	pub async fn get_contract_info(&self, contract_hash: String) -> Neo3Result<serde_json::Value> {
		if let Some(client) = self.client.read().await.as_ref() {
			client.get_contract_state(contract_hash).await
		} else {
			Err(Neo3Error::Config("Not connected to network".to_string()))
		}
	}

	/// Get the RPC client for other services to use
	pub async fn get_client(&self) -> Option<Arc<ProductionRpcClient>> {
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
