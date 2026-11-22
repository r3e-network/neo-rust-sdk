// DeFi Module for Neo CLI
//
// This module provides production-backed token commands for Neo N3 DeFi workflows.
//
// Current support includes:
//  - NEP-17 token metadata lookup
//  - NEP-17 balance checks
//  - NEP-17 transfers
//
// Feature Requirements:
//  - The `futures` feature is required for all async operations in this module
//  - `ledger` feature provides optional hardware wallet support

pub mod tokens;
mod types;
pub mod utils;

use crate::{commands::wallet::CliState, errors::CliError};
use clap::Args;
use neo3::prelude::*;
use primitive_types::H160;
use std::{path::PathBuf, str::FromStr}; // Import AddressExtension trait for address_to_scripthash

/// DeFi operations on Neo blockchain
///
/// This module provides commands for interacting with various DeFi protocols
/// available on the Neo blockchain, including token management, swaps, liquidity
/// provision, staking, and more.
#[derive(Args, Debug, Clone)]
pub struct DefiArgs {
	/// Path to wallet file
	#[arg(short, long)]
	pub wallet: Option<PathBuf>,

	/// Wallet password
	#[arg(short, long)]
	pub password: Option<String>,

	#[clap(subcommand)]
	pub command: DefiCommands,
}

/// Defines all DeFi-related commands available in the Neo CLI
///
/// This enum contains commands backed by real RPC calls and signed
/// transactions. Protocol-specific adapters should only be added here when
/// their contract addresses, methods, slippage rules, and signing flows are
/// covered by tests.
#[derive(clap::Subcommand, Debug, Clone)]
pub enum DefiCommands {
	/// Get token information
	Token {
		/// Token contract address or symbol
		contract: String,
	},

	/// Check token balance for an address
	Balance {
		/// Token contract address or symbol
		contract: String,

		/// Address to check balance for
		address: String,
	},

	/// Transfer tokens to an address
	Transfer {
		/// Token contract address or symbol
		token: String,

		/// Destination address
		to: String,

		/// Amount to transfer
		amount: String,

		/// Optional data to include with the transfer
		data: Option<String>,
	},
}

/// Create a ContractParameter from an address or script hash string
///
/// # Arguments
/// * `value` - Address or script hash string
///
/// # Returns
/// * `Result<ContractParameter, CliError>` - ContractParameter with the script hash or error
pub fn create_h160_param(value: &str) -> Result<ContractParameter, CliError> {
	// Try to parse as an address first
	match Address::from_str(value) {
		Ok(address) => {
			// Use address_to_scripthash method instead of address_to_script_hash
			match address.address_to_scripthash() {
				Ok(script_hash) => return Ok(ContractParameter::h160(&script_hash)),
				Err(e) => {
					// Address format is valid but conversion failed
					return Err(CliError::InvalidArgument(
						format!("Failed to convert address to script hash: {}", value),
						e.to_string(),
					));
				},
			}
		},
		Err(_) => {
			// Not an address, try as a script hash
			match H160::from_str(value) {
				Ok(script_hash) => return Ok(ContractParameter::h160(&script_hash)),
				Err(_) => {
					// Try handling common token symbols
					match value.to_uppercase().as_str() {
						"NEO" => {
							return create_h160_param("ef4073a0f2b305a38ec4050e4d3d28bc40ea63f5");
						},
						"GAS" => {
							return create_h160_param("d2a4cff31913016155e38e474a2c06d08be276cf");
						},
						_ => {
							return Err(CliError::InvalidArgument(
								format!("Invalid address or script hash: {}", value),
								"Please provide a valid Neo address or script hash".to_string(),
							));
						},
					}
				},
			}
		},
	}
}

/// Handle DeFi command processing
///
/// This function dispatches DeFi commands to their appropriate handlers
/// based on the provided arguments. It handles wallet loading and authentication
/// before executing the specific command.
///
/// # Arguments
/// * `args` - The DeFi command arguments
/// * `state` - The CLI state containing wallet and RPC client
///
/// # Returns
/// * `Result<(), CliError>` - Success or error
pub async fn handle_defi_command(args: DefiArgs, state: &mut CliState) -> Result<(), CliError> {
	match args.command {
		DefiCommands::Token { contract } => tokens::get_token_info(&contract, state).await,
		DefiCommands::Balance { contract, address } => {
			tokens::get_token_balance(&contract, &address, state).await
		},
		DefiCommands::Transfer { token, to, amount, data: _ } => {
			tokens::transfer_token(&token, &to, &amount, state).await
		},
	}
}
