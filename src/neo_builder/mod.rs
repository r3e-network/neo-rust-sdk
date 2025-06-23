//! # Neo Builder Module (v0.1.8)
//!
//! Advanced tooling for constructing Neo N3 transactions and smart contract scripts.
//!
//! ## Overview
//!
//! The neo_builder module provides a comprehensive set of utilities for constructing
//! and manipulating Neo N3 transactions and scripts. It offers a flexible API for
//! building various types of transactions, from simple transfers to complex
//! multi-signature contract invocations.
//!
//! ## Key Components
//!
//! ### Transaction Building
//!
//! - **Transaction Builder**: Fluent API for creating and configuring transactions
//! - **Fee Calculation**: Automatic network and system fee calculation
//! - **Signer Management**: Support for multiple transaction signers with different scopes
//! - **Witness Configuration**: Tools for creating and managing transaction witnesses
//! - **Attribute Handling**: Support for transaction attributes
//!
//! ### Script Construction
//!
//! - **Script Builder**: Create VM scripts for contract invocation
//! - **Opcode Support**: Full support for Neo VM opcodes
//! - **Parameter Handling**: Type-safe handling of contract parameters
//! - **Verification Scripts**: Utilities for building signature verification scripts
//!
//! ### Advanced Features
//!
//! - **Multi-signature Support**: Create and work with multi-signature accounts
//! - **Helper Methods**: Convenience methods for common operations
//! - **Serialization**: Serialization utilities for network transmission
//!
//! ## Examples
//!
//! ### Building Transactions and Scripts
//!
//! ```no_run
//! use neo3::neo_builder::{ScriptBuilder, Signer, WitnessScope, TransactionSigner};
//! use neo3::neo_types::{ContractParameter, ScriptHash};
//! use std::str::FromStr;
//!
//! fn basic_examples() -> Result<(), Box<dyn std::error::Error>> {
//!     // 1. Create a simple script builder
//!     let mut script_builder = ScriptBuilder::new();
//!     script_builder.push_data("Hello Neo!".as_bytes().to_vec());
//!     let script = script_builder.to_bytes();
//!     println!("Script length: {} bytes", script.len());
//!     
//!     // 2. Create a transaction signer
//!     let script_hash = ScriptHash::from_str("0x1234567890123456789012345678901234567890")?;
//!     let _signer = Signer::TransactionSigner(
//!         TransactionSigner::new(script_hash, vec![WitnessScope::CalledByEntry])
//!     );
//!     
//!     // 3. Example contract call
//!     let mut contract_builder = ScriptBuilder::new();
//!     let gas_token = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
//!     contract_builder.contract_call(
//!         &gas_token,
//!         "balanceOf",
//!         &[ContractParameter::h160(&script_hash)],
//!         None,
//!     )?;
//!     let contract_script = contract_builder.to_bytes();
//!     println!("Contract script length: {} bytes", contract_script.len());
//!     
//!     Ok(())
//! }
//! ```

pub use error::*;
pub use script::*;
pub use transaction::*;
pub use utils::*;

mod error;
mod script;
mod transaction;
mod utils;

pub fn add(left: usize, right: usize) -> usize {
	left + right
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn it_works() {
		let result = add(2, 2);
		assert_eq!(result, 4);
	}
}
