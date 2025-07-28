//! A [JsonRpcProvider] implementation that serves as a wrapper around two different [JsonRpcProvider]
//! and uses a dedicated client for read and the other for write operations

use crate::neo_clients::{JsonRpcProvider, ProviderError};
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Display;
use thiserror::Error;

/// A client containing two clients.
///
/// One is used for _read_ operations
/// One is used for _write_ operations that consume gas `["neo_sendTransaction",
/// "neo_sendRawTransaction"]`
///
/// **Note**: if the method is unknown this client falls back to the _read_ client
// # Example
#[derive(Debug, Clone)]
pub struct RwClient<Read, Write> {
	/// client used to read
	r: Read,
	/// client used to write
	w: Write,
}

impl<Read, Write> RwClient<Read, Write> {
	/// Creates a new client using two different clients
	///
	/// # Example
	///
	/// ```no_run
	/// use neo3::neo_clients::{HttpProvider, RwClient};
	/// use url::Url;
	///
	/// # #[cfg(feature = "ws")]
	/// async fn t() -> Result<(), Box<dyn std::error::Error>> {
	///     let http = HttpProvider::new(Url::parse("http://localhost:8545").unwrap())?;
	///     # #[cfg(feature = "ws")]
	///     let ws = neo3::neo_clients::Ws::connect("ws://localhost:8545").await?;
	///     # #[cfg(feature = "ws")]
	///     let rw = RwClient::new(http, ws);
	///     Ok(())
	/// }
	/// ```
	pub fn new(r: Read, w: Write) -> RwClient<Read, Write> {
		Self { r, w }
	}

	/// Returns the client used for read operations
	pub fn read_client(&self) -> &Read {
		&self.r
	}

	/// Returns the client used for write operations
	pub fn write_client(&self) -> &Write {
		&self.w
	}

	/// Returns a new `RwClient` with transposed clients
	pub fn transpose(self) -> RwClient<Write, Read> {
		let RwClient { r, w } = self;
		RwClient::new(w, r)
	}

	/// Consumes the client and returns the underlying clients
	pub fn split(self) -> (Read, Write) {
		let RwClient { r, w } = self;
		(r, w)
	}
}

#[derive(Error, Debug)]
/// Error thrown when using either read or write client
pub enum RwClientError<Read, Write>
where
	Read: JsonRpcProvider,
	<Read as JsonRpcProvider>::Error: Sync + Send + 'static + Display,
	Write: JsonRpcProvider,
	<Write as JsonRpcProvider>::Error: Sync + Send + 'static + Display,
{
	/// Thrown if the _read_ request failed
	#[error("Read error: {0}")]
	Read(Read::Error),
	#[error("Write error: {0}")]
	/// Thrown if the _write_ request failed
	Write(Write::Error),
}

impl<Read, Write> From<RwClientError<Read, Write>> for ProviderError
where
	Read: JsonRpcProvider + 'static,
	<Read as JsonRpcProvider>::Error: Sync + Send + 'static + Display,
	Write: JsonRpcProvider + 'static,
	<Write as JsonRpcProvider>::Error: Sync + Send + 'static + Display,
{
	fn from(src: RwClientError<Read, Write>) -> Self {
		ProviderError::CustomError(src.to_string())
	}
}

#[cfg_attr(target_arch = "wasm32", async_trait(? Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<Read, Write> JsonRpcProvider for RwClient<Read, Write>
where
	Read: JsonRpcProvider + 'static,
	<Read as JsonRpcProvider>::Error: Sync + Send + 'static + Display,
	Write: JsonRpcProvider + 'static,
	<Write as JsonRpcProvider>::Error: Sync + Send + 'static + Display,
{
	type Error = RwClientError<Read, Write>;

	/// Sends a POST request with the provided method and the params serialized as JSON
	/// over HTTP
	async fn fetch<T, R>(&self, method: &str, params: T) -> Result<R, Self::Error>
	where
		T: std::fmt::Debug + Serialize + Send + Sync,
		R: DeserializeOwned + Send,
	{
		match method {
			"neo_sendTransaction" | "neo_sendRawTransaction" => {
				self.w.fetch(method, params).await.map_err(RwClientError::Write)
			},
			_ => self.r.fetch(method, params).await.map_err(RwClientError::Read),
		}
	}
}
