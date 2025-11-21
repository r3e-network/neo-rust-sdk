use crate::{ContractMethodToken, TypeError};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[derive(Serialize, Deserialize, Default, Hash, Clone, Debug, PartialEq)]
#[serde_as]
pub struct ContractNef {
	#[serde(default)]
	pub magic: i32,
	#[serde(default)]
	pub compiler: String,
	#[serde(default)]
	pub source: String,
	#[serde_as(as = "Vec<ContractMethodToken>")]
	pub tokens: Vec<ContractMethodToken>,
	#[serde(default)]
	pub script: String,
	#[serde(default)]
	pub checksum: i64,
}

impl ContractNef {
	pub fn new(
		magic: i32,
		compiler: String,
		source: Option<String>,
		tokens: Vec<ContractMethodToken>,
		script: String,
		checksum: i64,
	) -> Self {
		Self {
			magic,
			compiler,
			source: source.unwrap_or_default(),
			tokens,
			script,
			checksum,
		}
	}

	pub fn get_first_token(&self) -> Result<&ContractMethodToken, TypeError> {
		if self.tokens.is_empty() {
			return Err(TypeError::IndexOutOfBounds(
				"This contract does not have any method tokens.".to_string(),
			));
		}
		self.get_token(0)
	}

	pub fn get_token(&self, index: usize) -> Result<&ContractMethodToken, TypeError> {
		if index >= self.tokens.len() {
			return Err(TypeError::IndexOutOfBounds(format!(
				"This contract nef only has {} method tokens.",
				self.tokens.len()
			)));
		}
		Ok(&self.tokens[index])
	}
}
