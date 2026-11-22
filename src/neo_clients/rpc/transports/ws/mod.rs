#![allow(missing_docs)]

use std::fmt::{self, Debug};

use async_trait::async_trait;
use futures_channel::{mpsc, oneshot};
use primitive_types::U256;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::value::{RawValue, to_raw_value};

pub use error::*;
use manager::{RequestManager, SharedChannelMap};
pub use types::ConnectionDetails;
use types::*;

#[cfg(not(target_arch = "wasm32"))]
use super::Authorization;
use crate::neo_clients::{JsonRpcProvider, ProviderError, PubsubClient, RpcClient};

mod backend;

mod manager;

mod error;
mod types;
#[derive(Clone)]
pub struct WsClient {
	// Used to send instructions to the `RequestManager`
	instructions: mpsc::UnboundedSender<Instruction>,
	// Used to receive sub notifications channels with the backend
	channel_map: SharedChannelMap,
}

impl WsClient {
	/// Establishes a new websocket connection
	pub async fn connect(conn: impl Into<ConnectionDetails>) -> Result<Self, WsClientError> {
		let (man, this) = RequestManager::connect(conn.into()).await?;
		man.spawn();
		Ok(this)
	}

	/// Establishes a new websocket connection with auto-reconnects.
	pub async fn connect_with_reconnects(
		conn: impl Into<ConnectionDetails>,
		reconnects: usize,
	) -> Result<Self, WsClientError> {
		let (man, this) = RequestManager::connect_with_reconnects(conn.into(), reconnects).await?;
		man.spawn();
		Ok(this)
	}

	#[cfg(not(target_arch = "wasm32"))]
	/// Establishes a new websocket connection. This method allows specifying a custom websocket
	/// configuration, see the [tungstenite docs](https://docs.rs/tungstenite/latest/tungstenite/protocol/struct.WebSocketConfig.html) for all avaible options.
	pub async fn connect_with_config(
		conn: impl Into<ConnectionDetails>,
		config: impl Into<WebSocketConfig>,
	) -> Result<Self, WsClientError> {
		let (man, this) = RequestManager::connect_with_config(conn.into(), config.into()).await?;
		man.spawn();
		Ok(this)
	}

	#[cfg(not(target_arch = "wasm32"))]
	/// Establishes a new websocket connection with auto-reconnects. This method allows specifying a
	/// custom websocket configuration, see the [tungstenite docs](https://docs.rs/tungstenite/latest/tungstenite/protocol/struct.WebSocketConfig.html) for all avaible options.
	pub async fn connect_with_config_and_reconnects(
		conn: impl Into<ConnectionDetails>,
		config: impl Into<WebSocketConfig>,
		reconnects: usize,
	) -> Result<Self, WsClientError> {
		let (man, this) = RequestManager::connect_with_config_and_reconnects(
			conn.into(),
			config.into(),
			reconnects,
		)
		.await?;
		man.spawn();
		Ok(this)
	}

	#[tracing::instrument(skip(self, params), err)]
	async fn make_request<R>(&self, method: &str, params: Box<RawValue>) -> Result<R, WsClientError>
	where
		R: DeserializeOwned,
	{
		let (tx, rx) = oneshot::channel();
		let instruction = Instruction::Request { method: method.to_owned(), params, sender: tx };
		self.instructions
			.unbounded_send(instruction)
			.map_err(|_| WsClientError::UnexpectedClose)?;

		let res = rx.await.map_err(|_| WsClientError::UnexpectedClose)??;
		tracing::trace!(len = res.get().len(), "Received response from request manager");
		let resp = serde_json::from_str(res.get())?;
		tracing::trace!("Deserialization success");
		Ok(resp)
	}
}

impl fmt::Debug for WsClient {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Ws").finish_non_exhaustive()
	}
}

#[cfg_attr(target_arch = "wasm32", async_trait(? Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl JsonRpcProvider for WsClient {
	type Error = WsClientError;

	async fn fetch<T, R>(&self, method: &str, params: T) -> Result<R, WsClientError>
	where
		T: Debug + Serialize + Send + Sync,
		R: DeserializeOwned,
	{
		let params = to_raw_value(&params)?;
		let res = self.make_request(method, params).await?;

		Ok(res)
	}
}

impl PubsubClient for WsClient {
	type NotificationStream = mpsc::UnboundedReceiver<Box<RawValue>>;

	fn subscribe<T: Into<U256>>(&self, id: T) -> Result<Self::NotificationStream, WsClientError> {
		// due to the behavior of the request manager, we know this map has
		// been populated by the time the `request()` call returns
		let id = id.into();
		self.channel_map
			.lock()
			.unwrap_or_else(|e| e.into_inner())
			.remove(&id)
			.ok_or(WsClientError::UnknownSubscription(id))
	}

	fn unsubscribe<T: Into<U256>>(&self, id: T) -> Result<(), WsClientError> {
		self.instructions
			.unbounded_send(Instruction::Unsubscribe { id: id.into() })
			.map_err(|_| WsClientError::UnexpectedClose)
	}
}

impl RpcClient<WsClient> {
	/// Direct connection to a websocket endpoint. Defaults to 5 reconnects.
	///
	/// # Examples
	///
	/// Connect to server via URL
	///
	/// ```no_run
	/// use neo3::neo_clients::{APITrait, RpcClient, Ws};
	///
	/// # async fn t() -> Result<(), Box<dyn std::error::Error>> {
	/// let ws = RpcClient::<Ws>::connect("ws://localhost:10334/ws").await?;
	/// let height = ws.get_block_count().await?;
	/// println!("Block count: {height}");
	/// # Ok(())
	/// # }
	/// ```
	///
	/// Connect with authentication, see also [Self::connect_with_auth]
	///
	/// ```no_run
	/// use neo3::neo_clients::{APITrait, Authorization, ConnectionDetails, RpcClient, Ws};
	///
	/// # async fn t() -> Result<(), Box<dyn std::error::Error>> {
	/// let auth = Authorization::basic("user", "pass");
	/// let opts = ConnectionDetails::new("ws://localhost:10334/ws", Some(auth));
	/// let ws = RpcClient::<Ws>::connect(opts).await?;
	/// let height = ws.get_block_count().await?;
	/// println!("Block count: {height}");
	/// # Ok(())
	/// # }
	/// ```
	pub async fn connect(url: impl Into<ConnectionDetails>) -> Result<Self, ProviderError> {
		let ws = WsClient::connect(url).await?;
		Ok(Self::new(ws))
	}

	/// Direct connection to a websocket endpoint, with a set number of
	/// reconnection attempts
	pub async fn connect_with_reconnects(
		url: impl Into<ConnectionDetails>,
		reconnects: usize,
	) -> Result<Self, ProviderError> {
		let ws = WsClient::connect_with_reconnects(url, reconnects).await?;
		Ok(Self::new(ws))
	}

	/// Connect to a WS RPC provider with authentication details
	#[cfg(not(target_arch = "wasm32"))]
	pub async fn connect_with_auth(
		url: impl AsRef<str>,
		auth: Authorization,
	) -> Result<Self, ProviderError> {
		let conn = ConnectionDetails::new(url, Some(auth));
		let ws = WsClient::connect(conn).await?;
		Ok(Self::new(ws))
	}

	/// Connect to a WS RPC provider with authentication details and a set
	/// number of reconnection attempts
	#[cfg(not(target_arch = "wasm32"))]
	pub async fn connect_with_auth_and_reconnects(
		url: impl AsRef<str>,
		auth: Authorization,
		reconnects: usize,
	) -> Result<Self, ProviderError> {
		let conn = ConnectionDetails::new(url, Some(auth));
		let ws = WsClient::connect_with_reconnects(conn, reconnects).await?;
		Ok(Self::new(ws))
	}
}
