#![allow(unexpected_cfgs, dead_code)]

use crate::errors::CliError;
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

/// Retry configuration for network operations
#[derive(Debug, Clone)]
pub struct RetryConfig {
	pub max_attempts: u32,
	pub initial_delay: Duration,
	pub max_delay: Duration,
	pub exponential_base: f64,
	pub jitter: bool,
}

impl Default for RetryConfig {
	fn default() -> Self {
		Self {
			max_attempts: 3,
			initial_delay: Duration::from_millis(500),
			max_delay: Duration::from_secs(30),
			exponential_base: 2.0,
			jitter: true,
		}
	}
}

impl RetryConfig {
	/// Create a configuration for critical operations
	pub fn critical() -> Self {
		Self {
			max_attempts: 5,
			initial_delay: Duration::from_millis(1000),
			max_delay: Duration::from_secs(60),
			exponential_base: 2.0,
			jitter: true,
		}
	}

	/// Create a configuration for fast retry
	pub fn fast() -> Self {
		Self {
			max_attempts: 3,
			initial_delay: Duration::from_millis(100),
			max_delay: Duration::from_secs(5),
			exponential_base: 1.5,
			jitter: false,
		}
	}
}

/// Retry mechanism with exponential backoff
pub struct RetryHandler {
	config: RetryConfig,
}

impl RetryHandler {
	pub fn new(config: RetryConfig) -> Self {
		Self { config }
	}

	/// Execute an async operation with retry logic
	pub async fn retry<F, Fut, T>(&self, mut operation: F) -> Result<T, CliError>
	where
		F: FnMut() -> Fut,
		Fut: Future<Output = Result<T, CliError>>,
	{
		let mut attempt = 0;
		let mut last_error: Option<String> = None;

		while attempt < self.config.max_attempts {
			match operation().await {
				Ok(result) => return Ok(result),
				Err(e) => {
					attempt += 1;
					last_error = Some(e.to_string());

					// Check if error is retryable
					if !self
						.is_retryable(&CliError::Network(last_error.clone().unwrap_or_default()))
					{
						return Err(CliError::Network(last_error.unwrap_or_default()));
					}

					if attempt < self.config.max_attempts {
						let delay = self.calculate_delay(attempt);
						log::warn!(
							"Operation failed (attempt {}/{}), retrying in {:?}: {}",
							attempt,
							self.config.max_attempts,
							delay,
							e
						);
						sleep(delay).await;
					}
				},
			}
		}

		Err(CliError::Network(
			last_error.unwrap_or_else(|| "Max retry attempts exceeded".to_string()),
		))
	}

	/// Calculate delay with exponential backoff and optional jitter
	fn calculate_delay(&self, attempt: u32) -> Duration {
		let base_delay = self.config.initial_delay.as_millis() as f64;
		let exponential = base_delay * self.config.exponential_base.powi(attempt as i32 - 1);

		let mut delay_ms = exponential.min(self.config.max_delay.as_millis() as f64);

		if self.config.jitter {
			use rand::Rng;
			let jitter = rand::thread_rng().gen_range(0.8..1.2);
			delay_ms *= jitter;
		}

		Duration::from_millis(delay_ms as u64)
	}

	/// Determine if an error is retryable
	fn is_retryable(&self, error: &CliError) -> bool {
		match error {
			CliError::Network(_) => true,
			CliError::Timeout(_) => true,
			CliError::RpcError(_) => true,
			CliError::Config(_) => false,
			CliError::InvalidInput(_) => false,
			CliError::Wallet(_) => false,
			CliError::Security(_) => false,
			_ => false,
		}
	}
}

/// Error recovery strategies
pub enum RecoveryStrategy {
	Retry(RetryConfig),
	Fallback(String),
	CircuitBreaker { threshold: u32, timeout: Duration },
	Graceful,
}

/// Enhanced error handler with recovery mechanisms
pub struct ErrorHandler {
	retry_handler: RetryHandler,
	circuit_breakers:
		std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, CircuitBreaker>>>,
}

impl ErrorHandler {
	pub fn new() -> Self {
		Self {
			retry_handler: RetryHandler::new(RetryConfig::default()),
			circuit_breakers: std::sync::Arc::new(std::sync::Mutex::new(
				std::collections::HashMap::new(),
			)),
		}
	}

	/// Handle an error with appropriate recovery strategy
	pub async fn handle_with_recovery<F, Fut, T>(
		&self,
		operation_name: &str,
		strategy: RecoveryStrategy,
		operation: F,
	) -> Result<T, CliError>
	where
		F: Fn() -> Fut,
		Fut: Future<Output = Result<T, CliError>>,
	{
		match strategy {
			RecoveryStrategy::Retry(config) => {
				let handler = RetryHandler::new(config);
				handler.retry(operation).await
			},
			RecoveryStrategy::Fallback(fallback_msg) => match operation().await {
				Ok(result) => Ok(result),
				Err(e) => {
					log::warn!("Operation {} failed, using fallback: {}", operation_name, e);
					Err(CliError::Other(fallback_msg))
				},
			},
			RecoveryStrategy::CircuitBreaker { threshold, timeout } => {
				self.with_circuit_breaker(operation_name, threshold, timeout, operation).await
			},
			RecoveryStrategy::Graceful => match operation().await {
				Ok(result) => Ok(result),
				Err(e) => {
					log::error!("Operation {} failed gracefully: {}", operation_name, e);
					Err(e)
				},
			},
		}
	}

	/// Execute with circuit breaker pattern
	async fn with_circuit_breaker<F, Fut, T>(
		&self,
		name: &str,
		threshold: u32,
		timeout: Duration,
		operation: F,
	) -> Result<T, CliError>
	where
		F: Fn() -> Fut,
		Fut: Future<Output = Result<T, CliError>>,
	{
		{
			let mut breakers = self.circuit_breakers.lock().unwrap();
			let breaker = breakers
				.entry(name.to_string())
				.or_insert_with(|| CircuitBreaker::new(threshold, timeout));

			if !breaker.is_closed() {
				return Err(CliError::Network(format!("Circuit breaker {} is open", name)));
			}
		}

		let result = operation().await;

		let mut breakers = self.circuit_breakers.lock().unwrap();
		let breaker =
			breakers.entry(name.to_string()).or_insert_with(|| CircuitBreaker::new(threshold, timeout));

		match result {
			Ok(result) => {
				breaker.on_success();
				Ok(result)
			},
			Err(e) => {
				breaker.on_failure();
				Err(e)
			},
		}
	}
}

/// Circuit breaker implementation
#[derive(Debug)]
struct CircuitBreaker {
	state: CircuitState,
	failure_count: u32,
	threshold: u32,
	timeout: Duration,
	last_failure_time: Option<std::time::Instant>,
}

#[derive(Debug, PartialEq)]
enum CircuitState {
	Closed,
	Open,
	HalfOpen,
}

impl CircuitBreaker {
	fn new(threshold: u32, timeout: Duration) -> Self {
		Self {
			state: CircuitState::Closed,
			failure_count: 0,
			threshold,
			timeout,
			last_failure_time: None,
		}
	}

	fn is_closed(&mut self) -> bool {
		match self.state {
			CircuitState::Closed => true,
			CircuitState::HalfOpen => true,
			CircuitState::Open => {
				if let Some(last_failure) = self.last_failure_time {
					if last_failure.elapsed() > self.timeout {
						self.state = CircuitState::HalfOpen;
						true
					} else {
						false
					}
				} else {
					false
				}
			},
		}
	}

	fn on_success(&mut self) {
		self.failure_count = 0;
		self.state = CircuitState::Closed;
		self.last_failure_time = None;
	}

	fn on_failure(&mut self) {
		self.failure_count += 1;
		self.last_failure_time = Some(std::time::Instant::now());

		if self.failure_count >= self.threshold {
			self.state = CircuitState::Open;
		}
	}
}

/// Comprehensive error context for better debugging
#[derive(Debug, Clone)]
pub struct ErrorContext {
	pub operation: String,
	pub timestamp: chrono::DateTime<chrono::Utc>,
	pub context: std::collections::HashMap<String, String>,
	pub stack_trace: Option<String>,
}

impl ErrorContext {
	pub fn new(operation: &str) -> Self {
		Self {
			operation: operation.to_string(),
			timestamp: chrono::Utc::now(),
			context: std::collections::HashMap::new(),
			stack_trace: None,
		}
	}

	pub fn with_context(mut self, key: &str, value: &str) -> Self {
		self.context.insert(key.to_string(), value.to_string());
		self
	}

	pub fn with_stack_trace(mut self) -> Self {
		self.stack_trace = Some(std::backtrace::Backtrace::capture().to_string());
		self
	}
}

/// Error reporting for monitoring
pub struct ErrorReporter {
	errors: std::sync::Arc<std::sync::Mutex<Vec<ErrorContext>>>,
}

impl ErrorReporter {
	pub fn new() -> Self {
		Self { errors: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())) }
	}

	pub fn report(&self, error: ErrorContext) {
		let mut errors = self.errors.lock().unwrap();
		errors.push(error.clone());

		// Log the error
		log::error!("Error in {}: {:?}", error.operation, error.context);

		// In production, send to monitoring service
		#[cfg(feature = "monitoring")]
		self.send_to_monitoring(&error);
	}

	#[cfg(feature = "monitoring")]
	fn send_to_monitoring(&self, error: &ErrorContext) {
		// Send to Sentry, DataDog, etc.
	}

	pub fn get_recent_errors(&self, count: usize) -> Vec<ErrorContext> {
		let errors = self.errors.lock().unwrap();
		errors.iter().rev().take(count).cloned().collect()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_retry_handler() {
		let handler = RetryHandler::new(RetryConfig::fast());
		let attempts = std::sync::Arc::new(std::sync::Mutex::new(0));

		let result = handler
			.retry(|| {
				let attempts = attempts.clone();
				async move {
					let mut guard = attempts.lock().unwrap();
					*guard += 1;
					if *guard < 3 {
						Err(CliError::Network("Test error".to_string()))
					} else {
						Ok("Success")
					}
				}
			})
			.await;

		assert!(result.is_ok());
		assert_eq!(result.unwrap(), "Success");
		assert_eq!(*attempts.lock().unwrap(), 3);
	}

	#[test]
	fn test_circuit_breaker() {
		let mut breaker = CircuitBreaker::new(3, Duration::from_secs(10));

		assert!(breaker.is_closed());

		breaker.on_failure();
		breaker.on_failure();
		breaker.on_failure();

		assert!(!breaker.is_closed());
		assert_eq!(breaker.state, CircuitState::Open);
	}
}
