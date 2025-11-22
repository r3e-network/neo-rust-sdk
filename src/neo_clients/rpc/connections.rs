use std::fmt::Debug;

use crate::neo_clients::ProviderError;
use async_trait::async_trait;
use auto_impl::auto_impl;
use serde::{de::DeserializeOwned, Serialize};

#[cfg_attr(target_arch = "wasm32", async_trait(? Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[auto_impl(&, Box, Arc)]
/// Trait which must be implemented by data transports to be used with the Neo
/// JSON-RPC provider.
pub trait JsonRpcProvider: Debug + Send + Sync {
	/// A JSON-RPC Error
	type Error: Into<ProviderError>;

	/// Sends a request with the provided JSON-RPC and parameters serialized as JSON
	async fn fetch<T, R>(&self, method: &str, params: T) -> Result<R, Self::Error>
	where
		T: Debug + Serialize + Send + Sync,
		R: DeserializeOwned + Send;
}

/// Backwards-compatible alias for `JsonRpcProvider`.
pub use JsonRpcProvider as JsonRpcClient;
