//! Hardfork definitions for Neo N3 blockchain.
//!
//! This module defines the various hardforks that have been implemented in the Neo N3 blockchain.
//! Each hardfork represents a set of protocol changes that are activated at a specific block height.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Represents the various hardforks implemented in the Neo N3 blockchain.
///
/// Hardforks are used to introduce new features or fix issues in the protocol.
/// Each hardfork is activated at a specific block height configured in the protocol settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Hardfork {
	/// Aspidochelone hardfork - First hardfork after Neo N3 launch
	#[serde(rename = "HF_Aspidochelone")]
	Aspidochelone = 0,

	/// Basilisk hardfork - Enhanced contract features
	#[serde(rename = "HF_Basilisk")]
	Basilisk = 1,

	/// Cockatrice hardfork - Committee change notifications, Keccak256 support
	#[serde(rename = "HF_Cockatrice")]
	Cockatrice = 2,

	/// Domovoi hardfork - Various optimizations
	#[serde(rename = "HF_Domovoi")]
	Domovoi = 3,

	/// Echidna hardfork - Notary contract, Policy updates (milliseconds per block, etc.)
	#[serde(rename = "HF_Echidna")]
	Echidna = 4,

	/// Faun hardfork - Treasury contract, Whitelist fee contracts, exec fee factor with decimals
	#[serde(rename = "HF_Faun")]
	Faun = 5,

	/// Gorgon hardfork - SHL/SHR fix, VM JumpTable refactoring, updated crypto verification
	#[serde(rename = "HF_Gorgon")]
	Gorgon = 6,

	/// Huyao hardfork - RunStats groundwork for dynamic opcode pricing, NeoGo-aligned
	/// reference counting, SETITEM Buffer fix. Last hardfork activated via protocol
	/// configuration; later hardforks are activated on-chain via Policy's `activateHardfork`.
	/// Introduced with the Neo v3.11.x (neo-vm v3.11.2) release series.
	#[serde(rename = "HF_Huyao")]
	Huyao = 7,

	/// Iara hardfork - First hardfork that can be activated via Policy's `activateHardfork` call.
	#[serde(rename = "HF_Iara")]
	Iara = 8,

	/// Jackalope hardfork - Future protocol enhancements (Policy-activated).
	#[serde(rename = "HF_Jackalope")]
	Jackalope = 9,
}

/// The last hardfork that is activated exclusively via protocol configuration
/// (`hardforks` in the node's config). Hardforks after [`Hardfork::Huyao`] are
/// activated on-chain via a committee call to native Policy's `activateHardfork`.
///
/// Mirrors `ProtocolSettings.LastConfigManagedHardfork` in Neo v3.11.2.
pub const LAST_CONFIG_MANAGED_HARDFORK: Hardfork = Hardfork::Huyao;

impl Hardfork {
	/// Returns all hardforks in order.
	pub fn all() -> &'static [Hardfork] {
		&[
			Hardfork::Aspidochelone,
			Hardfork::Basilisk,
			Hardfork::Cockatrice,
			Hardfork::Domovoi,
			Hardfork::Echidna,
			Hardfork::Faun,
			Hardfork::Gorgon,
			Hardfork::Huyao,
			Hardfork::Iara,
			Hardfork::Jackalope,
		]
	}

	/// Returns `true` if this hardfork is activated via protocol configuration
	/// (hardforks up to and including Huyao), `false` if it can only be
	/// activated on-chain via Policy's `activateHardfork`.
	pub fn is_config_managed(&self) -> bool {
		*self <= LAST_CONFIG_MANAGED_HARDFORK
	}

	/// Returns the on-chain name of the hardfork as used by native Policy
	/// (the enum name without the `HF_` prefix, e.g. `"Iara"`).
	pub fn policy_name(&self) -> &'static str {
		match self {
			Hardfork::Aspidochelone => "Aspidochelone",
			Hardfork::Basilisk => "Basilisk",
			Hardfork::Cockatrice => "Cockatrice",
			Hardfork::Domovoi => "Domovoi",
			Hardfork::Echidna => "Echidna",
			Hardfork::Faun => "Faun",
			Hardfork::Gorgon => "Gorgon",
			Hardfork::Huyao => "Huyao",
			Hardfork::Iara => "Iara",
			Hardfork::Jackalope => "Jackalope",
		}
	}

	/// Returns the hardfork name as used in configuration files.
	pub fn name(&self) -> &'static str {
		match self {
			Hardfork::Aspidochelone => "HF_Aspidochelone",
			Hardfork::Basilisk => "HF_Basilisk",
			Hardfork::Cockatrice => "HF_Cockatrice",
			Hardfork::Domovoi => "HF_Domovoi",
			Hardfork::Echidna => "HF_Echidna",
			Hardfork::Faun => "HF_Faun",
			Hardfork::Gorgon => "HF_Gorgon",
			Hardfork::Huyao => "HF_Huyao",
			Hardfork::Iara => "HF_Iara",
			Hardfork::Jackalope => "HF_Jackalope",
		}
	}

	/// Returns a short description of what this hardfork introduced.
	pub fn description(&self) -> &'static str {
		match self {
			Hardfork::Aspidochelone => "First hardfork after Neo N3 launch",
			Hardfork::Basilisk => "Enhanced contract features",
			Hardfork::Cockatrice => "Committee change notifications, Keccak256 support",
			Hardfork::Domovoi => "Various optimizations",
			Hardfork::Echidna => "Notary contract, Policy updates",
			Hardfork::Faun => "Treasury contract, Whitelist fee contracts",
			Hardfork::Gorgon => "SHL/SHR fix, VM JumpTable refactoring, crypto verification",
			Hardfork::Huyao => {
				"Dynamic opcode pricing groundwork (RunStats), NeoGo-aligned ref counting"
			},
			Hardfork::Iara => "First Policy-activated hardfork (activateHardfork)",
			Hardfork::Jackalope => "Future protocol enhancements (Policy-activated)",
		}
	}
}

impl fmt::Display for Hardfork {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.name())
	}
}

impl FromStr for Hardfork {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"hf_aspidochelone" | "aspidochelone" => Ok(Hardfork::Aspidochelone),
			"hf_basilisk" | "basilisk" => Ok(Hardfork::Basilisk),
			"hf_cockatrice" | "cockatrice" => Ok(Hardfork::Cockatrice),
			"hf_domovoi" | "domovoi" => Ok(Hardfork::Domovoi),
			"hf_echidna" | "echidna" => Ok(Hardfork::Echidna),
			"hf_faun" | "faun" => Ok(Hardfork::Faun),
			"hf_gorgon" | "gorgon" => Ok(Hardfork::Gorgon),
			"hf_huyao" | "huyao" => Ok(Hardfork::Huyao),
			"hf_iara" | "iara" => Ok(Hardfork::Iara),
			"hf_jackalope" | "jackalope" => Ok(Hardfork::Jackalope),
			_ => Err(format!("Unknown hardfork: {}", s)),
		}
	}
}

impl TryFrom<u8> for Hardfork {
	type Error = String;

	fn try_from(value: u8) -> Result<Self, Self::Error> {
		match value {
			0 => Ok(Hardfork::Aspidochelone),
			1 => Ok(Hardfork::Basilisk),
			2 => Ok(Hardfork::Cockatrice),
			3 => Ok(Hardfork::Domovoi),
			4 => Ok(Hardfork::Echidna),
			5 => Ok(Hardfork::Faun),
			6 => Ok(Hardfork::Gorgon),
			7 => Ok(Hardfork::Huyao),
			8 => Ok(Hardfork::Iara),
			9 => Ok(Hardfork::Jackalope),
			_ => Err(format!("Invalid hardfork value: {}", value)),
		}
	}
}

impl From<Hardfork> for u8 {
	fn from(hardfork: Hardfork) -> Self {
		hardfork as u8
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_hardfork_ordering() {
		assert!(Hardfork::Aspidochelone < Hardfork::Basilisk);
		assert!(Hardfork::Basilisk < Hardfork::Cockatrice);
		assert!(Hardfork::Cockatrice < Hardfork::Domovoi);
		assert!(Hardfork::Domovoi < Hardfork::Echidna);
		assert!(Hardfork::Echidna < Hardfork::Faun);
		assert!(Hardfork::Faun < Hardfork::Gorgon);
		assert!(Hardfork::Gorgon < Hardfork::Huyao);
		assert!(Hardfork::Huyao < Hardfork::Iara);
		assert!(Hardfork::Iara < Hardfork::Jackalope);
	}

	#[test]
	fn test_hardfork_byte_values_match_csharp() {
		// Ordinal values must mirror the C# `Hardfork : byte` enum in neo v3.11.2.
		assert_eq!(u8::from(Hardfork::Gorgon), 6);
		assert_eq!(u8::from(Hardfork::Huyao), 7);
		assert_eq!(u8::from(Hardfork::Iara), 8);
		assert_eq!(u8::from(Hardfork::Jackalope), 9);
	}

	#[test]
	fn test_last_config_managed_hardfork() {
		// Neo v3.11.2: hardforks up to (and including) Huyao are config-managed;
		// later hardforks are activated on-chain via Policy.activateHardfork.
		assert_eq!(LAST_CONFIG_MANAGED_HARDFORK, Hardfork::Huyao);
		for hf in Hardfork::all() {
			assert_eq!(hf.is_config_managed(), *hf <= Hardfork::Huyao);
		}
		assert!(Hardfork::Huyao.is_config_managed());
		assert!(!Hardfork::Iara.is_config_managed());
		assert!(!Hardfork::Jackalope.is_config_managed());
	}

	#[test]
	fn test_policy_name() {
		assert_eq!(Hardfork::Iara.policy_name(), "Iara");
		assert_eq!(Hardfork::Huyao.policy_name(), "Huyao");
		assert_eq!(Hardfork::Jackalope.policy_name(), "Jackalope");
	}

	#[test]
	fn test_hardfork_from_str() {
		assert_eq!(Hardfork::from_str("HF_Aspidochelone").unwrap(), Hardfork::Aspidochelone);
		assert_eq!(Hardfork::from_str("aspidochelone").unwrap(), Hardfork::Aspidochelone);
		assert_eq!(Hardfork::from_str("HF_Faun").unwrap(), Hardfork::Faun);
	}

	#[test]
	fn test_hardfork_display() {
		assert_eq!(Hardfork::Aspidochelone.to_string(), "HF_Aspidochelone");
		assert_eq!(Hardfork::Faun.to_string(), "HF_Faun");
	}

	#[test]
	fn test_hardfork_try_from_u8() {
		assert_eq!(Hardfork::try_from(0).unwrap(), Hardfork::Aspidochelone);
		assert_eq!(Hardfork::try_from(5).unwrap(), Hardfork::Faun);
		assert!(Hardfork::try_from(255).is_err());
	}

	#[test]
	fn test_hardfork_all() {
		let all = Hardfork::all();
		assert_eq!(all.len(), 10);
		assert_eq!(all[0], Hardfork::Aspidochelone);
		assert_eq!(all[6], Hardfork::Gorgon);
		assert_eq!(all[7], Hardfork::Huyao);
		assert_eq!(all[8], Hardfork::Iara);
		assert_eq!(all[9], Hardfork::Jackalope);
	}
}
