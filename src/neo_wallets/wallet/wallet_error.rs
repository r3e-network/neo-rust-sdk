use crate::{
	builder::{BuilderError, TransactionError},
	crypto::CryptoError,
	neo_clients::ProviderError,
};
use p256::ecdsa;
use thiserror::Error;

/// Errors that may occur within wallet operations.
///
/// This enum encompasses a range of errors that can arise when interacting with
/// cryptocurrency wallets, including but not limited to account state issues, cryptographic
/// errors, and IO operations. It is designed to be comprehensive, covering errors
/// from underlying libraries (such as ECDSA operations, hex encoding/decoding) as well
/// as wallet-specific errors like missing key pairs or default accounts.
///
/// # Variants
///
/// - `AccountState`: Represents errors related to an account's state, such as when an account
///   cannot be found or is in an invalid state for the requested operation.
/// - `NoKeyPair`: Indicates that a key pair was expected but none was found.
/// - `EcdsaError`: Wraps errors from the `ecdsa` crate, typically related to signature operations.
/// - `HexError`: Wraps errors from the `hex` crate, often arising during hex encoding or decoding.
/// - `IoError`: Wraps standard IO errors that might occur during file operations.
/// - `NoDefaultAccount`: Signals that a default account was expected but not set.
/// - `InvalidKeyPair`: General error for when a key pair is invalid or fails validation.
/// - `CryptoError`: Wraps cryptographic errors, potentially from operations like hashing or encryption.
/// - `TransactionError`: Encapsulates errors that may occur during transaction creation or processing.
/// - `BuilderError`: Wraps errors that occur during the construction of complex objects, possibly due to invalid parameters.
/// - `DecryptionError`: Indicates an error occurred during account decryption.
/// - `SigningError`: Indicates an error occurred during transaction signing.
/// - `FileError`: Indicates an error occurred during file operations.
/// - `ParseError`: Indicates an error occurred during parsing operations.
/// - `ImportError`: Indicates an error occurred during key import operations.
/// - `InvalidPassword`: Indicates that an invalid password was provided.
/// - `NoAccounts`: Indicates that no accounts were found in the wallet.
/// - `YubiHsmError`: Wraps errors related to YubiHSM operations.
/// - `ProviderError`: Wraps errors from the RPC provider.
///
/// # Examples
///
/// Handling a `WalletError` might look like this:
///
/// ```
/// # use neo3::prelude::*;
/// # fn main() -> Result<(), wallets::WalletError> {
/// let result = some_wallet_operation();
///     match result {
///         Ok(_) => println!("Operation successful"),
///         Err(wallets::WalletError::NoKeyPair) => println!("Key pair missing"),
///         Err(e) => println!("An error occurred: {:?}", e),
///     }
/// #    Ok(())
/// # }
/// # fn some_wallet_operation() -> Result<(), wallets::WalletError> {
/// #    Err(wallets::WalletError::NoKeyPair)
/// # }
/// ```
///
/// This approach allows for precise error handling and reporting, facilitating debugging and user feedback.
#[derive(Error, Debug)]
pub enum WalletError {
	/// Error indicating an issue with the account's state, such as being locked or
	/// insufficient funds. The contained message provides additional detail.
	#[error("Account state error: {0}")]
	AccountState(String),

	/// Indicates that no key pair is available for a cryptographic operation, possibly
	/// because it has not been generated or imported.
	#[error("No key pair")]
	NoKeyPair,

	/// Wraps errors from the `ecdsa` crate, related to ECDSA signature operations.
	/// This could include errors during signature generation or verification.
	#[error(transparent)]
	EcdsaError(#[from] ecdsa::Error),

	/// Represents errors encountered during hex encoding or decoding operations,
	/// such as an invalid hex character or incorrect string length.
	#[error(transparent)]
	HexError(#[from] hex::FromHexError),

	/// Encapsulates errors arising from IO operations, like reading from or writing to
	/// files. This includes file not found, permissions issues, and other file-related errors.
	#[error(transparent)]
	IoError(#[from] std::io::Error),

	/// Signifies that the wallet does not have a designated default account, which might
	/// be required for certain operations or configurations.
	#[error("No default account")]
	NoDefaultAccount,

	/// Used when a key pair is found to be invalid, such as when a private key does not
	/// match the public key, or the key pair cannot be used for signing due to corruption.
	#[error("Invalid key pair")]
	SignHashError,

	/// Wraps generic cryptographic errors that might occur during operations such as
	/// encryption, decryption, hashing, or key generation.
	#[error(transparent)]
	CryptoError(#[from] CryptoError),

	/// Covers errors related to the creation, signing, or broadcasting of transactions,
	/// including invalid transaction formats, insufficient gas, or nonce issues.
	#[error(transparent)]
	TransactionError(#[from] TransactionError),

	/// Indicates issues encountered during the construction or configuration of wallet
	/// components, such as invalid parameters or configurations that cannot be applied.
	#[error(transparent)]
	BuilderError(#[from] BuilderError),

	/// Indicates an invalid signature
	#[error("Invalid signature")]
	VerifyError,

	/// Errors related to Ledger hardware wallet operations
	#[error("Ledger error: {0}")]
	LedgerError(String),

	/// Error indicating no accounts in wallet
	#[error("No accounts in wallet")]
	NoAccounts,

	/// Errors related to YubiHSM operations
	#[error("YubiHSM error: {0}")]
	YubiHsmError(String),

	/// Errors from the RPC provider
	#[error(transparent)]
	ProviderError(#[from] ProviderError),

	/// Errors during account decryption
	#[error("Decryption error: {0}")]
	DecryptionError(String),

	/// Errors during transaction signing
	#[error("Signing error: {0}")]
	SigningError(String),

	/// Errors during file operations
	#[error("File error: {0}")]
	FileError(String),

	/// Errors during parsing operations
	#[error("Parse error: {0}")]
	ParseError(String),

	/// Errors during key import operations
	#[error("Import error: {0}")]
	ImportError(String),

	/// Invalid password provided
	#[error("Invalid password")]
	InvalidPassword,

	/// Errors during deserialization
	#[error("Deserialization error: {0}")]
	DeserializationError(String),
}
