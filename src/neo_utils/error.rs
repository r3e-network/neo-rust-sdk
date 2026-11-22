//! Error handling utilities for the Neo N3 SDK.

/// Converts an Option to a Result with a custom error message.
///
/// # Examples
///
/// ```
/// use neo3::prelude::*;
/// use neo3::neo_utils::error::option_to_result;
///
/// let value: Option<u32> = Some(42);
/// let result = option_to_result(value, || NeoError::validation("value", None::<&str>, "Value is None"));
/// assert_eq!(result.unwrap(), 42);
/// ```
pub fn option_to_result<T, E, F>(option: Option<T>, err_fn: F) -> Result<T, E>
where
	F: FnOnce() -> E,
{
	option.ok_or_else(err_fn)
}

/// Adds context to an error.
///
/// # Examples
///
/// ```rust
/// use neo3::prelude::*;
/// use neo3::neo_utils::error::with_context;
///
/// let result: Result<u32, NeoError> = Err(NeoError::validation("value", None::<&str>, "Original error"));
/// let result_with_context = with_context(result, || "Additional context", |msg| NeoError::validation("value", None::<&str>, &msg));
/// assert!(result_with_context.is_err());
/// ```
pub fn with_context<T, E, C, F, G>(
	result: Result<T, E>,
	context_fn: F,
	error_mapper: G,
) -> Result<T, E>
where
	E: std::fmt::Display,
	F: FnOnce() -> C,
	C: std::fmt::Display,
	G: FnOnce(String) -> E,
{
	result.map_err(|err| {
		let context = context_fn();
		error_mapper(format!("{}: {}", context, err))
	})
}

/// Converts a Result to an Option, logging the error if present.
///
/// # Examples
///
/// ```
/// use neo3::prelude::*;
/// use neo3::neo_utils::error::result_to_option;
///
/// let result: Result<u32, NeoError> = Ok(42);
/// let option = result_to_option(result);
/// assert_eq!(option, Some(42));
/// ```
pub fn result_to_option<T, E: std::fmt::Display>(result: Result<T, E>) -> Option<T> {
	match result {
		Ok(value) => Some(value),
		Err(err) => {
			tracing::warn!(error = %err, "Converting error result to None");
			None
		},
	}
}

/// Attempts to execute a fallible operation multiple times before giving up.
///
/// # Examples
///
/// ```
/// use neo3::prelude::*;
/// use neo3::neo_utils::error::retry;
/// use std::time::Duration;
///
/// async fn fallible_operation() -> Result<u32, NeoError> {
///     // Some operation that might fail
///     Ok(42)
/// }
///
/// # async fn example() -> Result<(), NeoError> {
/// let result = retry(
///     || async { fallible_operation().await },
///     3,
///     Duration::from_millis(100)
/// ).await;
/// # Ok(())
/// # }
/// ```
pub async fn retry<T, E, F, Fut>(
	operation: F,
	max_attempts: usize,
	delay: std::time::Duration,
) -> Result<T, E>
where
	F: Fn() -> Fut,
	Fut: std::future::Future<Output = Result<T, E>>,
	E: std::fmt::Display,
{
	if max_attempts == 0 {
		return operation().await;
	}

	let mut attempts = 0;
	let mut last_error = None;

	while attempts < max_attempts {
		match operation().await {
			Ok(value) => return Ok(value),
			Err(err) => {
				attempts += 1;
				if attempts < max_attempts {
					tracing::warn!(attempt = attempts, error = %err, "Attempt failed; retrying");
					tokio::time::sleep(delay).await;
				}
				last_error = Some(err);
			},
		}
	}

	match last_error {
		Some(err) => Err(err),
		None => operation().await,
	}
}

#[cfg(test)]
mod tests {
	use super::retry;
	use std::{
		sync::{
			Arc,
			atomic::{AtomicUsize, Ordering},
		},
		time::Duration,
	};

	#[tokio::test]
	async fn test_retry_zero_attempts_runs_once_ok() {
		let calls = Arc::new(AtomicUsize::new(0));
		let calls_for_closure = calls.clone();

		let result: Result<u32, &'static str> = retry(
			move || {
				let calls = calls_for_closure.clone();
				async move {
					calls.fetch_add(1, Ordering::SeqCst);
					Ok(42)
				}
			},
			0,
			Duration::from_millis(1),
		)
		.await;

		assert_eq!(result.unwrap(), 42);
		assert_eq!(calls.load(Ordering::SeqCst), 1);
	}

	#[tokio::test]
	async fn test_retry_zero_attempts_runs_once_err() {
		let result: Result<(), &'static str> =
			retry(|| async { Err("fail") }, 0, Duration::from_millis(1)).await;
		assert_eq!(result.unwrap_err(), "fail");
	}
}
