//! Neo GUI Library
//!
//! This library provides the core functionality for the Neo GUI desktop application,
//! including wallet management, network connectivity, transaction handling, and settings.

#![warn(clippy::all)]

pub mod api;
pub mod commands;
pub mod services;
pub mod state;

// Re-export commonly used types
pub use api::ApiResponse;
pub use state::AppState;

// Re-export service modules
pub use services::{
	network::NetworkService, settings::SettingsService, transaction::TransactionService,
	wallet::WalletService,
};

// Re-export command modules
pub use commands::wallet;

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
