use futures_channel::{mpsc, oneshot};
use futures_util::{FutureExt, select, sink::SinkExt, stream::StreamExt};
use serde_json::value::RawValue;
use tracing::{error, trace};

use crate::config::NeoConstants;

use super::{WsClientError, types::*};

fn truncate_for_log(s: &str, max_bytes: usize) -> &str {
	if s.len() <= max_bytes {
		return s;
	}
	let mut end = max_bytes;
	while end > 0 && !s.is_char_boundary(end) {
		end -= 1;
	}
	&s[..end]
}

/// `BackendDriver` drives a specific `WsBackend`. It can be used to issue
/// requests, receive responses, see errors, and shut down the backend.
pub(super) struct BackendDriver {
	// Pubsub items from the backend, received via WS
	pub(super) to_handle: mpsc::UnboundedReceiver<PubSubItem>,
	// Notification from the backend of a terminal error
	pub(super) error: oneshot::Receiver<()>,

	// Requests that the backend should dispatch
	pub(super) dispatcher: mpsc::UnboundedSender<Box<RawValue>>,
	// Notify the backend of intentional shutdown
	shutdown: oneshot::Sender<()>,
}

impl BackendDriver {
	pub(super) fn shutdown(self) {
		// don't care if it fails, as that means the backend is gone anyway
		let _ = self.shutdown.send(());
	}

	#[cfg(all(test, not(target_arch = "wasm32")))]
	pub(super) fn new_for_test() -> Self {
		let (_to_handle_tx, to_handle) = mpsc::unbounded();
		let (_error_tx, error) = oneshot::channel();
		let (dispatcher, _dispatcher_rx) = mpsc::unbounded();
		let (shutdown, _shutdown_rx) = oneshot::channel();

		Self { to_handle, error, dispatcher, shutdown }
	}
}

/// `WsBackend` dispatches requests and routes responses and notifications. It
/// also has a simple ping-based keepalive (when not compiled to wasm), to
/// prevent inactivity from triggering server-side closes
///
/// The `WsBackend` shuts down when instructed to by the `RequestManager` or
/// when the `RequestManager` drops (because the inbound channel will close)
pub(super) struct WsBackend {
	server: InternalStream,

	// channel to the manager, through which to send items received via WS
	handler: mpsc::UnboundedSender<PubSubItem>,
	// notify manager of an error causing this task to halt
	error: oneshot::Sender<()>,

	// channel of inbound requests to dispatch
	to_dispatch: mpsc::UnboundedReceiver<Box<RawValue>>,
	// notification from manager of intentional shutdown
	shutdown: oneshot::Receiver<()>,
}

impl WsBackend {
	#[cfg(target_arch = "wasm32")]
	pub(super) async fn connect(
		details: ConnectionDetails,
	) -> Result<(Self, BackendDriver), WsClientError> {
		let wsio = WsMeta::connect(details.url, None).await.map_err(WsClientError::from)?.1.fuse();

		Ok(Self::new(wsio))
	}

	#[cfg(not(target_arch = "wasm32"))]
	pub(super) async fn connect(
		details: ConnectionDetails,
	) -> Result<(Self, BackendDriver), WsClientError> {
		let max_message_size = NeoConstants::max_rpc_message_size();
		let config = WebSocketConfig::default()
			.max_message_size(Some(max_message_size))
			.max_frame_size(Some(max_message_size));

		let connect_fut = connect_async_with_config(details, Some(config), false);
		let ws = if let Some(timeout) = NeoConstants::rpc_request_timeout() {
			match tokio::time::timeout(timeout, connect_fut).await {
				Ok(res) => res?,
				Err(_) => {
					let err = std::io::Error::new(
						std::io::ErrorKind::TimedOut,
						format!("WebSocket connect timed out after {timeout:?}"),
					);
					return Err(WsError::Io(err).into());
				},
			}
		} else {
			connect_fut.await?
		}
		.0
		.fuse();
		Ok(Self::new(ws))
	}

	#[cfg(not(target_arch = "wasm32"))]
	pub(super) async fn connect_with_config(
		details: ConnectionDetails,
		config: WebSocketConfig,
		disable_nagle: bool,
	) -> Result<(Self, BackendDriver), WsClientError> {
		let connect_fut = connect_async_with_config(details, Some(config), disable_nagle);
		let ws = if let Some(timeout) = NeoConstants::rpc_request_timeout() {
			match tokio::time::timeout(timeout, connect_fut).await {
				Ok(res) => res?,
				Err(_) => {
					let err = std::io::Error::new(
						std::io::ErrorKind::TimedOut,
						format!("WebSocket connect timed out after {timeout:?}"),
					);
					return Err(WsError::Io(err).into());
				},
			}
		} else {
			connect_fut.await?
		}
		.0
		.fuse();
		Ok(Self::new(ws))
	}

	fn new(server: InternalStream) -> (Self, BackendDriver) {
		let (handler, to_handle) = mpsc::unbounded();
		let (dispatcher, to_dispatch) = mpsc::unbounded();
		let (error_tx, error_rx) = oneshot::channel();
		let (shutdown_tx, shutdown_rx) = oneshot::channel();

		(
			WsBackend { server, handler, error: error_tx, to_dispatch, shutdown: shutdown_rx },
			BackendDriver { to_handle, error: error_rx, dispatcher, shutdown: shutdown_tx },
		)
	}

	async fn handle_text(&mut self, t: String) -> Result<(), WsClientError> {
		let max_message_size = NeoConstants::max_rpc_message_size();
		if t.len() > max_message_size {
			return Err(WsClientError::JsonError(serde::de::Error::custom(format!(
				"WebSocket message exceeded {} bytes",
				max_message_size
			))));
		}

		trace!(len = t.len(), preview = truncate_for_log(&t, 512), "Received message");
		match serde_json::from_str(&t) {
			Ok(item) => {
				trace!(%item, "Deserialized message");
				let res = self.handler.unbounded_send(item);
				if res.is_err() {
					return Err(WsClientError::DeadChannel);
				}
			},
			Err(e) => {
				error!(e = %e, "Failed to deserialize message");
				return Err(WsClientError::JsonError(e));
			},
		}
		Ok(())
	}

	#[cfg(not(target_arch = "wasm32"))]
	async fn handle(&mut self, item: WsStreamItem) -> Result<(), WsClientError> {
		match item {
			Ok(item) => match item {
				Message::Text(t) => self.handle_text(t.to_string()).await,
				// https://github.com/snapview/tungstenite-rs/blob/42b8797e8b7f39efb7d9322dc8af3e9089db4f7d/src/protocol/mod.rs#L172-L175
				Message::Ping(_) => Ok(()),
				Message::Pong(_) => Ok(()),
				Message::Frame(_) => Ok(()),

				Message::Binary(buf) => Err(WsClientError::UnexpectedBinary(buf.to_vec())),
				Message::Close(frame) => {
					if let Some(frame) = frame {
						error!("Close frame: {}", frame);
					}
					Err(WsClientError::UnexpectedClose)
				},
			},
			Err(e) => {
				error!(err = %e, "Error response from WS");
				Err(e.into())
			},
		}
	}

	#[cfg(target_arch = "wasm32")]
	async fn handle(&mut self, item: WsStreamItem) -> Result<(), WsClientError> {
		match item {
			Message::Text(inner) => self.handle_text(inner).await,
			Message::Binary(buf) => Err(WsClientError::UnexpectedBinary(buf)),
		}
	}

	pub(super) fn spawn(mut self) {
		let fut = async move {
			let mut err = false;
			loop {
				#[cfg(not(target_arch = "wasm32"))]
				let keepalive = tokio::time::sleep(std::time::Duration::from_secs(10)).fuse();
				#[cfg(not(target_arch = "wasm32"))]
				tokio::pin!(keepalive);

				// in wasm, we don't ping. as ping doesn't exist in our wasm lib
				#[cfg(target_arch = "wasm32")]
				let mut keepalive = futures_util::future::pending::<()>().fuse();

				select! {
					_ = keepalive => {
						#[cfg(not(target_arch = "wasm32"))]
						if let Err(e) = self.server.send(Message::Ping(Vec::new().into())).await {
							error!(err = %e, "WS connection error");
							err = true;
							break
						}
						#[cfg(target_arch = "wasm32")]
						{
							// Keepalive is a pending future on wasm builds.
						}
					}
					resp = self.server.next() => {
						match resp {
							Some(item) => {
								err = self.handle(item).await.is_err();
								if err { break }
							},
							None => {
								error!("WS server has gone away");
								err = true;
								break
							},
						}
					}
					// we've received a new dispatch, so we send it via
					// websocket
					inst = self.to_dispatch.next() => {
						match inst {
							Some(msg) => {
								if let Err(e) =
									self.server.send(Message::Text(msg.to_string().into())).await
								{
									error!(err = %e, "WS connection error");
									err = true;
									break
								}
							},
							// dispatcher has gone away
							None => {
								break
							},
						}
					},
					// break on shutdown recv, or on shutdown recv error
					_ = &mut self.shutdown => {
						break
					},
				}
			}
			if err {
				let _ = self.error.send(());
			}
		};

		#[cfg(target_arch = "wasm32")]
		super::spawn_local(fut);

		#[cfg(not(target_arch = "wasm32"))]
		tokio::spawn(fut);
	}
}
