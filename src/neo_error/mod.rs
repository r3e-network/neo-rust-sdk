use thiserror::Error;

// Include the unified error module for improved developer experience
pub mod unified;

/// Comprehensive error types for the Neo3 SDK
#[derive(Error, Debug)]
pub enum Neo3Error {
	/// Cryptographic operation errors
	#[error("Cryptographic error: {0}")]
	Crypto(#[from] CryptoError),

	/// Wallet operation errors
	#[error("Wallet error: {0}")]
	Wallet(#[from] WalletError),

	/// Network/RPC communication errors
	#[error("Network error: {0}")]
	Network(#[from] NetworkError),

	/// Transaction building/validation errors
	#[error("Transaction error: {0}")]
	Transaction(#[from] TransactionError),

	/// Smart contract interaction errors
	#[error("Contract error: {0}")]
	Contract(#[from] ContractError),

	/// Serialization/deserialization errors
	#[error("Serialization error: {0}")]
	Serialization(#[from] SerializationError),

	/// Configuration errors
	#[error("Configuration error: {0}")]
	Config(String),

	/// Generic errors with context
	#[error("Error: {message}")]
	Generic { message: String },

	/// Unsupported operation error
	#[error("Unsupported operation: {0}")]
	UnsupportedOperation(String),
}

#[derive(Error, Debug)]
pub enum CryptoError {
	#[error("Invalid private key: {0}")]
	InvalidPrivateKey(String),

	#[error("Invalid public key: {0}")]
	InvalidPublicKey(String),

	#[error("Signature verification failed")]
	SignatureVerificationFailed,

	#[error("Key generation failed: {0}")]
	KeyGenerationFailed(String),

	#[error("Hash operation failed: {0}")]
	HashFailed(String),

	#[error("Encryption failed: {0}")]
	EncryptionFailed(String),

	#[error("Decryption failed: {0}")]
	DecryptionFailed(String),
}

#[derive(Error, Debug)]
pub enum WalletError {
	#[error("Wallet not found: {0}")]
	NotFound(String),

	#[error("Invalid password")]
	InvalidPassword,

	#[error("Account not found: {0}")]
	AccountNotFound(String),

	#[error("Wallet is locked")]
	WalletLocked,

	#[error("Backup operation failed: {0}")]
	BackupFailed(String),

	#[error("Recovery operation failed: {0}")]
	RecoveryFailed(String),

	#[error("Invalid wallet format: {0}")]
	InvalidFormat(String),

	#[error("IO error: {0}")]
	Io(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum NetworkError {
	#[error("Connection failed: {0}")]
	ConnectionFailed(String),

	#[error("Request timeout")]
	Timeout,

	#[error("Invalid response: {0}")]
	InvalidResponse(String),

	#[error("RPC error: {code} - {message}")]
	RpcError { code: i32, message: String },

	#[error("Network unreachable: {0}")]
	NetworkUnreachable(String),

	#[error("Rate limit exceeded")]
	RateLimitExceeded,

	#[error("HTTP error: {0}")]
	Http(#[from] reqwest::Error),
}

#[derive(Error, Debug)]
pub enum TransactionError {
	#[error("Invalid transaction: {0}")]
	Invalid(String),

	#[error("Insufficient funds: required {required}, available {available}")]
	InsufficientFunds { required: u64, available: u64 },

	#[error("Transaction too large: {size} bytes (max: {max})")]
	TooLarge { size: usize, max: usize },

	#[error("Invalid signature")]
	InvalidSignature,

	#[error("Transaction expired")]
	Expired,

	#[error("Nonce too low: {provided} (expected: {expected})")]
	NonceTooLow { provided: u64, expected: u64 },

	#[error("Gas limit exceeded: {used} (limit: {limit})")]
	GasLimitExceeded { used: u64, limit: u64 },
}

#[derive(Error, Debug)]
pub enum ContractError {
	#[error("Contract not found: {0}")]
	NotFound(String),

	#[error("Method not found: {0}")]
	MethodNotFound(String),

	#[error("Invalid parameters: {0}")]
	InvalidParameters(String),

	#[error("Execution failed: {0}")]
	ExecutionFailed(String),

	#[error("Insufficient gas: {0}")]
	InsufficientGas(String),

	#[error("Contract deployment failed: {0}")]
	DeploymentFailed(String),
}

#[derive(Error, Debug)]
pub enum SerializationError {
	#[error("JSON error: {0}")]
	Json(#[from] serde_json::Error),

	#[error("Invalid format: {0}")]
	InvalidFormat(String),

	#[error("Encoding error: {0}")]
	Encoding(String),

	#[error("Decoding error: {0}")]
	Decoding(String),
}

/// Result type alias for Neo3 operations
pub type Neo3Result<T> = Result<T, Neo3Error>;

/// Legacy alias for backward compatibility
pub type NeoError = Neo3Error;

// Additional From implementations for common error types
impl From<std::io::Error> for Neo3Error {
	fn from(err: std::io::Error) -> Self {
		Neo3Error::Wallet(WalletError::Io(err))
	}
}

impl From<serde_json::Error> for Neo3Error {
	fn from(err: serde_json::Error) -> Self {
		Neo3Error::Serialization(SerializationError::Json(err))
	}
}

/// Trait for adding context to errors
pub trait ErrorContext<T> {
	fn with_context<F>(self, f: F) -> Neo3Result<T>
	where
		F: FnOnce() -> String;
}

impl<T, E> ErrorContext<T> for Result<T, E>
where
	E: Into<Neo3Error>,
{
	fn with_context<F>(self, f: F) -> Neo3Result<T>
	where
		F: FnOnce() -> String,
	{
		self.map_err(|e| {
			let base_error = e.into();
			Neo3Error::Generic { message: format!("{}: {}", f(), base_error) }
		})
	}
}

/// Macro for creating context-aware errors
#[macro_export]
macro_rules! neo3_error {
    ($msg:expr) => {
        Neo3Error::Generic {
            message: $msg.to_string(),
        }
    };
    ($fmt:expr, $($arg:tt)*) => {
        Neo3Error::Generic {
            message: format!($fmt, $($arg)*),
        }
    };
}

/// Macro for early return with context
#[macro_export]
macro_rules! ensure {
    ($cond:expr, $msg:expr) => {
        if !$cond {
            return Err(neo3_error!($msg));
        }
    };
    ($cond:expr, $fmt:expr, $($arg:tt)*) => {
        if !$cond {
            return Err(neo3_error!($fmt, $($arg)*));
        }
    };
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_error_context() {
		let result: Result<(), std::io::Error> =
			Err(std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"));

		let with_context = result.with_context(|| "Failed to read configuration file".to_string());

		assert!(with_context.is_err());
		let error_msg = with_context.unwrap_err().to_string();
		assert!(error_msg.contains("Failed to read configuration file"));
		assert!(error_msg.contains("file not found"));
	}

	#[test]
	fn test_error_macros() {
		let error = neo3_error!("Something went wrong");
		assert_eq!(error.to_string(), "Error: Something went wrong");

		let error = neo3_error!("Value {} is invalid", 42);
		assert_eq!(error.to_string(), "Error: Value 42 is invalid");
	}
}
