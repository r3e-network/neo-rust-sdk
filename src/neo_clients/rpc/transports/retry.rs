//! A [JsonRpcProvider] implementation that retries requests filtered by [RetryPolicy]
//! with an exponential backoff.

use std::{
	fmt::Debug,
	sync::atomic::{AtomicU32, Ordering},
	time::Duration,
};

use super::{
	common::{JsonRpcError, MAX_PROVIDER_RETRY_AFTER},
	http_provider::ClientError,
};
use crate::neo_clients::{JsonRpcProvider, ProviderError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;
use tracing::trace;

/// [RetryPolicy] defines logic for which [JsonRpcProvider::Error] instances should
/// the client retry the request and try to recover from.
pub trait RetryPolicy<E>: Send + Sync + Debug {
	/// Whether to retry the request based on the given `error`
	fn should_retry(&self, error: &E) -> bool;

	/// Providers may include the `backoff` in the error response directly.
	///
	/// The retry client caps hints to 60 seconds.
	fn backoff_hint(&self, error: &E) -> Option<Duration>;
}

/// [RetryClient] presents as a wrapper around [JsonRpcProvider] that will retry
/// requests based with an exponential backoff and filtering based on [RetryPolicy].
///
/// The `RetryPolicy`, mainly for rate-limiting errors, can be adjusted for specific applications,
/// endpoints. In addition to the `RetryPolicy` errors due to connectivity issues, like timed out
/// connections or responses in range `5xx` can be retried separately.
///
/// # Example
///
/// ```
/// use neo3::neo_clients::{HttpProvider, HttpRateLimitRetryPolicy, RetryClientBuilder};
/// use std::time::Duration;
/// use url::Url;
///
/// async fn demo() {
///     let http = HttpProvider::new(Url::parse("http://localhost:10332").unwrap()).unwrap();
///     let client = RetryClientBuilder::default()
///         .rate_limit_retries(10)
///         .timeout_retries(3)
///         .initial_backoff(Duration::from_millis(500))
///         .build(http, Box::new(HttpRateLimitRetryPolicy::default()));
/// }
/// ```
#[derive(Debug)]
pub struct RetryClient<T>
where
	T: JsonRpcProvider,
	T::Error: Sync + Send + 'static + Debug,
{
	inner: T,
	requests_enqueued: AtomicU32,
	/// The policy to use to determine whether to retry a request due to rate limiting
	policy: Box<dyn RetryPolicy<T::Error>>,
	/// How many connection `TimedOut` should be retried.
	timeout_retries: u32,
	/// How many retries for rate limited responses
	rate_limit_retries: u32,
	/// How long to wait initially
	initial_backoff: Duration,
	/// available CPU per second
	compute_units_per_second: u64,
}

impl<T> RetryClient<T>
where
	T: JsonRpcProvider,
	T::Error: Sync + Send + 'static + Debug,
{
	/// Creates a new `RetryClient` that wraps a client and adds retry and backoff support
	///
	/// # Example
	///
	/// ```
	/// use neo3::neo_clients::{HttpProvider, HttpRateLimitRetryPolicy, RetryClient};
	/// use std::time::Duration;
	/// use url::Url;
	///
	/// async fn demo() {
	///     let http = HttpProvider::new(Url::parse("http://localhost:10332").unwrap()).unwrap();
	///     let backoff_timeout = 3000; // in ms
	///     let max_retries = 10;
	///     let client = RetryClient::new(http, Box::new(HttpRateLimitRetryPolicy::default()), max_retries, backoff_timeout);
	/// }
	/// ```
	pub fn new(
		inner: T,
		policy: Box<dyn RetryPolicy<T::Error>>,
		max_retry: u32,
		// in milliseconds
		initial_backoff: u64,
	) -> Self {
		RetryClientBuilder::default()
			.initial_backoff(Duration::from_millis(initial_backoff))
			.rate_limit_retries(max_retry)
			.build(inner, policy)
	}

	/// Sets the free compute units per second limit.
	///
	/// This is the maximum number of weighted request that can be handled per second by the
	/// endpoint before rate limit kicks in.
	///
	/// This is used to guesstimate how long to wait until to retry again
	pub fn set_compute_units(&mut self, cpus: u64) -> &mut Self {
		self.compute_units_per_second = cpus;
		self
	}
}

/// Builder for a [`RetryClient`]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RetryClientBuilder {
	/// How many connection `TimedOut` should be retried.
	timeout_retries: u32,
	/// How many retries for rate limited responses
	rate_limit_retries: u32,
	/// How long to wait initially
	initial_backoff: Duration,
	/// available CPU per second
	compute_units_per_second: u64,
}

// === impl RetryClientBuilder ===

impl RetryClientBuilder {
	/// Sets the number of retries after a connection times out
	///
	/// **Note:** this will only be used for `request::Error::TimedOut`
	#[must_use]
	pub fn timeout_retries(mut self, timeout_retries: u32) -> Self {
		self.timeout_retries = timeout_retries;
		self
	}

	/// How many retries for rate limited responses
	#[must_use]
	pub fn rate_limit_retries(mut self, rate_limit_retries: u32) -> Self {
		self.rate_limit_retries = rate_limit_retries;
		self
	}

	/// Sets the number of assumed available compute units per second
	///
	/// See also, <https://github.com/alchemyplatform/alchemy-docs/blob/master/documentation/compute-units.md#rate-limits-cups>
	#[must_use]
	pub fn compute_units_per_second(mut self, compute_units_per_second: u64) -> Self {
		self.compute_units_per_second = compute_units_per_second;
		self
	}

	/// Sets the duration to wait initially before retrying.
	///
	/// Subsequent delays double up to a 60-second cap.
	#[must_use]
	pub fn initial_backoff(mut self, initial_backoff: Duration) -> Self {
		self.initial_backoff = initial_backoff;
		self
	}

	/// Creates the `RetryClient` with the configured settings
	pub fn build<T>(self, client: T, policy: Box<dyn RetryPolicy<T::Error>>) -> RetryClient<T>
	where
		T: JsonRpcProvider,
		T::Error: Sync + Send + 'static + Debug,
	{
		let RetryClientBuilder {
			timeout_retries,
			rate_limit_retries,
			initial_backoff,
			compute_units_per_second,
		} = self;
		RetryClient {
			inner: client,
			requests_enqueued: AtomicU32::new(0),
			policy,
			timeout_retries,
			rate_limit_retries,
			initial_backoff,
			compute_units_per_second,
		}
	}
}

// Some sensible defaults
impl Default for RetryClientBuilder {
	fn default() -> Self {
		Self {
			timeout_retries: 3,
			// this should be enough to even out heavy loads
			rate_limit_retries: 10,
			initial_backoff: Duration::from_millis(1000),
			// alchemy max cpus <https://github.com/alchemyplatform/alchemy-docs/blob/master/documentation/compute-units.md#rate-limits-cups>
			compute_units_per_second: 330,
		}
	}
}

/// Error thrown when:
/// 1. Internal client throws an error we do not wish to try to recover from.
/// 2. Params serialization failed.
/// 3. Request timed out i.e. max retries were already made.
#[derive(Error, Debug)]
pub enum RetryClientError {
	/// Internal provider error
	#[error(transparent)]
	ProviderError(ProviderError),
	/// Timeout while making requests
	#[error("request timed out")]
	TimeoutError,
	/// (De)Serialization error
	#[error(transparent)]
	SerdeJson(serde_json::Error),
}

struct EnqueuedRequest<'a> {
	counter: &'a AtomicU32,
}

impl<'a> EnqueuedRequest<'a> {
	fn new(counter: &'a AtomicU32) -> (Self, u64) {
		let ahead = u64::from(counter.fetch_add(1, Ordering::SeqCst));
		(Self { counter }, ahead)
	}
}

impl Drop for EnqueuedRequest<'_> {
	fn drop(&mut self) {
		self.counter.fetch_sub(1, Ordering::SeqCst);
	}
}

fn retry_backoff(initial: Duration, retry_number: u32) -> Duration {
	let exponent = retry_number.saturating_sub(1).min(u32::BITS - 1);
	initial.saturating_mul(1_u32 << exponent).min(MAX_PROVIDER_RETRY_AFTER)
}

fn with_jitter(backoff: Duration) -> Duration {
	let jitter_bound = u64::try_from(backoff.as_millis() / 4).unwrap_or(u64::MAX);
	if jitter_bound == 0 {
		return backoff;
	}

	let jitter = Duration::from_millis(rand::random::<u64>() % jitter_bound);
	backoff.saturating_add(jitter).min(MAX_PROVIDER_RETRY_AFTER)
}

async fn sleep_backoff(backoff: Duration) {
	#[cfg(target_arch = "wasm32")]
	futures_timer::Delay::new(backoff).await;

	#[cfg(not(target_arch = "wasm32"))]
	tokio::time::sleep(backoff).await;
}

impl From<RetryClientError> for ProviderError {
	fn from(src: RetryClientError) -> Self {
		match src {
			RetryClientError::ProviderError(err) => err,
			RetryClientError::SerdeJson(err) => err.into(),
			RetryClientError::TimeoutError => ProviderError::CustomError(src.to_string()),
		}
	}
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(? Send))]
impl<T> JsonRpcProvider for RetryClient<T>
where
	T: JsonRpcProvider + 'static,
	T::Error: Sync + Send + 'static + Debug,
{
	type Error = RetryClientError;

	async fn fetch<A, R>(&self, method: &str, params: A) -> Result<R, Self::Error>
	where
		A: Debug + Serialize + Send + Sync,
		R: DeserializeOwned + Send,
	{
		// Helper type that caches the `params` value across several retries
		// This is necessary because the wrapper provider is supposed to skip he `params` if it's of
		// size 0, see `crate::transports::common::Request`
		enum RetryParams<Params> {
			Value(Params),
			Zst(()),
		}

		let params = if std::mem::size_of::<A>() == 0 {
			RetryParams::Zst(())
		} else {
			let params = serde_json::to_value(params).map_err(RetryClientError::SerdeJson)?;
			RetryParams::Value(params)
		};

		let (_enqueued_request, ahead_in_queue) = EnqueuedRequest::new(&self.requests_enqueued);

		let mut rate_limit_retry_number: u32 = 0;
		let mut timeout_retries: u32 = 0;

		loop {
			let err;

			// hack to not hold `R` across an await in the sleep future and prevent requiring
			// R: Send + Sync
			{
				let resp = match params {
					RetryParams::Value(ref params) => self.inner.fetch(method, params).await,
					RetryParams::Zst(unit) => self.inner.fetch(method, unit).await,
				};
				match resp {
					Ok(ret) => return Ok(ret),
					Err(err_) => err = err_,
				}
			}

			let should_retry = self.policy.should_retry(&err);
			if should_retry {
				rate_limit_retry_number += 1;
				if rate_limit_retry_number > self.rate_limit_retries {
					trace!("request timed out after {} retries", self.rate_limit_retries);
					return Err(RetryClientError::TimeoutError);
				}

				let current_queued_requests =
					u64::from(self.requests_enqueued.load(Ordering::SeqCst));

				// try to extract the requested backoff from the error or compute the next backoff
				// based on retry count
				let mut next_backoff = self
					.policy
					.backoff_hint(&err)
					.map(|hint| hint.min(MAX_PROVIDER_RETRY_AFTER))
					.unwrap_or_else(|| {
						retry_backoff(self.initial_backoff, rate_limit_retry_number)
					});

				// requests are usually weighted and can vary from 10 CU to several 100 CU, cheaper
				// requests are more common some example alchemy weights:
				// - `neo_getStorageAt`: 17
				// - `neo_getBlockByNumber`: 16
				// - `neo_newFilter`: 20
				//
				// (coming from forking mode) assuming here that storage request will be the driver
				// for Rate limits we choose `17` as the average cost of any request
				const AVG_COST: u64 = 17u64;
				let seconds_to_wait_for_compute_budget = compute_unit_offset_in_secs(
					AVG_COST,
					self.compute_units_per_second,
					current_queued_requests,
					ahead_in_queue,
				);
				next_backoff = next_backoff
					.saturating_add(Duration::from_secs(seconds_to_wait_for_compute_budget))
					.min(MAX_PROVIDER_RETRY_AFTER);
				next_backoff = with_jitter(next_backoff);

				trace!("retrying and backing off for {:?}", next_backoff);
				sleep_backoff(next_backoff).await;
			} else {
				let err: ProviderError = err.into();
				if timeout_retries < self.timeout_retries && maybe_connectivity(&err) {
					timeout_retries += 1;
					let next_backoff =
						with_jitter(retry_backoff(self.initial_backoff, timeout_retries));
					trace!(error = %err, delay = ?next_backoff, "retrying due to spurious network");
					sleep_backoff(next_backoff).await;
					continue;
				}

				trace!(error = %err, "should not retry");
				return Err(RetryClientError::ProviderError(err));
			}
		}
	}
}

/// Implements [RetryPolicy] that will retry requests that errored with
/// status code 429 i.e. TOO_MANY_REQUESTS
///
/// Some upstream JSON-RPC gateways also return transient errors like `"header not found"` during
/// load balancing; those are treated as retryable as well.
#[derive(Debug, Default)]
pub struct HttpRateLimitRetryPolicy;

impl RetryPolicy<ClientError> for HttpRateLimitRetryPolicy {
	fn should_retry(&self, error: &ClientError) -> bool {
		match error {
			ClientError::ReqwestError(err) => {
				err.status() == Some(reqwest::StatusCode::TOO_MANY_REQUESTS)
			},
			ClientError::JsonRpcError(err) => err.is_retryable(),
			ClientError::SerdeJson { text, .. } => {
				// some providers send invalid JSON RPC in the error case (no `id:u64`), but the
				// text should be a `JsonRpcError`
				#[derive(Deserialize)]
				struct Resp {
					error: JsonRpcError,
				}

				if let Ok(resp) = serde_json::from_str::<Resp>(text) {
					return resp.error.is_retryable();
				}
				false
			},
		}
	}

	fn backoff_hint(&self, error: &ClientError) -> Option<Duration> {
		match error {
			ClientError::JsonRpcError(error) => error.retry_after(),
			_ => None,
		}
	}
}

/// Calculates an offset in seconds by taking into account the number of currently queued requests,
/// number of requests that were ahead in the queue when the request was first issued, the average
/// cost a weighted request (heuristic), and the number of available compute units per seconds.
///
/// Returns the number of seconds (the unit the remote endpoint measures compute budget) a request
/// is supposed to wait to not get rate limited. The budget per second is
/// `compute_units_per_second`, assuming an average cost of `avg_cost` this allows (in theory)
/// `compute_units_per_second / avg_cost` requests per seconds without getting rate limited.
/// By taking into account the number of concurrent request and the position in queue when the
/// request was first issued and determine the number of seconds a request is supposed to wait, if
/// at all
fn compute_unit_offset_in_secs(
	avg_cost: u64,
	compute_units_per_second: u64,
	current_queued_requests: u64,
	ahead_in_queue: u64,
) -> u64 {
	let request_capacity_per_second =
		compute_units_per_second.checked_div(avg_cost).unwrap_or_default().max(1);
	if current_queued_requests > request_capacity_per_second {
		current_queued_requests
			.min(ahead_in_queue)
			.saturating_div(request_capacity_per_second)
	} else {
		0
	}
}

/// Checks whether the `error` is the result of a connectivity issue, like
/// `request::Error::TimedOut`
fn maybe_connectivity(err: &ProviderError) -> bool {
	matches!(err, ProviderError::HTTPError(_)) && err.is_retryable()
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::neo_clients::HttpProvider;
	use tokio::io::{AsyncReadExt, AsyncWriteExt};
	use url::Url;

	// assumed average cost of a request
	const AVG_COST: u64 = 17u64;
	const COMPUTE_UNITS: u64 = 330u64;

	fn compute_offset(current_queued_requests: u64, ahead_in_queue: u64) -> u64 {
		compute_unit_offset_in_secs(
			AVG_COST,
			COMPUTE_UNITS,
			current_queued_requests,
			ahead_in_queue,
		)
	}

	#[test]
	fn can_measure_unit_offset_single_request() {
		let current_queued_requests = 1;
		let ahead_in_queue = 0;
		let to_wait = compute_offset(current_queued_requests, ahead_in_queue);
		assert_eq!(to_wait, 0);

		let current_queued_requests = 19;
		let ahead_in_queue = 18;
		let to_wait = compute_offset(current_queued_requests, ahead_in_queue);
		assert_eq!(to_wait, 0);
	}

	#[test]
	fn can_measure_unit_offset_1x_over_budget() {
		let current_queued_requests = 20;
		let ahead_in_queue = 19;
		let to_wait = compute_offset(current_queued_requests, ahead_in_queue);
		// need to wait 1 second
		assert_eq!(to_wait, 1);
	}

	#[test]
	fn can_measure_unit_offset_2x_over_budget() {
		let current_queued_requests = 49;
		let ahead_in_queue = 48;
		let to_wait = compute_offset(current_queued_requests, ahead_in_queue);
		// need to wait 1 second
		assert_eq!(to_wait, 2);

		let current_queued_requests = 49;
		let ahead_in_queue = 20;
		let to_wait = compute_offset(current_queued_requests, ahead_in_queue);
		// need to wait 1 second
		assert_eq!(to_wait, 1);
	}

	#[test]
	fn zero_compute_units_do_not_panic() {
		assert_eq!(compute_unit_offset_in_secs(AVG_COST, 0, 1, 0), 0);
		assert_eq!(compute_unit_offset_in_secs(AVG_COST, 0, 3, 2), 2);
	}

	#[test]
	fn exponential_backoff_doubles_and_caps() {
		let initial = Duration::from_millis(500);
		assert_eq!(retry_backoff(initial, 1), Duration::from_millis(500));
		assert_eq!(retry_backoff(initial, 2), Duration::from_secs(1));
		assert_eq!(retry_backoff(initial, 8), Duration::from_secs(60));
		assert_eq!(retry_backoff(initial, u32::MAX), Duration::from_secs(60));
	}

	async fn retry_client_for_responses(
		responses: Vec<String>,
	) -> (RetryClient<HttpProvider>, tokio::task::JoinHandle<()>) {
		retry_client_for_responses_with_backoff(responses, Duration::ZERO).await
	}

	async fn retry_client_for_responses_with_backoff(
		responses: Vec<String>,
		initial_backoff: Duration,
	) -> (RetryClient<HttpProvider>, tokio::task::JoinHandle<()>) {
		let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
		let address = listener.local_addr().unwrap();
		let server = tokio::spawn(async move {
			for response in responses {
				let (mut stream, _) = listener.accept().await.unwrap();
				let mut request = [0_u8; 2048];
				let _ = stream.read(&mut request).await.unwrap();
				stream.write_all(response.as_bytes()).await.unwrap();
			}
		});
		let provider =
			HttpProvider::new(Url::parse(&format!("http://{address}")).unwrap()).unwrap();
		let client = RetryClientBuilder::default()
			.timeout_retries(1)
			.rate_limit_retries(0)
			.initial_backoff(initial_backoff)
			.build(provider, Box::<HttpRateLimitRetryPolicy>::default());
		(client, server)
	}

	#[tokio::test]
	async fn retries_truncated_response_then_succeeds() {
		let truncated = "HTTP/1.1 200 OK\r\nContent-Length: 128\r\nConnection: close\r\n\r\n{\"jsonrpc\":\"2.0\"}".to_string();
		let body = r#"{"jsonrpc":"2.0","id":2,"result":42}"#;
		let success = format!(
			"HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
			body.len()
		);
		let (client, server) = retry_client_for_responses(vec![truncated, success]).await;

		let result: u64 = client.fetch("getblockcount", Vec::<u8>::new()).await.unwrap();

		assert_eq!(result, 42);
		server.await.unwrap();
	}

	#[tokio::test]
	async fn connectivity_retries_wait_before_retrying() {
		let truncated = "HTTP/1.1 200 OK\r\nContent-Length: 128\r\nConnection: close\r\n\r\n{\"jsonrpc\":\"2.0\"}".to_string();
		let body = r#"{"jsonrpc":"2.0","id":2,"result":42}"#;
		let success = format!(
			"HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
			body.len()
		);
		let initial_backoff = Duration::from_millis(25);
		let (client, server) =
			retry_client_for_responses_with_backoff(vec![truncated, success], initial_backoff)
				.await;
		let started = tokio::time::Instant::now();

		let result: u64 = client.fetch("getblockcount", Vec::<u8>::new()).await.unwrap();

		assert_eq!(result, 42);
		assert!(started.elapsed() >= initial_backoff);
		server.await.unwrap();
	}

	#[tokio::test]
	async fn retry_exhaustion_releases_queue_accounting() {
		let body = "rate limited";
		let response = format!(
			"HTTP/1.1 429 Too Many Requests\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
			body.len()
		);
		let (client, server) = retry_client_for_responses(vec![response]).await;

		let result = client.fetch::<_, serde_json::Value>("getblockcount", Vec::<u8>::new()).await;

		assert!(matches!(result, Err(RetryClientError::TimeoutError)));
		assert_eq!(client.requests_enqueued.load(Ordering::SeqCst), 0);
		server.await.unwrap();
	}

	#[test]
	fn can_extract_backoff() {
		let resp = r#"{"rate": {"allowed_rps": 1, "backoff_seconds": 30, "current_rps": 1.1}, "see": "https://example.com/dashboard"}"#;

		let err = ClientError::JsonRpcError(JsonRpcError {
			code: 0,
			message: "daily request count exceeded, request rate limited".to_string(),
			data: Some(serde_json::from_str(resp).unwrap()),
		});
		let backoff = HttpRateLimitRetryPolicy.backoff_hint(&err).unwrap();
		assert_eq!(backoff, Duration::from_secs(30));

		let err = ClientError::JsonRpcError(JsonRpcError {
			code: 0,
			message: "daily request count exceeded, request rate limited".to_string(),
			data: Some(serde_json::Value::String("blocked".to_string())),
		});
		let backoff = HttpRateLimitRetryPolicy.backoff_hint(&err);
		assert!(backoff.is_none());
	}

	#[test]
	fn test_alchemy_ip_rate_limit() {
		let s = "{\"code\":-32016,\"message\":\"Your IP has exceeded its requests per second capacity. To increase your rate limits, please sign up for a free Alchemy account at https://www.alchemy.com/optimism.\"}";
		let err: JsonRpcError = serde_json::from_str(s).unwrap();
		let err = ClientError::JsonRpcError(err);

		let should_retry = HttpRateLimitRetryPolicy.should_retry(&err);
		assert!(should_retry);
	}

	#[test]
	fn test_rate_limit_omitted_id() {
		let s = r#"{"jsonrpc":"2.0","error":{"code":-32016,"message":"Your IP has exceeded its requests per second capacity. To increase your rate limits, please sign up for a free Alchemy account at https://www.alchemy.com/optimism."},"id":null}"#;

		let err = ClientError::SerdeJson {
			err: serde::de::Error::custom("unexpected notification over HTTP transport"),
			text: s.to_string(),
		};

		let should_retry = HttpRateLimitRetryPolicy.should_retry(&err);
		assert!(should_retry);
	}
}
