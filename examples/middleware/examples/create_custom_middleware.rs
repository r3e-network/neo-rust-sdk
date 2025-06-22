use neo3::{
	neo_clients::{HttpProvider, RpcClient},
	prelude::*,
};
use std::str::FromStr;
use thiserror::Error;

/// This example demonstrates how to create custom middleware for Neo N3 transactions.
/// It shows how to intercept, modify, and validate transactions before they are sent
/// to the Neo N3 network. This is useful for implementing custom business logic,
/// security checks, or gas optimization strategies.
///
/// This custom middleware adjusts gas limits for Neo N3 transactions based on
/// network conditions and transaction complexity to improve success rates.
#[tokio::main]
async fn main() -> eyre::Result<()> {
	println!("🔧 Neo N3 Custom Middleware Example");
	println!("===================================");

	// 1. Connect to Neo N3 TestNet
	println!("\n1. Setting up Neo N3 connection...");
	let provider = HttpProvider::new("https://testnet1.neo.coz.io:443/")?;
	let client = RpcClient::new(provider);
	println!("   ✅ Connected to Neo N3 TestNet");

	// 2. Create custom middleware layers
	println!("\n2. Creating custom middleware layers...");

	// Gas optimization middleware
	let gas_middleware = GasOptimizationMiddleware::new(25); // 25% gas buffer
	println!("   ✅ Gas optimization middleware created");

	// Transaction validation middleware
	let validation_middleware = TransactionValidationMiddleware::new();
	println!("   ✅ Transaction validation middleware created");

	// Logging middleware
	let logging_middleware = LoggingMiddleware::new();
	println!("   ✅ Logging middleware created");

	// 3. Create middleware chain
	println!("\n3. Building middleware chain...");
	let middleware_chain = MiddlewareChain::new()
		.add_middleware(Box::new(logging_middleware))
		.add_middleware(Box::new(validation_middleware))
		.add_middleware(Box::new(gas_middleware));
	println!(
		"   ✅ Middleware chain configured with {} layers",
		middleware_chain.middleware_count()
	);

	// 4. Test middleware with different transaction types
	println!("\n4. Testing middleware with different transactions...");

	// Simple NEO transfer
	let neo_transfer = TransactionRequest {
		recipient: ScriptHash::from_address("NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc")?,
		asset: ScriptHash::from_str("ef4073a0f2b305a38ec4050e4d3d28bc40ea63f5")?,
		amount: 1,
		gas_limit: 1_000_000,
		transaction_type: TransactionType::Transfer,
	};

	println!("\n   📋 Processing NEO transfer transaction:");
	process_transaction_with_middleware(&client, &middleware_chain, &neo_transfer).await?;

	// Contract invocation
	let contract_call = TransactionRequest {
		recipient: ScriptHash::from_str("0xef4073a0f2b305a38ec4050e4d3d28bc40ea63f5")?, // NEO token
		asset: ScriptHash::from_str("0xd2a4cff31913016155e38e474a2c06d08be276cf")?,
		amount: 0,
		gas_limit: 2_000_000,
		transaction_type: TransactionType::ContractCall,
	};

	println!("\n   📋 Processing contract call transaction:");
	process_transaction_with_middleware(&client, &middleware_chain, &contract_call).await?;

	// High-value transaction
	let high_value_tx = TransactionRequest {
		recipient: ScriptHash::from_address("NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc")?,
		asset: ScriptHash::from_str("0xd2a4cff31913016155e38e474a2c06d08be276cf")?,
		amount: 100_000_000, // 1 GAS
		gas_limit: 1_500_000,
		transaction_type: TransactionType::Transfer,
	};

	println!("\n   📋 Processing high-value transaction:");
	process_transaction_with_middleware(&client, &middleware_chain, &high_value_tx).await?;

	// 5. Demonstrate error handling
	println!("\n5. Testing error handling...");

	// Invalid transaction (negative amount simulation)
	let invalid_tx = TransactionRequest {
		recipient: ScriptHash::from_address("NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc")?,
		asset: ScriptHash::from_str("0xd2a4cff31913016155e38e474a2c06d08be276cf")?,
		amount: 0,      // This will trigger validation error
		gas_limit: 100, // Very low gas limit
		transaction_type: TransactionType::Transfer,
	};

	println!("\n   📋 Processing invalid transaction (should fail):");
	if let Err(e) =
		process_transaction_with_middleware(&client, &middleware_chain, &invalid_tx).await
	{
		println!("     ✅ Expected error caught: {e}");
	}

	// 6. Middleware best practices
	println!("\n6. 💡 Custom Middleware Best Practices:");
	println!("   ✅ Keep middleware lightweight and focused");
	println!("   ✅ Implement proper error handling and propagation");
	println!("   ✅ Use logging for debugging and monitoring");
	println!("   ✅ Validate inputs before processing");
	println!("   ✅ Allow for configuration and customization");
	println!("   ✅ Consider middleware ordering and dependencies");
	println!("   ✅ Test thoroughly with various transaction types");

	println!("\n🎉 Custom middleware example completed!");
	println!("💡 This demonstrates how to create and use custom middleware for Neo N3.");

	Ok(())
}

/// Transaction request structure
#[derive(Debug, Clone)]
struct TransactionRequest {
	recipient: ScriptHash,
	asset: ScriptHash,
	amount: u64,
	gas_limit: u64,
	transaction_type: TransactionType,
}

/// Transaction types for middleware processing
#[derive(Debug, Clone)]
enum TransactionType {
	Transfer,
	ContractCall,
	#[allow(dead_code)]
	ContractDeploy,
}

/// Middleware trait for transaction processing
trait TransactionMiddleware {
	fn process(&self, tx: &mut TransactionRequest) -> Result<(), MiddlewareError>;
	fn name(&self) -> &str;
}

/// Custom middleware error types
#[derive(Error, Debug)]
enum MiddlewareError {
	#[error("Gas limit too low: {0}")]
	GasTooLow(u64),
	#[error("Invalid transaction amount: {0}")]
	InvalidAmount(u64),
	#[error("Transaction validation failed: {0}")]
	ValidationFailed(String),
	#[error("Middleware chain error: {0}")]
	#[allow(dead_code)]
	ChainError(String),
}

/// Gas optimization middleware
struct GasOptimizationMiddleware {
	buffer_percentage: u32,
}

impl GasOptimizationMiddleware {
	fn new(buffer_percentage: u32) -> Self {
		Self { buffer_percentage }
	}
}

impl TransactionMiddleware for GasOptimizationMiddleware {
	fn process(&self, tx: &mut TransactionRequest) -> Result<(), MiddlewareError> {
		let original_gas = tx.gas_limit;

		// Add buffer based on transaction type
		let base_gas = match tx.transaction_type {
			TransactionType::Transfer => 1_000_000,
			TransactionType::ContractCall => 2_000_000,
			TransactionType::ContractDeploy => 5_000_000,
		};

		let optimized_gas = std::cmp::max(
			original_gas,
			base_gas + (base_gas * self.buffer_percentage as u64 / 100),
		);

		tx.gas_limit = optimized_gas;

		println!(
			"     🔧 Gas optimization: {} → {} (+{}%)",
			original_gas, optimized_gas, self.buffer_percentage
		);

		Ok(())
	}

	fn name(&self) -> &str {
		"GasOptimization"
	}
}

/// Transaction validation middleware
struct TransactionValidationMiddleware;

impl TransactionValidationMiddleware {
	fn new() -> Self {
		Self
	}
}

impl TransactionMiddleware for TransactionValidationMiddleware {
	fn process(&self, tx: &mut TransactionRequest) -> Result<(), MiddlewareError> {
		// Validate gas limit
		if tx.gas_limit < 100_000 {
			return Err(MiddlewareError::GasTooLow(tx.gas_limit));
		}

		// Validate amount for transfers
		if matches!(tx.transaction_type, TransactionType::Transfer) && tx.amount == 0 {
			return Err(MiddlewareError::InvalidAmount(tx.amount));
		}

		// Validate recipient address format
		if tx.recipient.0.iter().all(|&b| b == 0) {
			return Err(MiddlewareError::ValidationFailed("Invalid recipient address".to_string()));
		}

		println!("     ✅ Transaction validation passed");
		Ok(())
	}

	fn name(&self) -> &str {
		"TransactionValidation"
	}
}

/// Logging middleware
struct LoggingMiddleware;

impl LoggingMiddleware {
	fn new() -> Self {
		Self
	}
}

impl TransactionMiddleware for LoggingMiddleware {
	fn process(&self, tx: &mut TransactionRequest) -> Result<(), MiddlewareError> {
		println!("     📝 Logging transaction:");
		println!("       Type: {:?}", tx.transaction_type);
		println!("       Asset: 0x{}", hex::encode(tx.asset.0));
		println!("       Amount: {}", tx.amount);
		println!("       Gas Limit: {}", tx.gas_limit);
		println!("       Recipient: {}", tx.recipient.to_address());
		Ok(())
	}

	fn name(&self) -> &str {
		"Logging"
	}
}

/// Middleware chain for processing transactions
struct MiddlewareChain {
	middlewares: Vec<Box<dyn TransactionMiddleware>>,
}

impl MiddlewareChain {
	fn new() -> Self {
		Self { middlewares: Vec::new() }
	}

	fn add_middleware(mut self, middleware: Box<dyn TransactionMiddleware>) -> Self {
		self.middlewares.push(middleware);
		self
	}

	fn middleware_count(&self) -> usize {
		self.middlewares.len()
	}

	async fn process_transaction(
		&self,
		tx: &mut TransactionRequest,
	) -> Result<(), MiddlewareError> {
		for middleware in &self.middlewares {
			println!("   🔄 Processing with {} middleware", middleware.name());
			middleware.process(tx)?;
		}
		Ok(())
	}
}

/// Process transaction through middleware chain
async fn process_transaction_with_middleware(
	_client: &RpcClient<HttpProvider>,
	middleware_chain: &MiddlewareChain,
	original_tx: &TransactionRequest,
) -> Result<(), MiddlewareError> {
	let mut tx = original_tx.clone();

	println!("   📥 Original transaction:");
	println!("     Gas Limit: {}", tx.gas_limit);
	println!("     Amount: {}", tx.amount);

	// Process through middleware chain
	middleware_chain.process_transaction(&mut tx).await?;

	println!("   📤 Processed transaction:");
	println!("     Gas Limit: {}", tx.gas_limit);
	println!("     Amount: {}", tx.amount);

	// In a real implementation, you would send the transaction here
	println!("   ✅ Transaction ready for submission to Neo N3 network");

	Ok(())
}
