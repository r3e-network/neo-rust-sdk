//! Famous contract addresses and information for Neo N3 mainnet and testnet.
//!
//! This module provides script hashes and information about well-known
//! contracts deployed on Neo N3 networks that developers may want to interact with.

use crate::neo_types::script_hash::ScriptHash;
use std::{fmt, str::FromStr};

use crate::neo_contract::ContractError;

/// Enum defining which Neo N3 network a contract is deployed on
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Network {
	/// Neo N3 Mainnet
	MainNet,
	/// Neo N3 Testnet
	TestNet,
	/// Private network
	PrivateNet,
}

impl fmt::Display for Network {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let label = match self {
			Network::MainNet => "mainnet",
			Network::TestNet => "testnet",
			Network::PrivateNet => "privatenet",
		};
		write!(f, "{}", label)
	}
}

impl FromStr for Network {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"mainnet" => Ok(Network::MainNet),
			"testnet" => Ok(Network::TestNet),
			"privatenet" => Ok(Network::PrivateNet),
			_ => Err(format!("Unknown network: {}", s)),
		}
	}
}

/// Represents a famous contract with its metadata
#[derive(Debug, Clone)]
pub struct FamousContract {
	/// Script hash of the contract
	pub script_hash: ScriptHash,
	/// Human-readable name of the contract
	pub name: String,
	/// Optional description of what the contract does
	pub description: Option<String>,
	/// Network the contract is deployed on
	pub network: Network,
	/// Contract type or category
	pub contract_type: String,
}

impl FamousContract {
	/// Creates a new famous contract instance
	pub fn new(
		script_hash: &str,
		name: &str,
		description: Option<&str>,
		network: Network,
		contract_type: &str,
	) -> Result<Self, ContractError> {
		let script_hash = ScriptHash::from_str(script_hash)
			.map_err(|e| ContractError::InvalidScriptHash(format!("Invalid script hash: {}", e)))?;

		Ok(Self {
			script_hash,
			name: name.to_string(),
			description: description.map(|d| d.to_string()),
			network,
			contract_type: contract_type.to_string(),
		})
	}
}

// Mainnet DeFi contracts
pub fn flamingo_flm_token() -> FamousContract {
	FamousContract::new(
		"0x4d9eab13620fe3569ba3b0e56e2877739e4145e3",
		"Flamingo FLM Token",
		Some("Native governance token for the Flamingo Finance platform"),
		Network::MainNet,
		"NEP-17 Token",
	)
	.unwrap()
}

pub fn flamingo_flamingo_finance() -> FamousContract {
	FamousContract::new(
		"0x1a4e5b62b908c758417eb525ecba58752a947f2b",
		"Flamingo Finance",
		Some("Interoperable, full-stack DeFi protocol on Neo"),
		Network::MainNet,
		"DeFi Platform",
	)
	.unwrap()
}

// Mainnet NFT contracts
pub fn ghostmarket() -> FamousContract {
	FamousContract::new(
		"0xced5862a6c2f0c70b82b8017e845fb1a31c62c9c",
		"GhostMarket",
		Some("Multi-chain NFT marketplace"),
		Network::MainNet,
		"NFT Marketplace",
	)
	.unwrap()
}

pub fn neoburger_dao() -> FamousContract {
	FamousContract::new(
		"0x48c40d4666f93408be1bef038b6722404d9a4c2a",
		"NeoBurger DAO",
		Some("Governance platform and vote delegation for Neo"),
		Network::MainNet,
		"DAO",
	)
	.unwrap()
}

pub fn neocompound() -> FamousContract {
	FamousContract::new(
		"0xcd21f4a5dc6a6da341764e7dc9f15f8b38880f49",
		"NeoCompound",
		Some("Gas staking platform on Neo N3"),
		Network::MainNet,
		"Staking",
	)
	.unwrap()
}

// Mainnet infrastructure contracts
pub fn neo_name_service() -> FamousContract {
	FamousContract::new(
		"0x7a8fcf0392cd625647907afa8e45cc66872b596b",
		"Neo Name Service",
		Some("Domain name service for Neo N3"),
		Network::MainNet,
		"Name Service",
	)
	.unwrap()
}

pub fn bridge_neo_to_eth() -> FamousContract {
	FamousContract::new(
		"0xd8dd5a0871eb44992cda9c6b49b3954206d6c8a5",
		"Poly Network Bridge (Neo)",
		Some("Cross-chain bridge connecting Neo to other blockchains"),
		Network::MainNet,
		"Bridge",
	)
	.unwrap()
}

// Testnet contracts
pub fn testnet_nns() -> FamousContract {
	FamousContract::new(
		"0x50ac1c37690cc2cfc594472833cf57e299e1d367",
		"Testnet NNS",
		Some("Neo Name Service on testnet"),
		Network::TestNet,
		"Name Service",
	)
	.unwrap()
}

pub fn testnet_faucet() -> FamousContract {
	FamousContract::new(
		"0xd65c5d2764b3850a7f7ab14e04f866e9ceab46e1",
		"Testnet Faucet",
		Some("Contract for distributing testnet NEO and GAS"),
		Network::TestNet,
		"Utility",
	)
	.unwrap()
}

/// Returns all famous contracts for the specified network
pub fn get_famous_contracts(network: Network) -> Vec<FamousContract> {
	match network {
		Network::MainNet => vec![
			flamingo_flm_token(),
			flamingo_flamingo_finance(),
			ghostmarket(),
			neoburger_dao(),
			neocompound(),
			neo_name_service(),
			bridge_neo_to_eth(),
		],
		Network::TestNet => vec![testnet_nns(), testnet_faucet()],
		Network::PrivateNet => vec![],
	}
}

/// Returns all famous contracts across all networks
pub fn get_all_famous_contracts() -> Vec<FamousContract> {
	let mut contracts = get_famous_contracts(Network::MainNet);
	contracts.extend(get_famous_contracts(Network::TestNet));
	contracts
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_get_famous_contracts() {
		let mainnet_contracts = get_famous_contracts(Network::MainNet);
		assert!(!mainnet_contracts.is_empty());

		let testnet_contracts = get_famous_contracts(Network::TestNet);
		assert!(!testnet_contracts.is_empty());

		let all_contracts = get_all_famous_contracts();
		assert_eq!(all_contracts.len(), mainnet_contracts.len() + testnet_contracts.len());
	}
}
