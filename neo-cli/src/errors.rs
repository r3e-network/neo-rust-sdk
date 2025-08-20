use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
	#[error("Invalid argument: {0} - {1}")]
	InvalidArgument(String, String),

	#[error("Wallet error: {0}")]
	Wallet(String),

	#[error("File system error: {0}")]
	FileSystem(String),

	#[error("File error: {0}")]
	FileError(String),

	#[error("JSON error: {0}")]
	Json(String),

	#[error("Network error: {0}")]
	Network(String),

	#[error("Network error: {0}")]
	NetworkError(String),

	#[error("Transaction error: {0}")]
	Transaction(String),

	#[error("RPC error: {0}")]
	Rpc(String),

	#[error("NeoFS error: {0}")]
	NeoFS(String),

	#[error("Contract error: {0}")]
	Contract(String),

	#[error("Authentication error: {0}")]
	Authentication(String),

	#[error("Security error: {0}")]
	Security(String),

	#[error("Timeout error: {0}")]
	Timeout(String),

	#[error("RPC error: {0}")]
	RpcError(String),

	#[error("Not implemented: {0}")]
	NotImplemented(String),

	#[error("Configuration error: {0}")]
	Config(String),

	#[error("Input error: {0}")]
	Input(String),

	#[error("SDK error: {0}")]
	Sdk(String),

	#[error("Transaction builder error: {0}")]
	TransactionBuilder(String),

	#[error("Builder error: {0}")]
	Builder(String),

	#[error("Unknown error: {0}")]
	Unknown(String),

	#[error("Invalid input: {0}")]
	InvalidInput(String),

	#[error("Invalid format: {0}")]
	InvalidFormat(String),

	#[error("Provider error: {0}")]
	Provider(String),

	#[error("External error: {0}")]
	External(String),

	#[error("User cancelled: {0}")]
	UserCancelled(String),

	#[error("Transaction failed: {0}")]
	TransactionFailed(String),

	#[error("No RPC client connected")]
	NoRpcClient,

	#[error("No account loaded")]
	NoAccount,

	#[error("No wallet loaded")]
	NoWallet,

	#[error("Wallet not loaded: {0}")]
	WalletNotLoaded(String),

	#[error("Wallet operation failed: {0}")]
	WalletOperation(String),

	#[error("Invalid operation: {0}")]
	InvalidOperation(String),

	#[error("IO error: {0}")]
	Io(#[from] std::io::Error),

	#[error("IO error: {0}")]
	IoError(std::io::Error),

	#[error("Serde JSON error: {0}")]
	SerdeJson(#[from] serde_json::Error),

	#[error("Reqwest error: {0}")]
	Reqwest(#[from] reqwest::Error),

	#[error("Tokio join error: {0}")]
	TokioJoin(#[from] tokio::task::JoinError),

	#[error("Error: {0}")]
	Other(String),
}

// Implement From trait for String
impl From<String> for CliError {
	fn from(error: String) -> Self {
		CliError::Other(error)
	}
}

// Implement From trait for &str
impl From<&str> for CliError {
	fn from(error: &str) -> Self {
		CliError::Other(error.to_string())
	}
}

// Comment out the utils::error implementation to resolve the error
/*
// Implement From trait for utils::error::CliError
impl From<crate::utils::error::CliError> for CliError {
	fn from(error: crate::utils::error::CliError) -> Self {
		match error {
			crate::utils::error::CliError::Config(s) => CliError::Config(s),
			crate::utils::error::CliError::Wallet(s) => CliError::Wallet(s),
			crate::utils::error::CliError::Network(s) => CliError::Network(s),
			crate::utils::error::CliError::Rpc(s) => CliError::Rpc(s),
			crate::utils::error::CliError::Input(s) => CliError::Input(s),
			crate::utils::error::CliError::Io(e) => CliError::Io(e),
			crate::utils::error::CliError::Sdk(s) => CliError::Sdk(s),
			crate::utils::error::CliError::Transaction(s) => CliError::Transaction(s),
			crate::utils::error::CliError::TransactionBuilder(s) => CliError::TransactionBuilder(s),
			crate::utils::error::CliError::Builder(s) => CliError::Builder(s),
			crate::utils::error::CliError::Unknown(s) => CliError::Unknown(s),
			crate::utils::error::CliError::Anyhow(s) => CliError::Other(s),
			crate::utils::error::CliError::InvalidInput(s) => CliError::InvalidInput(s),
			crate::utils::error::CliError::InvalidFormat(s) => CliError::InvalidFormat(s),
			crate::utils::error::CliError::NotImplemented(s) => CliError::NotImplemented(s),
			crate::utils::error::CliError::External(s) => CliError::External(s),
			_ => CliError::Other(format!("Unknown error: {:?}", error)),
		}
	}
}
*/

// Implement From trait for BuilderError
impl From<neo3::neo_builder::BuilderError> for CliError {
	fn from(error: neo3::neo_builder::BuilderError) -> Self {
		CliError::Builder(error.to_string())
	}
}

// Implement From trait for TransactionError
impl From<neo3::neo_builder::TransactionError> for CliError {
	fn from(error: neo3::neo_builder::TransactionError) -> Self {
		CliError::Transaction(error.to_string())
	}
}

// Implement From trait for ProviderError
impl From<neo3::neo_clients::ProviderError> for CliError {
	fn from(error: neo3::neo_clients::ProviderError) -> Self {
		CliError::Sdk(error.to_string())
	}
}
