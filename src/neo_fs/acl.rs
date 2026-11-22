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

//! # NeoFS Access Control
//!
//! This module provides types and functions for working with NeoFS access control.

use time::OffsetDateTime;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::neo_fs::types::{AccessPermission, ContainerId, OwnerId};

/// Operation that can be performed on an object or container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
	/// Container operations
	Container(ContainerOperation),
	/// Object operations
	Object(ObjectOperation),
}

/// Container operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContainerOperation {
	/// Get container metadata
	Get,
	/// Update container metadata
	Put,
	/// Delete container
	Delete,
	/// Get extended ACL
	GetEACL,
	/// Set extended ACL
	SetEACL,
}

/// Object operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectOperation {
	/// Get object data
	Get,
	/// Upload object
	Put,
	/// Get object metadata
	Head,
	/// Search objects
	Search,
	/// Delete object
	Delete,
	/// Get object range (partial data)
	Range,
	/// Get object hash
	Hash,
}

/// Access target for EACL rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
	/// Role that the target applies to
	pub role: TargetRole,
	/// Keys that define the target
	pub keys: Vec<String>,
}

/// Target role in EACL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetRole {
	/// Target is an object/container owner
	Owner,
	/// Target is part of a specific group
	Group,
	/// Target refers to specific users
	Users,
	/// Target includes any authenticated user
	Others,
}

/// Action to perform for matching EACL rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
	/// Allow the operation
	Allow,
	/// Deny the operation
	Deny,
}

/// Filter for EACL rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
	/// Header key to match
	pub key: String,
	/// Value to match
	pub value: String,
	/// Matching operation
	pub operation: FilterOperation,
}

/// Filter operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterOperation {
	/// Equals
	Eq,
	/// Not equals
	Ne,
	/// Greater than
	Gt,
	/// Greater than or equals
	Ge,
	/// Less than
	Lt,
	/// Less than or equals
	Le,
}

/// Single EACL rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EACLRecord {
	/// Operation the rule applies to
	pub operation: Operation,
	/// Action to take (allow/deny)
	pub action: Action,
	/// Target the rule applies to
	pub target: Target,
	/// Filters for additional matching
	pub filters: Vec<Filter>,
}

/// Extended Access Control List
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EACL {
	/// Container the EACL applies to
	pub container_id: ContainerId,
	/// EACL records (rules)
	pub records: Vec<EACLRecord>,
}

impl EACL {
	/// Creates a new EACL for the specified container
	pub fn new(container_id: ContainerId) -> Self {
		Self { container_id, records: Vec::new() }
	}

	/// Adds a record to the EACL
	pub fn add_record(&mut self, record: EACLRecord) {
		self.records.push(record);
	}
}

/// Bearer token for delegated access to NeoFS resources
#[derive(Clone, Serialize, Deserialize)]
pub struct BearerToken {
	/// Token owner
	pub owner_id: OwnerId,
	/// Token ID
	pub token_id: String,
	/// When the token expires
	pub expiration: OffsetDateTime,
	/// Allowed operations
	pub operations: Vec<AccessPermission>,
	/// Container this token grants access to
	pub container_id: ContainerId,
	/// Signature to validate the token
	pub signature: Vec<u8>,
}

impl fmt::Debug for BearerToken {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("BearerToken")
			.field("owner_id", &self.owner_id)
			.field("token_id", &self.token_id)
			.field("expiration", &self.expiration)
			.field("operations", &self.operations.len())
			.field("container_id", &self.container_id)
			.field("signature_len", &self.signature.len())
			.finish()
	}
}

/// Session token for authenticated access to NeoFS resources
#[derive(Clone, Serialize, Deserialize)]
pub struct SessionToken {
	/// Token ID
	pub token_id: String,
	/// Identity of the user
	pub owner_id: OwnerId,
	/// When the session expires
	pub expiration: OffsetDateTime,
	/// Session key
	pub session_key: String,
	/// Signature to validate the session
	pub signature: Vec<u8>,
}

impl fmt::Debug for SessionToken {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("SessionToken")
			.field("token_id", &self.token_id)
			.field("owner_id", &self.owner_id)
			.field("expiration", &self.expiration)
			.field("session_key", &"<redacted>")
			.field("signature_len", &self.signature.len())
			.finish()
	}
}
