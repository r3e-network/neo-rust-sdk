use crate::{
	services::network::{BlockIdentifier, NetworkType},
	ApiResponse, AppState,
};
use serde::{Deserialize, Serialize};
use tauri::{command, State};

#[derive(Debug, Deserialize)]
pub struct ConnectNetworkRequest {
	pub endpoint: String,
	pub network_type: String,
}

#[derive(Debug, Serialize)]
pub struct NetworkStatus {
	pub connected: bool,
	pub endpoint: Option<String>,
	pub network_type: String,
	pub block_height: Option<u64>,
	pub peer_count: Option<u32>,
}

/// Connect to a Neo network
#[command]
pub async fn connect_to_network(
	request: ConnectNetworkRequest,
	state: State<'_, AppState>,
) -> Result<ApiResponse<bool>, String> {
	log::info!("Connecting to network: {} ({})", request.endpoint, request.network_type);

	let network_type = match request.network_type.as_str() {
		"mainnet" => NetworkType::Mainnet,
		"testnet" => NetworkType::Testnet,
		"private" => NetworkType::Private,
		_ => NetworkType::Testnet,
	};

	match state.network_service.connect(request.endpoint.clone(), network_type).await {
		Ok(_) => {
			log::info!("Successfully connected to network: {}", request.endpoint);

			// Update transaction service with new client
			if let Some(client) = state.network_service.get_client().await {
				state.transaction_service.set_rpc_client(client).await;
			}

			Ok(ApiResponse::success(true))
		},
		Err(e) => {
			log::error!("Failed to connect to network: {}", e);
			Ok(ApiResponse::error(format!("Failed to connect: {}", e)))
		},
	}
}

/// Disconnect from the network
#[command]
pub async fn disconnect_from_network(
	state: State<'_, AppState>,
) -> Result<ApiResponse<bool>, String> {
	log::info!("Disconnecting from network");

	state.network_service.disconnect().await;
	// Transaction service will handle the disconnected state internally

	log::info!("Disconnected from network");
	Ok(ApiResponse::success(true))
}

/// Get network status
#[command]
pub async fn get_network_status(
	state: State<'_, AppState>,
) -> Result<ApiResponse<NetworkStatus>, String> {
	log::info!("Getting network status");

	let status = state.network_service.get_status().await;

	let response = NetworkStatus {
		connected: status.connected,
		endpoint: status.endpoint,
		network_type: match status.network_type {
			NetworkType::Mainnet => "mainnet".to_string(),
			NetworkType::Testnet => "testnet".to_string(),
			NetworkType::Private => "private".to_string(),
		},
		block_height: status.block_height,
		peer_count: status.peer_count,
	};

	Ok(ApiResponse::success(response))
}

/// Get block information
#[command]
pub async fn get_block_info(
	block_identifier: String,
	state: State<'_, AppState>,
) -> Result<ApiResponse<serde_json::Value>, String> {
	log::info!("Getting block info: {}", block_identifier);

	let identifier = if block_identifier == "latest" {
		BlockIdentifier::Latest
	} else if let Ok(height) = block_identifier.parse::<u64>() {
		BlockIdentifier::Height(height)
	} else {
		BlockIdentifier::Hash(block_identifier)
	};

	match state.network_service.get_block_info(identifier).await {
		Ok(block_info) => {
			log::info!("Successfully retrieved block info");
			Ok(ApiResponse::success(block_info))
		},
		Err(e) => {
			log::error!("Failed to get block info: {}", e);
			Ok(ApiResponse::error(format!("Failed to get block info: {}", e)))
		},
	}
}

/// Get transaction information
#[command]
pub async fn get_transaction_info(
	tx_hash: String,
	state: State<'_, AppState>,
) -> Result<ApiResponse<serde_json::Value>, String> {
	log::info!("Getting transaction info: {}", tx_hash);

	match state.network_service.get_transaction_info(tx_hash).await {
		Ok(tx_info) => {
			log::info!("Successfully retrieved transaction info");
			Ok(ApiResponse::success(tx_info))
		},
		Err(e) => {
			log::error!("Failed to get transaction info: {}", e);
			Ok(ApiResponse::error(format!("Failed to get transaction info: {}", e)))
		},
	}
}

/// Get network contract information
#[command]
pub async fn get_network_contract_info(
	contract_hash: String,
	state: State<'_, AppState>,
) -> Result<ApiResponse<serde_json::Value>, String> {
	log::info!("Getting contract info: {}", contract_hash);

	match state.network_service.get_contract_info(contract_hash).await {
		Ok(contract_info) => {
			log::info!("Successfully retrieved contract info");
			Ok(ApiResponse::success(contract_info))
		},
		Err(e) => {
			log::error!("Failed to get contract info: {}", e);
			Ok(ApiResponse::error(format!("Failed to get contract info: {}", e)))
		},
	}
}
