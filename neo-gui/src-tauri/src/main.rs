// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::{DateTime, Utc};
use neo3::neo_clients::ProductionRpcClient;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

mod commands;
mod services;

use services::{
	network::NetworkService, settings::SettingsService, transaction::TransactionService,
	wallet::WalletService,
};

// Application state with full NeoRust SDK integration
pub struct AppState {
	pub wallet_service: Arc<WalletService>,
	pub network_service: Arc<NetworkService>,
	pub transaction_service: Arc<TransactionService>,
	pub settings_service: Arc<SettingsService>,
	pub rpc_client: Arc<Mutex<Option<ProductionRpcClient>>>,
}

impl Default for AppState {
	fn default() -> Self {
		Self {
			wallet_service: Arc::new(WalletService::new()),
			network_service: Arc::new(NetworkService::new()),
			transaction_service: Arc::new(TransactionService::new()),
			settings_service: Arc::new(SettingsService::new()),
			rpc_client: Arc::new(Mutex::new(None)),
		}
	}
}

// Common types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
	pub success: bool,
	pub data: Option<T>,
	pub error: Option<String>,
	pub timestamp: DateTime<Utc>,
}

impl<T> ApiResponse<T> {
	pub fn success(data: T) -> Self {
		Self { success: true, data: Some(data), error: None, timestamp: Utc::now() }
	}

	pub fn error(message: String) -> Self {
		Self { success: false, data: None, error: Some(message), timestamp: Utc::now() }
	}
}

fn main() {
	env_logger::init();

	tauri::Builder::default()
		.manage(AppState::default())
		.invoke_handler(tauri::generate_handler![
			// Wallet commands
			commands::wallet::create_wallet,
			commands::wallet::open_wallet,
			commands::wallet::close_wallet,
			commands::wallet::list_wallets,
			commands::wallet::get_wallet_info,
			commands::wallet::create_address,
			commands::wallet::import_private_key,
			commands::wallet::export_private_key,
			commands::wallet::get_balance,
			commands::wallet::send_transaction,
			commands::wallet::get_transaction_history,
			// Network commands
			commands::network::connect_to_network,
			commands::network::disconnect_from_network,
			commands::network::get_network_status,
			commands::network::get_block_info,
			commands::network::get_transaction_info,
			commands::network::get_network_contract_info,
			// Contract commands
			commands::contract::deploy_contract,
			commands::contract::invoke_contract,
			commands::contract::get_contract_info,
			// DeFi commands
			commands::defi::get_token_info,
			commands::defi::swap_tokens,
			commands::defi::add_liquidity,
			commands::defi::remove_liquidity,
			commands::defi::stake_tokens,
			commands::defi::unstake_tokens,
			commands::defi::get_pool_info,
			commands::defi::get_dex_prices,
			// NFT commands
			commands::nft::mint_nft,
			commands::nft::transfer_nft,
			commands::nft::get_nft_info,
			commands::nft::list_user_nfts,
			commands::nft::get_nft_market_data,
			// Settings commands
			commands::settings::get_settings,
			commands::settings::update_settings,
			commands::settings::reset_settings,
			// Utility commands
			commands::utils::encode_data,
			commands::utils::decode_data,
			commands::utils::hash_data,
			commands::utils::validate_address,
			commands::utils::format_amount,
		])
		.setup(|_app| {
			// Initialize application
			log::info!("Neo GUI application starting...");
			Ok(())
		})
		.on_window_event(|_window, event| match event {
			tauri::WindowEvent::CloseRequested { .. } => {
				log::info!("Application closing...");
				// Perform cleanup here
			},
			_ => {},
		})
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
