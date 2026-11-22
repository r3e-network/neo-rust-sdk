use std::{
	cell::RefCell,
	convert::Infallible,
	fmt::Debug,
	hash::BuildHasherDefault,
	io,
	path::Path,
	sync::{
		Arc,
		atomic::{AtomicU64, Ordering},
	},
	thread,
};

use async_trait::async_trait;
use bytes::{Buf, BytesMut};
use futures_channel::mpsc;
use futures_util::stream::StreamExt;
use primitive_types::U256;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Deserializer, value::RawValue};
use thiserror::Error;
use tokio::{
	io::{AsyncReadExt, AsyncWriteExt, BufReader},
	runtime,
	sync::oneshot::{self, error::RecvError},
};

use rustc_hash::FxHasher;

use crate::config::NeoConstants;
use crate::neo_clients::{JsonRpcProvider, ProviderError, PubsubClient, RpcError};

use super::common::{JsonRpcError, Params, Request, Response};

use self::imp::*;

type FxHashMap<K, V> = std::collections::HashMap<K, V, BuildHasherDefault<FxHasher>>;

type Pending = oneshot::Sender<Result<Box<RawValue>, JsonRpcError>>;
type Subscription = mpsc::UnboundedSender<Box<RawValue>>;

#[derive(Debug)]
struct PendingRequest {
	sender: Pending,
	deadline: Option<std::time::Instant>,
}

impl PendingRequest {
	fn new(sender: Pending, timeout: Option<core::time::Duration>) -> Self {
		Self { sender, deadline: timeout.map(|t| std::time::Instant::now() + t) }
	}
}

#[cfg(unix)]
#[doc(hidden)]
mod imp {
	use tokio::net::UnixStream;

	pub(super) type Stream = UnixStream;
	pub(super) type ReadHalf<'a> = tokio::io::ReadHalf<&'a mut UnixStream>;
	pub(super) type WriteHalf<'a> = tokio::io::WriteHalf<&'a mut UnixStream>;

	#[allow(dead_code)]
	pub(super) async fn connect(path: impl AsRef<std::path::Path>) -> std::io::Result<Stream> {
		UnixStream::connect(path).await
	}

	pub(super) fn split(stream: &mut Stream) -> (ReadHalf<'_>, WriteHalf<'_>) {
		tokio::io::split(stream)
	}
}

#[cfg(windows)]
#[doc(hidden)]
mod imp {
	use std::{
		io,
		path::Path,
		pin::Pin,
		task::{Context, Poll},
		time::Duration,
	};

	use tokio::{
		io::{AsyncRead, AsyncWrite, ReadBuf},
		net::windows::named_pipe::{ClientOptions, NamedPipeClient},
		time::sleep,
	};

	/// Win32 `ERROR_PIPE_BUSY` (231): all named-pipe instances are busy.
	/// Inlined instead of pulling in `winapi` for a single constant.
	const ERROR_PIPE_BUSY: u32 = 231;

	/// Wrapper around [NamedPipeClient] to have the same methods as a UnixStream.
	///
	/// Should not be exported.
	#[repr(transparent)]
	pub(super) struct Stream(pub NamedPipeClient);

	pub(super) type ReadHalf<'a> = tokio::io::ReadHalf<&'a mut Stream>;
	pub(super) type WriteHalf<'a> = tokio::io::WriteHalf<&'a mut Stream>;

	impl Stream {
		pub(super) async fn connect(addr: impl AsRef<Path>) -> Result<Self, io::Error> {
			let addr = addr.as_ref().as_os_str();
			loop {
				match ClientOptions::new().open(addr) {
					Ok(client) => break Ok(Self(client)),
					Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) => (),
					Err(e) => break Err(e),
				}

				sleep(Duration::from_millis(50)).await;
			}
		}
	}

	impl AsyncRead for Stream {
		fn poll_read(
			self: Pin<&mut Self>,
			cx: &mut Context<'_>,
			buf: &mut ReadBuf<'_>,
		) -> Poll<io::Result<()>> {
			let this = Pin::new(&mut self.get_mut().0);
			this.poll_read(cx, buf)
		}
	}

	impl AsyncWrite for Stream {
		fn poll_write(
			self: Pin<&mut Self>,
			cx: &mut Context<'_>,
			buf: &[u8],
		) -> Poll<io::Result<usize>> {
			let this = Pin::new(&mut self.get_mut().0);
			this.poll_write(cx, buf)
		}

		fn poll_write_vectored(
			self: Pin<&mut Self>,
			cx: &mut Context<'_>,
			bufs: &[io::IoSlice<'_>],
		) -> Poll<io::Result<usize>> {
			let this = Pin::new(&mut self.get_mut().0);
			this.poll_write_vectored(cx, bufs)
		}

		fn is_write_vectored(&self) -> bool {
			self.0.is_write_vectored()
		}

		fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
			let this = Pin::new(&mut self.get_mut().0);
			this.poll_flush(cx)
		}

		fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
			let this = Pin::new(&mut self.get_mut().0);
			this.poll_shutdown(cx)
		}
	}

	pub(super) fn split(stream: &mut Stream) -> (ReadHalf<'_>, WriteHalf<'_>) {
		tokio::io::split(stream)
	}
}

#[cfg_attr(unix, doc = "A JSON-RPC Client over Unix IPC.")]
#[cfg_attr(windows, doc = "A JSON-RPC Client over named pipes.")]
///
/// # Example
///
/// ```no_run
/// # async fn foo() -> Result<(), Box<dyn std::error::Error>> {
/// use neo3::neo_clients::Ipc;
///
/// // the ipc's path
#[cfg_attr(unix, doc = r#"let path = "/tmp/neo-node.ipc";"#)]
#[cfg_attr(windows, doc = r#"let path = r"\\.\pipe\neo-node.ipc";"#)]
/// let ipc = Ipc::connect(path).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Ipc {
	id: Arc<AtomicU64>,
	request_tx: mpsc::UnboundedSender<TransportMessage>,
}

#[derive(Debug)]
enum TransportMessage {
	Request { id: u64, request: Box<[u8]>, sender: Pending },
	Subscribe { id: U256, sink: Subscription },
	Unsubscribe { id: U256 },
}

impl Ipc {
	#[cfg_attr(unix, doc = "Connects to the Unix socket at the provided path.")]
	#[cfg_attr(windows, doc = "Connects to the named pipe at the provided path.\n")]
	#[cfg_attr(
		windows,
		doc = r"Note: the path must be the fully qualified, like: `\\.\pipe\<name>`."
	)]
	pub async fn connect(path: impl AsRef<Path>) -> Result<Self, IpcError> {
		let id = Arc::new(AtomicU64::new(1));
		let (request_tx, request_rx) = mpsc::unbounded();

		let stream = Stream::connect(path).await?;
		spawn_ipc_server(stream, request_rx)?;

		Ok(Self { id, request_tx })
	}

	fn send(&self, msg: TransportMessage) -> Result<(), IpcError> {
		self.request_tx
			.unbounded_send(msg)
			.map_err(|_| IpcError::ChannelError("IPC server receiver dropped".to_string()))?;

		Ok(())
	}
}

#[async_trait]
impl JsonRpcProvider for Ipc {
	type Error = IpcError;

	async fn fetch<T, R>(&self, method: &str, params: T) -> Result<R, IpcError>
	where
		T: Debug + Serialize + Send + Sync,
		R: DeserializeOwned,
	{
		let next_id = self.id.fetch_add(1, Ordering::SeqCst);

		// Create the request and initialize the response channel
		let (sender, receiver) = oneshot::channel();
		let payload = TransportMessage::Request {
			id: next_id,
			request: serde_json::to_vec(&Request::new(next_id, method, params))?.into_boxed_slice(),
			sender,
		};

		// Send the request to the IPC server to be handled.
		self.send(payload)?;

		// Wait for the response from the IPC server.
		let res = receiver.await??;

		// Parse JSON response.
		Ok(serde_json::from_str(res.get())?)
	}
}

impl PubsubClient for Ipc {
	type NotificationStream = mpsc::UnboundedReceiver<Box<RawValue>>;

	fn subscribe<T: Into<U256>>(&self, id: T) -> Result<Self::NotificationStream, IpcError> {
		let (sink, stream) = mpsc::unbounded();
		self.send(TransportMessage::Subscribe { id: id.into(), sink })?;
		Ok(stream)
	}

	fn unsubscribe<T: Into<U256>>(&self, id: T) -> Result<(), IpcError> {
		self.send(TransportMessage::Unsubscribe { id: id.into() })
	}
}

fn spawn_ipc_server(
	stream: Stream,
	request_rx: mpsc::UnboundedReceiver<TransportMessage>,
) -> Result<(), IpcError> {
	// 256 Kb should be more than enough for this thread, as all unbounded data
	// growth occurs on heap-allocated data structures and buffers and the call
	// stack is not going to do anything crazy either
	const STACK_SIZE: usize = 1 << 18;
	// spawn a light-weight thread with a thread-local async runtime just for
	// sending and receiving data over the IPC socket
	thread::Builder::new()
		.name("ipc-server-thread".to_string())
		.stack_size(STACK_SIZE)
		.spawn(move || {
			let rt = match runtime::Builder::new_current_thread().enable_io().build() {
				Ok(rt) => rt,
				Err(err) => {
					tracing::error!(error = %err, "failed to create ipc-server-thread async runtime");
					return;
				},
			};

			rt.block_on(run_ipc_server(stream, request_rx));
		})
		.map_err(|e| IpcError::ChannelError(format!("failed to spawn ipc server thread: {e}")))?;
	Ok(())
}

async fn run_ipc_server(mut stream: Stream, request_rx: mpsc::UnboundedReceiver<TransportMessage>) {
	// the shared state for both reads & writes
	let shared = Shared {
		pending: FxHashMap::with_capacity_and_hasher(64, BuildHasherDefault::default()).into(),
		subs: FxHashMap::with_capacity_and_hasher(64, BuildHasherDefault::default()).into(),
	};

	// split the stream and run two independent concurrently (local), thereby
	// allowing reads and writes to occurr concurrently
	let (reader, writer) = split(&mut stream);
	let read = shared.handle_ipc_reads(reader);
	let write = shared.handle_ipc_writes(writer, request_rx);

	// run both loops concurrently, until either encounts an error
	match futures_util::try_join!(read, write) {
		Err(e) => match e {
			IpcError::ServerExit => {},
			err => tracing::error!(?err, "exiting IPC server due to error"),
		},
	}
}

struct Shared {
	pending: RefCell<FxHashMap<u64, PendingRequest>>,
	subs: RefCell<FxHashMap<U256, Subscription>>,
}

impl Shared {
	fn next_pending_deadline(&self) -> Option<std::time::Instant> {
		self.pending.borrow().values().filter_map(|p| p.deadline).min()
	}

	fn expire_timed_out_requests(&self, timeout: core::time::Duration) {
		let now = std::time::Instant::now();
		let mut expired: Vec<Pending> = Vec::new();

		{
			let mut pending = self.pending.borrow_mut();
			let expired_ids: Vec<u64> = pending
				.iter()
				.filter_map(|(&id, req)| req.deadline.is_some_and(|d| d <= now).then_some(id))
				.collect();

			for id in expired_ids {
				if let Some(req) = pending.remove(&id) {
					expired.push(req.sender);
				}
			}
		}

		if expired.is_empty() {
			return;
		}

		let err = JsonRpcError {
			code: -32000,
			message: format!("request timed out after {timeout:?}"),
			data: None,
		};

		for sender in expired {
			let _ = sender.send(Err(err.clone()));
		}
	}

	async fn handle_ipc_reads(&self, reader: ReadHalf<'_>) -> Result<Infallible, IpcError> {
		let mut reader = BufReader::new(reader);
		let mut buf = BytesMut::with_capacity(4096);
		let max_buffer_size: usize = NeoConstants::max_rpc_message_size();

		loop {
			// try to read the next batch of bytes into the buffer
			let read = reader.read_buf(&mut buf).await?;
			if read == 0 {
				// eof, socket was closed
				return Err(IpcError::ServerExit);
			}

			if buf.len() > max_buffer_size {
				return Err(io::Error::new(
					io::ErrorKind::InvalidData,
					format!("IPC message exceeded {max_buffer_size} bytes"),
				)
				.into());
			}

			// parse the received bytes into 0-n jsonrpc messages
			let read = self.handle_bytes(&buf)?;
			// split off all bytes that were parsed into complete messages
			// any remaining bytes that correspond to incomplete messages remain
			// in the buffer
			buf.advance(read);
		}
	}

	async fn handle_ipc_writes(
		&self,
		mut writer: WriteHalf<'_>,
		mut request_rx: mpsc::UnboundedReceiver<TransportMessage>,
	) -> Result<Infallible, IpcError> {
		let request_timeout = NeoConstants::rpc_request_timeout();

		loop {
			if let Some(timeout) = request_timeout {
				let deadline = self.next_pending_deadline();
				let sleep = match deadline {
					Some(deadline) => {
						tokio::time::sleep_until(tokio::time::Instant::from_std(deadline))
					},
					None => tokio::time::sleep(timeout),
				};

				tokio::select! {
					_ = sleep => {
						self.expire_timed_out_requests(timeout);
					}
					msg = request_rx.next() => {
						let Some(msg) = msg else { break };
						self.handle_write_message(&mut writer, msg, request_timeout).await?;
					}
				}
			} else {
				let Some(msg) = request_rx.next().await else { break };
				self.handle_write_message(&mut writer, msg, request_timeout).await?;
			}
		}

		// the request receiver will only be closed if the sender instance
		// located within the transport handle is dropped, this is not truly an
		// error but leads to the `try_join` in `run_ipc_server` to cancel the
		// read half future
		Err(IpcError::ServerExit)
	}

	async fn handle_write_message(
		&self,
		writer: &mut WriteHalf<'_>,
		msg: TransportMessage,
		request_timeout: Option<core::time::Duration>,
	) -> Result<(), IpcError> {
		use TransportMessage::*;
		match msg {
			Request { id, request, sender } => {
				let prev = self
					.pending
					.borrow_mut()
					.insert(id, PendingRequest::new(sender, request_timeout));
				if prev.is_some() {
					tracing::warn!(%id, "replaced pending IPC request (ID collision or misuse)");
				}

				if let Err(err) = writer.write_all(&request).await {
					tracing::error!("IPC connection error: {:?}", err);
					self.pending.borrow_mut().remove(&id);
				}
			},
			Subscribe { id, sink } => {
				if self.subs.borrow_mut().insert(id, sink).is_some() {
					tracing::warn!(%id, "replaced already-registered subscription");
				}
			},
			Unsubscribe { id } => {
				if self.subs.borrow_mut().remove(&id).is_none() {
					tracing::warn!(%id, "attempted to unsubscribe from non-existent subscription");
				}
			},
		}
		Ok(())
	}

	fn handle_bytes(&self, bytes: &BytesMut) -> Result<usize, IpcError> {
		// deserialize all complete jsonrpc responses in the buffer
		let mut de = Deserializer::from_slice(bytes.as_ref()).into_iter();
		for response in de.by_ref() {
			match response {
				Ok(response) => match response {
					Response::Success { id, result } => {
						self.send_response(id, Ok(result.to_owned()))
					},
					Response::Error { id, error } => self.send_response(id, Err(error)),
					Response::Notification { params, .. } => self.send_notification(params),
				},
				Err(err) if err.is_eof() => break,
				Err(err) => return Err(IpcError::JsonError(err)),
			}
		}

		Ok(de.byte_offset())
	}

	fn send_response(&self, id: u64, result: Result<Box<RawValue>, JsonRpcError>) {
		// retrieve the channel sender for responding to the pending request
		let response_tx = match self.pending.borrow_mut().remove(&id) {
			Some(req) => req.sender,
			None => {
				tracing::warn!(%id, "no pending request exists for the response ID");
				return;
			},
		};

		// a failure to send the response indicates that the pending request has
		// been dropped in the mean time
		let _ = response_tx.send(result);
	}

	/// Sends notification through the channel based on the ID of the subscription.
	/// This handles streaming responses.
	fn send_notification(&self, params: Params<'_>) {
		// retrieve the channel sender for notifying the subscription stream
		let subs = self.subs.borrow();
		let tx = match subs.get(&params.subscription) {
			Some(tx) => tx,
			None => {
				tracing::warn!(
					id = ?params.subscription,
					"no subscription exists for the notification ID"
				);
				return;
			},
		};

		// a failure to send the response indicates that the pending request has
		// been dropped in the mean time (and should have been unsubscribed!)
		let _ = tx.unbounded_send(params.result.to_owned());
	}
}

/// Error thrown when sending or receiving an IPC message.
#[derive(Debug, Error)]
pub enum IpcError {
	/// Thrown if deserialization failed
	#[error(transparent)]
	JsonError(#[from] serde_json::Error),

	/// std IO error forwarding.
	#[error(transparent)]
	IoError(#[from] io::Error),

	/// Server responded to the request with a valid JSON-RPC error response
	#[error(transparent)]
	JsonRpcError(#[from] JsonRpcError),

	/// Internal channel failed
	#[error("{0}")]
	ChannelError(String),

	/// Listener for request result is gone
	#[error(transparent)]
	RequestCancelled(#[from] RecvError),

	/// IPC server exited
	#[error("The IPC server has exited")]
	ServerExit,
}

impl From<IpcError> for ProviderError {
	fn from(src: IpcError) -> Self {
		match src {
			IpcError::JsonRpcError(err) => ProviderError::JsonRpcError(err),
			IpcError::JsonError(err) => ProviderError::SerdeJson(err),
			IpcError::IoError(err) => ProviderError::CustomError(err.to_string()),
			IpcError::ChannelError(msg) => ProviderError::CustomError(msg),
			IpcError::RequestCancelled(err) => ProviderError::CustomError(err.to_string()),
			IpcError::ServerExit => ProviderError::CustomError("The IPC server has exited".into()),
		}
	}
}

impl RpcError for IpcError {
	fn as_error_response(&self) -> Option<&super::JsonRpcError> {
		if let IpcError::JsonRpcError(err) = self { Some(err) } else { None }
	}

	fn as_serde_error(&self) -> Option<&serde_json::Error> {
		match self {
			IpcError::JsonError(err) => Some(err),
			_ => None,
		}
	}
}

/* NOTE: These tests require the testcontainers crate to be added as a dev-dependency.
   Uncomment this module after adding:
   - testcontainers = "0.23"
   - testcontainers-modules = { version = "0.11", features = ["geth"] }
   to the [dev-dependencies] section in Cargo.toml

#[allow(unexpected_cfgs)]
#[cfg(all(test, feature = "ipc", feature = "container-tests", not(target_arch = "wasm32")))]
mod tests {
	use std::time::Duration;

	use tempfile::NamedTempFile;

	use super::Ipc;
	use crate::{
		neo_types::{block::Block, TxHash},
		prelude::U256,
	};
	use futures_util::StreamExt;
	use testcontainers_modules::geth::{Geth, GethInstance};

	async fn connect() -> (Ipc, GethInstance) {
		let temp_file = NamedTempFile::new().unwrap();
		let path = temp_file.into_temp_path().to_path_buf();
		let geth = Geth::default().block_time(1u64).ipc_path(&path).spawn();

		// [Windows named pipes](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipes)
		// are located at `\\<machine_address>\pipe\<pipe_name>`.
		#[cfg(windows)]
		let path = format!(r"\\.\pipe\{}", path.display());
		let ipc = Ipc::connect(path).await.unwrap();

		(ipc, geth)
	}

	#[tokio::test]
	async fn request() {
		let (ipc, _geth) = connect().await;

		let block_num: U256 = ipc.fetch("neo_blockNumber", ()).await.unwrap();
		tokio::time::sleep(Duration::from_secs(2)).await;
		let block_num2: U256 = ipc.fetch("neo_blockNumber", ()).await.unwrap();
		assert!(block_num2 > block_num);
	}

	#[tokio::test]
	async fn subscription() {
		let (ipc, _geth) = connect().await;

		// Subscribing requires sending the sub request and then subscribing to
		// the returned sub_id
		let sub_id: U256 = ipc.fetch("neo_subscribe", ["newHeads"]).await.unwrap();
		let stream = ipc.subscribe(sub_id).unwrap();

		let blocks: Vec<u64> = stream
			.take(3)
			.map(|item| {
				let block: Block<TxHash> = serde_json::from_str(item.get()).unwrap();
				block.index
			})
			.collect()
			.await;
		// `[1, 2, 3]` or `[2, 3, 4]` etc, depending on test latency
		assert_eq!(blocks[2], blocks[1] + 1);
		assert_eq!(blocks[1], blocks[0] + 1);
	}
}
*/
