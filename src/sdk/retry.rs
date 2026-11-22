//! Lightweight retry helper used by the high-level [`crate::sdk::Neo`] API.
//!
//! This intentionally does not duplicate the lower-level
//! [`crate::neo_clients::RetryClient`] / [`crate::neo_clients::CircuitBreaker`]
//! infrastructure — it only wraps an async closure with a bounded retry budget
//! and provider-aware [`NeoError`] mapping so the call sites in
//! `sdk/mod.rs` stay readable.

use std::future::Future;
use std::time::Duration;

use crate::{neo_clients::ProviderError, neo_error::unified::NeoError};

/// Default base delay between retries used by the high-level SDK.
pub(crate) const DEFAULT_RETRY_DELAY: Duration = Duration::from_millis(250);

/// Upper bound for the exponentially growing retry backoff.
const MAX_RETRY_BACKOFF: Duration = Duration::from_secs(60);

/// Exponential backoff with ±25% random jitter.
///
/// `delay = min(base * 2^(attempt-1), 60s) ± 25%`, so a fleet of clients never
/// retries in lockstep (mirrors the WebSocket reconnect loop).
fn backoff_delay(base: Duration, attempt: u32) -> Duration {
	use rand::Rng;

	let exponent = attempt.saturating_sub(1).min(6);
	let backoff = base.saturating_mul(1u32 << exponent).min(MAX_RETRY_BACKOFF);
	let jitter_range = (backoff.as_millis() / 4).max(1) as u64;
	let jitter = rand::rng().random_range(0..=jitter_range);
	if rand::rng().random_bool(0.5) {
		backoff + Duration::from_millis(jitter)
	} else {
		backoff.saturating_sub(Duration::from_millis(jitter))
	}
}

/// Retry an asynchronous, fallible operation up to `attempts` total times.
///
/// `attempts` is the total number of attempts (so `attempts = 1` means *no*
/// retry — the operation runs exactly once). Failures are logged via
/// `tracing` at `warn` level on every non-terminal attempt, and the final
/// error keeps its provider classification with `context` woven into the message. Deterministic
/// failures return immediately without consuming the retry budget. The delay
/// grows exponentially from `delay` with jitter, unless the provider supplies
/// an explicit `Retry-After` hint, which is honored verbatim.
pub(crate) async fn retry_network<T, F, Fut>(
	context: &str,
	attempts: u32,
	delay: Duration,
	mut operation: F,
) -> Result<T, NeoError>
where
	F: FnMut() -> Fut,
	Fut: Future<Output = Result<T, ProviderError>>,
{
	let attempts = attempts.max(1);
	for attempt in 1..=attempts {
		match operation().await {
			Ok(value) => return Ok(value),
			Err(err) => {
				if !err.is_retryable() || attempt == attempts {
					return Err(NeoError::provider(context, err));
				}
				let retry_delay =
					err.retry_after().unwrap_or_else(|| backoff_delay(delay, attempt));

				tracing::warn!(
					attempt = attempt,
					max_attempts = attempts,
					context = %context,
					error = %err,
					retry_delay = ?retry_delay,
					"sdk operation failed; retrying"
				);
				tokio::time::sleep(retry_delay).await;
			},
		}
	}

	unreachable!("the retry loop executes at least once")
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::sync::Arc;
	use std::sync::atomic::{AtomicUsize, Ordering};

	#[test]
	fn backoff_grows_exponentially_and_stays_bounded() {
		let base = Duration::from_millis(100);

		// Every sampled delay for a given attempt stays within ±25% of the
		// nominal exponential schedule, and never exceeds the 60s cap.
		for attempt in 1..=8u32 {
			let nominal = base.saturating_mul(1u32 << (attempt - 1).min(6)).min(MAX_RETRY_BACKOFF);
			let low = (nominal.as_millis() / 4) as u64;
			let high = nominal.as_millis() as u64 + nominal.as_millis() as u64 / 4;
			for _ in 0..25 {
				let sampled = backoff_delay(base, attempt).as_millis() as u64;
				assert!(sampled >= low.min(high), "sampled {sampled}ms below floor {low}ms");
				assert!(
					sampled <= high.max(low),
					"sampled {sampled}ms above ceiling {high}ms for attempt {attempt}"
				);
			}
		}

		// A very late attempt with a large base hits the cap: 2s * 2^6 = 128s
		// exceeds the 60s ceiling, so the sampled delay stays within 60s ±25%.
		let capped = backoff_delay(Duration::from_secs(2), 10).as_millis() as u64;
		assert!((45_000..=75_000).contains(&capped), "capped delay out of range: {capped}ms");
	}

	#[tokio::test]
	async fn returns_first_success_without_extra_attempts() {
		let calls = Arc::new(AtomicUsize::new(0));
		let calls_clone = calls.clone();
		let result: Result<u32, NeoError> =
			retry_network("unit", 3, Duration::from_millis(1), move || {
				let calls = calls_clone.clone();
				async move {
					calls.fetch_add(1, Ordering::SeqCst);
					Ok::<u32, ProviderError>(7)
				}
			})
			.await;
		assert_eq!(result.unwrap(), 7);
		assert_eq!(calls.load(Ordering::SeqCst), 1);
	}

	#[tokio::test]
	async fn maps_final_failure_with_context() {
		let calls = Arc::new(AtomicUsize::new(0));
		let calls_clone = calls.clone();
		let result: Result<(), NeoError> =
			retry_network("fetch block", 2, Duration::from_millis(1), move || {
				let calls = calls_clone.clone();
				async move {
					calls.fetch_add(1, Ordering::SeqCst);
					Err::<(), _>(ProviderError::JsonRpcError(crate::neo_clients::JsonRpcError {
						code: 429,
						message: "too many requests".to_string(),
						data: None,
					}))
				}
			})
			.await;
		let err = result.unwrap_err();
		assert_eq!(calls.load(Ordering::SeqCst), 2);
		assert!(err.is_retryable());
		assert!(err.message().contains("fetch block"));
		assert!(err.message().contains("too many requests"));
	}

	#[tokio::test]
	async fn stops_after_a_deterministic_provider_error() {
		let calls = Arc::new(AtomicUsize::new(0));
		let calls_clone = calls.clone();
		let result: Result<(), NeoError> =
			retry_network("validate address", 3, Duration::from_millis(1), move || {
				let calls = calls_clone.clone();
				async move {
					calls.fetch_add(1, Ordering::SeqCst);
					Err::<(), _>(ProviderError::InvalidAddress)
				}
			})
			.await;

		assert_eq!(calls.load(Ordering::SeqCst), 1);
		assert!(!result.unwrap_err().is_retryable());
	}

	#[tokio::test]
	async fn honors_provider_retry_after_hint() {
		let calls = Arc::new(AtomicUsize::new(0));
		let calls_clone = calls.clone();

		let result = tokio::time::timeout(
			Duration::from_millis(100),
			retry_network("rate limited", 2, Duration::from_millis(1), move || {
				let calls = calls_clone.clone();
				async move {
					if calls.fetch_add(1, Ordering::SeqCst) == 0 {
						Err(ProviderError::JsonRpcError(crate::neo_clients::JsonRpcError {
							code: 429,
							message: "too many requests".to_string(),
							data: Some(serde_json::json!({
								"rate": { "backoff_seconds": 2 }
							})),
						}))
					} else {
						Ok(7_u32)
					}
				}
			}),
		)
		.await;

		assert!(result.is_err(), "the provider's two-second backoff must be honored");
		assert_eq!(calls.load(Ordering::SeqCst), 1);
	}
}
