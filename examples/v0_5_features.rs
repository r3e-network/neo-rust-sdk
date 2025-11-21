//! Example demonstrating new features in NeoRust v0.5.1
//!
//! This example showcases the major features introduced in v0.5.x:
//! - WebSocket real-time events
//! - HD Wallet with BIP-39/44
//! - Transaction simulation
//! - High-level SDK API

use bip39::Language;
use neo3::neo_builder::ScriptBuilder;
use neo3::neo_clients::{HttpProvider, RpcClient};
use neo3::neo_error::unified::NeoError as UnifiedNeoError;
use neo3::sdk::{
	hd_wallet::*,
	transaction_simulator::*,
	websocket::{SubscriptionType, WebSocketClient},
	Neo,
};
use std::sync::Arc;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("🚀 NeoRust v0.5.1 Feature Demonstration\n");

	// ========================================
	// 1. High-Level SDK API
	// ========================================
	println!("📡 Connecting to Neo TestNet...");
	let neo = Neo::testnet().await?;
	println!("✅ Connected!\n");

	// Simple balance check with new API
	let address = "NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc";
	println!("💰 Checking balance for {}", address);
	let balance = neo.get_balance(address).await?;
	println!("   NEO: {}", balance.neo);
	println!("   GAS: {}", balance.gas);
	if !balance.tokens.is_empty() {
		println!("   Other tokens:");
		for token in &balance.tokens {
			println!("     - {} ({}): {}", token.symbol, token.contract, token.amount);
		}
	}
	println!();

	// ========================================
	// 2. HD Wallet (BIP-39/44)
	// ========================================
	println!("🔑 HD Wallet Demonstration");
	println!("   Generating 12-word mnemonic wallet...");

	// Generate new HD wallet
	let mut hd_wallet = HDWallet::generate(12, None)?;
	println!("   Mnemonic: {}", hd_wallet.mnemonic_phrase());

	// Derive multiple accounts from single seed
	println!("\n   Deriving accounts:");
	for i in 0..3 {
		let path = format!("m/44'/888'/0'/0/{}", i);
		let account = hd_wallet.derive_account(&path)?;
		println!("   Account {} ({}): {}", i, path, account.get_address());
	}

	// Demonstrate wallet restoration
	println!("\n   Restoring wallet from mnemonic...");
	let test_mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
	let _restored_wallet = HDWallet::from_phrase(test_mnemonic, None, Language::English)?;
	println!("   ✅ Wallet restored successfully!");
	println!();

	// ========================================
	// 3. WebSocket Real-time Events
	// ========================================
	println!("🌐 WebSocket Real-time Events");
	println!("   Note: Requires a Neo node with WebSocket support");
	// Keep types referenced even when the sample connection block is commented out.
	let _ = SubscriptionType::NewBlocks;
	let _ = std::any::type_name::<WebSocketClient>();

	// Example WebSocket connection (commented out - requires running node)
	/*
	let mut ws_client = WebSocketClient::new("ws://localhost:10332/ws").await?;
	ws_client.connect().await?;

	// Subscribe to new blocks
	let block_handle = ws_client.subscribe(SubscriptionType::NewBlocks).await?;
	println!("   ✅ Subscribed to new blocks");

	// Subscribe to contract events
	let contract = ScriptHash::from_address("NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc")?;
	let event_handle = ws_client.subscribe(
		SubscriptionType::ContractEvents(contract)
	).await?;
	println!("   ✅ Subscribed to contract events");

	// Process events (in real app, this would be in a separate task)
	if let Some(mut receiver) = ws_client.take_event_receiver() {
		tokio::spawn(async move {
			while let Some((sub_type, event)) = receiver.recv().await {
				match event {
					EventData::NewBlock { height, hash, .. } => {
						println!("   📦 New block #{} ({})", height, hash);
					}
					EventData::ContractEvent { event_name, .. } => {
						println!("   📨 Contract event: {}", event_name);
					}
					_ => {}
				}
			}
		});
	}
	*/
	println!("   (WebSocket example code available - see source)");
	println!();

	// ========================================
	// 4. Transaction Simulation
	// ========================================
	println!("🔮 Transaction Simulation");
	println!("   Simulating a token transfer...");

	// Create RPC client for simulation
	let provider = HttpProvider::new("https://testnet1.neo.coz.io:443")?;
	let client = Arc::new(RpcClient::new(provider));

	// Create transaction simulator
	let mut simulator = TransactionSimulator::new(client.clone());

	// Build a simple transfer script (example)
	let script = ScriptBuilder::new()
		.push_data(vec![0x01, 0x02, 0x03]) // Example script
		.to_bytes();

	// Simulate the transaction
	let signers = vec![]; // Would include actual signers in real scenario
	let simulation_result = simulator.simulate_script(&script, signers).await?;

	println!("   Simulation Results:");
	println!("   - Success: {}", simulation_result.success);
	println!("   - VM State: {:?}", simulation_result.vm_state);
		println!("   - Gas Consumed: {} GAS", simulation_result.gas_consumed as f64 / 100_000_000.0);
		println!("   - Total Fee: {} GAS", simulation_result.total_fee as f64 / 100_000_000.0);

	// Display warnings if any
	if !simulation_result.warnings.is_empty() {
			println!("\n   ⚠️ Warnings:");
			for warning in &simulation_result.warnings {
				println!("     - {:?}: {}", warning.level, warning.message);
				if let Some(suggestion) = &warning.suggestion {
					println!("       💡 {}", suggestion);
				}
			}
	}

	// Display optimization suggestions
	if !simulation_result.suggestions.is_empty() {
		println!("\n   💡 Optimization Suggestions:");
		for suggestion in &simulation_result.suggestions {
			println!("     - {}", suggestion.description);
			if let Some(savings) = suggestion.gas_savings {
				println!("       Potential savings: {} GAS", savings as f64 / 100_000_000.0);
			}
		}
	}
	println!();

	// ========================================
	// 5. Error Handling with Recovery
	// ========================================
	println!("🛡️ Enhanced Error Handling");
	println!("   Demonstrating error recovery suggestions...");

	// Simulate an error with recovery suggestions
	match neo.get_balance("invalid_address").await {
		Ok(_) => println!("   Unexpected success"),
		Err(e) => {
			println!("   Error: {}", e);

			let recovery = match &e {
				UnifiedNeoError::Network { recovery, .. }
				| UnifiedNeoError::Wallet { recovery, .. }
				| UnifiedNeoError::Contract { recovery, .. }
				| UnifiedNeoError::Transaction { recovery, .. }
				| UnifiedNeoError::Configuration { recovery, .. }
				| UnifiedNeoError::Validation { recovery, .. }
				| UnifiedNeoError::InsufficientFunds { recovery, .. }
				| UnifiedNeoError::Timeout { recovery, .. }
				| UnifiedNeoError::RateLimit { recovery, .. }
				| UnifiedNeoError::Other { recovery, .. } => Some(recovery),
			};

			if let Some(recovery) = recovery {
				if !recovery.suggestions.is_empty() {
					println!("   Recovery suggestions:");
					for suggestion in &recovery.suggestions {
						println!("     💡 {}", suggestion);
					}
				}
				if !recovery.docs.is_empty() {
					println!("   📚 Documentation:");
					for doc in &recovery.docs {
						println!("     - {}", doc);
					}
				}
			}
		},
	}
	println!();

	// ========================================
	// Summary
	// ========================================
	println!("✨ Summary");
	println!("   NeoRust v0.5.1 provides:");
	println!("   ✅ 50-70% code reduction with high-level API");
	println!("   ✅ Real-time events via WebSocket");
	println!("   ✅ HD wallets with BIP-39/44 support");
	println!("   ✅ Transaction simulation for gas estimation");
	println!("   ✅ Enhanced error handling with recovery");
	println!("   ✅ Interactive CLI wizard (neo-cli wizard)");
	println!("   ✅ Project templates for quick starts");
	println!("\n🎉 Ready for production use!");

	Ok(())
}
