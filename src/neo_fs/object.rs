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

//! # NeoFS Object Operations
//!
//! This module provides functionality for managing NeoFS objects.
//! Objects in NeoFS are the actual data files stored in containers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::neo_fs::types::{Attributes, ContainerId, ObjectId, ObjectType, OwnerId};

/// Represents a storage object in NeoFS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Object {
	/// Object unique identifier
	pub id: Option<ObjectId>,
	/// Container identifier where the object is stored
	pub container_id: ContainerId,
	/// Owner's identifier
	pub owner_id: OwnerId,
	/// Object type
	pub object_type: ObjectType,
	/// Content payload
	#[serde(skip_serializing, skip_deserializing)]
	pub payload: Vec<u8>,
	/// User-defined attributes and system headers
	pub attributes: Attributes,
	/// Creation timestamp
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(with = "chrono::serde::ts_seconds_option")]
	pub created_at: Option<DateTime<Utc>>,
}

impl Object {
	/// Creates a new object in the specified container
	pub fn new(container_id: ContainerId, owner_id: OwnerId) -> Self {
		Self {
			id: None,
			container_id,
			owner_id,
			object_type: ObjectType::Regular,
			payload: Vec::new(),
			attributes: Attributes::new(),
			created_at: None,
		}
	}

	/// Sets the object payload from a byte array
	pub fn with_payload(mut self, payload: Vec<u8>) -> Self {
		self.payload = payload;
		self
	}

	/// Sets the object type
	pub fn with_type(mut self, object_type: ObjectType) -> Self {
		self.object_type = object_type;
		self
	}

	/// Adds an attribute to the object
	pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
		self.attributes.add(key, value);
		self
	}

	/// Sets the object's file name
	pub fn with_filename(self, filename: impl Into<String>) -> Self {
		self.with_attribute("FileName", filename)
	}

	/// Sets the object's content type
	pub fn with_content_type(self, content_type: impl Into<String>) -> Self {
		self.with_attribute("Content-Type", content_type)
	}

	/// Returns the size of the object's payload in bytes
	pub fn size(&self) -> usize {
		self.payload.len()
	}
}

/// Helper structs for multipart upload operations
/// Represents an initialized multipart upload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipartUpload {
	/// Object unique identifier (assigned after completion)
	pub id: Option<ObjectId>,
	/// Container identifier where the object is stored
	pub container_id: ContainerId,
	/// Owner's identifier
	pub owner_id: OwnerId,
	/// Upload identifier
	pub upload_id: String,
	/// User-defined attributes
	pub attributes: Attributes,
	/// Part size for multipart upload (in bytes)
	pub part_size: u64,
	/// Maximum number of parts
	pub max_parts: u64,
}

/// Represents a single part in a multipart upload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Part {
	/// Part number (starting from 1)
	pub part_number: u32,
	/// Payload data for this part
	#[serde(skip_serializing, skip_deserializing)]
	pub payload: Vec<u8>,
}

impl Part {
	/// Creates a new upload part
	pub fn new(part_number: u32, payload: Vec<u8>) -> Self {
		Self { part_number, payload }
	}
}

/// Represents the result of a completed multipart upload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipartUploadResult {
	/// Object unique identifier for the uploaded object
	pub object_id: ObjectId,
	/// Container identifier
	pub container_id: ContainerId,
}
