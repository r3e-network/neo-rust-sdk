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
/// let result = option_to_result(value, || NeoError::Generic { message: "Value is None".to_string() });
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
/// ```ignore
/// use neo3::prelude::*;
/// use neo3::neo_utils::error::with_context;
///
/// let result: Result<u32, NeoError> = Err(NeoError::Generic { message: "Original error".to_string() });
/// let result_with_context = with_context(result, || "Additional context", |e| e);
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
			eprintln!("Error: {}", err);
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
	let mut attempts = 0;
	let mut last_error = None;

	while attempts < max_attempts {
		match operation().await {
			Ok(value) => return Ok(value),
			Err(err) => {
				attempts += 1;
				if attempts < max_attempts {
					eprintln!("Attempt {} failed: {}. Retrying...", attempts, err);
					tokio::time::sleep(delay).await;
				}
				last_error = Some(err);
			},
		}
	}

	Err(last_error.expect("Should have at least one error after failed attempts"))
}
