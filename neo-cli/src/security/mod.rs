#![allow(unused_imports, dead_code)]

/// Security module for Neo CLI
/// Provides comprehensive security features including keychain integration,
/// error handling with retry logic, session management, and network failover
pub mod error_handler;
pub mod keychain;
pub mod network_failover;
pub mod session;

pub use error_handler::{ErrorHandler, ErrorReporter, RecoveryStrategy, RetryConfig, RetryHandler};
pub use keychain::{KeychainManager, SecureCredential, SecureWalletStorage};
pub use network_failover::{
	EndpointHealth, FailoverConfig, NetworkFailover, NetworkFailoverBuilder,
};
pub use session::{Session, SessionConfig, SessionGuard, SessionManager};

/// Initialize security features
pub fn initialize_security() -> Result<SecurityContext, crate::errors::CliError> {
	let keychain = SecureWalletStorage::new()?;
	let session_manager = SessionManager::new(SessionConfig::default());
	let error_handler = ErrorHandler::new();

	Ok(SecurityContext { keychain, session_manager, error_handler })
}

/// Security context containing all security components
pub struct SecurityContext {
	pub keychain: SecureWalletStorage,
	pub session_manager: SessionManager,
	pub error_handler: ErrorHandler,
}

impl SecurityContext {
	/// Create a new security context with default configuration
	pub fn new() -> Result<Self, crate::errors::CliError> {
		initialize_security()
	}

	/// Create a new security context with custom configuration
	pub fn with_config(session_config: SessionConfig) -> Result<Self, crate::errors::CliError> {
		let keychain = SecureWalletStorage::new()?;
		let session_manager = SessionManager::new(session_config);
		let error_handler = ErrorHandler::new();

		Ok(Self { keychain, session_manager, error_handler })
	}
}
