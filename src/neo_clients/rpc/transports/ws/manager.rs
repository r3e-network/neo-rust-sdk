use std::{
	collections::{BTreeMap, HashMap},
	sync::{
		Arc, Mutex,
		atomic::{AtomicU64, Ordering},
	},
};

use futures_channel::{mpsc, oneshot};
use futures_util::{FutureExt, StreamExt, select_biased};
use primitive_types::U256;
use serde_json::value::{RawValue, to_raw_value};

#[cfg(not(target_arch = "wasm32"))]
use futures_util::future::Either;

use super::super::JsonRpcError;

#[cfg(not(target_arch = "wasm32"))]
use super::WebSocketConfig;
use super::{
	ActiveSub, ConnectionDetails, InFlight, Instruction, Notification, PubSubItem, Response, SubId,
	WsClient, WsClientError,
	backend::{BackendDriver, WsBackend},
};
#[cfg(not(target_arch = "wasm32"))]
use crate::config::NeoConstants;

pub(super) type SharedChannelMap =
	Arc<Mutex<HashMap<U256, mpsc::UnboundedReceiver<Box<RawValue>>>>>;

pub(super) const DEFAULT_RECONNECTS: usize = 5;

/// This struct manages the relationship between the u64 request ID, and U256
/// server-side subscription ID. It does this by aliasing the server ID to the
/// request ID, and returning the Request ID to the caller (hiding the server
/// ID in the SubscriptionManager internals.) Giving the caller a "fake"
/// subscription id allows the subscription to behave consistently across
/// reconnections
struct SubscriptionManager {
	// Active subs indexed by request id
	subs: BTreeMap<u64, ActiveSub>,
	// Maps active server-side IDs to local subscription IDs
	aliases: HashMap<U256, u64>,
	// Used to share notification channels with the WsClient(s)
	channel_map: SharedChannelMap,
}

impl SubscriptionManager {
	fn new(channel_map: SharedChannelMap) -> Self {
		Self { subs: Default::default(), aliases: Default::default(), channel_map }
	}

	fn reset_server_ids(&mut self) {
		self.aliases.clear();
		for sub in self.subs.values_mut() {
			sub.current_server_id = None;
		}
	}

	fn count(&self) -> usize {
		self.subs.len()
	}

	fn add_alias(&mut self, sub: U256, id: u64) {
		if let Some(entry) = self.subs.get_mut(&id) {
			entry.current_server_id = Some(sub);
		}
		self.aliases.insert(sub, id);
	}

	fn remove_alias(&mut self, server_id: U256) {
		if let Some(id) = self.aliases.get(&server_id)
			&& let Some(sub) = self.subs.get_mut(id)
		{
			sub.current_server_id = None;
		}
		self.aliases.remove(&server_id);
	}

	#[tracing::instrument(skip(self))]
	fn end_subscription(&mut self, id: u64) -> Option<Box<RawValue>> {
		// Ensure any channel created during subscription establishment is cleaned up, even if
		// the request fails before the user calls `subscribe`.
		self.channel_map
			.lock()
			.unwrap_or_else(|e| e.into_inner())
			.remove(&U256::from(id));

		if let Some(sub) = self.subs.remove(&id) {
			if let Some(server_id) = sub.current_server_id {
				tracing::debug!(server_id = format!("0x{server_id:x}"), "Ending subscription");
				self.remove_alias(server_id);
				// drop the receiver as we don't need the result
				let (channel, _) = oneshot::channel();
				// Serialization errors are ignored, and result in the request
				// not being dispatched. This is fine, as worst case it will
				// result in the server sending us notifications we ignore
				let unsub_request = InFlight {
					method: "neo_unsubscribe".to_string(),
					params: SubId(server_id).serialize_raw().ok()?,
					channel,
					#[cfg(not(target_arch = "wasm32"))]
					deadline: None,
				};
				// reuse the RPC ID. this is somewhat dirty.
				return unsub_request.serialize_raw(id).ok();
			}
			tracing::trace!("No current server id");
		}
		tracing::trace!("Cannot end unknown subscription");
		None
	}

	#[tracing::instrument(skip_all, fields(server_id = ? notification.subscription))]
	fn handle_notification(&mut self, notification: Notification) {
		let server_id = notification.subscription;

		let Some(id) = self.aliases.get(&server_id).copied() else {
			tracing::debug!(
				server_id = format!("0x{server_id:x}"),
				"No aliased subscription found"
			);
			return;
		};

		let Some(active) = self.subs.get(&id) else {
			tracing::trace!(id, "Aliased subscription found, but not active");
			self.aliases.remove(&server_id);
			return;
		};

		tracing::debug!(id, "Forwarding notification to listener");
		// send the notification over the channel
		let send_res = active.channel.unbounded_send(notification.result);

		// receiver has dropped, so we drop the sub
		if send_res.is_err() {
			tracing::debug!(id, "Listener dropped. Dropping alias and subs");
			// Note: We remove the subscription mappings but don't send an unsubscribe request
			// to the server. This is because the channel receiver has already been dropped,
			// indicating the consumer is no longer interested in the subscription.
			// The server will continue sending notifications which we'll ignore.
			self.aliases.remove(&server_id);
			self.subs.remove(&id);
		}
	}

	fn req_success(&mut self, id: u64, result: Box<RawValue>) -> Box<RawValue> {
		if let Ok(server_id) = serde_json::from_str::<SubId>(result.get()) {
			tracing::debug!(id, server_id = %server_id.0, "Registering new sub alias");
			self.add_alias(server_id.0, id);
			let client_id = U256::from(id);
			match to_raw_value(&format!("0x{client_id:x}")) {
				Ok(raw) => raw,
				Err(e) => {
					// Best-effort: if we can't serialize the aliased subscription ID, fall back to
					// the server-provided subscription id instead of panicking.
					tracing::warn!(
						error = %e,
						id,
						"Failed to encode aliased subscription id; returning server id"
					);
					result
				},
			}
		} else {
			result
		}
	}

	fn has(&self, id: u64) -> bool {
		self.subs.contains_key(&id)
	}

	fn to_reissue(&self) -> impl Iterator<Item = (&u64, &ActiveSub)> {
		self.subs.iter()
	}

	fn service_subscription_request(
		&mut self,
		id: u64,
		params: Box<RawValue>,
	) -> Result<Box<RawValue>, WsClientError> {
		let (tx, rx) = mpsc::unbounded();

		let active_sub = ActiveSub { params, channel: tx, current_server_id: None };
		let req = active_sub.serialize_raw(id)?;

		// Explicit scope for the lock
		// This insertion should be made BEFORE the request returns.
		// So we make it before the request is even dispatched :)
		{
			self.channel_map.lock().unwrap_or_else(|e| e.into_inner()).insert(id.into(), rx);
		}
		self.subs.insert(id, active_sub);

		Ok(req)
	}
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
	use super::*;

	fn make_manager(timeout: core::time::Duration) -> RequestManager {
		let backend = BackendDriver::new_for_test();
		let conn = ConnectionDetails::new("ws://localhost:10334/ws", None);
		let (_instructions_tx, instructions) = mpsc::unbounded();

		RequestManager {
			id: AtomicU64::new(1),
			reconnects: 0,
			subs: SubscriptionManager::new(Default::default()),
			reqs: Default::default(),
			backend,
			conn,
			config: None,
			request_timeout: Some(timeout),
			instructions,
		}
	}

	#[tokio::test]
	async fn expires_timed_out_requests() {
		let mut manager = make_manager(core::time::Duration::from_millis(10));
		let (tx, rx) = oneshot::channel::<Response>();

		manager.reqs.insert(
			1,
			InFlight {
				method: "test_method".to_string(),
				params: to_raw_value(&()).unwrap(),
				channel: tx,
				deadline: Some(std::time::Instant::now() - core::time::Duration::from_secs(1)),
			},
		);

		manager.expire_timed_out_requests();

		let response = rx.await.unwrap();
		let err = response.unwrap_err();
		assert_eq!(err.code, -32000);
		assert!(err.message.contains("request timed out"));
		assert!(manager.reqs.is_empty());
	}

	#[tokio::test]
	async fn cleans_up_subscription_on_timeout() {
		let mut manager = make_manager(core::time::Duration::from_millis(10));

		let id = 7u64;
		let params = to_raw_value(&["newHeads"]).unwrap();
		manager.subs.service_subscription_request(id, params.clone()).unwrap();

		assert!(
			manager
				.subs
				.channel_map
				.lock()
				.unwrap_or_else(|e| e.into_inner())
				.contains_key(&U256::from(id))
		);

		let (tx, rx) = oneshot::channel::<Response>();
		manager.reqs.insert(
			id,
			InFlight {
				method: "neo_subscribe".to_string(),
				params,
				channel: tx,
				deadline: Some(std::time::Instant::now() - core::time::Duration::from_secs(1)),
			},
		);

		manager.expire_timed_out_requests();

		let response = rx.await.unwrap();
		assert!(response.is_err());
		assert!(!manager.subs.has(id));
		assert!(
			!manager
				.subs
				.channel_map
				.lock()
				.unwrap_or_else(|e| e.into_inner())
				.contains_key(&U256::from(id))
		);
	}
}

/// The `RequestManager` holds copies of all pending requests (as `InFlight`),
/// and active subscriptions (as `ActiveSub`). When reconnection occurs, all
/// pending requests are re-dispatched to the new backend, and all active subs
/// are re-subscribed
///
///  `RequestManager` holds a `BackendDriver`, to communicate with the current
/// backend. Reconnection is accomplished by instantiating a new `WsBackend` and
/// swapping out the manager's `BackendDriver`.
///
/// In order to provide continuity of subscription IDs to the client, the
/// `RequestManager` also keeps a `SubscriptionManager`. See the
/// `SubscriptionManager` docstring for more complete details
///
/// The behavior is accessed by the WsClient frontend, which implements ]
/// `JsonRpcClient`. The `WsClient` is cloneable, so no need for an arc :). It
/// communicates to the request manager via a channel, and receives
/// notifications in a shared map for the client to retrieve
///
/// The `RequestManager` shuts down and drops when all `WsClient` instances have
/// been dropped (because all instruction channel `UnboundedSender` instances
/// will have dropped).
pub(super) struct RequestManager {
	// Next JSON-RPC Request ID
	id: AtomicU64,
	// How many times we should reconnect the backend before erroring
	reconnects: usize,
	// Subscription manager
	subs: SubscriptionManager,
	// Requests for which a response has not been receivedc
	reqs: BTreeMap<u64, InFlight>,
	// Control of the active WS backend
	backend: BackendDriver,
	// The URL and optional auth info for the connection
	conn: ConnectionDetails,
	#[cfg(not(target_arch = "wasm32"))]
	// An Option wrapping a tungstenite WebsocketConfig. If None, the default config is used.
	config: Option<WebSocketConfig>,
	#[cfg(not(target_arch = "wasm32"))]
	request_timeout: Option<core::time::Duration>,
	// Instructions from the user-facing providers
	instructions: mpsc::UnboundedReceiver<Instruction>,
}

impl RequestManager {
	fn next_id(&mut self) -> u64 {
		self.id.fetch_add(1, Ordering::Relaxed)
	}

	pub(super) async fn connect(
		conn: ConnectionDetails,
	) -> Result<(Self, WsClient), WsClientError> {
		Self::connect_with_reconnects(conn, DEFAULT_RECONNECTS).await
	}

	async fn connect_internal(
		conn: ConnectionDetails,
	) -> Result<
		(
			BackendDriver,
			(mpsc::UnboundedSender<Instruction>, mpsc::UnboundedReceiver<Instruction>),
			SharedChannelMap,
		),
		WsClientError,
	> {
		let (ws, backend) = WsBackend::connect(conn).await?;

		ws.spawn();

		Ok((backend, mpsc::unbounded(), Default::default()))
	}

	#[cfg(target_arch = "wasm32")]
	pub(super) async fn connect_with_reconnects(
		conn: ConnectionDetails,
		reconnects: usize,
	) -> Result<(Self, WsClient), WsClientError> {
		let (backend, (instructions_tx, instructions_rx), channel_map) =
			Self::connect_internal(conn.clone()).await?;

		Ok((
			Self {
				id: Default::default(),
				reconnects,
				subs: SubscriptionManager::new(channel_map.clone()),
				reqs: Default::default(),
				backend,
				conn,
				instructions: instructions_rx,
			},
			WsClient { instructions: instructions_tx, channel_map },
		))
	}

	#[cfg(not(target_arch = "wasm32"))]
	pub(super) async fn connect_with_reconnects(
		conn: ConnectionDetails,
		reconnects: usize,
	) -> Result<(Self, WsClient), WsClientError> {
		let (backend, (instructions_tx, instructions_rx), channel_map) =
			Self::connect_internal(conn.clone()).await?;

		Ok((
			Self {
				id: Default::default(),
				reconnects,
				subs: SubscriptionManager::new(channel_map.clone()),
				reqs: Default::default(),
				backend,
				conn,
				config: None,
				request_timeout: NeoConstants::rpc_request_timeout(),
				instructions: instructions_rx,
			},
			WsClient { instructions: instructions_tx, channel_map },
		))
	}

	#[cfg(not(target_arch = "wasm32"))]
	pub(super) async fn connect_with_config(
		conn: ConnectionDetails,
		config: WebSocketConfig,
	) -> Result<(Self, WsClient), WsClientError> {
		Self::connect_with_config_and_reconnects(conn, config, DEFAULT_RECONNECTS).await
	}

	#[cfg(not(target_arch = "wasm32"))]
	pub(super) async fn connect_with_config_and_reconnects(
		conn: ConnectionDetails,
		config: WebSocketConfig,
		reconnects: usize,
	) -> Result<(Self, WsClient), WsClientError> {
		let (backend, (instructions_tx, instructions_rx), channel_map) =
			Self::connect_internal(conn.clone()).await?;

		Ok((
			Self {
				id: Default::default(),
				reconnects,
				subs: SubscriptionManager::new(channel_map.clone()),
				reqs: Default::default(),
				backend,
				conn,
				config: Some(config),
				request_timeout: NeoConstants::rpc_request_timeout(),
				instructions: instructions_rx,
			},
			WsClient { instructions: instructions_tx, channel_map },
		))
	}

	#[cfg(target_arch = "wasm32")]
	async fn reconnect_backend(&mut self) -> Result<(WsBackend, BackendDriver), WsClientError> {
		WsBackend::connect(self.conn.clone()).await
	}

	#[cfg(not(target_arch = "wasm32"))]
	async fn reconnect_backend(&mut self) -> Result<(WsBackend, BackendDriver), WsClientError> {
		if let Some(config) = self.config {
			WsBackend::connect_with_config(self.conn.clone(), config, false).await
		} else {
			WsBackend::connect(self.conn.clone()).await
		}
	}

	async fn reconnect(&mut self) -> Result<(), WsClientError> {
		if self.reconnects == 0 {
			return Err(WsClientError::TooManyReconnects);
		}
		self.reconnects -= 1;

		tracing::info!(remaining = self.reconnects, url = self.conn.url, "Reconnecting to backend");
		// create the new backend
		let (s, mut backend) = self.reconnect_backend().await?;

		// spawn the new backend
		s.spawn();

		// swap out the backend
		std::mem::swap(&mut self.backend, &mut backend);

		// rename for clarity
		let mut old_backend = backend;

		// Drain anything in the backend
		tracing::debug!("Draining old backend to_handle channel");
		while let Some(to_handle) = old_backend.to_handle.next().await {
			self.handle(to_handle);
		}

		// issue a shutdown command (even though it's likely gone)
		old_backend.shutdown();

		// Clear stale server-side subscription IDs before re-subscribing.
		self.subs.reset_server_ids();

		tracing::debug!(count = self.subs.count(), "Re-starting active subscriptions");
		let req_cnt = self.reqs.len();

		// reissue subscriptions
		for (id, sub) in self.subs.to_reissue() {
			let (tx, _rx) = oneshot::channel();
			let in_flight = InFlight {
				method: "neo_subscribe".to_string(),
				params: sub.params.clone(),
				channel: tx,
				#[cfg(not(target_arch = "wasm32"))]
				deadline: self.request_timeout.map(|timeout| std::time::Instant::now() + timeout),
			};
			// Need an entry in reqs to ensure response with new server sub ID is processed
			self.reqs.insert(*id, in_flight);
		}

		tracing::debug!(count = req_cnt, "Re-issuing pending requests");
		// reissue requests, including the re-subscription requests we just added above
		for (id, req) in self.reqs.iter() {
			self.backend
				.dispatcher
				.unbounded_send(req.serialize_raw(*id)?)
				.map_err(|_| WsClientError::DeadChannel)?;
		}
		tracing::info!(subs = self.subs.count(), reqs = req_cnt, "Re-connection complete");

		Ok(())
	}

	#[tracing::instrument(skip(self, result))]
	fn req_success(&mut self, id: u64, result: Box<RawValue>) {
		// pending fut is missing, this is fine
		tracing::trace!(len = result.get().len(), "Success response received");
		if let Some(req) = self.reqs.remove(&id) {
			tracing::debug!("Sending result to request listener");
			// Allow subscription manager to rewrite the result if the request
			// corresponds to a known ID
			let result = if self.subs.has(id) { self.subs.req_success(id, result) } else { result };
			let _ = req.channel.send(Ok(result));
		} else {
			tracing::trace!("No InFlight found");
		}
	}

	fn req_fail(&mut self, id: u64, error: JsonRpcError) {
		// pending fut is missing, this is fine
		if let Some(req) = self.reqs.remove(&id) {
			if self.subs.has(id) {
				let _ = self.subs.end_subscription(id);
			}
			// pending fut has been dropped, this is fine
			let _ = req.channel.send(Err(error));
		}
	}

	fn handle(&mut self, item: PubSubItem) {
		match item {
			PubSubItem::Success { id, result } => self.req_success(id, result),
			PubSubItem::Error { id, error } => self.req_fail(id, error),
			PubSubItem::Notification { params } => self.subs.handle_notification(params),
		}
	}

	#[tracing::instrument(skip(self, params, sender))]
	fn service_request(
		&mut self,
		id: u64,
		method: String,
		params: Box<RawValue>,
		sender: oneshot::Sender<Response>,
	) -> Result<(), WsClientError> {
		let in_flight = InFlight {
			method,
			params,
			channel: sender,
			#[cfg(not(target_arch = "wasm32"))]
			deadline: self.request_timeout.map(|timeout| std::time::Instant::now() + timeout),
		};
		let req = in_flight.serialize_raw(id)?;

		// Ordering matters here. We want this block above the unbounded send,
		// and after the serialization
		if in_flight.method == "neo_subscribe" {
			self.subs.service_subscription_request(id, in_flight.params.clone())?;
		}

		// Must come after self.subs.service_subscription_request. Do not re-order
		tracing::debug!("Dispatching request to backend");
		self.backend
			.dispatcher
			.unbounded_send(req)
			.map_err(|_| WsClientError::DeadChannel)?;

		self.reqs.insert(id, in_flight);
		Ok(())
	}

	#[cfg(not(target_arch = "wasm32"))]
	fn next_request_deadline(&self) -> Option<std::time::Instant> {
		self.reqs.values().filter_map(|req| req.deadline).min()
	}

	#[cfg(not(target_arch = "wasm32"))]
	fn expire_timed_out_requests(&mut self) {
		let Some(timeout) = self.request_timeout else {
			return;
		};

		let now = std::time::Instant::now();
		let mut expired_ids = Vec::new();
		for (id, req) in self.reqs.iter() {
			if req.deadline.is_some_and(|d| d <= now) {
				expired_ids.push(*id);
			}
		}

		for id in expired_ids {
			let Some(req) = self.reqs.remove(&id) else {
				continue;
			};

			if self.subs.has(id) {
				let _ = self.subs.end_subscription(id);
			}

			let _ = req.channel.send(Err(JsonRpcError {
				code: -32000,
				message: format!("request timed out after {timeout:?}: {}", req.method),
				data: None,
			}));
		}
	}

	fn service_instruction(&mut self, instruction: Instruction) -> Result<(), WsClientError> {
		match instruction {
			Instruction::Request { method, params, sender } => {
				let id = self.next_id();
				self.service_request(id, method, params, sender)?;
			},
			Instruction::Unsubscribe { id } => {
				if let Some(req) = self.subs.end_subscription(id.low_u64()) {
					self.backend
						.dispatcher
						.unbounded_send(req)
						.map_err(|_| WsClientError::DeadChannel)?;
				}
			},
		}
		Ok(())
	}

	pub(super) fn spawn(mut self) {
		let fut = async move {
			let result = loop {
				#[cfg(not(target_arch = "wasm32"))]
				self.expire_timed_out_requests();

				#[cfg(not(target_arch = "wasm32"))]
				let request_timeout = {
					let fut = if let Some(deadline) = self.next_request_deadline() {
						Either::Left(tokio::time::sleep_until(tokio::time::Instant::from_std(
							deadline,
						)))
					} else {
						Either::Right(futures_util::future::pending::<()>())
					};
					fut.fuse()
				};

				#[cfg(target_arch = "wasm32")]
				let request_timeout = futures_util::future::pending::<()>().fuse();

				futures_util::pin_mut!(request_timeout);

				// We bias the loop so that we always handle messages before
				// reconnecting, and always reconnect before dispatching new
				// requests
				select_biased! {
					item_opt = self.backend.to_handle.next() => {
						match item_opt {
							Some(item) => self.handle(item),
							// Backend is gone, so reconnect
							None => if let Err(e) = self.reconnect().await {
								break Err(e);
							}
						}
					},
					_ = &mut self.backend.error => {
						if let Err(e) = self.reconnect().await {
							break Err(e);
						}
					},
					_ = request_timeout => {
						#[cfg(not(target_arch = "wasm32"))]
						self.expire_timed_out_requests();
					},
					inst_opt = self.instructions.next() => {
						match inst_opt {
							Some(instruction) => if let Err(e) = self.service_instruction(instruction) { break Err(e)},
							// User-facing side is gone, so just exit
							None => break Ok(()),
						}
					}
				}
			};
			if let Err(err) = result {
				tracing::error!(%err, "Error during reconnection");
			}
			// Issue the shutdown command. we don't care if it is received
			self.backend.shutdown();
		};

		#[cfg(target_arch = "wasm32")]
		super::spawn_local(fut);

		#[cfg(not(target_arch = "wasm32"))]
		tokio::spawn(fut);
	}
}
