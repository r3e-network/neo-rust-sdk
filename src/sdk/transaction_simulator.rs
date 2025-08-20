//! Transaction simulation for gas estimation and state preview
//! 
//! Provides functionality to simulate transactions before submission,
//! allowing developers to estimate gas costs and preview state changes.

use crate::neo_error::unified::{NeoError, ErrorRecovery};
use crate::neo_types::{ScriptHash, ContractParameter, StackItem, VMState};
use crate::neo_builder::{TransactionBuilder, ScriptBuilder, Transaction, Signer};
use crate::neo_clients::{RpcClient, HttpProvider, APITrait};
use crate::neo_protocol::{Account, AccountTrait};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Transaction simulation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    /// Whether the transaction would succeed
    pub success: bool,
    /// VM state after execution
    pub vm_state: VMState,
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
pub struct TransactionSimulator {
    client: Arc<RpcClient<HttpProvider>>,
    cache: HashMap<String, CachedSimulation>,
    optimization_rules: Vec<OptimizationRule>,
}

impl TransactionSimulator {
    /// Create a new transaction simulator
    pub fn new(client: Arc<RpcClient<HttpProvider>>) -> Self {
        Self {
            client,
            cache: HashMap::new(),
            optimization_rules: Self::default_optimization_rules(),
        }
    }

    /// Simulate a transaction before submission
    pub async fn simulate_transaction(
        &mut self,
        tx: &Transaction<'_>,
    ) -> Result<SimulationResult, NeoError> {
        // Check cache first
        let tx_hash = self.calculate_tx_hash(tx);
        if let Some(cached) = self.cache.get(&tx_hash) {
            if cached.is_valid() {
                return Ok(cached.result.clone());
            }
        }

        // Perform simulation
        let result = self.perform_simulation(tx).await?;
        
        // Cache the result
        self.cache.insert(tx_hash, CachedSimulation {
            result: result.clone(),
            timestamp: std::time::SystemTime::now(),
        });

        Ok(result)
    }

    /// Simulate a script execution
    pub async fn simulate_script(
        &mut self,
        script: &[u8],
        signers: Vec<Signer>,
    ) -> Result<SimulationResult, NeoError> {
        // Use invokecontractverify RPC method
        let result = self.client.invoke_script(
            script,
            signers.clone(),
        ).await.map_err(|e| NeoError::Network {
            message: format!("Failed to simulate script: {}", e),
            source: None,
            recovery: ErrorRecovery::new()
                .suggest("Check script validity")
                .suggest("Verify signers have sufficient balance"),
        })?;

        // Parse the invocation result
        self.parse_invocation_result(result, script, signers).await
    }

    /// Estimate gas for a contract call
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
            })?
            .to_bytes();

        // Simulate the script
        let simulation = self.simulate_script(&script, signers).await?;

        Ok(GasEstimate {
            system_fee: simulation.system_fee,
            network_fee: simulation.network_fee,
            total_fee: simulation.total_fee,
            gas_consumed: simulation.gas_consumed,
            safety_margin: (simulation.total_fee as f64 * 0.1) as u64, // 10% safety margin
            warnings: simulation.warnings,
        })
    }

    /// Preview state changes for a transaction
    pub async fn preview_state_changes(
        &mut self,
        tx: &Transaction<'_>,
    ) -> Result<StateChanges, NeoError> {
        let simulation = self.simulate_transaction(tx).await?;
        Ok(simulation.state_changes)
    }

    /// Perform the actual simulation
    async fn perform_simulation(&self, tx: &Transaction<'_>) -> Result<SimulationResult, NeoError> {
        // Get current blockchain state
        let block_height = self.client.get_block_count().await.map_err(|e| NeoError::Network {
            message: format!("Failed to get block height: {}", e),
            source: None,
            recovery: ErrorRecovery::new(),
        })?;

        // Simulate the transaction script
        let invocation_result = self.client.invoke_script(
            &tx.script,
            tx.signers.clone(),
        ).await.map_err(|e| NeoError::Network {
            message: format!("Failed to invoke script: {}", e),
            source: None,
            recovery: ErrorRecovery::new()
                .suggest("Check transaction script")
                .suggest("Verify signers and witnesses"),
        })?;

        // Parse the result
        let mut result = self.parse_invocation_result(
            invocation_result,
            &tx.script,
            tx.signers.clone(),
        ).await?;

        // Apply optimization rules
        result.suggestions = self.apply_optimization_rules(tx, &result);

        // Add warnings
        result.warnings = self.analyze_for_warnings(tx, &result);

        Ok(result)
    }

    /// Parse invocation result into simulation result
    async fn parse_invocation_result(
        &self,
        result: crate::neo_types::InvocationResult,
        script: &[u8],
        signers: Vec<Signer>,
    ) -> Result<SimulationResult, NeoError> {
        // Determine success
        let success = matches!(result.state, VMState::Halt);

        // Parse notifications
        let notifications = self.parse_notifications(&result);

        // Parse state changes
        let state_changes = self.analyze_state_changes(&result, script).await?;

        // Calculate fees
        let gas_consumed = result.gas_consumed.parse::<u64>().unwrap_or(0);
        let system_fee = self.calculate_system_fee(gas_consumed);
        let network_fee = self.calculate_network_fee(script.len(), signers.len());
        
        Ok(SimulationResult {
            success,
            vm_state: result.state,
            gas_consumed,
            system_fee,
            network_fee,
            total_fee: system_fee + network_fee,
            state_changes,
            notifications,
            return_values: result.stack.unwrap_or_default(),
            warnings: Vec::new(),
            suggestions: Vec::new(),
        })
    }

    /// Parse notifications from invocation result
    fn parse_notifications(&self, result: &crate::neo_types::InvocationResult) -> Vec<Notification> {
        result.notifications
            .as_ref()
            .map(|notifications| {
                notifications.iter().map(|n| Notification {
                    contract: n.contract.clone(),
                    event_name: n.event_name.clone(),
                    state: serde_json::to_value(&n.state).unwrap_or(serde_json::Value::Null),
                }).collect()
            })
            .unwrap_or_default()
    }

    /// Analyze state changes from the invocation
    async fn analyze_state_changes(
        &self,
        result: &crate::neo_types::InvocationResult,
        script: &[u8],
    ) -> Result<StateChanges, NeoError> {
        let mut storage = HashMap::new();
        let mut balances = HashMap::new();
        let mut transfers = Vec::new();
        let deployments = Vec::new();
        let updates = Vec::new();

        // Parse notifications for transfers and balance changes
        if let Some(notifications) = &result.notifications {
            for notification in notifications {
                if notification.event_name == "Transfer" {
                    // Parse NEP-17 transfer
                    let parsed_notification = Notification {
                        contract: notification.contract.clone(),
                        event_name: notification.event_name.clone(),
                        state: serde_json::to_value(&notification.state).unwrap_or(serde_json::Value::Null),
                    };
                    if let Some(transfer) = self.parse_transfer_notification(&parsed_notification).await {
                        transfers.push(transfer);
                        
                        // Update balance changes
                        // This is simplified - in production, track actual amounts
                        // In a real implementation, we would parse the StackItem properly
                    }
                }
            }
        }

        Ok(StateChanges {
            storage,
            balances,
            transfers,
            deployments,
            updates,
        })
    }

    /// Parse a transfer notification  
    async fn parse_transfer_notification(&self, notification: &Notification) -> Option<TokenTransfer> {
        // Get token info
        let token_symbol = self.get_token_symbol(&notification.contract).await.unwrap_or_else(|_| "UNKNOWN".to_string());
        
        // Parse the state JSON - in a real implementation this would properly parse the StackItem
        // For now, return a simplified version
        Some(TokenTransfer {
            token: notification.contract.clone(),
            symbol: token_symbol,
            from: "sender".to_string(), // Would parse from state
            to: "receiver".to_string(), // Would parse from state
            amount: "0".to_string(), // Would parse from state
        })
    }

    /// Get token symbol from contract
    async fn get_token_symbol(&self, contract: &ScriptHash) -> Result<String, NeoError> {
        // Call the symbol method on the contract
        let result = self.client.invoke_function(
            contract,
            "symbol".to_string(),
            vec![],
            None,
        ).await.map_err(|e| NeoError::Contract {
            message: format!("Failed to get token symbol: {}", e),
            source: None,
            recovery: ErrorRecovery::new(),
        })?;

        // Parse the result
        if let Some(stack) = result.stack {
            if let Some(item) = stack.first() {
                // Convert stack item to string
                return Ok("TOKEN".to_string()); // Simplified - parse actual value
            }
        }

        Ok("UNKNOWN".to_string())
    }

    /// Calculate system fee
    fn calculate_system_fee(&self, gas_consumed: u64) -> u64 {
        // Add a small buffer for safety
        (gas_consumed as f64 * 1.1) as u64
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
    fn apply_optimization_rules(&self, tx: &Transaction<'_>, result: &SimulationResult) -> Vec<OptimizationSuggestion> {
        let mut suggestions = Vec::new();

        for rule in &self.optimization_rules {
            if let Some(suggestion) = rule.check(tx, result) {
                suggestions.push(suggestion);
            }
        }

        suggestions
    }

    /// Analyze for warnings
    fn analyze_for_warnings(&self, tx: &Transaction<'_>, result: &SimulationResult) -> Vec<SimulationWarning> {
        let mut warnings = Vec::new();

        // Check if transaction failed
        if !result.success {
            warnings.push(SimulationWarning {
                level: WarningLevel::Error,
                message: "Transaction simulation failed".to_string(),
                suggestion: Some("Review script logic and ensure all preconditions are met".to_string()),
            });
        }

        // Check high gas consumption
        if result.gas_consumed > 10_000_000 {
            warnings.push(SimulationWarning {
                level: WarningLevel::Warning,
                message: format!("High gas consumption: {} GAS", result.gas_consumed as f64 / 100_000_000.0),
                suggestion: Some("Consider optimizing the script or breaking into smaller transactions".to_string()),
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

    /// Calculate transaction hash for caching
    fn calculate_tx_hash(&self, tx: &Transaction<'_>) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&tx.script);
        format!("{:x}", hasher.finalize())
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
enum OptimizationRule {
    BatchTransfers,
    UseNativeContracts,
    MinimizeStorageOps,
    CacheRepeatedCalls,
}

impl OptimizationRule {
    fn check(&self, tx: &Transaction<'_>, result: &SimulationResult) -> Option<OptimizationSuggestion> {
        match self {
            Self::BatchTransfers => {
                if result.notifications.iter().filter(|n| n.event_name == "Transfer").count() > 3 {
                    Some(OptimizationSuggestion {
                        optimization_type: OptimizationType::BatchOperations,
                        description: "Multiple transfers detected. Consider batching.".to_string(),
                        gas_savings: Some(result.gas_consumed / 10),
                        implementation: Some("Use a batch transfer method or combine operations".to_string()),
                    })
                } else {
                    None
                }
            }
            Self::UseNativeContracts => {
                // Check if using non-native contracts for common operations
                None // Simplified
            }
            Self::MinimizeStorageOps => {
                if result.state_changes.storage.values().map(|v| v.len()).sum::<usize>() > 10 {
                    Some(OptimizationSuggestion {
                        optimization_type: OptimizationType::ReduceStorageOperations,
                        description: "Many storage operations detected".to_string(),
                        gas_savings: Some(result.gas_consumed / 20),
                        implementation: Some("Cache values in memory and batch storage updates".to_string()),
                    })
                } else {
                    None
                }
            }
            Self::CacheRepeatedCalls => {
                // Detect repeated contract calls
                None // Simplified
            }
        }
    }
}

/// Cached simulation result
struct CachedSimulation {
    result: SimulationResult,
    timestamp: std::time::SystemTime,
}

impl CachedSimulation {
    fn is_valid(&self) -> bool {
        // Cache valid for 60 seconds
        self.timestamp.elapsed().map(|d| d.as_secs() < 60).unwrap_or(false)
    }
}

/// Gas estimation result
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
pub struct TransactionSimulatorBuilder {
    client: Option<Arc<RpcClient<HttpProvider>>>,
    cache_duration: std::time::Duration,
    optimization_rules: Vec<OptimizationRule>,
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
    pub fn client(mut self, client: Arc<RpcClient<HttpProvider>>) -> Self {
        self.client = Some(client);
        self
    }

    /// Set cache duration
    pub fn cache_duration(mut self, duration: std::time::Duration) -> Self {
        self.cache_duration = duration;
        self
    }

    /// Add optimization rule
    pub fn add_optimization_rule(mut self, rule: OptimizationRule) -> Self {
        self.optimization_rules.push(rule);
        self
    }

    /// Build the simulator
    pub fn build(self) -> Result<TransactionSimulator, NeoError> {
        let client = self.client.ok_or_else(|| NeoError::Contract {
            message: "RPC client is required".to_string(),
            source: None,
            recovery: ErrorRecovery::new()
                .suggest("Provide an RPC client using .client() method"),
        })?;

        let mut simulator = TransactionSimulator::new(client);
        if !self.optimization_rules.is_empty() {
            simulator.optimization_rules = self.optimization_rules;
        }

        Ok(simulator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            vm_state: VMState::Halt,
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
}