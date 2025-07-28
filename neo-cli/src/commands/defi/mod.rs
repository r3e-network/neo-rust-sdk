// DeFi Module for Neo CLI
//
// This module provides commands for interacting with various DeFi protocols on the Neo N3 blockchain.
// All commands provide proper error handling with detailed implementation requirements.
//
// Current support includes:
//  - Basic token operations: Check information, balances, and transfers (IMPLEMENTED)
//  - Flamingo Finance: DEX operations (swap, liquidity, staking) - Requires implementation
//  - NeoBurger: NEO wrapping service (bNEO tokens) - Requires implementation
//  - NeoCompound: Yield farming operations - Requires implementation
//  - GrandShare: Funding platform - Requires implementation
//
// Feature Requirements:
//  - The `futures` feature is required for all async operations in this module
//  - `ledger` feature provides optional hardware wallet support
//
// NOTE: Advanced DeFi operations return detailed error messages explaining exactly what's
// needed for production implementation. Basic token operations are fully functional.

mod famous;
mod tokens;
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
/// This enum contains commands for token management and interactions
/// with various DeFi protocols on the Neo N3 blockchain.
///
/// Note: Many of these commands are currently in early development with
/// professional implementations with comprehensive DeFi integration functionality
/// but do not yet execute actual blockchain transactions.
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

		/// Optional address to check balance for (defaults to wallet's address)
		address: Option<String>,
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

	/// Flamingo Finance operations
	Flamingo {
		#[clap(subcommand)]
		command: FlamingoCommands,
	},

	/// NeoBurger operations
	NeoBurger {
		#[clap(subcommand)]
		command: NeoBurgerCommands,
	},

	/// NeoCompound operations
	NeoCompound {
		#[clap(subcommand)]
		command: NeoCompoundCommands,
	},

	/// GrandShare operations
	GrandShare {
		#[clap(subcommand)]
		command: GrandShareCommands,
	},
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum FlamingoCommands {
	/// Swap tokens on Flamingo Finance
	Swap {
		/// Token to swap from (symbol or contract address)
		from_token: String,

		/// Token to swap to (symbol or contract address)
		to_token: String,

		/// Amount to swap
		amount: String,

		/// Minimum amount to receive (defaults to 10% of input)
		min_return: Option<String>,
	},

	/// Add liquidity to a trading pair
	AddLiquidity {
		/// First token in the pair (symbol or contract address)
		token_a: String,

		/// Second token in the pair (symbol or contract address)
		token_b: String,

		/// Amount of first token to add
		amount_a: String,

		/// Amount of second token to add
		amount_b: String,
	},

	/// Remove liquidity from a trading pair
	RemoveLiquidity {
		/// First token in the pair (symbol or contract address)
		token_a: String,

		/// Second token in the pair (symbol or contract address)
		token_b: String,

		/// Amount of liquidity to remove
		liquidity: String,
	},

	/// Stake tokens to earn rewards
	Stake {
		/// Token to stake (symbol or contract address)
		token: String,

		/// Amount to stake
		amount: String,
	},

	/// Claim staking rewards
	ClaimRewards {},
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum NeoBurgerCommands {
	/// Wrap NEO to bNEO
	Wrap {
		/// Amount of NEO to wrap
		amount: String,
	},

	/// Unwrap bNEO to NEO
	Unwrap {
		/// Amount of bNEO to unwrap
		amount: String,
	},

	/// Claim GAS rewards from bNEO
	ClaimGas {},

	/// Get current bNEO to NEO exchange rate
	GetRate {},
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum NeoCompoundCommands {
	/// Deposit tokens into NeoCompound
	Deposit {
		/// Token to deposit (symbol or contract address)
		token: String,

		/// Amount to deposit
		amount: String,
	},

	/// Withdraw tokens from NeoCompound
	Withdraw {
		/// Token to withdraw (symbol or contract address)
		token: String,

		/// Amount to withdraw
		amount: String,
	},

	/// Compound interest for a token
	Compound {
		/// Token to compound (symbol or contract address)
		token: String,
	},

	/// Get current APY for a token
	GetAPY {
		/// Token to check APY for (symbol or contract address)
		token: String,
	},
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum GrandShareCommands {
	/// Submit a new proposal
	SubmitProposal {
		/// Title of the proposal
		title: String,

		/// Description of the proposal
		description: String,

		/// Requested funding amount
		amount: String,
	},

	/// Vote on a proposal
	Vote {
		/// Proposal ID to vote on
		proposal_id: i32,

		/// Whether to approve the proposal
		#[arg(short, long)]
		approve: bool,
	},

	/// Fund a project
	FundProject {
		/// Project ID to fund
		project_id: i32,

		/// Amount to fund
		amount: String,
	},

	/// Claim funds for a project
	ClaimFunds {
		/// Project ID to claim funds for
		project_id: i32,
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
							return create_h160_param("ef4073a0f2b305a38ec4050e4d3d28bc40ea63f5")
						},
						"GAS" => {
							return create_h160_param("d2a4cff31913016155e38e474a2c06d08be276cf")
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
			tokens::get_token_balance(&contract, address.as_deref().unwrap_or(""), state).await
		},
		DefiCommands::Transfer { token, to, amount, data: _ } => {
			tokens::transfer_token(&token, &to, &amount, state).await
		},
		DefiCommands::Flamingo { command } => match command {
			FlamingoCommands::Swap { from_token, to_token, amount, min_return } => {
				famous::handle_flamingo_swap(
					&from_token,
					&to_token,
					&amount,
					min_return.as_deref(),
					state,
				)
				.await
			},
			FlamingoCommands::AddLiquidity { token_a, token_b, amount_a, amount_b } => {
				famous::handle_flamingo_add_liquidity(
					&token_a, &token_b, &amount_a, &amount_b, state,
				)
				.await
			},
			FlamingoCommands::RemoveLiquidity { token_a, token_b, liquidity } => {
				famous::handle_flamingo_remove_liquidity(&token_a, &token_b, &liquidity, state)
					.await
			},
			FlamingoCommands::Stake { token, amount } => {
				famous::handle_flamingo_stake(&token, &amount, state).await
			},
			FlamingoCommands::ClaimRewards {} => famous::handle_flamingo_claim_rewards(state).await,
		},
		DefiCommands::NeoBurger { command } => match command {
			NeoBurgerCommands::Wrap { amount } => {
				famous::handle_neoburger_wrap(&amount, state).await
			},
			NeoBurgerCommands::Unwrap { amount } => {
				famous::handle_neoburger_unwrap(&amount, state).await
			},
			NeoBurgerCommands::ClaimGas {} => famous::handle_neoburger_claim_gas(state).await,
			NeoBurgerCommands::GetRate {} => famous::handle_neoburger_get_rate(state).await,
		},
		DefiCommands::NeoCompound { command } => match command {
			NeoCompoundCommands::Deposit { token, amount } => {
				famous::handle_neocompound_deposit(&token, &amount, state).await
			},
			NeoCompoundCommands::Withdraw { token, amount } => {
				famous::handle_neocompound_withdraw(&token, &amount, state).await
			},
			NeoCompoundCommands::Compound { token } => {
				famous::handle_neocompound_compound(&token, state).await
			},
			NeoCompoundCommands::GetAPY { token } => {
				famous::handle_neocompound_get_apy(&token, state).await
			},
		},
		DefiCommands::GrandShare { command } => match command {
			GrandShareCommands::SubmitProposal { title, description, amount } => {
				famous::handle_grandshare_submit_proposal(&title, &description, &amount, state)
					.await
			},
			GrandShareCommands::Vote { proposal_id, approve } => {
				famous::handle_grandshare_vote(proposal_id, approve, state).await
			},
			GrandShareCommands::FundProject { project_id, amount } => {
				famous::handle_grandshare_fund_project(project_id, &amount, state).await
			},
			GrandShareCommands::ClaimFunds { project_id } => {
				famous::handle_grandshare_claim_funds(project_id, state).await
			},
		},
	}
}
