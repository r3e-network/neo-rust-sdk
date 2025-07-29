// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use neo_gui::{AppState, command_exports as commands};

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
		.on_window_event(|_window, event| if let tauri::WindowEvent::CloseRequested { .. } = event {
  				log::info!("Application closing...");
  				// Perform cleanup here
  			})
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
