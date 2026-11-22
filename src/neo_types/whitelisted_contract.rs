//! Whitelisted contract type for fee exemptions.
//!
//! This module defines the WhitelistedContract type which is used by the Policy contract
//! to manage contracts that are exempt from execution fees.
//!
//! Introduced in Neo 3.9 (HF_Faun).

use primitive_types::H160;
use serde::{Deserialize, Serialize};

use crate::neo_types::StackItem;

/// Represents a contract method that is whitelisted for fee exemption.
///
/// When a contract method is whitelisted, invocations of that method will use
/// the specified fixed fee instead of the calculated execution fee.
///
/// This feature was introduced in Neo 3.9 (HF_Faun) to allow the committee
/// to designate certain contract methods as having reduced or zero fees.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhitelistedContract {
	/// The script hash of the whitelisted contract.
	pub contract_hash: H160,

	/// The name of the whitelisted method.
	pub method: String,

	/// The number of arguments the method takes.
	/// This is used to uniquely identify overloaded methods.
	pub arg_count: i32,

	/// The fixed execution fee for this method (in GAS units).
	/// This replaces the calculated execution fee when the method is invoked.
	pub fixed_fee: i64,
}

impl WhitelistedContract {
	/// Creates a new WhitelistedContract.
	pub fn new(contract_hash: H160, method: String, arg_count: i32, fixed_fee: i64) -> Self {
		Self { contract_hash, method, arg_count, fixed_fee }
	}

	/// Attempts to parse a WhitelistedContract from a stack item.
	///
	/// The stack item should be a Struct with 4 elements:
	/// [contract_hash, method, arg_count, fixed_fee]
	pub fn from_stack_item(item: &StackItem) -> Result<Self, String> {
		match item {
			StackItem::Struct { value } | StackItem::Array { value } if value.len() >= 4 => {
				let contract_hash =
					value[0]
						.as_bytes()
						.and_then(|bytes| {
							if bytes.len() == 20 { Some(H160::from_slice(&bytes)) } else { None }
						})
						.ok_or_else(|| "Invalid contract hash".to_string())?;

				let method =
					value[1].as_string().ok_or_else(|| "Invalid method name".to_string())?;

				let arg_count =
					value[2].as_int().ok_or_else(|| "Invalid arg count".to_string())? as i32;

				let fixed_fee = value[3].as_int().ok_or_else(|| "Invalid fixed fee".to_string())?;

				Ok(Self { contract_hash, method, arg_count, fixed_fee })
			},
			_ => Err("Expected Struct or Array with 4 elements".to_string()),
		}
	}
}

impl std::fmt::Display for WhitelistedContract {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"WhitelistedContract {{ contract: 0x{:x}, method: {}, args: {}, fee: {} }}",
			self.contract_hash, self.method, self.arg_count, self.fixed_fee
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_whitelisted_contract_new() {
		let hash = H160::from_low_u64_be(1);
		let contract = WhitelistedContract::new(hash, "transfer".to_string(), 4, 100000);

		assert_eq!(contract.contract_hash, hash);
		assert_eq!(contract.method, "transfer");
		assert_eq!(contract.arg_count, 4);
		assert_eq!(contract.fixed_fee, 100000);
	}

	#[test]
	fn test_whitelisted_contract_display() {
		let hash = H160::zero();
		let contract = WhitelistedContract::new(hash, "test".to_string(), 2, 50000);

		let display = format!("{}", contract);
		assert!(display.contains("test"));
		assert!(display.contains("2"));
		assert!(display.contains("50000"));
	}
}
