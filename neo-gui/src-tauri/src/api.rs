//! API Response Types
//!
//! This module defines the standard API response format used throughout the application.

use serde::{Deserialize, Serialize};

/// Standard API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
	/// Whether the operation was successful
	pub success: bool,
	/// The response data (if successful)
	pub data: Option<T>,
	/// Error message (if failed)
	pub error: Option<String>,
	/// Optional additional metadata
	pub metadata: Option<serde_json::Value>,
}

impl<T> ApiResponse<T> {
	/// Create a successful response with data
	pub fn success(data: T) -> Self {
		Self { success: true, data: Some(data), error: None, metadata: None }
	}

	/// Create a successful response with data and metadata
	pub fn success_with_metadata(data: T, metadata: serde_json::Value) -> Self {
		Self { success: true, data: Some(data), error: None, metadata: Some(metadata) }
	}

	/// Create an error response
	pub fn error(error: String) -> Self {
		Self { success: false, data: None, error: Some(error), metadata: None }
	}

	/// Create an error response with metadata
	pub fn error_with_metadata(error: String, metadata: serde_json::Value) -> Self {
		Self { success: false, data: None, error: Some(error), metadata: Some(metadata) }
	}

	/// Check if the response is successful
	pub fn is_success(&self) -> bool {
		self.success
	}

	/// Check if the response is an error
	pub fn is_error(&self) -> bool {
		!self.success
	}

	/// Get the data if successful, otherwise return None
	pub fn data(&self) -> Option<&T> {
		self.data.as_ref()
	}

	/// Get the error message if failed, otherwise return None
	pub fn get_error(&self) -> Option<&String> {
		self.error.as_ref()
	}

	/// Get the metadata if present
	pub fn metadata(&self) -> Option<&serde_json::Value> {
		self.metadata.as_ref()
	}

	/// Convert to Result type
	pub fn into_result(self) -> Result<T, String> {
		if self.success {
			self.data.ok_or_else(|| "Success response missing data".to_string())
		} else {
			Err(self.error.unwrap_or_else(|| "Unknown error".to_string()))
		}
	}
}

impl<T> From<Result<T, String>> for ApiResponse<T> {
	fn from(result: Result<T, String>) -> Self {
		match result {
			Ok(data) => Self::success(data),
			Err(error) => Self::error(error),
		}
	}
}

impl<T> From<Result<T, Box<dyn std::error::Error>>> for ApiResponse<T> {
	fn from(result: Result<T, Box<dyn std::error::Error>>) -> Self {
		match result {
			Ok(data) => Self::success(data),
			Err(error) => Self::error(error.to_string()),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_success_response() {
		let response = ApiResponse::success("test_data".to_string());
		assert!(response.is_success());
		assert!(!response.is_error());
		assert_eq!(response.data(), Some(&"test_data".to_string()));
		assert_eq!(response.get_error(), None);
	}

	#[test]
	fn test_error_response() {
		let response: ApiResponse<String> = ApiResponse::error("test_error".to_string());
		assert!(!response.is_success());
		assert!(response.is_error());
		assert_eq!(response.data(), None);
		assert_eq!(response.get_error(), Some(&"test_error".to_string()));
	}

	#[test]
	fn test_success_with_metadata() {
		let metadata = serde_json::json!({"timestamp": "2023-01-01T00:00:00Z"});
		let response =
			ApiResponse::success_with_metadata("test_data".to_string(), metadata.clone());
		assert!(response.is_success());
		assert_eq!(response.data(), Some(&"test_data".to_string()));
		assert_eq!(response.metadata(), Some(&metadata));
	}

	#[test]
	fn test_into_result_success() {
		let response = ApiResponse::success("test_data".to_string());
		let result = response.into_result();
		assert!(result.is_ok());
		assert_eq!(result.unwrap(), "test_data");
	}

	#[test]
	fn test_into_result_error() {
		let response: ApiResponse<String> = ApiResponse::error("test_error".to_string());
		let result = response.into_result();
		assert!(result.is_err());
		assert_eq!(result.unwrap_err(), "test_error");
	}

	#[test]
	fn test_from_result_ok() {
		let result: Result<String, String> = Ok("test_data".to_string());
		let response = ApiResponse::from(result);
		assert!(response.is_success());
		assert_eq!(response.data(), Some(&"test_data".to_string()));
	}

	#[test]
	fn test_from_result_err() {
		let result: Result<String, String> = Err("test_error".to_string());
		let response = ApiResponse::from(result);
		assert!(response.is_error());
		assert_eq!(response.get_error(), Some(&"test_error".to_string()));
	}
}
