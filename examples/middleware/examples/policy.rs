use chrono::Timelike;
use neo3::{
	neo_clients::{HttpProvider, RpcClient},
	neo_types::ScriptHash,
	prelude::*,
};
use std::str::FromStr;

/// Policy middleware for Neo N3 provides a way to inject custom logic into transaction processing
/// and contract interactions. This allows you to define security rules, compliance checks,
/// and custom validation logic that should be enforced before transactions are sent.
///
/// This example demonstrates various policy patterns for Neo N3 transactions including
/// security checks, spending limits, and compliance validation.
#[tokio::main]
async fn main() -> eyre::Result<()> {
	println!("🛡️ Neo N3 Transaction Policy Middleware Example");
	println!("===============================================");

	// 1. Connect to Neo N3 TestNet
	println!("\n1. Setting up Neo N3 connection...");
	let provider = HttpProvider::new("https://testnet1.neo.coz.io:443/")?;
	let _client = RpcClient::new(provider);
	println!("   ✅ Connected to Neo N3 TestNet");

	// 2. Initialize policy manager
	println!("\n2. Initializing transaction policy manager...");
	let mut policy_manager = TransactionPolicyManager::new();
	println!("   ✅ Policy manager initialized");

	// 3. Configure security policies
	println!("\n3. Configuring security policies...");

	// Add spending limit policy
	let spending_limit_policy = SpendingLimitPolicy::new(100_000_000); // 1 GAS limit
	policy_manager.add_policy(Box::new(spending_limit_policy));
	println!("   ✅ Spending limit policy added (1 GAS max)");

	// Add whitelist policy
	let mut whitelist_policy = WhitelistPolicy::new();
	whitelist_policy.add_allowed_contract("0xef4073a0f2b305a38ec4050e4d3d28bc40ea63f5"); // NEO token
	whitelist_policy.add_allowed_contract("0xd2a4cff31913016155e38e474a2c06d08be276cf"); // GAS token
	policy_manager.add_policy(Box::new(whitelist_policy));
	println!("   ✅ Contract whitelist policy added");

	// Add time-based policy
	let time_policy = TimeBasedPolicy::new(9, 17); // Only allow 9 AM - 5 PM UTC
	policy_manager.add_policy(Box::new(time_policy));
	println!("   ✅ Time-based policy added (business hours only)");

	// Add reject-all policy for demonstration
	let reject_all_policy = RejectAllPolicy::new();

	// 4. Test different transaction scenarios
	println!("\n4. Testing transaction policy validation...");

	// Test valid transaction (without reject-all policy)
	let valid_transaction = TransactionRequest {
		recipient: ScriptHash::from_address("NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc")?,
		asset: ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?,
		amount: 50_000_000, // 0.5 GAS
		contract_hash: Some(ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?),
	};

	println!("\n   📋 Testing valid transaction (0.5 GAS transfer):");
	match policy_manager.validate_transaction(&valid_transaction).await {
		Ok(_) => println!("     ✅ Transaction approved by all policies"),
		Err(e) => println!("     ❌ Transaction rejected: {e}"),
	}

	// Test transaction exceeding spending limit
	let high_value_transaction = TransactionRequest {
		recipient: ScriptHash::from_address("NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc")?,
		asset: ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?,
		amount: 200_000_000, // 2 GAS (exceeds limit)
		contract_hash: Some(ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?),
	};

	println!("\n   📋 Testing high-value transaction (2 GAS transfer):");
	match policy_manager.validate_transaction(&high_value_transaction).await {
		Ok(_) => println!("     ✅ Transaction approved"),
		Err(e) => println!("     ❌ Transaction rejected: {e}"),
	}

	// Test unauthorized contract interaction
	let unauthorized_contract = TransactionRequest {
		recipient: ScriptHash::from_address("NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc")?,
		asset: ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?,
		amount: 10_000_000, // 0.1 GAS
		contract_hash: Some(ScriptHash::from_str("1234567890abcdef1234567890abcdef12345678")?),
	};

	println!("\n   📋 Testing unauthorized contract interaction:");
	match policy_manager.validate_transaction(&unauthorized_contract).await {
		Ok(_) => println!("     ✅ Transaction approved"),
		Err(e) => println!("     ❌ Transaction rejected: {e}"),
	}

	// 5. Test reject-all policy
	println!("\n5. Testing reject-all policy...");
	let mut strict_manager = TransactionPolicyManager::new();
	strict_manager.add_policy(Box::new(reject_all_policy));

	match strict_manager.validate_transaction(&valid_transaction).await {
		Ok(_) => println!("   ❌ Unexpected approval!"),
		Err(e) => println!("   ✅ Expected rejection: {e}"),
	}

	// 6. Demonstrate policy composition
	println!("\n6. Policy composition and priority...");

	let mut priority_manager = TransactionPolicyManager::new();
	priority_manager.add_policy(Box::new(SpendingLimitPolicy::new(50_000_000))); // 0.5 GAS
	priority_manager.add_policy(Box::new(EmergencyStopPolicy::new(false))); // Not stopped

	let test_transaction = TransactionRequest {
		recipient: ScriptHash::from_address("NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc")?,
		asset: ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?,
		amount: 30_000_000, // 0.3 GAS
		contract_hash: Some(ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?),
	};

	match priority_manager.validate_transaction(&test_transaction).await {
		Ok(_) => println!("   ✅ Transaction passed all composed policies"),
		Err(e) => println!("   ❌ Transaction failed policy check: {e}"),
	}

	// 7. Policy best practices
	println!("\n7. 💡 Neo N3 Policy Best Practices:");
	println!("   ✅ Implement spending limits to prevent large losses");
	println!("   ✅ Use contract whitelists for security");
	println!("   ✅ Add time-based restrictions for compliance");
	println!("   ✅ Implement emergency stop mechanisms");
	println!("   ✅ Log all policy decisions for audit trails");
	println!("   ✅ Test policies thoroughly before deployment");
	println!("   ✅ Use layered security with multiple policies");

	println!("\n🎉 Transaction policy middleware example completed!");
	println!("💡 This demonstrates security and compliance policies for Neo N3.");

	Ok(())
}

/// Transaction request structure for policy validation
#[derive(Debug, Clone)]
struct TransactionRequest {
	#[allow(dead_code)]
	recipient: ScriptHash,
	#[allow(dead_code)]
	asset: ScriptHash,
	amount: u64,
	contract_hash: Option<ScriptHash>,
}

/// Policy validation result
type PolicyResult = Result<(), String>;

/// Trait for transaction policies
trait TransactionPolicy {
	fn validate(&self, transaction: &TransactionRequest) -> PolicyResult;
	fn name(&self) -> &str;
}

/// Transaction policy manager
struct TransactionPolicyManager {
	policies: Vec<Box<dyn TransactionPolicy>>,
}

impl TransactionPolicyManager {
	fn new() -> Self {
		Self { policies: Vec::new() }
	}

	fn add_policy(&mut self, policy: Box<dyn TransactionPolicy>) {
		self.policies.push(policy);
	}

	async fn validate_transaction(&self, transaction: &TransactionRequest) -> PolicyResult {
		for policy in &self.policies {
			if let Err(e) = policy.validate(transaction) {
				return Err(format!("{}: {}", policy.name(), e));
			}
		}
		Ok(())
	}
}

/// Spending limit policy
struct SpendingLimitPolicy {
	max_amount: u64,
}

impl SpendingLimitPolicy {
	fn new(max_amount: u64) -> Self {
		Self { max_amount }
	}
}

impl TransactionPolicy for SpendingLimitPolicy {
	fn validate(&self, transaction: &TransactionRequest) -> PolicyResult {
		if transaction.amount > self.max_amount {
			Err(format!("Amount {} exceeds spending limit {}", transaction.amount, self.max_amount))
		} else {
			Ok(())
		}
	}

	fn name(&self) -> &str {
		"SpendingLimit"
	}
}

/// Contract whitelist policy
struct WhitelistPolicy {
	allowed_contracts: std::collections::HashSet<String>,
}

impl WhitelistPolicy {
	fn new() -> Self {
		Self { allowed_contracts: std::collections::HashSet::new() }
	}

	fn add_allowed_contract(&mut self, contract_hash: &str) {
		self.allowed_contracts.insert(contract_hash.to_lowercase());
	}
}

impl TransactionPolicy for WhitelistPolicy {
	fn validate(&self, transaction: &TransactionRequest) -> PolicyResult {
		if let Some(contract_hash) = &transaction.contract_hash {
			let hash_str = format!("0x{}", hex::encode(contract_hash.0)).to_lowercase();
			if !self.allowed_contracts.contains(&hash_str) {
				return Err(format!("Contract {hash_str} not in whitelist"));
			}
		}
		Ok(())
	}

	fn name(&self) -> &str {
		"ContractWhitelist"
	}
}

/// Time-based policy (business hours only)
struct TimeBasedPolicy {
	start_hour: u32,
	end_hour: u32,
}

impl TimeBasedPolicy {
	fn new(start_hour: u32, end_hour: u32) -> Self {
		Self { start_hour, end_hour }
	}
}

impl TransactionPolicy for TimeBasedPolicy {
	fn validate(&self, _transaction: &TransactionRequest) -> PolicyResult {
		let current_hour = chrono::Utc::now().hour();

		if current_hour < self.start_hour || current_hour >= self.end_hour {
			Err(format!(
				"Transactions only allowed between {}:00 and {}:00 UTC (current: {}:00)",
				self.start_hour, self.end_hour, current_hour
			))
		} else {
			Ok(())
		}
	}

	fn name(&self) -> &str {
		"TimeBased"
	}
}

/// Reject all policy (for testing)
struct RejectAllPolicy;

impl RejectAllPolicy {
	fn new() -> Self {
		Self
	}
}

impl TransactionPolicy for RejectAllPolicy {
	fn validate(&self, _transaction: &TransactionRequest) -> PolicyResult {
		Err("All transactions rejected by policy".to_string())
	}

	fn name(&self) -> &str {
		"RejectAll"
	}
}

/// Emergency stop policy
struct EmergencyStopPolicy {
	is_stopped: bool,
}

impl EmergencyStopPolicy {
	fn new(is_stopped: bool) -> Self {
		Self { is_stopped }
	}
}

impl TransactionPolicy for EmergencyStopPolicy {
	fn validate(&self, _transaction: &TransactionRequest) -> PolicyResult {
		if self.is_stopped {
			Err("Emergency stop activated - all transactions blocked".to_string())
		} else {
			Ok(())
		}
	}

	fn name(&self) -> &str {
		"EmergencyStop"
	}
}
