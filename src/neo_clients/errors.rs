use std::{error::Error, fmt::Debug, sync::Arc};

use crate::{crypto::CryptoError, neo_clients::JsonRpcError, TypeError};
use thiserror::Error;

#[derive(Debug, Error)]
/// An error thrown when making a call to the provider
pub enum ProviderError {
	/// An error during NNS name resolution
	#[error("nns name not found: {0}")]
	NnsError(String),
	/// Invalid reverse NNS name
	#[error("reverse nns name not pointing to itself: {0}")]
	NnsNotOwned(String),
	/// Error in underlying lib `serde_json`
	#[error(transparent)]
	SerdeJson(#[from] serde_json::Error),
	/// Error in underlying lib `hex`
	#[error(transparent)]
	HexError(#[from] hex::FromHexError),
	/// Error in underlying lib `reqwest`
	#[error(transparent)]
	HTTPError(#[from] Arc<reqwest::Error>),
	/// Reponse error
	#[error(transparent)]
	JsonRpcError(#[from] JsonRpcError),
	/// Custom error from unknown source
	#[error("custom error: {0}")]
	CustomError(String),
	/// RPC method is not supported by this provider
	#[error("unsupported RPC")]
	UnsupportedRPC,
	/// Node is not supported by this provider
	#[error("unsupported node client")]
	UnsupportedNodeClient,
	/// Signer is not available to this provider.
	#[error("Attempted to sign a transaction with no available signer. Hint: did you mean to use a SignerMiddleware?"
    )]
	SignerUnavailable,
	#[error("Illegal state: {0}")]
	IllegalState(String),
	#[error("Invalid address")]
	InvalidAddress,
	#[error(transparent)]
	CryptoError(#[from] CryptoError),
	#[error(transparent)]
	TypeError(#[from] TypeError),
	#[error("Invalid password")]
	InvalidPassword,
	/// Error parsing data
	#[error("Parse error: {0}")]
	ParseError(String),
	/// Error locking a mutex
	#[error("Lock error")]
	LockError,
	/// Protocol not found
	#[error("Protocol not found")]
	ProtocolNotFound,
	/// Network not found
	#[error("Network not found")]
	NetworkNotFound,
}

impl PartialEq for ProviderError {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(ProviderError::SerdeJson(a), ProviderError::SerdeJson(b)) => {
				a.to_string() == b.to_string()
			},
			(ProviderError::HTTPError(a), ProviderError::HTTPError(b)) => a.status() == b.status(),
			(ProviderError::CustomError(a), ProviderError::CustomError(b)) => a == b,
			(ProviderError::UnsupportedRPC, ProviderError::UnsupportedRPC) => true,
			(ProviderError::UnsupportedNodeClient, ProviderError::UnsupportedNodeClient) => true,
			(ProviderError::SignerUnavailable, ProviderError::SignerUnavailable) => true,
			(ProviderError::IllegalState(a), ProviderError::IllegalState(b)) => a == b,
			(ProviderError::InvalidAddress, ProviderError::InvalidAddress) => true,
			(ProviderError::CryptoError(a), ProviderError::CryptoError(b)) => a == b,
			(ProviderError::TypeError(a), ProviderError::TypeError(b)) => a == b,
			(ProviderError::InvalidPassword, ProviderError::InvalidPassword) => true,
			_ => false,
		}
	}
}

// Implementing Clone manually for `ProviderError`
impl Clone for ProviderError {
	fn clone(&self) -> Self {
		match self {
			ProviderError::NnsError(message) => ProviderError::NnsError(message.clone()),
			ProviderError::NnsNotOwned(message) => ProviderError::NnsNotOwned(message.clone()),
			ProviderError::SerdeJson(error) => ProviderError::SerdeJson(serde_json::Error::io(
				std::io::Error::new(std::io::ErrorKind::Other, error.to_string()),
			)),
			ProviderError::HexError(error) => ProviderError::HexError(error.clone()),
			ProviderError::HTTPError(error) => ProviderError::HTTPError(Arc::clone(error)),
			ProviderError::JsonRpcError(error) => ProviderError::JsonRpcError(error.clone()),
			ProviderError::CustomError(message) => ProviderError::CustomError(message.clone()),
			ProviderError::UnsupportedRPC => ProviderError::UnsupportedRPC,
			ProviderError::UnsupportedNodeClient => ProviderError::UnsupportedNodeClient,
			ProviderError::SignerUnavailable => ProviderError::SignerUnavailable,
			ProviderError::IllegalState(message) => ProviderError::IllegalState(message.clone()),
			ProviderError::InvalidAddress => ProviderError::InvalidAddress,
			ProviderError::CryptoError(error) => ProviderError::CryptoError(error.clone()),
			ProviderError::TypeError(error) => ProviderError::TypeError(error.clone()),
			ProviderError::InvalidPassword => ProviderError::InvalidPassword,
			ProviderError::ParseError(message) => ProviderError::ParseError(message.clone()),
			ProviderError::LockError => ProviderError::LockError,
			ProviderError::ProtocolNotFound => ProviderError::ProtocolNotFound,
			ProviderError::NetworkNotFound => ProviderError::NetworkNotFound,
		}
	}
}
