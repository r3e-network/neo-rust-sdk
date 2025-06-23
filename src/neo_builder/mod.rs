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
//! use neo3::prelude::*;
//! use neo3::neo_builder::{TransactionBuilder, ScriptBuilder};
//! use neo3::neo_protocol::Account;
//! use neo3::neo_types::{ContractParameter, Signer, WitnessScope};
//! use neo3::neo_clients::{HttpProvider, RpcClient};
//! use std::str::FromStr;
//!
//! async fn transaction_example() -> Result<(), Box<dyn std::error::Error>> {
//!     // Set up connections
//!     let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
//!     let client = RpcClient::new(provider);
//!     
//!     // Create an account for signing
//!     let account = Account::from_wif("KwVEKk78X65fDrJ3VgqHLcpPpbQVfJLjXrkFUCozHQBJ5nT2xwP8")?;
//!     
//!     // Define transaction participants
//!     let sender = account.get_script_hash();
//!     let recipient = ScriptHash::from_str("NUVPACTpQvd2HHmBgFjJJRWwVXJiR3uAEh")?;
//!     
//!     // Get the GAS token hash
//!     let gas_token = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
//!     
//!     // 1. Create a script for transferring GAS tokens
//!     let script = ScriptBuilder::build_contract_call(
//!         &gas_token,
//!         "transfer",
//!         &[
//!             ContractParameter::hash160(&sender),
//!             ContractParameter::hash160(&recipient),
//!             ContractParameter::integer(1_0000_0000), // 1 GAS
//!             ContractParameter::any(None),             // No data
//!         ],
//!     )?;
//!     
//!     // 2. Build a transaction
//!     let current_block = client.get_block_count().await?;
//!     
//!     let tx_builder = TransactionBuilder::new()
//!         .version(0)                                    // Transaction version
//!         .nonce(1234)                                   // Random nonce
//!         .valid_until_block(current_block + 100)        // Expiration block
//!         .script(script)                                // Contract invocation script
//!         .add_signer(Signer::with_scope(                // Transaction signer
//!             sender.clone(),
//!             WitnessScope::CalledByEntry,
//!             vec![],
//!             vec![],
//!             vec![],
//!         ));
//!     
//!     // 3. Calculate fees based on script (can also specify manual values)
//!     let tx_builder = tx_builder.calculate_network_fee(&account).await?;
//!     let tx_builder = tx_builder.calculate_system_fee(client.clone()).await?;
//!     
//!     // 4. Build the transaction and sign it
//!     let unsigned_tx = tx_builder.build();
//!     let signed_tx = unsigned_tx.sign(&client, &account).await?;
//!     
//!     // 5. Send the transaction
//!     let tx_id = client.send_raw_transaction(&signed_tx).await?;
//!     println!("Transaction sent successfully: {}", tx_id);
//!     
//!     // 6. Alternatively, build, sign, and send in a single chain of calls
//!     let quick_tx_id = TransactionBuilder::new()
//!         .version(0)
//!         .nonce(5678)
//!         .valid_until_block(current_block + 100)
//!         .script(script)
//!         .add_signer(Signer::called_by_entry(sender))
//!         .sign_and_send(&client, &account)
//!         .await?;
//!     
//!     println!("Quick transaction sent: {}", quick_tx_id);
//!     
//!     // 7. Advanced script building example
//!     let advanced_script = ScriptBuilder::new()
//!         // Add method arguments in reverse order (Neo VM uses a stack)
//!         .emit_push_string("Hello, Neo!")
//!         .emit_push_integer(123)
//!         .emit_push_byte_array(&recipient.as_bytes())
//!         
//!         // Call the contract method
//!         .emit_app_call(&gas_token, "someMethod", false)
//!         .to_script();
//!     
//!     println!("Advanced script length: {} bytes", advanced_script.len());
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
