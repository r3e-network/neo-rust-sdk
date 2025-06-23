#![feature(inherent_associated_types)]
#![allow(clippy::type_complexity)]
#![warn(missing_docs)]
#![deny(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! # Neo Clients
//!
//! Client interfaces for interacting with Neo N3 blockchain nodes.
//!
//! ## Overview
//!
//! The neo_clients module provides a comprehensive set of client interfaces for connecting to
//! and interacting with Neo N3 blockchain nodes. It includes:
//!
//! - RPC clients for making JSON-RPC calls to Neo nodes
//! - Multiple transport providers (HTTP, WebSocket, IPC)
//! - Subscription support for real-time blockchain events
//! - Mock clients for testing
//! - Extension traits for domain-specific functionality
//! - Error handling for network and protocol issues
//!
//! The module is designed to be flexible, allowing developers to choose the appropriate
//! client implementation and transport mechanism for their specific use case.
//!
//! ## Examples
//!
//! ### Connecting to a Neo N3 node using HTTP
//!
//! ```rust
//! use neo3::neo_clients::{HttpProvider, RpcClient};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create an HTTP provider connected to a Neo N3 TestNet node
//!     let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
//!     
//!     // Create an RPC client with the provider
//!     let client = RpcClient::new(provider);
//!     
//!     // Get the current block count
//!     let block_count = client.get_block_count().await?;
//!     println!("Current block count: {}", block_count);
//!     
//!     // Get information about the blockchain
//!     let version = client.get_version().await?;
//!     println!("Node version: {}", version.user_agent);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### Using WebSocket for real-time updates
//!
//! ```rust
//! # #[cfg(feature = "ws")]
//! use neo3::neo_clients::{Ws, RpcClient};
//!
//! # #[cfg(feature = "ws")]
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to a Neo N3 node using WebSocket
//!     let ws = Ws::connect("wss://testnet1.neo.org:4443/ws").await?;
//!     let client = RpcClient::new(ws);
//!     
//!     // Get basic blockchain information over WebSocket
//!     let block_count = client.get_block_count().await?;
//!     println!("Current block count: {}", block_count);
//!     
//!     let version = client.get_version().await?;
//!     println!("Node version: {}", version.user_agent);
//!     
//!     Ok(())
//! }
//! ```

use lazy_static::lazy_static;

use crate::config::NeoConstants;
pub use api_trait::*;
pub use cache::{Cache, CacheConfig, CacheStats, RpcCache};
pub use circuit_breaker::{
	CircuitBreaker, CircuitBreakerConfig, CircuitBreakerStats, CircuitState,
};
pub use connection_pool::{ConnectionPool, PoolConfig, PoolStats};
pub use errors::ProviderError;
pub use ext::*;
pub use mock_client::MockClient;
pub use production_client::{ProductionClientConfig, ProductionClientStats, ProductionRpcClient};
pub use rpc::*;
#[allow(deprecated)]
pub use test_provider::{MAINNET, TESTNET};
pub use utils::*;

mod api_trait;
mod cache;
mod circuit_breaker;
mod connection_pool;
/// Errors
mod errors;
mod ext;
mod mock_blocks;
mod mock_client;
mod production_client;
mod rpc;
mod rx;
/// Crate utilities and type aliases
mod utils;

lazy_static! {
	pub static ref HTTP_PROVIDER: RpcClient<Http> = {
		let url_str =
			std::env::var("ENDPOINT").unwrap_or_else(|_| NeoConstants::SEED_1.to_string());
		let url = url::Url::parse(&url_str).expect("Failed to parse URL");
		let http_provider = Http::new(url).expect("Failed to create HTTP provider");
		RpcClient::new(http_provider)
	};
}

#[allow(missing_docs)]
/// Pre-instantiated Infura HTTP clients which rotate through multiple API keys
/// to prevent rate limits
mod test_provider {
	use std::{convert::TryFrom, iter::Cycle, slice::Iter, sync::Mutex};

	use once_cell::sync::Lazy;

	use super::*;

	// List of infura keys to rotate through so we don't get rate limited
	const INFURA_KEYS: &[&str] = &["15e8aaed6f894d63a0f6a0206c006cdd"];

	pub static MAINNET: Lazy<TestProvider> =
		Lazy::new(|| TestProvider::new(INFURA_KEYS, "mainnet"));

	pub static TESTNET: Lazy<TestProvider> =
		Lazy::new(|| TestProvider::new(INFURA_KEYS, "testnet"));

	#[derive(Debug)]
	pub struct TestProvider {
		network: String,
		keys: Mutex<Cycle<Iter<'static, &'static str>>>,
	}

	impl TestProvider {
		pub fn new(keys: &'static [&'static str], network: impl Into<String>) -> Self {
			Self { keys: keys.iter().cycle().into(), network: network.into() }
		}

		pub fn url(&self) -> String {
			let Self { network, keys } = self;
			let key = keys.lock().unwrap().next().unwrap();
			format!("https://{network}.infura.io/v3/{key}")
		}

		pub fn provider(&self) -> RpcClient<Http> {
			let url_str = self.url();
			let url = url::Url::parse(&url_str).expect("Failed to parse URL");
			let http_provider = Http::new(url).expect("Failed to create HTTP provider");
			RpcClient::new(http_provider)
		}

		#[cfg(feature = "ws")]
		pub async fn ws(&self) -> RpcClient<crate::Ws> {
			let url = format!(
				"wss://{}.infura.neo.io/ws/v3/{}",
				self.network,
				self.keys.lock().unwrap().next().unwrap()
			);
			RpcClient::connect(url.as_str()).await.unwrap()
		}
	}
}
