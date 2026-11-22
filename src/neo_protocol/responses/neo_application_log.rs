use primitive_types::H256;
use serde::{Deserialize, Serialize};

use crate::{TypeError, neo_protocol::LogNotification};
use neo3::prelude::{StackItem, VMState, deserialize_h256, serialize_h256};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ApplicationLog {
	#[serde(rename = "txid")]
	#[serde(serialize_with = "serialize_h256")]
	#[serde(deserialize_with = "deserialize_h256")]
	pub transaction_id: H256,
	#[serde(default)]
	pub executions: Vec<Execution>,
}

impl Default for ApplicationLog {
	fn default() -> Self {
		Self { transaction_id: H256::zero(), executions: vec![] }
	}
}

impl ApplicationLog {
	pub fn get_first_execution(&self) -> Result<&Execution, TypeError> {
		if self.executions.is_empty() {
			return Err(TypeError::IndexOutOfBounds(
				"This transaction does not have any executions.".to_string(),
			));
		}
		self.get_execution(0)
	}

	pub fn get_execution(&self, index: usize) -> Result<&Execution, TypeError> {
		if index >= self.executions.len() {
			return Err(TypeError::IndexOutOfBounds(format!(
				"This transaction has only {} executions.",
				self.executions.len()
			)));
		}
		Ok(&self.executions[index])
	}
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Execution {
	pub trigger: String,
	#[serde(rename = "vmstate")]
	pub state: VMState,
	pub exception: Option<String>,
	#[serde(rename = "gasconsumed")]
	pub gas_consumed: String,
	#[serde(default)]
	pub stack: Vec<StackItem>,
	#[serde(default)]
	pub notifications: Vec<LogNotification>,
}

impl Execution {
	pub fn get_first_stack_item(&self) -> Result<&StackItem, TypeError> {
		if self.stack.is_empty() {
			return Err(TypeError::IndexOutOfBounds(
                "The stack is empty. This means that no items were left on the NeoVM stack after this execution."
                    .to_string(),
            ));
		}
		self.get_stack_item(0)
	}

	pub fn get_stack_item(&self, index: usize) -> Result<&StackItem, TypeError> {
		if index >= self.stack.len() {
			return Err(TypeError::IndexOutOfBounds(format!(
				"There were only {} items left on the NeoVM stack after this execution.",
				self.stack.len()
			)));
		}
		Ok(&self.stack[index])
	}

	pub fn get_first_notification(&self) -> Result<&LogNotification, TypeError> {
		if self.notifications.is_empty() {
			return Err(TypeError::IndexOutOfBounds(
				"This execution did not send any notifications.".to_string(),
			));
		}
		self.get_notification(0)
	}

	pub fn get_notification(&self, index: usize) -> Result<&LogNotification, TypeError> {
		if index >= self.notifications.len() {
			return Err(TypeError::IndexOutOfBounds(format!(
				"This execution only sent {} notifications.",
				self.notifications.len()
			)));
		}
		Ok(&self.notifications[index])
	}
}

impl Default for Execution {
	fn default() -> Self {
		Self {
			trigger: "".to_string(),
			state: VMState::Halt,
			exception: None,
			gas_consumed: "0".to_string(),
			stack: vec![],
			notifications: vec![],
		}
	}
}
