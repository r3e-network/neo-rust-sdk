//! Neo GUI Library
//!
//! This library provides the core functionality for the Neo GUI desktop application,
//! including wallet management, network connectivity, transaction handling, and settings.

#![warn(clippy::all)]

pub mod api;
pub mod commands;
pub mod services;

use std::sync::Arc;

// Re-export commonly used types
pub use api::ApiResponse;

// Re-export service modules
pub use services::{
	network::NetworkService, settings::SettingsService, transaction::TransactionService,
	wallet::WalletService,
};

// Re-export command modules
pub mod command_exports {
	pub use crate::commands::*;
}

// Application state with full NeoRust SDK integration
pub struct AppState {
	pub wallet_service: Arc<WalletService>,
	pub network_service: Arc<NetworkService>,
	pub transaction_service: Arc<TransactionService>,
	pub settings_service: Arc<SettingsService>,
}

impl Default for AppState {
	fn default() -> Self {
		Self {
			wallet_service: Arc::new(WalletService::new()),
			network_service: Arc::new(NetworkService::new()),
			transaction_service: Arc::new(TransactionService::new()),
			settings_service: Arc::new(SettingsService::new()),
		}
	}
}

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// Library description
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_version() {
		assert!(!VERSION.is_empty());
		assert!(VERSION.contains('.'));
	}

	#[test]
	fn test_name() {
		assert_eq!(NAME, "neo-gui");
	}

	#[test]
	fn test_description() {
		assert!(!DESCRIPTION.is_empty());
	}
}
