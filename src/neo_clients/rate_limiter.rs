#![allow(dead_code)]

use crate::neo_error::{Neo3Error, NetworkError};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};
use tokio::time::sleep;

/// Rate limiter for RPC calls to prevent overwhelming the network
pub struct RateLimiter {
	/// Maximum requests per window
	max_requests: u32,
	/// Time window for rate limiting
	window: Duration,
	/// Semaphore for concurrent request limiting
	semaphore: Arc<Semaphore>,
	/// Token bucket for rate limiting
	bucket: Arc<Mutex<TokenBucket>>,
}

/// Token bucket implementation for rate limiting
struct TokenBucket {
	/// Maximum number of tokens
	capacity: u32,
	/// Current number of tokens
	tokens: f64,
	/// Last refill time
	last_refill: Instant,
	/// Refill rate (tokens per second)
	refill_rate: f64,
}

impl RateLimiter {
	/// Create a new rate limiter
	///
	/// # Arguments
	/// * `max_requests` - Maximum requests per window
	/// * `window` - Time window for rate limiting
	/// * `max_concurrent` - Maximum concurrent requests
	pub fn new(max_requests: u32, window: Duration, max_concurrent: usize) -> Self {
		let refill_rate = max_requests as f64 / window.as_secs_f64();

		Self {
			max_requests,
			window,
			semaphore: Arc::new(Semaphore::new(max_concurrent)),
			bucket: Arc::new(Mutex::new(TokenBucket {
				capacity: max_requests,
				tokens: max_requests as f64,
				last_refill: Instant::now(),
				refill_rate,
			})),
		}
	}

	/// Acquire a permit to make a request
	///
	/// This will wait if rate limit is exceeded
	pub async fn acquire(&self) -> Result<RateLimitPermit<'_>, Neo3Error> {
		// First acquire semaphore permit for concurrency limiting
		let _sem_permit = self
			.semaphore
			.acquire()
			.await
			.map_err(|_| Neo3Error::Network(NetworkError::RateLimitExceeded))?;

		// Then check token bucket for rate limiting
		loop {
			let mut bucket = self.bucket.lock().await;

			// Refill tokens based on elapsed time
			let now = Instant::now();
			let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
			bucket.tokens =
				(bucket.tokens + elapsed * bucket.refill_rate).min(bucket.capacity as f64);
			bucket.last_refill = now;

			// Try to consume a token
			if bucket.tokens >= 1.0 {
				bucket.tokens -= 1.0;
				return Ok(RateLimitPermit { _semaphore: _sem_permit });
			}

			// Calculate wait time until next token
			let wait_time = Duration::from_secs_f64(1.0 / bucket.refill_rate);
			drop(bucket); // Release lock while waiting

			sleep(wait_time).await;
		}
	}

	/// Try to acquire a permit without waiting
	///
	/// Returns error if rate limit would be exceeded
	pub async fn try_acquire(&self) -> Result<RateLimitPermit<'_>, Neo3Error> {
		// Try to acquire semaphore permit
		let _sem_permit = self
			.semaphore
			.try_acquire()
			.map_err(|_| Neo3Error::Network(NetworkError::RateLimitExceeded))?;

		// Check token bucket
		let mut bucket = self.bucket.lock().await;

		// Refill tokens
		let now = Instant::now();
		let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
		bucket.tokens = (bucket.tokens + elapsed * bucket.refill_rate).min(bucket.capacity as f64);
		bucket.last_refill = now;

		// Try to consume a token
		if bucket.tokens >= 1.0 {
			bucket.tokens -= 1.0;
			Ok(RateLimitPermit { _semaphore: _sem_permit })
		} else {
			Err(Neo3Error::Network(NetworkError::RateLimitExceeded))
		}
	}

	/// Get current available tokens
	pub async fn available_tokens(&self) -> f64 {
		let mut bucket = self.bucket.lock().await;

		// Refill tokens
		let now = Instant::now();
		let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
		bucket.tokens = (bucket.tokens + elapsed * bucket.refill_rate).min(bucket.capacity as f64);
		bucket.last_refill = now;

		bucket.tokens
	}

	/// Reset the rate limiter
	pub async fn reset(&self) {
		let mut bucket = self.bucket.lock().await;
		bucket.tokens = bucket.capacity as f64;
		bucket.last_refill = Instant::now();
	}
}

/// Permit for rate-limited operation
pub struct RateLimitPermit<'a> {
	_semaphore: tokio::sync::SemaphorePermit<'a>,
}

/// Builder for configuring rate limiter
pub struct RateLimiterBuilder {
	max_requests: u32,
	window: Duration,
	max_concurrent: usize,
}

impl RateLimiterBuilder {
	/// Create a new builder with defaults
	pub fn new() -> Self {
		Self { max_requests: 100, window: Duration::from_secs(1), max_concurrent: 10 }
	}

	/// Set maximum requests per window
	pub fn max_requests(mut self, max: u32) -> Self {
		self.max_requests = max;
		self
	}

	/// Set time window
	pub fn window(mut self, window: Duration) -> Self {
		self.window = window;
		self
	}

	/// Set maximum concurrent requests
	pub fn max_concurrent(mut self, max: usize) -> Self {
		self.max_concurrent = max;
		self
	}

	/// Build the rate limiter
	pub fn build(self) -> RateLimiter {
		RateLimiter::new(self.max_requests, self.window, self.max_concurrent)
	}
}

impl Default for RateLimiterBuilder {
	fn default() -> Self {
		Self::new()
	}
}

/// Rate limiter presets for common scenarios
pub struct RateLimiterPresets;

impl RateLimiterPresets {
	/// Conservative rate limiting for public APIs
	pub fn conservative() -> RateLimiter {
		RateLimiterBuilder::new()
			.max_requests(10)
			.window(Duration::from_secs(1))
			.max_concurrent(5)
			.build()
	}

	/// Standard rate limiting for authenticated APIs
	pub fn standard() -> RateLimiter {
		RateLimiterBuilder::new()
			.max_requests(100)
			.window(Duration::from_secs(1))
			.max_concurrent(20)
			.build()
	}

	/// Aggressive rate limiting for internal APIs
	pub fn aggressive() -> RateLimiter {
		RateLimiterBuilder::new()
			.max_requests(1000)
			.window(Duration::from_secs(1))
			.max_concurrent(100)
			.build()
	}

	/// Custom rate limiting for specific needs
	pub fn custom(requests_per_second: u32, max_concurrent: usize) -> RateLimiter {
		RateLimiterBuilder::new()
			.max_requests(requests_per_second)
			.window(Duration::from_secs(1))
			.max_concurrent(max_concurrent)
			.build()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_rate_limiter_basic() {
		let limiter = RateLimiter::new(5, Duration::from_secs(1), 2);

		// Should allow 5 requests
		for _ in 0..5 {
			assert!(limiter.try_acquire().await.is_ok());
		}

		// 6th request should fail
		assert!(limiter.try_acquire().await.is_err());
	}

	#[tokio::test]
	async fn test_rate_limiter_refill() {
		let limiter = RateLimiter::new(2, Duration::from_secs(1), 10);

		// Use all tokens
		assert!(limiter.try_acquire().await.is_ok());
		assert!(limiter.try_acquire().await.is_ok());
		assert!(limiter.try_acquire().await.is_err());

		// Wait for refill
		sleep(Duration::from_millis(600)).await;

		// Should have ~1 token refilled
		assert!(limiter.try_acquire().await.is_ok());
	}

	#[tokio::test]
	async fn test_concurrent_limiting() {
		let limiter = Arc::new(RateLimiter::new(100, Duration::from_secs(1), 2));

		// Start 3 concurrent tasks
		let mut handles = vec![];
		for _ in 0..3 {
			let limiter = limiter.clone();
			handles.push(tokio::spawn(async move {
				// Acquire and immediately drop permit
				limiter.acquire().await.is_ok()
			}));
		}

		// Wait for all tasks to complete
		let results = futures_util::future::join_all(handles).await;
		// All tasks should complete successfully, with at most 2 running concurrently
		assert!(results.into_iter().all(|r| r.unwrap_or(false)));

		// Tokens should have been consumed
		let tokens = limiter.available_tokens().await;
		assert!(tokens < 100.0); // Some tokens consumed
	}

	#[test]
	fn test_builder() {
		let limiter = RateLimiterBuilder::new()
			.max_requests(50)
			.window(Duration::from_secs(2))
			.max_concurrent(15)
			.build();

		assert_eq!(limiter.max_requests, 50);
		assert_eq!(limiter.window, Duration::from_secs(2));
	}
}
