use std::{fmt::Debug, sync::Arc};

use crate::{TypeError, crypto::CryptoError, neo_clients::JsonRpcError};
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
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
	#[error(
		"Attempted to sign a transaction with no available signer. Hint: did you mean to use a SignerMiddleware?"
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

impl ProviderError {
	/// Returns `true` when repeating the same provider request may succeed.
	pub fn is_retryable(&self) -> bool {
		match self {
			Self::HTTPError(error) => is_retryable_http_error(error),
			Self::JsonRpcError(error) => error.is_retryable(),
			_ => false,
		}
	}

	/// Returns `true` when this error represents provider throttling.
	pub fn is_rate_limited(&self) -> bool {
		match self {
			Self::HTTPError(error) => {
				error.status() == Some(reqwest::StatusCode::TOO_MANY_REQUESTS)
			},
			Self::JsonRpcError(error) => error.is_rate_limited(),
			_ => false,
		}
	}

	/// Returns a provider-suggested delay before retrying, when present.
	///
	/// Untrusted hints are clamped to 60 seconds by the built-in transports.
	pub fn retry_after(&self) -> Option<std::time::Duration> {
		match self {
			Self::JsonRpcError(error) => error.retry_after(),
			_ => None,
		}
	}

	/// Returns the HTTP response status when this error carries one.
	pub fn http_status(&self) -> Option<reqwest::StatusCode> {
		match self {
			Self::HTTPError(error) => error.status(),
			_ => None,
		}
	}

	/// Returns `true` when a Neo node does not know a polled transaction yet.
	pub fn is_unknown_transaction(&self) -> bool {
		matches!(self, Self::JsonRpcError(error) if error.is_unknown_transaction())
	}

	/// Returns `true` when a Neo node already has a submitted transaction.
	pub fn is_already_known_transaction(&self) -> bool {
		matches!(self, Self::JsonRpcError(error) if error.is_already_known_transaction())
	}

	/// Returns `true` for deterministic Neo transaction-submission rejections.
	pub fn is_transaction_rejection(&self) -> bool {
		matches!(self, Self::JsonRpcError(error) if error.is_transaction_rejection())
	}
}

pub(crate) fn is_retryable_http_error(error: &reqwest::Error) -> bool {
	if error.is_timeout() || error.is_request() || error.is_body() || error.is_decode() {
		return true;
	}

	#[cfg(not(target_arch = "wasm32"))]
	if error.is_connect() {
		return true;
	}

	error.status().is_some_and(|status| {
		status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
	})
}

impl PartialEq for ProviderError {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(ProviderError::NnsError(a), ProviderError::NnsError(b)) => a == b,
			(ProviderError::NnsNotOwned(a), ProviderError::NnsNotOwned(b)) => a == b,
			(ProviderError::SerdeJson(a), ProviderError::SerdeJson(b)) => {
				a.to_string() == b.to_string()
			},
			(ProviderError::HexError(a), ProviderError::HexError(b)) => a == b,
			(ProviderError::HTTPError(a), ProviderError::HTTPError(b)) => a.status() == b.status(),
			(ProviderError::JsonRpcError(a), ProviderError::JsonRpcError(b)) => a == b,
			(ProviderError::CustomError(a), ProviderError::CustomError(b)) => a == b,
			(ProviderError::UnsupportedRPC, ProviderError::UnsupportedRPC) => true,
			(ProviderError::UnsupportedNodeClient, ProviderError::UnsupportedNodeClient) => true,
			(ProviderError::SignerUnavailable, ProviderError::SignerUnavailable) => true,
			(ProviderError::IllegalState(a), ProviderError::IllegalState(b)) => a == b,
			(ProviderError::InvalidAddress, ProviderError::InvalidAddress) => true,
			(ProviderError::CryptoError(a), ProviderError::CryptoError(b)) => a == b,
			(ProviderError::TypeError(a), ProviderError::TypeError(b)) => a == b,
			(ProviderError::InvalidPassword, ProviderError::InvalidPassword) => true,
			(ProviderError::ParseError(a), ProviderError::ParseError(b)) => a == b,
			(ProviderError::LockError, ProviderError::LockError) => true,
			(ProviderError::ProtocolNotFound, ProviderError::ProtocolNotFound) => true,
			(ProviderError::NetworkNotFound, ProviderError::NetworkNotFound) => true,
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
				std::io::Error::other(error.to_string()),
			)),
			ProviderError::HexError(error) => ProviderError::HexError(*error),
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn deterministic_provider_errors_are_not_retryable() {
		assert!(!ProviderError::InvalidAddress.is_retryable());
		assert!(!ProviderError::InvalidPassword.is_retryable());
		assert!(!ProviderError::UnsupportedRPC.is_retryable());
	}

	#[test]
	fn provider_preserves_unknown_transaction_classification() {
		for code in [-100, -103, -105] {
			let error = ProviderError::JsonRpcError(JsonRpcError {
				code,
				message: "transaction is not indexed yet".to_string(),
				data: None,
			});

			assert!(error.is_unknown_transaction());
			assert!(!error.is_retryable());
		}
	}
}
