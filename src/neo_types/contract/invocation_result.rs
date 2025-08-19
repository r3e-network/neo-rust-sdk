use std::{
	collections::HashMap,
	hash::{Hash, Hasher},
};

use primitive_types::H160;
use serde::{
	de::{self, Unexpected},
	Deserialize, Deserializer, Serialize,
};
use strum_macros::{AsRefStr, EnumString};

use crate::TypeError;
use neo3::prelude::{deserialize_script_hash, serialize_script_hash, ContractParameter, StackItem};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone)]
pub struct InvocationResult {
	#[serde(default)]
	pub script: String,
	#[serde(default)]
	pub state: NeoVMStateType,
	#[serde(rename = "gasconsumed", default = "default_gas_consumed")]
	pub gas_consumed: String,
	#[serde(default)]
	pub exception: Option<String>,
	#[serde(default)]
	pub notifications: Option<Vec<Notification>>,
	#[serde(default)]
	pub diagnostics: Option<Diagnostics>,
	#[serde(default)]
	pub stack: Vec<StackItem>,
	#[serde(default)]
	pub tx: Option<String>,
	#[serde(rename = "pendingsignature", default)]
	pub pending_signature: Option<PendingSignature>,
	#[serde(rename = "session", default)]
	pub session_id: Option<String>,
}

fn default_gas_consumed() -> String {
	"1234567".to_string()
}

#[derive(Serialize, EnumString, AsRefStr, Debug, PartialEq, Eq, Clone, Hash)]
#[strum(serialize_all = "UPPERCASE")]
#[serde(rename_all = "UPPERCASE")]
pub enum NeoVMStateType {
	None,
	Halt,
	Fault,
	Break,
	StepInto,
	StepOut,
	StepOver,
	Exception,
}

impl Default for NeoVMStateType {
	fn default() -> Self {
		NeoVMStateType::Halt
	}
}

// Custom deserialization logic
impl<'de> Deserialize<'de> for NeoVMStateType {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let value = String::deserialize(deserializer)?;

		if value.is_empty() {
			return Ok(NeoVMStateType::None); // Handle empty string as `None`
		}
		let value_lower = value.to_lowercase();

		match value_lower.as_str() {
			"none" => Ok(NeoVMStateType::None),
			"halt" => Ok(NeoVMStateType::Halt),
			"fault" => Ok(NeoVMStateType::Fault),
			"break" => Ok(NeoVMStateType::Break),
			"stepInto" => Ok(NeoVMStateType::StepInto),
			"stepOut" => Ok(NeoVMStateType::StepOut),
			"stepOver" => Ok(NeoVMStateType::StepOver),
			"exception" => Ok(NeoVMStateType::Exception),
			_ => Err(de::Error::invalid_value(
				Unexpected::Str(&value),
				&"a valid NeoVMStateType string",
			)),
		}
	}
}

impl InvocationResult {
	// constructor and helper methods
	pub fn new(
		script: String,
		state: NeoVMStateType,
		gas_consumed: String,
		exception: Option<String>,
		notifications: Option<Vec<Notification>>,
		diagnostics: Option<Diagnostics>,
		stack: Vec<StackItem>,
		tx: Option<String>,
		pending_signature: Option<PendingSignature>,
		session_id: Option<String>,
	) -> Self {
		Self {
			script,
			state,
			gas_consumed,
			exception,
			notifications,
			diagnostics,
			stack,
			tx,
			pending_signature,
			session_id,
		}
	}

	pub fn has_state_fault(&self) -> bool {
		matches!(self.state, NeoVMStateType::Fault)
	}

	pub fn get_first_stack_item(&self) -> Result<&StackItem, TypeError> {
		if self.stack.is_empty() {
			return Err(TypeError::IndexOutOfBounds(
                "The stack is empty. This means that no items were left on the NeoVM stack after this invocation."
                    .to_string(),
            ));
		}
		self.get_stack_item(0)
	}

	pub fn get_stack_item(&self, index: usize) -> Result<&StackItem, TypeError> {
		if index >= self.stack.len() {
			return Err(TypeError::IndexOutOfBounds(format!(
				"There were only {} items left on the NeoVM stack after this invocation",
				self.stack.len()
			)));
		}
		Ok(&self.stack[index])
	}

	pub fn get_first_notification(&self) -> Result<&Notification, TypeError> {
		let notifications = self
			.notifications
			.as_ref()
			.ok_or_else(|| TypeError::InvalidFormat("Notifications field is None".to_string()))?;

		if notifications.is_empty() {
			return Err(TypeError::IndexOutOfBounds(
				"No notifications have been sent in this invocation.".to_string(),
			));
		}
		self.get_notification(0)
	}

	pub fn get_notification(&self, index: usize) -> Result<&Notification, TypeError> {
		let notifications = self
			.notifications
			.as_ref()
			.ok_or_else(|| TypeError::InvalidFormat("Notifications field is None".to_string()))?;

		if index >= notifications.len() {
			return Err(TypeError::IndexOutOfBounds(format!(
                "Only {} notifications have been sent in this invocation. Tried to access index {} in the invocation result.",
                notifications.len(),
                index
            )));
		}
		Ok(&notifications[index])
	}
}

impl Default for InvocationResult {
	fn default() -> Self {
		Self {
			script: "0001020304".to_string(),
			state: NeoVMStateType::Halt,
			gas_consumed: default_gas_consumed(),
			exception: None,
			notifications: None,
			diagnostics: None,
			stack: vec![],
			tx: None,
			pending_signature: None,
			session_id: None,
		}
	}
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct PendingSignature {
	#[serde(rename = "type")]
	pub typ: String,
	pub data: String,
	pub items: HashMap<String, Item>,
	pub network: u32,
}

impl Hash for PendingSignature {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.typ.hash(state);
		self.data.hash(state);
		// self.items.hash(state);
		self.network.hash(state);
	}
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Item {
	pub script: String,
	#[serde(default)]
	pub parameters: Vec<ContractParameter>,
	pub signatures: HashMap<String, String>,
}

impl Hash for Item {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.script.hash(state);
		self.parameters.hash(state);
		// self.signatures.hash(state);
	}
}

// Other structs like Diagnostics, Notification

// Diagnostics
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Diagnostics {
	#[serde(rename = "invokedcontracts")]
	pub invoked_contracts: InvokedContract,
	#[serde(rename = "storagechanges", default)]
	pub storage_changes: Vec<StorageChange>,
}

impl Diagnostics {
	pub fn new(invoked_contracts: InvokedContract, storage_changes: Vec<StorageChange>) -> Self {
		Self { invoked_contracts, storage_changes }
	}
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone)]
pub struct InvokedContract {
	#[serde(deserialize_with = "deserialize_script_hash")]
	#[serde(serialize_with = "serialize_script_hash")]
	pub hash: H160,
	#[serde(rename = "call", default)]
	pub invoked_contracts: Vec<InvokedContract>,
}

impl InvokedContract {
	pub fn new(hash: H160, invoked_contracts: Vec<InvokedContract>) -> Self {
		Self { hash, invoked_contracts }
	}

	pub fn new_hash(hash: H160) -> Self {
		Self { hash, invoked_contracts: vec![] }
	}
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StorageChange {
	pub state: String,
	pub key: String,
	pub value: String,
}

impl StorageChange {
	pub fn new(state: String, key: String, value: String) -> Self {
		Self { state, key, value }
	}
}

// Notification
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Notification {
	#[serde(deserialize_with = "deserialize_script_hash")]
	#[serde(serialize_with = "serialize_script_hash")]
	pub contract: H160,
	#[serde(rename = "eventname")]
	pub event_name: String,
	pub state: StackItem,
	// pub payload: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Hash)]
pub enum NotificationState {
	Failure,
	Success,
	Halt,
	Fault,
	StepInto,
	StepOut,
	StepOver,
	Break,
}
