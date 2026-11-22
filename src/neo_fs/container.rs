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

//! # NeoFS Container
//!
//! This module provides types and functions for working with NeoFS containers.

use time::OffsetDateTime;
use serde::{Deserialize, Serialize};

use crate::neo_fs::types::{Attributes, ContainerId, OwnerId, PlacementPolicy};

/// Versioning information for a container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
	/// Major version number
	pub major: u32,
	/// Minor version number
	pub minor: u32,
}

impl Default for Version {
	fn default() -> Self {
		Self { major: 1, minor: 0 }
	}
}

/// Represents a storage container in NeoFS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
	/// Container ID
	pub id: Option<ContainerId>,

	/// Owner ID of the container
	pub owner_id: OwnerId,

	/// Basic ACL
	pub basic_acl: u32,

	/// Container name
	pub name: String,

	/// Container creation timestamp
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(with = "time::serde::timestamp::option")]
	pub creation: Option<OffsetDateTime>,

	/// Container version
	pub version: Option<Version>,

	/// Container attributes
	pub attributes: Attributes,

	/// Placement policy for the container
	pub placement_policy: PlacementPolicy,
}

impl Container {
	/// Creates a new container with the given ID and owner ID
	pub fn new(id: ContainerId, owner_id: OwnerId) -> Self {
		Self {
			id: Some(id),
			owner_id,
			basic_acl: 0,
			name: String::new(),
			creation: None,
			version: Some(Version::default()),
			attributes: Attributes::new(),
			placement_policy: Default::default(),
		}
	}

	/// Sets the basic ACL for the container
	#[must_use]
	pub fn with_basic_acl(mut self, acl: u32) -> Self {
		self.basic_acl = acl;
		self
	}

	/// Sets the container name
	#[must_use]
	pub fn with_name(mut self, name: String) -> Self {
		self.name = name;
		self
	}

	/// Sets the container creation timestamp
	#[must_use]
	pub fn with_creation(mut self, creation: OffsetDateTime) -> Self {
		self.creation = Some(creation);
		self
	}

	/// Sets the container version
	#[must_use]
	pub fn with_version(mut self, version: Version) -> Self {
		self.version = Some(version);
		self
	}

	/// Adds an attribute to the container
	#[must_use]
	pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
		self.attributes.add(key, value);
		self
	}
}

/// Permissions for the basic ACL
#[derive(Debug, Clone)]
pub struct BasicACL {
	/// Create and delete objects in the container
	pub put_allowed: bool,
	/// Read objects from the container
	pub get_allowed: bool,
	/// Query object headers from the container
	pub head_allowed: bool,
	/// Delete objects from the container
	pub delete_allowed: bool,
	/// List objects in the container
	pub list_allowed: bool,
}

impl BasicACL {
	/// Creates a basic ACL with all permissions set
	pub fn full_access() -> Self {
		Self {
			put_allowed: true,
			get_allowed: true,
			head_allowed: true,
			delete_allowed: true,
			list_allowed: true,
		}
	}

	/// Creates a basic ACL with read-only permissions
	pub fn read_only() -> Self {
		Self {
			put_allowed: false,
			get_allowed: true,
			head_allowed: true,
			delete_allowed: false,
			list_allowed: true,
		}
	}

	/// Converts basic ACL to a bitmask
	pub fn to_bitmask(&self) -> u32 {
		let mut bitmask = 0;

		if self.put_allowed {
			bitmask |= 0b00001;
		}
		if self.get_allowed {
			bitmask |= 0b00010;
		}
		if self.head_allowed {
			bitmask |= 0b00100;
		}
		if self.delete_allowed {
			bitmask |= 0b01000;
		}
		if self.list_allowed {
			bitmask |= 0b10000;
		}

		bitmask
	}

	/// Creates a basic ACL from a bitmask
	pub fn from_bitmask(bitmask: u32) -> Self {
		Self {
			put_allowed: (bitmask & 0b00001) != 0,
			get_allowed: (bitmask & 0b00010) != 0,
			head_allowed: (bitmask & 0b00100) != 0,
			delete_allowed: (bitmask & 0b01000) != 0,
			list_allowed: (bitmask & 0b10000) != 0,
		}
	}
}
