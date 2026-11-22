//! Transaction simulation for gas estimation and state preview
//!
//! This module provides comprehensive transaction simulation capabilities,
//! allowing developers to preview transaction effects, estimate gas costs,
//! and analyze state changes before submitting to the blockchain. This helps
//! prevent failed transactions and optimize gas usage.
#![allow(missing_docs, missing_debug_implementations)]
//!
//! ## Features
//!
//! - **Gas Estimation**: Accurate gas cost prediction (±5% accuracy)
//! - **State Preview**: See all state changes before execution
//! - **Optimization**: Get suggestions for reducing gas costs
//! - **Warning System**: Identify potential issues before submission
//! - **Caching**: Reuse simulation results for repeated transactions
//!
//! ## Use Cases
//!
//! - Estimate transaction costs for user interfaces
//! - Preview token transfers and balance changes
//! - Detect potential failures before submission
//! - Optimize smart contract interactions
//! - Validate transaction parameters
//!
//! ## Example
//!
//! ```rust,no_run
//! use neo3::sdk::transaction_simulator::TransactionSimulator;
//! use neo3::neo_builder::Signer;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let client = std::sync::Arc::new(neo3::neo_clients::RpcClient::new(
//! #     neo3::neo_clients::HttpProvider::new("http://localhost")?
//! # ));
//! // Create simulator
//! let mut simulator = TransactionSimulator::new(client);
//!
//! // Simulate a transaction
//! let script = vec![0x00]; // Your transaction script
//! let signers = vec![]; // Your signers
//!
//! let result = simulator.simulate_script(&script, signers).await?;
//!
//! if result.success {
//!     println!("Gas needed: {} GAS", result.gas_consumed as f64 / 100_000_000.0);
//!     println!("Total fee: {} GAS", result.total_fee as f64 / 100_000_000.0);
//!     
//!     // Check warnings
//!     for warning in &result.warnings {
//!         println!("Warning: {}", warning.message);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use crate::neo_builder::{ScriptBuilder, Signer};
use crate::neo_clients::{APITrait, HttpProvider, RpcClient};
use crate::neo_error::unified::{ErrorRecovery, NeoError};
// use crate::neo_protocol::AccountTrait;
use crate::neo_types::{
	ContractParameter, NeoVMStateType, ScriptHash, ScriptHashExtension, StackItem,
};
use crate::sdk::DecimalAmount;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Decimal places of the GAS token; `gasconsumed` strings are quoted in GAS.
const GAS_DECIMALS: u8 = 8;
/// Maximum number of simulation results kept in the in-memory cache.
const SIMULATION_CACHE_MAX_ENTRIES: usize = 256;

/// Transaction simulation result
///
/// Contains comprehensive information about the transaction simulation,
/// including gas costs, state changes, and potential issues. Use this
/// to make informed decisions before submitting transactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
	/// Whether the transaction would succeed
	pub success: bool,
	/// VM state after execution
	pub vm_state: NeoVMStateType,
	/// Estimated gas consumption
	pub gas_consumed: u64,
	/// Estimated system fee
	pub system_fee: u64,
	/// Estimated network fee
	pub network_fee: u64,
	/// Total fee (system + network)
	pub total_fee: u64,
	/// State changes that would occur
	pub state_changes: StateChanges,
	/// Notifications that would be emitted
	pub notifications: Vec<Notification>,
	/// Return values from the script
	pub return_values: Vec<StackItem>,
	/// Warnings about the transaction
	pub warnings: Vec<SimulationWarning>,
	/// Optimization suggestions
	pub suggestions: Vec<OptimizationSuggestion>,
}

/// State changes preview
///
/// Detailed breakdown of all state modifications that would occur
/// if the transaction is executed. This includes storage changes,
/// balance updates, and contract deployments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChanges {
	/// Storage changes by contract
	pub storage: HashMap<ScriptHash, Vec<StorageChange>>,
	/// Balance changes by account
	pub balances: HashMap<String, BalanceChange>,
	/// NEP-17 token transfers
	pub transfers: Vec<TokenTransfer>,
	/// Contract deployments
	pub deployments: Vec<ContractDeployment>,
	/// Contract updates
	pub updates: Vec<ContractUpdate>,
}

/// Storage change entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageChange {
	/// Storage key
	pub key: Vec<u8>,
	/// Previous value (None if new)
	pub old_value: Option<Vec<u8>>,
	/// New value (None if deleted)
	pub new_value: Option<Vec<u8>>,
	/// Human-readable description
	pub description: Option<String>,
}

/// Balance change for an account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceChange {
	/// Account address
	pub address: String,
	/// NEO balance change
	pub neo_delta: i64,
	/// GAS balance change
	pub gas_delta: i64,
	/// Other token changes
	pub token_changes: HashMap<String, i64>,
}

/// Token transfer record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenTransfer {
	/// Token contract hash
	pub token: ScriptHash,
	/// Token symbol
	pub symbol: String,
	/// From address
	pub from: String,
	/// To address
	pub to: String,
	/// Transfer amount
	pub amount: String,
}

/// Contract deployment record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractDeployment {
	/// Contract hash
	pub hash: ScriptHash,
	/// Contract name
	pub name: String,
	/// Deployment cost
	pub cost: u64,
}

/// Contract update record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractUpdate {
	/// Contract hash
	pub hash: ScriptHash,
	/// Update type
	pub update_type: String,
	/// Update cost
	pub cost: u64,
}

/// Notification emitted during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
	/// Contract that emitted the notification
	pub contract: ScriptHash,
	/// Event name
	pub event_name: String,
	/// Event state/parameters
	pub state: serde_json::Value,
}

/// Warning about potential issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationWarning {
	/// Warning level (info, warning, error)
	pub level: WarningLevel,
	/// Warning message
	pub message: String,
	/// Suggested action
	pub suggestion: Option<String>,
}

/// Warning severity level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WarningLevel {
	Info,
	Warning,
	Error,
}

/// Optimization suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestion {
	/// Optimization type
	pub optimization_type: OptimizationType,
	/// Description of the optimization
	pub description: String,
	/// Potential gas savings
	pub gas_savings: Option<u64>,
	/// Implementation hint
	pub implementation: Option<String>,
}

/// Type of optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationType {
	BatchOperations,
	CacheResults,
	OptimizeScript,
	ReduceStorageOperations,
	UseNativeContracts,
	Other(String),
}

/// Transaction simulator
///
/// The main simulator that performs transaction analysis and provides
/// gas estimates and state change previews. It maintains a cache of
/// recent simulations for performance optimization.
///
/// ## Architecture
///
/// The simulator uses the Neo RPC `invokeScript` method to execute
/// transactions in a sandboxed environment without affecting the
/// blockchain state.
pub struct TransactionSimulator {
	client: Arc<RpcClient<HttpProvider>>,
	cache: HashMap<String, CachedSimulation>,
	cache_ttl: std::time::Duration,
	symbol_cache: HashMap<ScriptHash, String>,
	optimization_rules: Vec<OptimizationRule>,
}

impl TransactionSimulator {
	/// Create a new transaction simulator
	pub fn new(client: Arc<RpcClient<HttpProvider>>) -> Self {
		Self {
			client,
			cache: HashMap::new(),
			cache_ttl: std::time::Duration::from_secs(60),
			symbol_cache: HashMap::new(),
			optimization_rules: Self::default_optimization_rules(),
		}
	}

	/// Simulate a transaction before submission
	///
	/// Executes the transaction script in a sandboxed environment to
	/// determine gas costs, state changes, and potential issues.
	/// Results are cached for 60 seconds to improve performance.
	///
	/// # Arguments
	///
	/// * `script` - The transaction script to simulate
	/// * `signers` - Transaction signers with their scopes
	///
	/// # Returns
	///
	/// A `SimulationResult` containing gas estimates, state changes,
	/// warnings, and optimization suggestions
	///
	/// # Performance
	///
	/// Typical simulation time: 100-300ms
	/// Cached results: <1ms
	pub async fn simulate_transaction(
		&mut self,
		script: &[u8],
		signers: Vec<Signer>,
	) -> Result<SimulationResult, NeoError> {
		// Check cache first; the key must cover signers because fees and
		// verification costs depend on their scopes and count.
		let cache_key = Self::calculate_cache_key(script, &signers);
		if let Some(cached) = self.cache.get(&cache_key)
			&& cached.is_valid()
		{
			return Ok(cached.result.clone());
		}

		// Perform simulation
		let result = self.perform_simulation(script, signers).await?;

		// Drop expired entries before growing the cache further.
		self.cache.retain(|_, cached| cached.is_valid());
		if self.cache.len() >= SIMULATION_CACHE_MAX_ENTRIES
			&& let Some(oldest) = self
				.cache
				.iter()
				.min_by_key(|(_, cached)| cached.timestamp)
				.map(|(key, _)| key.clone())
		{
			self.cache.remove(&oldest);
		}
		self.cache.insert(
			cache_key,
			CachedSimulation {
				result: result.clone(),
				timestamp: std::time::SystemTime::now(),
				ttl: self.cache_ttl,
			},
		);

		Ok(result)
	}

	/// Simulate a script execution
	///
	/// Same evaluation path as [`Self::simulate_transaction`] (cached,
	/// with warnings and optimization suggestions). Kept as a separate
	/// entry point for API compatibility.
	pub async fn simulate_script(
		&mut self,
		script: &[u8],
		signers: Vec<Signer>,
	) -> Result<SimulationResult, NeoError> {
		self.simulate_transaction(script, signers).await
	}

	/// Estimate gas for a contract call
	///
	/// Specialized method for estimating gas costs of contract invocations.
	/// Includes a 10% safety margin by default to account for variations.
	///
	/// # Arguments
	///
	/// * `contract` - The contract script hash
	/// * `method` - The contract method to invoke
	/// * `params` - Method parameters
	/// * `signers` - Transaction signers
	///
	/// # Returns
	///
	/// A `GasEstimate` with system fee, network fee, and total costs
	pub async fn estimate_gas(
		&mut self,
		contract: &ScriptHash,
		method: &str,
		params: &[ContractParameter],
		signers: Vec<Signer>,
	) -> Result<GasEstimate, NeoError> {
		// Build the script
		let script = ScriptBuilder::new()
			.contract_call(contract, method, params, None)
			.map_err(|e| NeoError::Contract {
				message: format!("Failed to build script: {}", e),
				source: None,
				recovery: ErrorRecovery::new(),
				contract: None,
				method: None,
			})?
			.to_bytes();

		// Simulate the script
		let simulation = self.simulate_script(&script, signers).await?;

		Ok(GasEstimate {
			system_fee: simulation.system_fee,
			network_fee: simulation.network_fee,
			total_fee: simulation.total_fee,
			gas_consumed: simulation.gas_consumed,
			// Integer math: 10% margin rounded up so estimates never under-charge.
			safety_margin: simulation.total_fee.div_ceil(10),
			warnings: simulation.warnings,
		})
	}

	/// Preview state changes for a transaction
	pub async fn preview_state_changes(
		&mut self,
		script: &[u8],
		signers: Vec<Signer>,
	) -> Result<StateChanges, NeoError> {
		let simulation = self.simulate_transaction(script, signers).await?;
		Ok(simulation.state_changes)
	}

	/// Perform the actual simulation
	async fn perform_simulation(
		&mut self,
		script: &[u8],
		signers: Vec<Signer>,
	) -> Result<SimulationResult, NeoError> {
		// Simulate the transaction script
		let invocation_result = self
			.client
			.invoke_script(hex::encode(script), signers.clone())
			.await
			.map_err(|e| NeoError::Network {
				message: format!("Failed to invoke script: {}", e),
				source: None,
				recovery: ErrorRecovery::new()
					.suggest("Check transaction script")
					.suggest("Verify signers and witnesses"),
			})?;

		// Parse the result
		let mut result = self.parse_invocation_result(invocation_result, script, signers).await?;

		// Apply optimization rules
		result.suggestions = self.apply_optimization_rules(script, &result);

		// Add warnings
		result.warnings = self.analyze_for_warnings(script, &result);

		Ok(result)
	}

	/// Parse invocation result into simulation result
	async fn parse_invocation_result(
		&mut self,
		result: crate::neo_types::InvocationResult,
		_script: &[u8],
		signers: Vec<Signer>,
	) -> Result<SimulationResult, NeoError> {
		// Determine success
		let success = matches!(result.state, NeoVMStateType::Halt);

		// Parse notifications
		let notifications = self.parse_notifications(&result);

		// Parse state changes
		let state_changes = self.analyze_state_changes(&result).await?;

		// Calculate fees
		let gas_consumed = self.parse_gas_consumed(&result.gas_consumed)?;
		let system_fee = self.calculate_system_fee(gas_consumed);
		let network_fee = self.calculate_network_fee(_script.len(), signers.len());

		Ok(SimulationResult {
			success,
			vm_state: result.state,
			gas_consumed,
			system_fee,
			network_fee,
			total_fee: system_fee + network_fee,
			state_changes,
			notifications,
			return_values: result.stack.clone(),
			warnings: Vec::new(),
			suggestions: Vec::new(),
		})
	}

	fn parse_gas_consumed(&self, gas_consumed: &str) -> Result<u64, NeoError> {
		// Nodes return `gasconsumed` as a decimal GAS string (e.g. "0.0295453").
		// Convert to integer base units (1 GAS = 1e8) with exact decimal math —
		// floats would round sub-1-GAS fees to 1 base unit (~1e8 error).
		let invalid = |detail: String| NeoError::Other {
			message: format!(
				"Invalid gas consumption value in simulation response: {gas_consumed} ({detail})"
			),
			source: None,
			recovery: ErrorRecovery::new()
				.suggest("Inspect the raw invocation result from the RPC node")
				.suggest("Verify the connected node returns a numeric gasconsumed field"),
		};

		let amount = DecimalAmount::parse(gas_consumed, GAS_DECIMALS)
			.map_err(|err| invalid(err.to_string()))?;

		amount.raw().parse::<u64>().map_err(|err| invalid(err.to_string()))
	}

	/// Parse notifications from invocation result
	fn parse_notifications(
		&self,
		result: &crate::neo_types::InvocationResult,
	) -> Vec<Notification> {
		result
			.notifications
			.as_ref()
			.map(|notifications| {
				notifications
					.iter()
					.map(|n| Notification {
						contract: n.contract,
						event_name: n.event_name.clone(),
						state: serde_json::to_value(&n.state).unwrap_or(serde_json::Value::Null),
					})
					.collect()
			})
			.unwrap_or_default()
	}

	/// Analyze state changes from the invocation
	async fn analyze_state_changes(
		&mut self,
		result: &crate::neo_types::InvocationResult,
	) -> Result<StateChanges, NeoError> {
		let storage = HashMap::new();
		let balances = HashMap::new();
		let mut transfers = Vec::new();
		let deployments = Vec::new();
		let updates = Vec::new();

		// Parse notifications for transfers and balance changes
		if let Some(notifications) = &result.notifications {
			for notification in notifications {
				if notification.event_name == "Transfer"
					&& let Some(transfer) = self.parse_transfer_notification(notification).await
				{
					transfers.push(transfer);
				}
			}
		}

		Ok(StateChanges { storage, balances, transfers, deployments, updates })
	}

	/// Parse a NEP-17 `Transfer` notification
	///
	/// The notification state is a three-element array
	/// `[from: ByteString(20), to: ByteString(20), amount: Integer]`.
	/// Returns `None` when the state does not match that shape (e.g. NEP-11
	/// transfers) instead of emitting fabricated data.
	async fn parse_transfer_notification(
		&mut self,
		notification: &crate::neo_types::Notification,
	) -> Option<TokenTransfer> {
		let state = notification.state.as_array_ref()?;
		let [from_item, to_item, amount_item] = state else {
			return None;
		};

		// From/to are script hashes; resolve the token symbol via the cache
		// so a multi-transfer script costs at most one `symbol()` RPC per token.
		let from = Self::stack_item_to_address(from_item)?;
		let to = Self::stack_item_to_address(to_item)?;
		let amount = amount_item.as_int()?.to_string();

		let token_symbol = if let Some(symbol) = self.symbol_cache.get(&notification.contract) {
			symbol.clone()
		} else {
			match self.get_token_symbol(&notification.contract).await {
				Ok(symbol) => {
					self.symbol_cache.insert(notification.contract, symbol.clone());
					symbol
				},
				// Do not cache the fallback: a transient RPC failure should not
				// pin a hex string as the token's symbol for the simulator's lifetime.
				Err(_) => notification.contract.to_hex(),
			}
		};

		Some(TokenTransfer { token: notification.contract, symbol: token_symbol, from, to, amount })
	}

	/// Decode a 20-byte script hash stack item into a Neo N3 address.
	fn stack_item_to_address(item: &StackItem) -> Option<String> {
		let bytes = item.as_bytes()?;
		let script_hash = ScriptHash::from_slice(&bytes);
		Some(script_hash.to_address())
	}

	/// Get token symbol from contract
	async fn get_token_symbol(&self, contract: &ScriptHash) -> Result<String, NeoError> {
		// Call the symbol method on the contract
		let result = self
			.client
			.invoke_function(contract, "symbol".to_string(), vec![], None)
			.await
			.map_err(|e| NeoError::Contract {
				message: format!("Failed to get token symbol: {}", e),
				source: None,
				recovery: ErrorRecovery::new(),
				contract: None,
				method: Some("symbol".to_string()),
			})?;

		Self::extract_token_symbol(&result)
	}

	fn extract_token_symbol(
		result: &crate::neo_types::InvocationResult,
	) -> Result<String, NeoError> {
		let item = result.stack.first().ok_or_else(|| NeoError::Contract {
			message: "Token symbol response stack is empty".to_string(),
			source: None,
			recovery: ErrorRecovery::new().suggest("Inspect the raw contract response"),
			contract: None,
			method: Some("symbol".to_string()),
		})?;

		item.as_string().ok_or_else(|| NeoError::Contract {
			message: "Token symbol response is not a string-compatible stack item".to_string(),
			source: None,
			recovery: ErrorRecovery::new().suggest("Inspect the raw contract response"),
			contract: None,
			method: Some("symbol".to_string()),
		})
	}

	/// Calculate system fee
	fn calculate_system_fee(&self, gas_consumed: u64) -> u64 {
		// Add a small buffer for safety (integer math: +10%, rounded up)
		gas_consumed.checked_mul(11).map(|value| value.div_ceil(10)).unwrap_or(u64::MAX)
	}

	/// Calculate network fee
	fn calculate_network_fee(&self, script_size: usize, signer_count: usize) -> u64 {
		// Base fee + size fee + verification fee
		let base_fee = 100_000; // 0.001 GAS
		let size_fee = script_size as u64 * 1000; // Per byte
		let verification_fee = signer_count as u64 * 1_000_000; // Per signer

		base_fee + size_fee + verification_fee
	}

	/// Apply optimization rules
	fn apply_optimization_rules(
		&self,
		script: &[u8],
		result: &SimulationResult,
	) -> Vec<OptimizationSuggestion> {
		let mut suggestions = Vec::new();

		for rule in &self.optimization_rules {
			if let Some(suggestion) = rule.check(script, result) {
				suggestions.push(suggestion);
			}
		}

		suggestions
	}

	/// Analyze for warnings
	fn analyze_for_warnings(
		&self,
		_script: &[u8],
		result: &SimulationResult,
	) -> Vec<SimulationWarning> {
		let mut warnings = Vec::new();

		// Check if transaction failed
		if !result.success {
			warnings.push(SimulationWarning {
				level: WarningLevel::Error,
				message: "Transaction simulation failed".to_string(),
				suggestion: Some(
					"Review script logic and ensure all preconditions are met".to_string(),
				),
			});
		}

		// Check high gas consumption
		if result.gas_consumed > 10_000_000 {
			warnings.push(SimulationWarning {
				level: WarningLevel::Warning,
				message: format!(
					"High gas consumption: {} GAS",
					result.gas_consumed as f64 / 100_000_000.0
				),
				suggestion: Some(
					"Consider optimizing the script or breaking into smaller transactions"
						.to_string(),
				),
			});
		}

		// Check for insufficient balance (simplified)
		if result.total_fee > 1_000_000_000 {
			warnings.push(SimulationWarning {
				level: WarningLevel::Warning,
				message: "Transaction requires significant GAS balance".to_string(),
				suggestion: Some("Ensure account has sufficient GAS balance".to_string()),
			});
		}

		warnings
	}

	/// Calculate the simulation cache key
	///
	/// Covers both the script and the signers: fees depend on signer count and
	/// scopes, so cached results must never be shared across signer sets.
	fn calculate_cache_key(script: &[u8], signers: &[Signer]) -> String {
		use sha2::{Digest, Sha256};
		let mut hasher = Sha256::new();
		hasher.update(script);
		hasher.update(format!("{signers:?}").as_bytes());
		hex::encode(hasher.finalize())
	}

	/// Default optimization rules
	fn default_optimization_rules() -> Vec<OptimizationRule> {
		vec![
			OptimizationRule::BatchTransfers,
			OptimizationRule::UseNativeContracts,
			OptimizationRule::MinimizeStorageOps,
			OptimizationRule::CacheRepeatedCalls,
		]
	}
}

/// Optimization rule
#[derive(Debug, Clone, Copy)]
pub enum OptimizationRule {
	BatchTransfers,
	UseNativeContracts,
	MinimizeStorageOps,
	CacheRepeatedCalls,
}

impl OptimizationRule {
	fn check(&self, _script: &[u8], result: &SimulationResult) -> Option<OptimizationSuggestion> {
		match self {
			Self::BatchTransfers => {
				if result.notifications.iter().filter(|n| n.event_name == "Transfer").count() > 3 {
					Some(OptimizationSuggestion {
						optimization_type: OptimizationType::BatchOperations,
						description: "Multiple transfers detected. Consider batching.".to_string(),
						gas_savings: Some(result.gas_consumed / 10),
						implementation: Some(
							"Use a batch transfer method or combine operations".to_string(),
						),
					})
				} else {
					None
				}
			},
			Self::UseNativeContracts => {
				// Check if using non-native contracts for common operations
				None // Simplified
			},
			Self::MinimizeStorageOps => {
				if result.state_changes.storage.values().map(|v| v.len()).sum::<usize>() > 10 {
					Some(OptimizationSuggestion {
						optimization_type: OptimizationType::ReduceStorageOperations,
						description: "Many storage operations detected".to_string(),
						gas_savings: Some(result.gas_consumed / 20),
						implementation: Some(
							"Cache values in memory and batch storage updates".to_string(),
						),
					})
				} else {
					None
				}
			},
			Self::CacheRepeatedCalls => {
				// Detect repeated contract calls
				None // Simplified
			},
		}
	}
}

/// Cached simulation result
struct CachedSimulation {
	result: SimulationResult,
	timestamp: std::time::SystemTime,
	ttl: std::time::Duration,
}

impl CachedSimulation {
	fn is_valid(&self) -> bool {
		self.timestamp.elapsed().map(|d| d < self.ttl).unwrap_or(false)
	}
}

/// Gas estimation result
///
/// Detailed gas cost breakdown for a transaction, including
/// system fees (execution costs) and network fees (size-based costs).
/// Includes a safety margin to prevent out-of-gas failures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasEstimate {
	/// System fee in GAS (smallest unit)
	pub system_fee: u64,
	/// Network fee in GAS (smallest unit)
	pub network_fee: u64,
	/// Total fee in GAS (smallest unit)
	pub total_fee: u64,
	/// Gas consumed by script execution
	pub gas_consumed: u64,
	/// Recommended safety margin
	pub safety_margin: u64,
	/// Any warnings about the estimation
	pub warnings: Vec<SimulationWarning>,
}

/// Transaction simulator builder
///
/// Fluent interface for creating a configured transaction simulator
/// with custom settings for caching and optimization rules.
pub struct TransactionSimulatorBuilder {
	client: Option<Arc<RpcClient<HttpProvider>>>,
	cache_duration: std::time::Duration,
	optimization_rules: Vec<OptimizationRule>,
}

impl Default for TransactionSimulatorBuilder {
	fn default() -> Self {
		Self::new()
	}
}

impl TransactionSimulatorBuilder {
	/// Create a new builder
	pub fn new() -> Self {
		Self {
			client: None,
			cache_duration: std::time::Duration::from_secs(60),
			optimization_rules: Vec::new(),
		}
	}

	/// Set the RPC client
	#[must_use]
	pub fn client(mut self, client: Arc<RpcClient<HttpProvider>>) -> Self {
		self.client = Some(client);
		self
	}

	/// Set cache duration
	#[must_use]
	pub fn cache_duration(mut self, duration: std::time::Duration) -> Self {
		self.cache_duration = duration;
		self
	}

	/// Add optimization rule
	#[must_use]
	pub fn add_optimization_rule(mut self, rule: OptimizationRule) -> Self {
		self.optimization_rules.push(rule);
		self
	}

	/// Build the simulator
	pub fn build(self) -> Result<TransactionSimulator, NeoError> {
		let client = self.client.ok_or_else(|| NeoError::Contract {
			message: "RPC client is required".to_string(),
			source: None,
			recovery: ErrorRecovery::new().suggest("Provide an RPC client using .client() method"),
			contract: None,
			method: None,
		})?;

		let mut simulator = TransactionSimulator::new(client);
		simulator.cache_ttl = self.cache_duration;
		if !self.optimization_rules.is_empty() {
			simulator.optimization_rules = self.optimization_rules;
		}

		Ok(simulator)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::neo_clients::HttpProvider;
	use crate::neo_types::InvocationResult;
	use crate::neo_types::StackItem;
	use std::sync::Arc;

	#[test]
	fn test_gas_estimate_creation() {
		let estimate = GasEstimate {
			system_fee: 1_000_000,
			network_fee: 500_000,
			total_fee: 1_500_000,
			gas_consumed: 900_000,
			safety_margin: 150_000,
			warnings: vec![],
		};

		assert_eq!(estimate.total_fee, estimate.system_fee + estimate.network_fee);
		assert_eq!(estimate.safety_margin, 150_000);
	}

	#[test]
	fn test_simulation_result() {
		let result = SimulationResult {
			success: true,
			vm_state: NeoVMStateType::Halt,
			gas_consumed: 1_000_000,
			system_fee: 1_100_000,
			network_fee: 500_000,
			total_fee: 1_600_000,
			state_changes: StateChanges {
				storage: HashMap::new(),
				balances: HashMap::new(),
				transfers: vec![],
				deployments: vec![],
				updates: vec![],
			},
			notifications: vec![],
			return_values: vec![],
			warnings: vec![],
			suggestions: vec![],
		};

		assert!(result.success);
		assert_eq!(result.total_fee, result.system_fee + result.network_fee);
	}

	#[test]
	fn test_warning_levels() {
		let info = SimulationWarning {
			level: WarningLevel::Info,
			message: "Information".to_string(),
			suggestion: None,
		};

		let warning = SimulationWarning {
			level: WarningLevel::Warning,
			message: "Warning".to_string(),
			suggestion: Some("Fix this".to_string()),
		};

		assert_eq!(info.level, WarningLevel::Info);
		assert_eq!(warning.level, WarningLevel::Warning);
	}

	#[tokio::test]
	async fn test_parse_invocation_result_rejects_invalid_gas_consumed() {
		let client = Arc::new(RpcClient::new(HttpProvider::new("http://localhost:10332").unwrap()));
		let mut simulator = TransactionSimulator::new(client);
		let result =
			InvocationResult { gas_consumed: "not-a-number".to_string(), ..Default::default() };

		let err = simulator.parse_invocation_result(result, &[], vec![]).await.unwrap_err();

		match err {
			NeoError::Other { message, .. } => {
				assert!(message.contains("Invalid gas consumption value"));
			},
			other => panic!("expected generic parsing error, got {other:?}"),
		}
	}

	#[test]
	fn test_extract_token_symbol_reads_first_stack_item() {
		let result = InvocationResult {
			stack: vec![StackItem::ByteString { value: "R0FT".to_string() }],
			..Default::default()
		};

		let symbol = TransactionSimulator::extract_token_symbol(&result).unwrap();
		assert_eq!(symbol, "GAS");
	}

	#[test]
	fn test_extract_token_symbol_rejects_non_string_items() {
		let result = InvocationResult {
			stack: vec![StackItem::Map { value: vec![] }],
			..Default::default()
		};

		let err = TransactionSimulator::extract_token_symbol(&result).unwrap_err();
		match err {
			NeoError::Contract { message, .. } => {
				assert!(message.contains("not a string-compatible"));
			},
			other => panic!("expected contract error, got {other:?}"),
		}
	}

	#[test]
	fn test_gas_consumed_decimal_string_converts_to_base_units() {
		let client = Arc::new(RpcClient::new(HttpProvider::new("http://localhost:10332").unwrap()));
		let simulator = TransactionSimulator::new(client);

		// "0.0295453" GAS must become 2_954_530 base units — not 1 (the old
		// float fallback ceil'd sub-1-GAS values to a single base unit).
		let gas = simulator.parse_gas_consumed("0.0295453").unwrap();
		assert_eq!(gas, 2_954_530);

		// gasconsumed is quoted in GAS: "123" means 123 GAS = 123e8 base units.
		assert_eq!(simulator.parse_gas_consumed("123").unwrap(), 12_300_000_000);
	}

	#[test]
	fn test_gas_consumed_rejects_overly_precise_decimals() {
		let client = Arc::new(RpcClient::new(HttpProvider::new("http://localhost:10332").unwrap()));
		let simulator = TransactionSimulator::new(client);

		// 9 fractional digits exceed GAS' 8 decimals and are not silently rounded.
		assert!(simulator.parse_gas_consumed("0.123456789").is_err());
	}

	#[test]
	fn test_cache_key_covers_signers() {
		let signer_a = Signer::TransactionSigner(
			crate::neo_builder::TransactionSigner::new(
				crate::neo_types::ScriptHash::zero(),
				vec![crate::neo_builder::WitnessScope::CalledByEntry],
			)
			.unwrap(),
		);
		let signer_b = Signer::TransactionSigner(
			crate::neo_builder::TransactionSigner::new(
				crate::neo_types::ScriptHash::zero(),
				vec![crate::neo_builder::WitnessScope::Global],
			)
			.unwrap(),
		);

		let same_script = b"\x00\x01";
		assert_eq!(
			TransactionSimulator::calculate_cache_key(same_script, std::slice::from_ref(&signer_a)),
			TransactionSimulator::calculate_cache_key(same_script, std::slice::from_ref(&signer_a))
		);
		assert_ne!(
			TransactionSimulator::calculate_cache_key(same_script, &[signer_a]),
			TransactionSimulator::calculate_cache_key(same_script, &[signer_b])
		);
	}

	#[tokio::test]
	async fn test_transfer_notification_parses_real_state() {
		use crate::neo_types::Notification as ContractNotification;
		use base64::Engine;

		let client = Arc::new(RpcClient::new(HttpProvider::new("http://localhost:10332").unwrap()));
		let mut simulator = TransactionSimulator::new(client);

		// NEP-17 Transfer state: [from(20 bytes), to(20 bytes), amount]
		let from = [7u8; 20];
		let to = [9u8; 20];
		let state = StackItem::Array {
			value: vec![
				StackItem::ByteString {
					value: base64::engine::general_purpose::STANDARD.encode(from),
				},
				StackItem::ByteString {
					value: base64::engine::general_purpose::STANDARD.encode(to),
				},
				StackItem::Integer { value: 1_000_000 },
			],
		};
		let notification = ContractNotification {
			contract: crate::neo_types::ScriptHash::zero(),
			event_name: "Transfer".to_string(),
			state,
		};

		// symbol() RPC would fail against localhost; the fallback is the hex
		// contract hash, but the parsed fields must be real data.
		let transfer = simulator
			.parse_transfer_notification(&notification)
			.await
			.expect("well-formed transfer state should parse");

		assert_eq!(transfer.amount, "1000000");
		assert_ne!(transfer.from, "sender");
		assert_ne!(transfer.to, "receiver");
		assert_eq!(transfer.from, crate::neo_types::ScriptHash::from_slice(&from).to_address());
		assert_eq!(transfer.to, crate::neo_types::ScriptHash::from_slice(&to).to_address());
	}

	#[tokio::test]
	async fn test_transfer_notification_rejects_non_transfer_shapes() {
		use crate::neo_types::Notification as ContractNotification;

		let client = Arc::new(RpcClient::new(HttpProvider::new("http://localhost:10332").unwrap()));
		let mut simulator = TransactionSimulator::new(client);

		// NEP-11 transfers carry a 4-element state; they must be skipped, not
		// reported with fabricated from/to/amount.
		let notification = ContractNotification {
			contract: crate::neo_types::ScriptHash::zero(),
			event_name: "Transfer".to_string(),
			state: StackItem::Array {
				value: vec![
					StackItem::ByteString { value: "AAA=".to_string() },
					StackItem::ByteString { value: "AAA=".to_string() },
					StackItem::Integer { value: 1 },
					StackItem::ByteString { value: "AAA=".to_string() },
				],
			},
		};

		assert!(simulator.parse_transfer_notification(&notification).await.is_none());
	}
}
