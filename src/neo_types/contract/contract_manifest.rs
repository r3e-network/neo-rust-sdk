//! Contract Manifest types for Neo N3 smart contracts.
//!
//! The contract manifest is a JSON-formatted description of a smart contract that provides
//! essential information about the contract's structure, interfaces, and security permissions.
//! It is required for all Neo N3 smart contracts and is stored on-chain alongside the contract's bytecode.
//!
//! # Purpose
//!
//! The manifest serves several critical functions:
//!
//! - **Contract Identification**: Provides the contract name and supported standards (e.g., NEP-11, NEP-17)
//! - **Interface Definition**: Declares the contract's methods, parameters, return types, and events via the ABI
//! - **Security Boundaries**: Defines which other contracts this contract can call and which methods it trusts
//! - **Feature Declaration**: Specifies optional features and extensions the contract supports
//!
//! # Example Manifest Structure
//!
//! ```json
//! {
//!   "name": "MyToken",
//!   "groups": [],
//!   "features": {},
//!   "supportedstandards": ["NEP-17"],
//!   "abi": {
//!     "methods": [...],
//!     "events": [...]
//!   },
//!   "permissions": [{"contract": "*", "methods": ["transfer"]}],
//!   "trusts": [],
//!   "extra": null
//! }
//! ```
//!
//! # Security Model
//!
//! The permissions and trusts fields define the contract's security boundaries:
//!
//! - **Permissions**: Control which external contracts and methods this contract is allowed to invoke.
//!   The wildcard `"*"` can be used to grant broad permissions.
//!
//! - **Trusts**: Specifies which contracts are trusted. A contract can only call methods marked as `safe`
//!   on untrusted contracts unless explicitly permitted.

use std::{
	collections::HashMap,
	hash::{Hash, Hasher},
};

use serde::{Deserialize, Serialize};

use crate::{
	TypeError,
	neo_types::ContractParameter2,
	prelude::{deserialize_wildcard, serialize_wildcard},
};
use neo3::prelude::{ContractParameter, ContractParameterType};

/// The contract manifest describing a Neo N3 smart contract's metadata, interface, and permissions.
///
/// The manifest is a core component of every Neo N3 smart contract. It declares the contract's
/// interface (ABI), supported standards, security permissions, and other metadata. When a contract
/// is deployed, its manifest is stored on-chain and can be queried by clients and other contracts
/// to understand how to interact with it.
///
/// # Key Fields
///
/// - [`name`](Self::name): Human-readable contract name
/// - [`supported_standards`](Self::supported_standards): List of NEP standards the contract implements
/// - [`abi`](Self::abi): Application Binary Interface defining methods and events
/// - [`permissions`](Self::permissions): Security permissions for cross-contract calls
/// - [`trusts`](Self::trusts): List of trusted contracts
///
/// # Serialization
///
/// This struct supports serialization to/from JSON format as defined by the Neo N3 specification.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ContractManifest {
	/// The human-readable name of the contract.
	///
	/// This is optional and provides a friendly identifier for the contract.
	/// It does not need to be unique across the blockchain.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,

	/// A list of cryptographic groups this contract belongs to.
	///
	/// Groups are used for advanced cryptographic operations and multi-signature scenarios.
	/// Most contracts will have an empty groups list.
	#[serde(default)]
	pub groups: Vec<ContractGroup>,

	/// A map of additional features supported by this contract.
	///
	/// This extensibility mechanism allows contracts to declare support for
	/// protocol extensions or custom features. The keys are feature names,
	/// and values are feature-specific configuration data.
	#[serde(default)]
	pub features: HashMap<String, serde_json::Value>,

	/// List of Neo Enhancement Proposals (NEPs) this contract supports.
	///
	/// Common standards include:
	/// - `"NEP-11"`: Non-fungible token standard
	/// - `"NEP-17"`: Fungible token standard
	///
	/// These standards define expected interfaces and behaviors that clients
	/// can rely on when interacting with the contract.
	#[serde(rename = "supportedstandards")]
	pub supported_standards: Vec<String>,

	/// The Application Binary Interface (ABI) describing the contract's methods and events.
	///
	/// The ABI is essential for clients to know how to construct valid invocations
	/// and interpret return values. It includes:
	/// - Method names, parameters, return types, and offset in the script
	/// - Event names and their parameter types
	#[serde(skip_serializing_if = "Option::is_none")]
	pub abi: Option<ContractABI>,

	/// Security permissions defining which external contracts this contract can call.
	///
	/// Each permission specifies a target contract (or wildcard `"*"`) and a list
	/// of allowed method names (or wildcard for all methods). The Neo runtime
	/// enforces these permissions during cross-contract invocations.
	#[serde(default)]
	pub permissions: Vec<ContractPermission>,

	/// List of contract hashes or wildcards representing trusted contracts.
	///
	/// When a contract trusts another, it can safely call methods on that contract
	/// without additional permission checks. An empty list means the contract
	/// does not explicitly trust any other contracts.
	pub trusts: Vec<String>,

	/// Optional extra metadata for the contract.
	///
	/// This field allows contracts to include additional information such as:
	/// - Author information
	/// - Version details
	/// - Description or documentation links
	/// - Custom tags or categorization
	#[serde(skip_serializing_if = "Option::is_none")]
	pub extra: Option<HashMap<String, serde_json::Value>>,
}

impl ContractManifest {
	/// Creates a new contract manifest with the specified parameters.
	///
	/// # Arguments
	///
	/// * `name` - Optional human-readable contract name
	/// * `groups` - List of cryptographic groups
	/// * `features` - Optional map of feature configurations
	/// * `supported_standards` - List of supported NEP standards
	/// * `abi` - Optional contract ABI describing methods and events
	/// * `permissions` - List of security permissions for cross-contract calls
	/// * `trusts` - List of trusted contract hashes or wildcards
	/// * `extra` - Optional additional metadata
	///
	/// # Example
	///
	/// ```
	/// use neo3::neo_types::contract::ContractManifest;
	///
	/// let manifest = ContractManifest::new(
	///     Some("MyToken".to_string()),
	///     vec![],
	///     None,
	///     vec!["NEP-17".to_string()],
	///     None,
	///     vec![],
	///     vec![],
	///     None,
	/// );
	/// ```
	#[allow(clippy::too_many_arguments)]
	pub fn new(
		name: Option<String>,
		groups: Vec<ContractGroup>,
		features: Option<HashMap<String, serde_json::Value>>,
		supported_standards: Vec<String>,
		abi: Option<ContractABI>,
		permissions: Vec<ContractPermission>,
		trusts: Vec<String>,
		extra: Option<HashMap<String, serde_json::Value>>,
	) -> Self {
		Self {
			name,
			groups,
			features: features.unwrap_or_default(),
			supported_standards,
			abi,
			permissions,
			trusts,
			extra,
		}
	}

	/// Returns a reference to the supported standard at the given index.
	///
	/// # Arguments
	///
	/// * `index` - The index of the supported standard to retrieve
	///
	/// # Errors
	///
	/// Returns `TypeError::IndexOutOfBounds` if the index is greater than or equal
	/// to the number of supported standards.
	pub fn get_supported_standard(&self, index: usize) -> Result<&String, TypeError> {
		if index >= self.supported_standards.len() {
			return Err(TypeError::IndexOutOfBounds(format!(
				"This contract only supports {} standards. Tried to access a supported standard at index {} in the manifest",
				self.supported_standards.len(),
				index
			)));
		}
		Ok(&self.supported_standards[index])
	}

	/// Returns a reference to the first supported standard.
	///
	/// # Errors
	///
	/// Returns `TypeError::IndexOutOfBounds` if the contract does not support any standards.
	pub fn get_first_supported_standard(&self) -> Result<&String, TypeError> {
		if self.supported_standards.is_empty() {
			return Err(TypeError::IndexOutOfBounds(
				"This contract does not support any standard.".to_string(),
			));
		}
		self.get_supported_standard(0)
	}

	/// Returns a reference to the permission at the given index.
	///
	/// # Arguments
	///
	/// * `index` - The index of the permission to retrieve
	///
	/// # Errors
	///
	/// Returns `TypeError::IndexOutOfBounds` if the index is greater than or equal
	/// to the number of permissions.
	pub fn get_permission(&self, index: usize) -> Result<&ContractPermission, TypeError> {
		if index >= self.permissions.len() {
			return Err(TypeError::IndexOutOfBounds(format!(
				"This contract only has permission for {} contracts. Tried to access a permission at index {} in the manifest.",
				self.permissions.len(),
				index
			)));
		}
		Ok(&self.permissions[index])
	}

	/// Returns a reference to the first permission.
	///
	/// # Errors
	///
	/// Returns `TypeError::IndexOutOfBounds` if the contract does not have any permissions.
	/// This indicates the contract cannot invoke any other contract's methods unless they
	/// are marked as safe (read-only).
	pub fn get_first_permission(&self) -> Result<&ContractPermission, TypeError> {
		if self.permissions.is_empty() {
			return Err(TypeError::IndexOutOfBounds(
				"This contract does not have any permissions. It is not permitted to invoke any other contract's method if it is not marked safe (i.e., read-only).".to_string(),
			));
		}
		self.get_permission(0)
	}

	/// Returns a reference to the first trusted contract.
	///
	/// # Errors
	///
	/// Returns `TypeError::IndexOutOfBounds` if the contract does not trust any other contracts.
	pub fn get_first_trust(&self) -> Result<&String, TypeError> {
		if self.trusts.is_empty() {
			return Err(TypeError::IndexOutOfBounds(
				"This contract does not trust any other contracts.".to_string(),
			));
		}
		self.get_trust(0)
	}

	/// Returns a reference to the trusted contract at the given index.
	///
	/// # Arguments
	///
	/// * `index` - The index of the trusted contract to retrieve
	///
	/// # Errors
	///
	/// Returns `TypeError::IndexOutOfBounds` if the index is greater than or equal
	/// to the number of trusted contracts.
	pub fn get_trust(&self, index: usize) -> Result<&String, TypeError> {
		if index >= self.trusts.len() {
			return Err(TypeError::IndexOutOfBounds(format!(
				"This contract trusts only {} contracts. Tried to access a trusted contract at index {} in the manifest.",
				self.trusts.len(),
				index
			)));
		}
		Ok(&self.trusts[index])
	}
}

// impl Eq for ContractManifest
impl PartialEq for ContractManifest {
	fn eq(&self, other: &Self) -> bool {
		self.name == other.name
			&& self.groups == other.groups
			&& self.features == other.features
			&& self.supported_standards == other.supported_standards
			&& self.abi == other.abi
			&& self.permissions == other.permissions
			&& self.trusts == other.trusts
			&& self.extra == other.extra
	}
}

impl Hash for ContractManifest {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.name.hash(state);
		self.groups.hash(state);
		// self.features.hash(state);
		self.supported_standards.hash(state);
		self.abi.hash(state);
		self.permissions.hash(state);
		self.trusts.hash(state);
		// self.extra.hash(state);
	}
}

/// A cryptographic group definition for a smart contract.
///
/// Groups enable advanced cryptographic operations by associating a public key
/// and signature with the contract. This is used in multi-signature scenarios
/// and other cryptographic protocols where the contract needs to prove group membership.
#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Debug, Clone)]
pub struct ContractGroup {
	/// The public key associated with this group.
	pub pub_key: String,
	/// The signature proving membership in this group.
	pub signature: String,
}

/// The Application Binary Interface (ABI) describing a contract's methods and events.
///
/// The ABI is essential for clients to interact with the contract. It provides:
/// - Method definitions with names, parameters, return types, and bytecode offsets
/// - Event definitions for monitoring contract activity
///
/// When invoking a contract, clients use the ABI to properly encode arguments
/// and decode return values.
#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Clone)]
pub struct ContractABI {
	/// List of methods defined by this contract.
	pub methods: Vec<ContractMethod>,
	/// List of events that this contract can emit.
	///
	/// Events are logged during contract execution and can be monitored
	/// by clients to track contract state changes.
	#[serde(default)]
	pub events: Vec<ContractEvent>,
}

impl ContractABI {
	/// Creates a new contract ABI with the specified methods and events.
	///
	/// # Arguments
	///
	/// * `methods` - Optional list of contract methods
	/// * `events` - Optional list of contract events
	pub fn new(methods: Option<Vec<ContractMethod>>, events: Option<Vec<ContractEvent>>) -> Self {
		Self { methods: methods.unwrap_or_default(), events: events.unwrap_or_default() }
	}

	/// Returns a reference to the first method in the ABI.
	///
	/// # Errors
	///
	/// Returns `TypeError::IndexOutOfBounds` if the ABI contains no methods.
	/// Every functional contract should have at least one method.
	pub fn get_first_method(&self) -> Result<&ContractMethod, TypeError> {
		if self.methods.is_empty() {
			return Err(TypeError::IndexOutOfBounds(
	        	"This ABI does not contain any methods. It might be malformed, since every contract needs at least one method to be functional.".to_string(),
       		));
		}
		self.get_method(0)
	}

	/// Returns a reference to the method at the given index.
	///
	/// # Arguments
	///
	/// * `index` - The index of the method to retrieve
	///
	/// # Errors
	///
	/// Returns `TypeError::IndexOutOfBounds` if the index is greater than or equal
	/// to the number of methods.
	pub fn get_method(&self, index: usize) -> Result<&ContractMethod, TypeError> {
		if index >= self.methods.len() {
			return Err(TypeError::IndexOutOfBounds(format!(
				"This ABI only contains {} methods. Tried to access index {}.",
				self.methods.len(),
				index
			)));
		}
		Ok(&self.methods[index])
	}

	/// Returns a reference to the first event in the ABI.
	///
	/// # Errors
	///
	/// Returns `TypeError::IndexOutOfBounds` if the ABI contains no events.
	pub fn get_first_event(&self) -> Result<&ContractEvent, TypeError> {
		if self.events.is_empty() {
			return Err(TypeError::IndexOutOfBounds(
				"This ABI does not have any events.".to_string(),
			));
		}
		self.get_event(0)
	}

	/// Returns a reference to the event at the given index.
	///
	/// # Arguments
	///
	/// * `index` - The index of the event to retrieve
	///
	/// # Errors
	///
	/// Returns `TypeError::IndexOutOfBounds` if the index is greater than or equal
	/// to the number of events.
	pub fn get_event(&self, index: usize) -> Result<&ContractEvent, TypeError> {
		if index >= self.events.len() {
			return Err(TypeError::IndexOutOfBounds(format!(
				"This ABI only has {} events. Tried to access index {}.",
				self.events.len(),
				index
			)));
		}
		Ok(&self.events[index])
	}
}

/// A method definition in a contract's ABI.
///
/// Describes a single callable method including its name, parameters,
/// return type, and whether it is safe to call without a witness check.
#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
pub struct ContractMethod {
	/// The name of the method.
	pub name: String,
	/// The parameters this method accepts.
	pub parameters: Vec<ContractParameter2>,
	/// The bytecode offset where this method begins in the contract script.
	pub offset: usize,
	/// The return type of this method.
	#[serde(rename = "returntype")]
	pub return_type: ContractParameterType,
	/// Whether this method is safe to call without a witness check.
	///
	/// Safe methods are read-only operations that cannot modify state
	/// or transfer assets. They can be invoked more efficiently and
	/// without requiring a transaction signature.
	pub safe: bool,
}

impl ContractMethod {
	/// Creates a new contract method definition.
	///
	/// # Arguments
	///
	/// * `name` - The method name
	/// * `parameters` - Optional list of method parameters
	/// * `offset` - Bytecode offset in the contract script
	/// * `return_type` - The method's return type
	/// * `safe` - Whether the method is safe (read-only)
	pub fn new(
		name: String,
		parameters: Option<Vec<ContractParameter2>>,
		offset: usize,
		return_type: ContractParameterType,
		safe: bool,
	) -> Self {
		Self { name, parameters: parameters.unwrap_or_default(), offset, return_type, safe }
	}
}

/// An event definition in a contract's ABI.
///
/// Events are emitted during contract execution and logged to the blockchain.
/// Clients can monitor these events to track contract activity and state changes.
#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Debug, Clone)]
pub struct ContractEvent {
	/// The name of the event.
	pub name: String,
	/// The parameters included in this event.
	///
	/// When the event is emitted, these parameters provide additional context
	/// about what occurred.
	pub parameters: Vec<ContractParameter>,
}

/// A security permission for cross-contract invocations.
///
/// Permissions define which external contracts and methods this contract
/// is allowed to call. The Neo runtime enforces these permissions during
/// cross-contract invocations.
#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Clone)]
pub struct ContractPermission {
	/// The target contract hash or wildcard.
	///
	/// - A specific contract hash (e.g., "0x...") restricts calls to that contract
	/// - The wildcard `"*"` allows calls to any contract
	pub contract: String,
	/// The list of allowed method names or wildcard.
	///
	/// - Specific method names restrict which methods can be called
	/// - The wildcard `"*"` allows any method on the target contract
	///
	/// Serialized with custom handling for the wildcard character.
	#[serde(serialize_with = "serialize_wildcard")]
	#[serde(deserialize_with = "deserialize_wildcard")]
	pub methods: Vec<String>,
}

impl ContractPermission {
	/// Creates a new contract permission.
	///
	/// # Arguments
	///
	/// * `contract` - The target contract hash or wildcard "*"
	/// * `methods` - List of allowed method names or wildcard "*"
	///
	/// # Example
	///
	/// ```
	/// use neo3::neo_types::contract::ContractPermission;
	///
	/// // Allow calling 'transfer' method on any contract
	/// let permission = ContractPermission::new("*".to_string(), vec!["transfer".to_string()]);
	/// ```
	pub fn new(contract: String, methods: Vec<String>) -> Self {
		Self { contract, methods }
	}
}
