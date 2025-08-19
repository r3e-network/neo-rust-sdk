use crate::neo_error::{Neo3Error, Neo3Result};
use std::{
	sync::{
		atomic::{AtomicU32, Ordering},
		Arc,
	},
	time::{Duration, Instant},
};
use tokio::sync::RwLock;

/// Circuit breaker states
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
	/// Circuit is closed, requests flow normally
	Closed,
	/// Circuit is open, requests are rejected immediately
	Open,
	/// Circuit is half-open, testing if service has recovered
	HalfOpen,
}

impl Default for CircuitState {
	fn default() -> Self {
		CircuitState::Closed
	}
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
	/// Number of failures before opening the circuit
	pub failure_threshold: u32,
	/// Time to wait before transitioning from Open to HalfOpen
	pub timeout: Duration,
	/// Number of successful requests needed to close the circuit from HalfOpen
	pub success_threshold: u32,
	/// Time window for counting failures
	pub failure_window: Duration,
	/// Maximum number of requests allowed in HalfOpen state
	pub half_open_max_requests: u32,
}

impl Default for CircuitBreakerConfig {
	fn default() -> Self {
		Self {
			failure_threshold: 5,
			timeout: Duration::from_secs(60),
			success_threshold: 3,
			failure_window: Duration::from_secs(60),
			half_open_max_requests: 3,
		}
	}
}

/// Circuit breaker statistics
#[derive(Debug, Default)]
pub struct CircuitBreakerStats {
	pub total_requests: u64,
	pub successful_requests: u64,
	pub failed_requests: u64,
	pub rejected_requests: u64,
	pub state_transitions: u64,
	pub current_state: CircuitState,
	pub last_failure_time: Option<Instant>,
	pub last_success_time: Option<Instant>,
}

/// Circuit breaker implementation for protecting against cascading failures
pub struct CircuitBreaker {
	config: CircuitBreakerConfig,
	state: Arc<RwLock<CircuitState>>,
	failure_count: AtomicU32,
	success_count: AtomicU32,
	half_open_requests: AtomicU32,
	last_failure_time: Arc<RwLock<Option<Instant>>>,
	last_success_time: Arc<RwLock<Option<Instant>>>,
	stats: Arc<RwLock<CircuitBreakerStats>>,
}

impl CircuitBreaker {
	/// Create a new circuit breaker with the given configuration
	pub fn new(config: CircuitBreakerConfig) -> Self {
		Self {
			config,
			state: Arc::new(RwLock::new(CircuitState::Closed)),
			failure_count: AtomicU32::new(0),
			success_count: AtomicU32::new(0),
			half_open_requests: AtomicU32::new(0),
			last_failure_time: Arc::new(RwLock::new(None)),
			last_success_time: Arc::new(RwLock::new(None)),
			stats: Arc::new(RwLock::new(CircuitBreakerStats::default())),
		}
	}

	/// Execute a request through the circuit breaker
	pub async fn call<F, T>(&self, operation: F) -> Neo3Result<T>
	where
		F: std::future::Future<Output = Neo3Result<T>>,
	{
		// Update total requests
		{
			let mut stats = self.stats.write().await;
			stats.total_requests += 1;
		}

		// Check if we should allow the request
		if !self.should_allow_request().await {
			let mut stats = self.stats.write().await;
			stats.rejected_requests += 1;
			return Err(Neo3Error::Network(crate::neo_error::NetworkError::RateLimitExceeded));
		}

		// Execute the operation
		match operation.await {
			Ok(result) => {
				self.on_success().await;
				Ok(result)
			},
			Err(error) => {
				self.on_failure().await;
				Err(error)
			},
		}
	}

	/// Check if a request should be allowed based on current state
	async fn should_allow_request(&self) -> bool {
		let state = self.state.read().await;
		match *state {
			CircuitState::Closed => true,
			CircuitState::Open => {
				// Check if timeout has elapsed
				if let Some(last_failure) = *self.last_failure_time.read().await {
					if last_failure.elapsed() >= self.config.timeout {
						drop(state);
						self.transition_to_half_open().await;
						true
					} else {
						false
					}
				} else {
					false
				}
			},
			CircuitState::HalfOpen => {
				// Allow limited requests in half-open state
				let current_requests = self.half_open_requests.load(Ordering::Relaxed);
				current_requests < self.config.half_open_max_requests
			},
		}
	}

	/// Handle successful request
	async fn on_success(&self) {
		let mut stats = self.stats.write().await;
		stats.successful_requests += 1;
		stats.last_success_time = Some(Instant::now());
		drop(stats);

		*self.last_success_time.write().await = Some(Instant::now());

		let state = self.state.read().await;
		match *state {
			CircuitState::Closed => {
				// Reset failure count on success
				self.failure_count.store(0, Ordering::Relaxed);
			},
			CircuitState::HalfOpen => {
				let success_count = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
				if success_count >= self.config.success_threshold {
					drop(state);
					self.transition_to_closed().await;
				}
			},
			CircuitState::Open => {
				// This shouldn't happen, but reset if it does
				drop(state);
				self.transition_to_closed().await;
			},
		}
	}

	/// Handle failed request
	async fn on_failure(&self) {
		let mut stats = self.stats.write().await;
		stats.failed_requests += 1;
		stats.last_failure_time = Some(Instant::now());
		drop(stats);

		*self.last_failure_time.write().await = Some(Instant::now());

		let state = self.state.read().await;
		match *state {
			CircuitState::Closed => {
				let failure_count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
				if failure_count >= self.config.failure_threshold {
					drop(state);
					self.transition_to_open().await;
				}
			},
			CircuitState::HalfOpen => {
				// Any failure in half-open state transitions back to open
				drop(state);
				self.transition_to_open().await;
			},
			CircuitState::Open => {
				// Already open, nothing to do
			},
		}
	}

	/// Transition to closed state
	async fn transition_to_closed(&self) {
		let mut state = self.state.write().await;
		if *state != CircuitState::Closed {
			*state = CircuitState::Closed;
			self.failure_count.store(0, Ordering::Relaxed);
			self.success_count.store(0, Ordering::Relaxed);
			self.half_open_requests.store(0, Ordering::Relaxed);

			let mut stats = self.stats.write().await;
			stats.state_transitions += 1;
			stats.current_state = CircuitState::Closed;
		}
	}

	/// Transition to open state
	async fn transition_to_open(&self) {
		let mut state = self.state.write().await;
		if *state != CircuitState::Open {
			*state = CircuitState::Open;
			self.success_count.store(0, Ordering::Relaxed);
			self.half_open_requests.store(0, Ordering::Relaxed);

			let mut stats = self.stats.write().await;
			stats.state_transitions += 1;
			stats.current_state = CircuitState::Open;
		}
	}

	/// Transition to half-open state
	async fn transition_to_half_open(&self) {
		let mut state = self.state.write().await;
		if *state != CircuitState::HalfOpen {
			*state = CircuitState::HalfOpen;
			self.success_count.store(0, Ordering::Relaxed);
			self.half_open_requests.store(0, Ordering::Relaxed);

			let mut stats = self.stats.write().await;
			stats.state_transitions += 1;
			stats.current_state = CircuitState::HalfOpen;
		}
	}

	/// Get current circuit breaker state
	pub async fn get_state(&self) -> CircuitState {
		let state = self.state.read().await;
		state.clone()
	}

	/// Get circuit breaker statistics
	pub async fn get_stats(&self) -> CircuitBreakerStats {
		let stats = self.stats.read().await;
		CircuitBreakerStats {
			total_requests: stats.total_requests,
			successful_requests: stats.successful_requests,
			failed_requests: stats.failed_requests,
			rejected_requests: stats.rejected_requests,
			state_transitions: stats.state_transitions,
			current_state: stats.current_state.clone(),
			last_failure_time: stats.last_failure_time,
			last_success_time: stats.last_success_time,
		}
	}

	/// Reset the circuit breaker to closed state
	pub async fn reset(&self) {
		self.transition_to_closed().await;
		*self.last_failure_time.write().await = None;
		*self.last_success_time.write().await = None;

		let mut stats = self.stats.write().await;
		*stats = CircuitBreakerStats::default();
	}

	/// Force the circuit breaker to open state
	pub async fn force_open(&self) {
		self.transition_to_open().await;
	}

	/// Get failure rate (failures / total requests)
	pub async fn get_failure_rate(&self) -> f64 {
		let stats = self.stats.read().await;
		if stats.total_requests == 0 {
			0.0
		} else {
			stats.failed_requests as f64 / stats.total_requests as f64
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use tokio::time::{sleep, Duration};

	#[tokio::test]
	async fn test_circuit_breaker_closed_state() {
		let config = CircuitBreakerConfig { failure_threshold: 3, ..Default::default() };
		let cb = CircuitBreaker::new(config);

		// Successful requests should keep circuit closed
		for _ in 0..5 {
			let result = cb.call(async { Ok::<(), Neo3Error>(()) }).await;
			assert!(result.is_ok());
		}

		assert_eq!(cb.get_state().await, CircuitState::Closed);
	}

	#[tokio::test]
	async fn test_circuit_breaker_opens_on_failures() {
		let config = CircuitBreakerConfig { failure_threshold: 3, ..Default::default() };
		let cb = CircuitBreaker::new(config);

		// Generate failures to open circuit
		for _ in 0..3 {
			let result = cb
				.call(async {
					Err::<(), Neo3Error>(Neo3Error::Network(
						crate::neo_error::NetworkError::ConnectionFailed("test".to_string()),
					))
				})
				.await;
			assert!(result.is_err());
		}

		assert_eq!(cb.get_state().await, CircuitState::Open);
	}

	#[tokio::test]
	async fn test_circuit_breaker_half_open_transition() {
		let config = CircuitBreakerConfig {
			failure_threshold: 2,
			timeout: Duration::from_millis(100),
			..Default::default()
		};
		let cb = CircuitBreaker::new(config);

		// Open the circuit
		for _ in 0..2 {
			let _ = cb
				.call(async {
					Err::<(), Neo3Error>(Neo3Error::Network(
						crate::neo_error::NetworkError::ConnectionFailed("test".to_string()),
					))
				})
				.await;
		}
		assert_eq!(cb.get_state().await, CircuitState::Open);

		// Wait for timeout
		sleep(Duration::from_millis(150)).await;

		// Next request should transition to half-open
		let result = cb.call(async { Ok::<(), Neo3Error>(()) }).await;
		assert!(result.is_ok());
		assert_eq!(cb.get_state().await, CircuitState::HalfOpen);
	}

	#[tokio::test]
	async fn test_circuit_breaker_stats() {
		let cb = CircuitBreaker::new(CircuitBreakerConfig::default());

		// Make some requests
		let _ = cb.call(async { Ok::<(), Neo3Error>(()) }).await;
		let _ = cb
			.call(async {
				Err::<(), Neo3Error>(Neo3Error::Network(
					crate::neo_error::NetworkError::ConnectionFailed("test".to_string()),
				))
			})
			.await;

		let stats = cb.get_stats().await;
		assert_eq!(stats.total_requests, 2);
		assert_eq!(stats.successful_requests, 1);
		assert_eq!(stats.failed_requests, 1);
	}
}
