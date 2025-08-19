// Copyright (c) 2023-2025 R3E Network
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # NeoFS Error Handling
//!
//! This module provides error types for NeoFS operations.

use thiserror::Error;

/// Errors that can occur during NeoFS operations
#[derive(Error, Debug)]
pub enum NeoFSError {
	/// Connection error
	#[error("Connection error: {0}")]
	ConnectionError(String),

	/// Authentication error
	#[error("Authentication error: {0}")]
	AuthenticationError(String),

	/// Container error
	#[error("Container error: {0}")]
	ContainerError(String),

	/// Object error
	#[error("Object error: {0}")]
	ObjectError(String),

	/// Access control error
	#[error("Access control error: {0}")]
	ACLError(String),

	/// Serialization/deserialization error
	#[error("Serialization error: {0}")]
	SerializationError(String),

	/// Permission denied
	#[error("Permission denied: {0}")]
	PermissionDenied(String),

	/// Resource not found
	#[error("Resource not found: {0}")]
	NotFound(String),

	/// Invalid argument
	#[error("Invalid argument: {0}")]
	InvalidArgument(String),

	/// Operation timeout
	#[error("Operation timeout: {0}")]
	Timeout(String),

	/// Internal error
	#[error("Internal error: {0}")]
	InternalError(String),

	/// Conversion error
	#[error("Conversion error: {0}")]
	ConversionError(String),

	/// Not implemented
	#[error("Not implemented: {0}")]
	NotImplemented(String),

	/// Generic IO error
	#[error("IO error: {0}")]
	IOError(#[from] std::io::Error),

	/// Unexpected response
	#[error("Unexpected response: {0}")]
	UnexpectedResponse(String),
}

/// Result type for NeoFS operations
pub type NeoFSResult<T> = std::result::Result<T, NeoFSError>;
