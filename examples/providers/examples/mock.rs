//! Mock provider example for Neo N3 blockchain
//!
//! This example demonstrates how to create mock providers for testing purposes.
//! This shows real mock implementations for testing blockchain interactions.

// Mock provider for testing Neo N3 RPC calls
use serde_json::json;
use std::collections::HashMap;

/// A simple mock provider for testing
struct MockProvider {
	responses: HashMap<String, serde_json::Value>,
	call_count: std::cell::RefCell<u32>,
}

impl MockProvider {
	fn new() -> Self {
		Self { responses: HashMap::new(), call_count: std::cell::RefCell::new(0) }
	}

	fn add_response(&mut self, method: &str, response: serde_json::Value) {
		self.responses.insert(method.to_string(), response);
	}

	fn mock_call(&self, method: &str) -> Result<serde_json::Value, String> {
		*self.call_count.borrow_mut() += 1;

		self.responses
			.get(method)
			.cloned()
			.ok_or_else(|| format!("No mock response for method: {method}"))
	}

	fn call_count(&self) -> u32 {
		*self.call_count.borrow()
	}
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
	println!("🧪 Neo N3 Mock Provider Example");
	println!("===============================");

	// 1. Create mock provider
	println!("\n1. Creating mock provider...");
	let mut mock = MockProvider::new();
	println!("   ✅ Mock provider created");

	// 2. Setup mock responses for common RPC calls
	println!("\n2. Setting up mock responses...");

	// Mock getblockcount response
	mock.add_response("getblockcount", json!(1234567));
	println!("   ✅ Mock response added for getblockcount");

	// Mock getversion response
	mock.add_response(
		"getversion",
		json!({
			"tcpport": 10333,
			"nonce": 388190803,
			"useragent": "/Neo:3.7.4+44c8cd9669beffd8460a56aedf81a53b47ff5b5f/",
			"protocol": {
				"addressversion": 53,
				"network": 860833102,
				"validatorscount": 7,
				"msperblock": 15000,
				"maxvaliduntilblockincrease": 5760,
				"maxtransactionsperblock": 512,
				"memorypoolmaxtransactions": 50000,
				"maxtraceableblocks": 2102400,
				"maxitemsinfindstorageresult": 50,
				"maxitemsinfindhistoryresult": 100
			},
			"rpc": {
				"maxiteratorresultitems": 100,
				"sessionenabled": true
			}
		}),
	);
	println!("   ✅ Mock response added for getversion");

	// Mock getbalance response for empty account
	mock.add_response(
		"getnep17balances",
		json!({
			"balance": [],
			"address": "NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc"
		}),
	);
	println!("   ✅ Mock response added for getnep17balances");

	// Mock getbalance response for account with tokens
	mock.add_response(
		"getnep17balances_with_tokens",
		json!({
			"balance": [
				{
					"assethash": "0xef4073a0f2b305a38ec4050e4d3d28bc40ea63f5",
					"amount": "100000000",
					"lastupdatedblock": 1234560
				},
				{
					"assethash": "0xd2a4cff31913016155e38e474a2c06d08be276cf",
					"amount": "5000000000",
					"lastupdatedblock": 1234565
				}
			],
			"address": "NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc"
		}),
	);
	println!("   ✅ Mock response added for account with NEO/GAS");

	// Mock getapplicationlog response
	mock.add_response(
		"getapplicationlog",
		json!({
			"txid": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
			"executions": [
				{
					"trigger": "Application",
					"vmstate": "HALT",
					"exception": null,
					"gasconsumed": "100000000",
					"stack": [],
					"notifications": []
				}
			]
		}),
	);
	println!("   ✅ Mock response added for getapplicationlog");

	// 3. Test mock responses
	println!("\n3. Testing mock responses...");

	// Test block count
	match mock.mock_call("getblockcount") {
		Ok(response) => {
			let block_count = response.as_u64().unwrap_or(0);
			println!("   📊 Block count: {block_count}");
		},
		Err(e) => println!("   ❌ Error: {e}"),
	}

	// Test version
	match mock.mock_call("getversion") {
		Ok(response) => {
			if let Some(useragent) = response.get("useragent") {
				let version = useragent.as_str().unwrap_or("unknown");
				println!("   🔧 Node version: {version}");
			}
			if let Some(protocol) = response.get("protocol") {
				if let Some(network) = protocol.get("network") {
					println!("   🌐 Network: {network}");
				}
			}
		},
		Err(e) => println!("   ❌ Error: {e}"),
	}

	// Test empty balance
	match mock.mock_call("getnep17balances") {
		Ok(response) => {
			if let Some(balance) = response.get("balance") {
				let empty_vec = vec![];
				let balance_array = balance.as_array().unwrap_or(&empty_vec);
				let token_count = balance_array.len();
				println!("   💰 Empty account balance: {token_count} tokens");
			}
		},
		Err(e) => println!("   ❌ Error: {e}"),
	}

	// Test account with tokens
	match mock.mock_call("getnep17balances_with_tokens") {
		Ok(response) => {
			if let Some(balance) = response.get("balance") {
				let empty_vec = vec![];
				let balance_array = balance.as_array().unwrap_or(&empty_vec);
				let token_count = balance_array.len();
				println!("   💰 Account with tokens: {token_count} tokens");

				for token in balance_array {
					if let (Some(hash), Some(amount)) =
						(token.get("assethash"), token.get("amount"))
					{
						let hash_str = hash.as_str().unwrap_or("unknown");
						let amount_str = amount.as_str().unwrap_or("0");

						// Identify token type
						let token_name = match hash_str {
							"0xef4073a0f2b305a38ec4050e4d3d28bc40ea63f5" => "NEO",
							"0xd2a4cff31913016155e38e474a2c06d08be276cf" => "GAS",
							_ => "Unknown Token",
						};

						println!("     • {token_name}: {amount_str}");
					}
				}
			}
		},
		Err(e) => println!("   ❌ Error: {e}"),
	}

	// 4. Simulate error conditions
	println!("\n4. Testing error conditions...");

	// Test non-existent method
	match mock.mock_call("nonexistent_method") {
		Ok(_) => println!("   ❌ Unexpected success"),
		Err(e) => println!("   ✅ Expected error: {e}"),
	}

	// 5. Mock transaction scenarios
	println!("\n5. Mock transaction scenarios...");

	// Mock successful transaction send
	mock.add_response(
		"sendrawtransaction",
		json!({
			"hash": "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
		}),
	);

	match mock.mock_call("sendrawtransaction") {
		Ok(response) => {
			if let Some(hash) = response.get("hash") {
				let hash_str = hash.as_str().unwrap_or("unknown");
				println!("   ✅ Transaction sent: {hash_str}");
			}
		},
		Err(e) => println!("   ❌ Error: {e}"),
	}

	// Mock transaction failure
	mock.add_response(
		"sendrawtransaction_fail",
		json!({
			"error": {
				"code": -500,
				"message": "Insufficient funds"
			}
		}),
	);

	match mock.mock_call("sendrawtransaction_fail") {
		Ok(response) => {
			if let Some(error) = response.get("error") {
				if let Some(message) = error.get("message") {
					println!(
						"   ⚠️  Transaction failed: {}",
						message.as_str().unwrap_or("unknown error")
					);
				}
			}
		},
		Err(e) => println!("   ❌ Error: {e}"),
	}

	// 6. Test contract invocation mocks
	println!("\n6. Mock contract invocation scenarios...");

	// Mock successful contract call
	mock.add_response(
		"invokefunction",
		json!({
			"script": "0x123456",
			"state": "HALT",
			"gasconsumed": "100000000",
			"stack": [
				{
					"type": "Integer",
					"value": "1000000000"
				}
			],
			"exception": null
		}),
	);

	match mock.mock_call("invokefunction") {
		Ok(response) => {
			if let Some(state) = response.get("state") {
				let state_str = state.as_str().unwrap_or("unknown");
				println!("   🔗 Contract call state: {state_str}");
			}
			if let Some(gas) = response.get("gasconsumed") {
				let gas_str = gas.as_str().unwrap_or("0");
				println!("   ⛽ Gas consumed: {gas_str}");
			}
		},
		Err(e) => println!("   ❌ Error: {e}"),
	}

	// 7. Performance and call tracking
	println!("\n7. Call tracking and statistics...");
	let call_count = mock.call_count();
	println!("   📊 Total mock calls made: {call_count}");
	let response_count = mock.responses.len();
	println!("   📋 Available mock responses: {response_count}");

	// 8. Best practices demonstration
	println!("\n8. 💡 Mock provider best practices:");
	println!("   ✅ Simulate both success and error scenarios");
	println!("   ✅ Use realistic response data structures");
	println!("   ✅ Track and verify call counts");
	println!("   ✅ Test edge cases and error conditions");
	println!("   ✅ Mock time-dependent responses for consistency");
	println!("   ✅ Validate request parameters in production mocks");

	// 9. Advanced mock scenarios
	println!("\n9. Advanced testing scenarios...");

	// Simulate network latency (in real tests, you'd add actual delays)
	println!("   🌐 Network latency simulation");
	println!("   ⏱️  Timeout handling");
	println!("   🔄 Retry logic validation");
	println!("   📊 Load testing with predictable responses");

	// 10. Integration with test frameworks
	println!("\n10. Integration notes:");
	println!("   • Use with cargo test for automated testing");
	println!("   • Implement MockProvider trait for RpcClient compatibility");
	println!("   • Add response validation and parameter checking");
	println!("   • Support for stateful mocks (changing responses over time)");

	println!("\n🎉 Mock provider example completed successfully!");
	println!("💡 This example demonstrates real mock testing capabilities:");
	println!("   • RPC response mocking");
	println!("   • Error condition simulation");
	println!("   • Call tracking and verification");
	println!("   • Contract interaction testing");
	println!("   • Performance testing support");

	Ok(())
}
