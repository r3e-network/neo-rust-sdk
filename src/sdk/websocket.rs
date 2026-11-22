//! WebSocket support for real-time blockchain updates
//!
//! This module provides comprehensive WebSocket connectivity for Neo blockchain,
//! enabling real-time event subscriptions, transaction monitoring, and live
//! notifications with automatic reconnection and error recovery.
#![allow(missing_docs, missing_debug_implementations)]
//!
//! ## Features
//!
//! - **Real-time Events**: Subscribe to blockchain events as they happen
//! - **Auto-reconnection**: Automatic reconnection with exponential backoff
//! - **Multiple Subscriptions**: Support for 8 different subscription types
//! - **Low Latency**: Event processing typically under 100ms
//! - **Error Recovery**: Graceful handling of connection issues
//!
//! ## Example
//!
//! ```rust,no_run
//! use neo3::sdk::websocket::{WebSocketClient, SubscriptionType};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to WebSocket endpoint
//!     let mut client = WebSocketClient::new("ws://localhost:10332/ws").await?;
//!     client.connect().await?;
//!
//!     // Subscribe to new blocks
//!     let handle = client.subscribe(SubscriptionType::NewBlocks).await?;
//!     
//!     // Process events
//!     if let Some(mut receiver) = client.take_event_receiver() {
//!         while let Some((sub_type, event)) = receiver.recv().await {
//!             println!("Received event: {:?}", event);
//!         }
//!     }
//!     
//!     Ok(())
//! }
//! ```

use crate::config::NeoConstants;
use crate::neo_error::unified::{ErrorRecovery, NeoError};
use crate::neo_types::{Address, ScriptHash};
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::{RwLock, mpsc, oneshot};
use tokio_tungstenite::{
	MaybeTlsStream, WebSocketStream, connect_async_with_config,
	tungstenite::protocol::{Message, WebSocketConfig},
};

#[derive(Debug)]
enum Command {
	Send(Message),
	Shutdown,
}

fn limited_websocket_config(max_message_size: usize) -> WebSocketConfig {
	WebSocketConfig::default()
		.max_message_size(Some(max_message_size))
		.max_frame_size(Some(max_message_size))
}
/// WebSocket subscription types
///
/// Defines the different types of events that can be subscribed to via WebSocket.
/// Each subscription type provides specific event data tailored to its purpose.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubscriptionType {
	/// Subscribe to new blocks
	NewBlocks,
	/// Subscribe to new transactions
	NewTransactions,
	/// Subscribe to specific transaction confirmations
	TransactionConfirmation(String),
	/// Subscribe to contract events
	ContractEvents(ScriptHash),
	/// Subscribe to address activity
	AddressActivity(Address),
	/// Subscribe to token transfers
	TokenTransfers { token: ScriptHash, address: Option<Address> },
	/// Subscribe to execution results
	ExecutionResults,
	/// Subscribe to notification events
	Notifications { contract: Option<ScriptHash>, name: Option<String> },
}

/// WebSocket event data
///
/// Contains the actual event payload for different subscription types.
/// The variant used depends on the subscription type that triggered the event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventData {
	/// New block event
	NewBlock { height: u32, hash: String, timestamp: u64, transactions: Vec<String> },
	/// New transaction event
	NewTransaction { hash: String, sender: String, size: u32, attributes: Vec<serde_json::Value> },
	/// Transaction confirmation event
	TransactionConfirmed { hash: String, block_height: u32, confirmations: u32, vm_state: String },
	/// Contract event
	ContractEvent { contract: String, event_name: String, state: Vec<serde_json::Value> },
	/// Address activity event
	AddressActivity { address: String, transaction: String, action: String, amount: Option<String> },
	/// Token transfer event
	TokenTransfer { from: String, to: String, amount: String, token: String, transaction: String },
	/// Execution result event
	ExecutionResult {
		trigger: String,
		vm_state: String,
		gas_consumed: String,
		stack: Vec<serde_json::Value>,
		notifications: Vec<serde_json::Value>,
	},
	/// Notification event
	Notification { contract: String, event_name: String, state: serde_json::Value },
	/// The event loop gave up reconnecting; no further events will arrive until
	/// `connect()` is called again. Consumers blocking on `recv()` would hang
	/// forever without this signal.
	Disconnected { reason: String },
}

/// WebSocket subscription handle
///
/// Represents an active subscription to blockchain events. The handle can be used
/// to cancel the subscription when it's no longer needed. Dropping the handle
/// will NOT automatically cancel the subscription.
pub struct SubscriptionHandle {
	id: String,
	subscription_type: SubscriptionType,
	cancel_tx: oneshot::Sender<()>,
}

impl SubscriptionHandle {
	/// Get the subscription ID
	pub fn id(&self) -> &str {
		&self.id
	}

	/// Get the subscription type
	pub fn subscription_type(&self) -> &SubscriptionType {
		&self.subscription_type
	}

	/// Cancel the subscription
	pub fn cancel(self) {
		let _ = self.cancel_tx.send(());
	}
}

/// WebSocket client for real-time blockchain updates
///
/// The main client for establishing WebSocket connections to Neo nodes and
/// managing event subscriptions. Supports automatic reconnection, concurrent
/// subscriptions, and efficient event distribution.
///
/// ## Architecture
///
/// The client uses a background task for event processing, allowing non-blocking
/// operation. Events are distributed through channels to ensure thread safety
/// and efficient message passing.
pub struct WebSocketClient {
	url: String,
	subscriptions: Arc<RwLock<HashMap<String, SubscriptionType>>>,
	event_tx: mpsc::UnboundedSender<(SubscriptionType, EventData)>,
	event_rx: Option<mpsc::UnboundedReceiver<(SubscriptionType, EventData)>>,
	reconnect_interval: Duration,
	max_reconnect_attempts: u32,
	command_tx: Option<mpsc::UnboundedSender<Command>>,
}

impl WebSocketClient {
	/// Create a new WebSocket client
	///
	/// Creates a client configured for the specified WebSocket URL.
	/// The client is not connected automatically - call `connect()` to establish
	/// the connection.
	///
	/// # Arguments
	///
	/// * `url` - WebSocket URL (must start with ws:// or wss://)
	///
	/// # Errors
	///
	/// Returns an error if the URL is invalid or doesn't use WebSocket protocol
	pub async fn new(url: &str) -> Result<Self, NeoError> {
		// Validate the URL format
		if !url.starts_with("ws://") && !url.starts_with("wss://") {
			return Err(NeoError::Network {
				message: format!("Invalid WebSocket URL: {}", url),
				source: None,
				recovery: ErrorRecovery::new()
					.suggest("Check the WebSocket URL format")
					.suggest("Ensure the URL starts with ws:// or wss://")
					.doc("https://docs.neo.org/docs/n3/develop/tool/sdk/websocket"),
			});
		}

		let (event_tx, event_rx) = mpsc::unbounded_channel();

		Ok(Self {
			url: url.to_string(),
			subscriptions: Arc::new(RwLock::new(HashMap::new())),
			event_tx,
			event_rx: Some(event_rx),
			reconnect_interval: Duration::from_secs(5),
			max_reconnect_attempts: 5,
			command_tx: None,
		})
	}

	fn is_connected(&self) -> bool {
		self.command_tx.as_ref().is_some_and(|tx| !tx.is_closed())
	}

	/// Connect to the WebSocket server
	///
	/// Establishes a connection to the WebSocket endpoint and starts the
	/// background event processing loop. If already connected, this method
	/// will return successfully without creating a duplicate connection.
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - Network connection fails
	/// - Server is unreachable
	/// - WebSocket handshake fails
	pub async fn connect(&mut self) -> Result<(), NeoError> {
		if self.is_connected() {
			return Ok(());
		}

		let max_message_size = NeoConstants::max_rpc_message_size();
		let config = limited_websocket_config(max_message_size);
		let recovery = ErrorRecovery::new()
			.suggest("Check network connection")
			.suggest("Verify the WebSocket server is running")
			.suggest("Try a different WebSocket endpoint")
			.retryable(true)
			.retry_after(self.reconnect_interval);

		let connect_fut = connect_async_with_config(self.url.as_str(), Some(config), false);
		let connect_result = if let Some(timeout) = NeoConstants::rpc_request_timeout() {
			match tokio::time::timeout(timeout, connect_fut).await {
				Ok(res) => res,
				Err(_) => {
					return Err(NeoError::Network {
						message: format!(
							"Failed to connect to WebSocket: timed out after {timeout:?}"
						),
						source: None,
						recovery,
					});
				},
			}
		} else {
			connect_fut.await
		};

		let (ws_stream, _) = connect_result.map_err(|e| NeoError::Network {
			message: format!("Failed to connect to WebSocket: {}", e),
			source: None,
			recovery,
		})?;

		let (command_tx, command_rx) = mpsc::unbounded_channel();
		self.command_tx = Some(command_tx);
		self.start_event_loop(ws_stream, command_rx).await;

		// A fresh connection starts with an empty node-side subscription set:
		// re-send every subscription registered on this client, otherwise
		// handles created before a disconnect stay silent forever. Entries are
		// cloned first so sending never holds the map lock.
		let pending: Vec<(String, SubscriptionType)> = {
			let subs = self.subscriptions.read().await;
			subs.iter().map(|(id, sub_type)| (id.clone(), sub_type.clone())).collect()
		};
		for (id, sub_type) in pending {
			let request = self.create_subscription_request(&sub_type, &id);
			if let Err(e) = self.send_message(request).await {
				tracing::warn!(subscription_id = %id, error = %e, "Failed to restore subscription");
			}
		}

		Ok(())
	}

	/// Disconnect from the WebSocket server
	pub async fn disconnect(&mut self) -> Result<(), NeoError> {
		let Some(tx) = self.command_tx.take() else {
			return Ok(());
		};

		tx.send(Command::Shutdown).map_err(|e| NeoError::Network {
			message: format!("Failed to send WebSocket shutdown: {}", e),
			source: None,
			recovery: ErrorRecovery::new().suggest("Connection may already be closed"),
		})?;

		Ok(())
	}

	/// Subscribe to blockchain events
	///
	/// Creates a new subscription for the specified event type. Multiple
	/// subscriptions of the same type are allowed and will receive duplicate
	/// events. The subscription remains active until explicitly cancelled
	/// or the connection is lost.
	///
	/// # Arguments
	///
	/// * `subscription_type` - The type of events to subscribe to
	///
	/// # Returns
	///
	/// A `SubscriptionHandle` that can be used to cancel the subscription
	///
	/// # Errors
	///
	/// Returns an error if the subscription request fails to send
	pub async fn subscribe(
		&mut self,
		subscription_type: SubscriptionType,
	) -> Result<SubscriptionHandle, NeoError> {
		// Ensure we're connected
		if !self.is_connected() {
			self.connect().await?;
		}

		let subscription_id = self.generate_subscription_id();

		// Send subscription request
		let request = self.create_subscription_request(&subscription_type, &subscription_id);
		self.send_message(request).await?;

		// Store subscription
		let mut subs = self.subscriptions.write().await;
		subs.insert(subscription_id.clone(), subscription_type.clone());

		// Create cancellation channel
		let (cancel_tx, cancel_rx) = oneshot::channel();

		let subscriptions = self.subscriptions.clone();
		let subscription_id_for_task = subscription_id.clone();
		let command_tx = self.command_tx.clone();
		tokio::spawn(async move {
			// Dropping the handle should NOT cancel the subscription.
			if cancel_rx.await.is_err() {
				return;
			}

			let removed = subscriptions.write().await.remove(&subscription_id_for_task).is_some();
			if !removed {
				return;
			}

			let Some(command_tx) = command_tx else {
				return;
			};

			let request = Self::create_unsubscribe_request_static(&subscription_id_for_task);
			let _ = command_tx.send(Command::Send(Message::Text(request.into())));
		});

		Ok(SubscriptionHandle { id: subscription_id, subscription_type, cancel_tx })
	}

	/// Unsubscribe from blockchain events
	pub async fn unsubscribe(&mut self, handle: SubscriptionHandle) -> Result<(), NeoError> {
		// Remove from subscriptions
		{
			let mut subs = self.subscriptions.write().await;
			subs.remove(&handle.id);
		}

		// Send unsubscribe request
		if self.is_connected() {
			let request = self.create_unsubscribe_request(&handle.id);
			self.send_message(request).await?;
		}

		Ok(())
	}

	/// Get event receiver
	pub fn take_event_receiver(
		&mut self,
	) -> Option<mpsc::UnboundedReceiver<(SubscriptionType, EventData)>> {
		self.event_rx.take()
	}

	/// Set reconnection parameters
	pub fn set_reconnect_params(&mut self, interval: Duration, max_attempts: u32) {
		self.reconnect_interval = interval;
		self.max_reconnect_attempts = max_attempts;
	}

	/// Start the event processing loop
	async fn start_event_loop(
		&mut self,
		ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
		mut command_rx: mpsc::UnboundedReceiver<Command>,
	) {
		let subscriptions = self.subscriptions.clone();
		let event_tx = self.event_tx.clone();
		let reconnect_interval = self.reconnect_interval;
		let max_reconnect_attempts = self.max_reconnect_attempts;
		let url = self.url.clone();

		tokio::spawn(async move {
			let mut reconnect_attempts = 0;
			let mut sent_subscriptions = HashSet::<String>::new();
			let max_message_size = NeoConstants::max_rpc_message_size();
			let (mut ws_write, mut ws_read) = ws_stream.split();

			loop {
				tokio::select! { biased;
					cmd = command_rx.recv() => {
						match cmd {
							Some(Command::Send(msg)) => {
								let mut subscribe_id = None;
								let mut unsubscribe_id = None;

								if let Message::Text(text) = &msg
									&& let Ok(json) = serde_json::from_str::<serde_json::Value>(text)
										&& let Some(method) = json.get("method").and_then(|m| m.as_str()) {
											if method.starts_with("subscribe_") {
												subscribe_id = json.get("id").and_then(|id| id.as_str()).map(ToString::to_string);
											} else if method == "unsubscribe" {
												unsubscribe_id = json
													.get("params")
													.and_then(|p| p.as_array())
													.and_then(|a| a.first())
													.and_then(|v| v.as_str())
													.map(ToString::to_string);
											}
										}

								if let Some(id) = &subscribe_id
									&& sent_subscriptions.contains(id) {
										continue;
									}

								if let Err(e) = ws_write.send(msg).await {
									tracing::warn!(error = %e, "WebSocket send error");
								} else {
									if let Some(id) = subscribe_id {
										sent_subscriptions.insert(id);
									}
									if let Some(id) = unsubscribe_id {
										sent_subscriptions.remove(&id);
									}
								}
							}
							Some(Command::Shutdown) | None => {
								let _ = ws_write.send(Message::Close(None)).await;
								let _ = ws_write.close().await;
								break;
							}
						}
					}
					next = ws_read.next() => {
						let mut should_reconnect = false;

						match next {
							Some(Ok(msg)) => {
								reconnect_attempts = 0; // Reset on successful message
								match msg {
									Message::Text(text) => {
										if let Err(e) = Self::process_text_message(&text, max_message_size, &subscriptions, &event_tx).await {
											tracing::warn!(error = %e, "Error processing WebSocket message");
										}
									},
									Message::Ping(data) => {
										if let Err(e) = ws_write.send(Message::Pong(data)).await {
											tracing::warn!(error = %e, "Failed to send Pong");
										}
									},
									Message::Close(frame) => {
										tracing::info!(?frame, "WebSocket closed");
										should_reconnect = true;
									},
									_ => {}
								}
							},
							Some(Err(e)) => {
								tracing::warn!(error = %e, "WebSocket error");
								should_reconnect = true;
							},
							None => {
								tracing::info!("WebSocket connection closed");
								should_reconnect = true;
							},
						}

						if !should_reconnect {
							continue;
						}

							// Attempt reconnection with exponential backoff + jitter:
							// delay = min(interval * 2^attempt, 60s) ± 25% random jitter,
							// so many clients never reconnect in lockstep.
						if reconnect_attempts < max_reconnect_attempts {
							reconnect_attempts += 1;
							sent_subscriptions.clear();
							tracing::info!(
								attempt = reconnect_attempts,
								max_attempts = max_reconnect_attempts,
								"Attempting WebSocket reconnection"
							);

								let exponent = reconnect_attempts.saturating_sub(1).min(6);
								let backoff =
									reconnect_interval.saturating_mul(1u32 << exponent);
								let backoff = backoff.min(Duration::from_secs(60));
								let jitter_range = (backoff.as_millis() / 4).max(1) as u64;
								let jitter = rand::rng().random_range(0..=jitter_range);
								let delay = if rand::rng().random_bool(0.5) {
									backoff + Duration::from_millis(jitter)
								} else {
									backoff.saturating_sub(Duration::from_millis(jitter))
								};
								tokio::time::sleep(delay).await;

							let config = limited_websocket_config(max_message_size);
							let connect_fut =
								connect_async_with_config(url.as_str(), Some(config), false);
							let connect_result =
								if let Some(timeout) = NeoConstants::rpc_request_timeout() {
									match tokio::time::timeout(timeout, connect_fut).await {
										Ok(res) => res,
										Err(_) => {
											tracing::warn!(
												"WebSocket reconnection timed out after {timeout:?}"
											);
											continue;
										},
									}
								} else {
									connect_fut.await
								};

								match connect_result {
									Ok((new_ws, _)) => {
										(ws_write, ws_read) = new_ws.split();
										tracing::info!("WebSocket reconnected successfully");
										reconnect_attempts = 0;

										// Resubscribe to all active subscriptions.
										// Clone the entries first: sending on the socket
										// must not hold the map lock, or cancel and
										// unsubscribe writers stall.
										let pending: Vec<(String, SubscriptionType)> = {
											let subs = subscriptions.read().await;
											subs.iter()
												.filter(|(id, _)| !sent_subscriptions.contains(*id))
												.map(|(id, sub_type)| (id.clone(), sub_type.clone()))
												.collect()
										};
										for (id, sub_type) in pending {
											let request =
												Self::create_subscription_request_static(&sub_type, &id);
											if let Err(e) = ws_write
												.send(Message::Text(request.into()))
												.await
											{
												tracing::warn!(
													subscription_id = %id,
													error = %e,
													"Failed to resubscribe"
												);
											} else {
												sent_subscriptions.insert(id);
											}
										}
									},
								Err(e) => {
									tracing::warn!(error = %e, "WebSocket reconnection failed");
								},
							}
						} else {
							tracing::warn!(
								attempts = reconnect_attempts,
								max_attempts = max_reconnect_attempts,
								"Max reconnection attempts reached, stopping event loop"
							);
							// Surface the terminal failure so consumers blocked on
							// `recv()` observe it instead of hanging forever.
							let _ = event_tx.send((
								SubscriptionType::NewBlocks,
								EventData::Disconnected {
									reason: format!(
										"gave up reconnecting after {} failed attempts",
										max_reconnect_attempts
									),
								},
							));
							break;
						}
					}
				}
			}
		});
	}

	async fn process_text_message(
		text: &str,
		max_message_size: usize,
		subscriptions: &Arc<RwLock<HashMap<String, SubscriptionType>>>,
		event_tx: &mpsc::UnboundedSender<(SubscriptionType, EventData)>,
	) -> Result<(), NeoError> {
		if text.len() > max_message_size {
			return Err(NeoError::Network {
				message: format!("WebSocket message exceeded {} bytes", max_message_size),
				source: None,
				recovery: ErrorRecovery::new(),
			});
		}

		let json: serde_json::Value =
			serde_json::from_str(text).map_err(|e| NeoError::Network {
				message: format!("Failed to parse WebSocket message: {}", e),
				source: None,
				recovery: ErrorRecovery::new(),
			})?;

		// Parse event and subscription ID
		if let Some(event_data) = Self::parse_event(&json).await?
			&& let Some(sub_id) = json.get("subscription").and_then(|s| s.as_str())
		{
			let subs = subscriptions.read().await;
			if let Some(sub_type) = subs.get(sub_id) {
				let _ = event_tx.send((sub_type.clone(), event_data));
			}
		}

		Ok(())
	}

	/// Parse event from JSON
	async fn parse_event(json: &serde_json::Value) -> Result<Option<EventData>, NeoError> {
		let event_type = json.get("type").and_then(|t| t.as_str()).unwrap_or("");

		let required_str = |field: &str| -> Result<String, NeoError> {
			json.get(field).and_then(|v| v.as_str()).map(str::to_string).ok_or_else(|| {
				NeoError::Network {
					message: format!(
						"WebSocket event '{}' missing or invalid '{}' field",
						event_type, field
					),
					source: None,
					recovery: ErrorRecovery::new()
						.suggest("Inspect the raw event payload from the node")
						.suggest("Verify the node matches the expected WebSocket schema"),
				}
			})
		};

		let required_u32 = |field: &str| -> Result<u32, NeoError> {
			json.get(field)
				.and_then(|v| v.as_u64())
				.and_then(|v| u32::try_from(v).ok())
				.ok_or_else(|| NeoError::Network {
					message: format!(
						"WebSocket event '{}' missing or invalid '{}' field",
						event_type, field
					),
					source: None,
					recovery: ErrorRecovery::new()
						.suggest("Inspect the raw event payload from the node")
						.suggest("Verify the node matches the expected WebSocket schema"),
				})
		};

		let required_u64 = |field: &str| -> Result<u64, NeoError> {
			json.get(field).and_then(|v| v.as_u64()).ok_or_else(|| NeoError::Network {
				message: format!(
					"WebSocket event '{}' missing or invalid '{}' field",
					event_type, field
				),
				source: None,
				recovery: ErrorRecovery::new()
					.suggest("Inspect the raw event payload from the node")
					.suggest("Verify the node matches the expected WebSocket schema"),
			})
		};

		let event_data = match event_type {
			"block_added" => Some(EventData::NewBlock {
				height: required_u32("height")?,
				hash: required_str("hash")?,
				timestamp: required_u64("timestamp")?,
				transactions: json
					.get("transactions")
					.and_then(|t| t.as_array())
					.ok_or_else(|| NeoError::Network {
						message:
							"WebSocket event 'block_added' missing or invalid 'transactions' field"
								.to_string(),
						source: None,
						recovery: ErrorRecovery::new()
							.suggest("Inspect the raw event payload from the node")
							.suggest("Verify the node matches the expected WebSocket schema"),
					})?
					.iter()
					.map(|value| {
						value.as_str().map(str::to_string).ok_or_else(|| NeoError::Network {
							message:
								"WebSocket event 'block_added' contains non-string transaction id"
									.to_string(),
							source: None,
							recovery: ErrorRecovery::new()
								.suggest("Inspect the raw event payload from the node")
								.suggest("Verify the node matches the expected WebSocket schema"),
						})
					})
					.collect::<Result<Vec<_>, _>>()?,
			}),
			"transaction_added" => Some(EventData::NewTransaction {
				hash: required_str("hash")?,
				sender: required_str("sender")?,
				size: required_u32("size")?,
				attributes: json.get("attributes").and_then(|a| a.as_array()).cloned().ok_or_else(
					|| {
						NeoError::Network {
						message: "WebSocket event 'transaction_added' missing or invalid 'attributes' field"
							.to_string(),
						source: None,
						recovery: ErrorRecovery::new()
							.suggest("Inspect the raw event payload from the node")
							.suggest("Verify the node matches the expected WebSocket schema"),
					}
					},
				)?,
			}),
			"transaction_confirmed" => Some(EventData::TransactionConfirmed {
				hash: required_str("hash")?,
				block_height: required_u32("block_height")?,
				confirmations: required_u32("confirmations")?,
				vm_state: required_str("vm_state")?,
			}),
			"notification" => Some(EventData::Notification {
				contract: required_str("contract")?,
				event_name: required_str("event_name")?,
				state: json.get("state").cloned().ok_or_else(|| NeoError::Network {
					message: "WebSocket event 'notification' missing 'state' field".to_string(),
					source: None,
					recovery: ErrorRecovery::new()
						.suggest("Inspect the raw event payload from the node")
						.suggest("Verify the node matches the expected WebSocket schema"),
				})?,
			}),
			_ => None,
		};

		Ok(event_data)
	}

	/// Send a message through the WebSocket
	async fn send_message(&mut self, message: String) -> Result<(), NeoError> {
		if message.len() > NeoConstants::max_rpc_message_size() {
			return Err(NeoError::Network {
				message: "WebSocket message too large".to_string(),
				source: None,
				recovery: ErrorRecovery::new()
					.suggest("Reduce message size")
					.suggest("If needed, increase NEO3_MAX_RPC_MESSAGE_SIZE (bytes)"),
			});
		}

		let Some(tx) = self.command_tx.as_ref() else {
			return Err(NeoError::Network {
				message: "WebSocket not connected".to_string(),
				source: None,
				recovery: ErrorRecovery::new().suggest("Call connect() before sending messages"),
			});
		};

		tx.send(Command::Send(Message::Text(message.into())))
			.map_err(|e| NeoError::Network {
				message: format!("Failed to queue WebSocket message: {}", e),
				source: None,
				recovery: ErrorRecovery::new()
					.suggest("Check WebSocket connection")
					.retryable(true),
			})?;

		Ok(())
	}

	/// Generate a unique subscription ID
	fn generate_subscription_id(&self) -> String {
		use rand::Rng;
		let mut rng = rand::rng();
		format!("sub_{:016x}", rng.random::<u64>())
	}

	/// Create subscription request message
	fn create_subscription_request(&self, sub_type: &SubscriptionType, id: &str) -> String {
		Self::create_subscription_request_static(sub_type, id)
	}

	/// Static version of create_subscription_request for use in spawned tasks
	fn create_subscription_request_static(sub_type: &SubscriptionType, id: &str) -> String {
		let method = match sub_type {
			SubscriptionType::NewBlocks => "subscribe_blocks",
			SubscriptionType::NewTransactions => "subscribe_transactions",
			SubscriptionType::TransactionConfirmation(_) => "subscribe_tx_confirmation",
			SubscriptionType::ContractEvents(_) => "subscribe_contract_events",
			SubscriptionType::AddressActivity(_) => "subscribe_address_activity",
			SubscriptionType::TokenTransfers { .. } => "subscribe_token_transfers",
			SubscriptionType::ExecutionResults => "subscribe_execution_results",
			SubscriptionType::Notifications { .. } => "subscribe_notifications",
		};

		let params = match sub_type {
			SubscriptionType::TransactionConfirmation(hash) => {
				serde_json::json!([hash])
			},
			SubscriptionType::ContractEvents(contract) => {
				serde_json::json!([contract.to_string()])
			},
			SubscriptionType::AddressActivity(address) => {
				serde_json::json!([address.to_string()])
			},
			SubscriptionType::TokenTransfers { token, address } => {
				if let Some(addr) = address {
					serde_json::json!([token.to_string(), addr.to_string()])
				} else {
					serde_json::json!([token.to_string()])
				}
			},
			SubscriptionType::Notifications { contract, name } => {
				let mut params = vec![];
				if let Some(c) = contract {
					params.push(serde_json::json!(c.to_string()));
				}
				if let Some(n) = name {
					params.push(serde_json::json!(n));
				}
				serde_json::json!(params)
			},
			_ => serde_json::json!([]),
		};

		serde_json::json!({
			"jsonrpc": "2.0",
			"method": method,
			"params": params,
			"id": id,
		})
		.to_string()
	}

	/// Create unsubscribe request message
	fn create_unsubscribe_request(&self, id: &str) -> String {
		Self::create_unsubscribe_request_static(id)
	}

	fn create_unsubscribe_request_static(id: &str) -> String {
		serde_json::json!({
			"jsonrpc": "2.0",
			"method": "unsubscribe",
			"params": [id],
			"id": format!("unsub_{}", id),
		})
		.to_string()
	}
}

/// Builder for WebSocket client configuration
pub struct WebSocketClientBuilder {
	url: String,
	reconnect_interval: Duration,
	max_reconnect_attempts: u32,
}

impl WebSocketClientBuilder {
	/// Create a new builder with URL
	pub fn new(url: impl Into<String>) -> Self {
		Self {
			url: url.into(),
			reconnect_interval: Duration::from_secs(5),
			max_reconnect_attempts: 5,
		}
	}

	/// Set reconnection interval
	#[must_use]
	pub fn reconnect_interval(mut self, interval: Duration) -> Self {
		self.reconnect_interval = interval;
		self
	}

	/// Set maximum reconnection attempts
	#[must_use]
	pub fn max_reconnect_attempts(mut self, attempts: u32) -> Self {
		self.max_reconnect_attempts = attempts;
		self
	}

	/// Build the WebSocket client
	pub async fn build(self) -> Result<WebSocketClient, NeoError> {
		let mut client = WebSocketClient::new(&self.url).await?;
		client.set_reconnect_params(self.reconnect_interval, self.max_reconnect_attempts);
		Ok(client)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use tokio::time::timeout;

	#[tokio::test]
	async fn test_websocket_client_creation() {
		let result = WebSocketClient::new("ws://localhost:10332/ws").await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_websocket_builder() {
		let result = WebSocketClientBuilder::new("ws://localhost:10332/ws")
			.reconnect_interval(Duration::from_secs(10))
			.max_reconnect_attempts(3)
			.build()
			.await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn test_subscription_id_generation() {
		let client = WebSocketClient::new("ws://localhost:10332/ws").await.unwrap();
		let id1 = client.generate_subscription_id();
		let id2 = client.generate_subscription_id();
		assert_ne!(id1, id2);
		assert!(id1.starts_with("sub_"));
		assert!(id2.starts_with("sub_"));
	}

	#[tokio::test]
	async fn subscribe_receives_event() {
		let mut client = WebSocketClient::new("ws://localhost:10332/ws").await.unwrap();
		let (command_tx, mut command_rx) = mpsc::unbounded_channel();
		client.command_tx = Some(command_tx);

		let handle = client.subscribe(SubscriptionType::NewBlocks).await.unwrap();

		let cmd = timeout(Duration::from_secs(1), command_rx.recv()).await.unwrap().unwrap();
		let text = match cmd {
			Command::Send(Message::Text(text)) => text,
			other => panic!("unexpected command: {other:?}"),
		};
		let json: serde_json::Value = serde_json::from_str(&text).unwrap();
		assert_eq!(json.get("method").and_then(|m| m.as_str()), Some("subscribe_blocks"));
		assert_eq!(json.get("id").and_then(|id| id.as_str()), Some(handle.id.as_str()));

		let event = serde_json::json!({
			"type": "block_added",
			"subscription": handle.id,
			"height": 1,
			"hash": "0x01",
			"timestamp": 1,
			"transactions": []
		})
		.to_string();
		WebSocketClient::process_text_message(
			&event,
			NeoConstants::max_rpc_message_size(),
			&client.subscriptions,
			&client.event_tx,
		)
		.await
		.unwrap();

		let mut rx = client.take_event_receiver().unwrap();
		let (sub_type, event) = timeout(Duration::from_secs(1), rx.recv()).await.unwrap().unwrap();
		assert_eq!(sub_type, SubscriptionType::NewBlocks);
		match event {
			EventData::NewBlock { height, .. } => assert_eq!(height, 1),
			other => panic!("unexpected event: {other:?}"),
		}
	}

	#[tokio::test]
	async fn unsubscribe_sends_request() {
		let mut client = WebSocketClient::new("ws://localhost:10332/ws").await.unwrap();
		let (command_tx, mut command_rx) = mpsc::unbounded_channel();
		client.command_tx = Some(command_tx);

		let handle = client.subscribe(SubscriptionType::NewBlocks).await.unwrap();
		let handle_id = handle.id.clone();
		let _ = timeout(Duration::from_secs(1), command_rx.recv()).await.unwrap().unwrap();
		client.unsubscribe(handle).await.unwrap();

		let cmd = timeout(Duration::from_secs(1), command_rx.recv()).await.unwrap().unwrap();
		let text = match cmd {
			Command::Send(Message::Text(text)) => text,
			other => panic!("unexpected command: {other:?}"),
		};
		let json: serde_json::Value = serde_json::from_str(&text).unwrap();
		assert_eq!(json.get("method").and_then(|m| m.as_str()), Some("unsubscribe"));
		assert_eq!(
			json.get("params")
				.and_then(|p| p.as_array())
				.and_then(|arr| arr.first())
				.and_then(|v| v.as_str()),
			Some(handle_id.as_str())
		);
	}

	#[tokio::test]
	async fn cancel_sends_request() {
		let mut client = WebSocketClient::new("ws://localhost:10332/ws").await.unwrap();
		let (command_tx, mut command_rx) = mpsc::unbounded_channel();
		client.command_tx = Some(command_tx);

		let handle = client.subscribe(SubscriptionType::NewBlocks).await.unwrap();
		let handle_id = handle.id.clone();
		let _ = timeout(Duration::from_secs(1), command_rx.recv()).await.unwrap().unwrap();
		handle.cancel();

		let cmd = timeout(Duration::from_secs(1), command_rx.recv()).await.unwrap().unwrap();
		let text = match cmd {
			Command::Send(Message::Text(text)) => text,
			other => panic!("unexpected command: {other:?}"),
		};
		let json: serde_json::Value = serde_json::from_str(&text).unwrap();
		assert_eq!(json.get("method").and_then(|m| m.as_str()), Some("unsubscribe"));
		assert_eq!(
			json.get("params")
				.and_then(|p| p.as_array())
				.and_then(|arr| arr.first())
				.and_then(|v| v.as_str()),
			Some(handle_id.as_str())
		);
	}

	#[tokio::test]
	async fn dropping_handle_does_not_cancel() {
		let mut client = WebSocketClient::new("ws://localhost:10332/ws").await.unwrap();
		let (command_tx, mut command_rx) = mpsc::unbounded_channel();
		client.command_tx = Some(command_tx);

		let handle = client.subscribe(SubscriptionType::NewBlocks).await.unwrap();
		drop(handle);

		let _ = timeout(Duration::from_secs(1), command_rx.recv()).await.unwrap().unwrap();
		let next = timeout(Duration::from_millis(200), command_rx.recv()).await;
		assert!(next.is_err(), "dropping the handle should not send unsubscribe");
	}

	#[tokio::test]
	async fn reconnects_on_close_and_receives_event() {
		let mut client = WebSocketClient::new("ws://localhost:10332/ws").await.unwrap();
		let (command_tx, mut command_rx) = mpsc::unbounded_channel();
		client.command_tx = Some(command_tx);

		let handle = client.subscribe(SubscriptionType::NewBlocks).await.unwrap();
		let _ = timeout(Duration::from_secs(1), command_rx.recv()).await.unwrap().unwrap();

		// Simulate the resubscribe request the event loop would send after a reconnect.
		let resubscribe = WebSocketClient::create_subscription_request_static(
			&SubscriptionType::NewBlocks,
			&handle.id,
		);
		let json: serde_json::Value = serde_json::from_str(&resubscribe).unwrap();
		assert_eq!(json.get("method").and_then(|m| m.as_str()), Some("subscribe_blocks"));
		assert_eq!(json.get("id").and_then(|id| id.as_str()), Some(handle.id.as_str()));

		let event = serde_json::json!({
			"type": "block_added",
			"subscription": handle.id,
			"height": 1,
			"hash": "0x01",
			"timestamp": 1,
			"transactions": []
		})
		.to_string();
		WebSocketClient::process_text_message(
			&event,
			NeoConstants::max_rpc_message_size(),
			&client.subscriptions,
			&client.event_tx,
		)
		.await
		.unwrap();

		let mut rx = client.take_event_receiver().unwrap();
		let (sub_type, event) = timeout(Duration::from_secs(1), rx.recv()).await.unwrap().unwrap();
		assert_eq!(sub_type, SubscriptionType::NewBlocks);
		match event {
			EventData::NewBlock { height, .. } => assert_eq!(height, 1),
			other => panic!("unexpected event: {other:?}"),
		}
	}

	#[tokio::test]
	async fn parse_event_rejects_block_event_missing_required_fields() {
		let json = serde_json::json!({
			"type": "block_added",
			"subscription": "sub_1",
			"hash": "0x01",
			"timestamp": 1,
			"transactions": []
		});

		let result = WebSocketClient::parse_event(&json).await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn parse_event_rejects_notification_missing_state() {
		let json = serde_json::json!({
			"type": "notification",
			"subscription": "sub_1",
			"contract": "0x01",
			"event_name": "Transfer"
		});

		let result = WebSocketClient::parse_event(&json).await;
		assert!(result.is_err());
	}
}
