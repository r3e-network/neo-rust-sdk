use getset::Getters;
use serde::{Deserialize, Serialize};

use neo3::prelude::ContractParameterType;

/// Represents a NEP-6 contract.
#[derive(Clone, Debug, Serialize, Deserialize, Getters)]
pub struct NEP6Contract {
	/// The script associated with the contract.
	#[getset(get = "pub")]
	#[serde(rename = "script")]
	pub script: Option<String>,

	/// Indicates whether the contract is deployed.
	#[getset(get = "pub")]
	#[serde(rename = "deployed")]
	pub is_deployed: bool,

	/// The NEP-6 parameters associated with the contract.
	#[getset(get = "pub")]
	#[serde(rename = "parameters")]
	pub nep6_parameters: Vec<NEP6Parameter>,
}

/// Represents a NEP-6 parameter.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Getters)]
pub struct NEP6Parameter {
	/// The name of the parameter.
	#[getset(get = "pub")]
	#[serde(rename = "name")]
	pub param_name: String,

	/// The type of the parameter.
	#[getset(get = "pub")]
	#[serde(rename = "type")]
	pub param_type: ContractParameterType,
}

impl PartialEq for NEP6Contract {
	/// Checks if two `NEP6Contract` instances are equal.
	///
	/// # Example
	///
	/// ```
	/// use neo3::prelude::*;
	///
	/// # let contract1 = wallets::NEP6Contract { script: None, is_deployed: false, nep6_parameters: vec![] };
	/// # let contract2 = wallets::NEP6Contract { script: None, is_deployed: false, nep6_parameters: vec![] };
	/// assert_eq!(contract1, contract2);
	/// ```
	fn eq(&self, other: &Self) -> bool {
		self.script == other.script
			&& self.nep6_parameters == other.nep6_parameters
			&& self.is_deployed == other.is_deployed
	}
}
