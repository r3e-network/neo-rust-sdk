//! Neo N3 Contract Methods, ABI and Structured Data Example
//!
//! This example demonstrates how to interact with Neo N3 smart contracts
//! using structured parameters, method calls, and ABI-based interactions.

use neo3::{
	neo_clients::{APITrait, HttpProvider, RpcClient},
	neo_types::{
		ContractParameter, ContractParameterMap, ScriptHash, ScriptHashExtension, StackItem,
	},
};
use serde_json::{json, Value};
use std::{collections::HashMap, str::FromStr};

/// Represents a Neo N3 contract ABI method definition
#[derive(Debug, Clone)]
struct ContractMethod {
	name: String,
	parameters: Vec<MethodParameter>,
	return_type: String,
	#[allow(dead_code)]
	offset: u32,
	safe: bool,
}

/// Represents a method parameter in the contract ABI
#[derive(Debug, Clone)]
struct MethodParameter {
	name: String,
	param_type: String,
}

/// Represents structured data for contract calls
#[derive(Debug, Clone)]
struct TokenTransfer {
	from: String,
	to: String,
	amount: u64,
	data: Option<Vec<u8>>,
}

/// Represents a more complex struct for DeFi operations
#[derive(Debug, Clone)]
struct SwapOperation {
	token_in: String,
	token_out: String,
	amount_in: u64,
	min_amount_out: u64,
	deadline: u64,
	recipient: String,
}

/// Simulated contract ABI parser and interaction helper
struct ContractABI {
	contract_hash: ScriptHash,
	methods: HashMap<String, ContractMethod>,
	client: RpcClient<HttpProvider>,
}

impl ContractABI {
	/// Create a new contract ABI instance
	fn new(contract_hash: ScriptHash, client: RpcClient<HttpProvider>) -> Self {
		Self { contract_hash, methods: HashMap::new(), client }
	}

	/// Load methods from a contract manifest
	fn load_from_manifest(&mut self, manifest: &Value) -> Result<(), Box<dyn std::error::Error>> {
		if let Some(abi) = manifest.get("abi") {
			if let Some(methods) = abi.get("methods").and_then(|m| m.as_array()) {
				for method in methods {
					let name = method
						.get("name")
						.and_then(|n| n.as_str())
						.unwrap_or("unknown")
						.to_string();
					let return_type = method
						.get("returntype")
						.and_then(|r| r.as_str())
						.unwrap_or("Void")
						.to_string();
					let offset = method.get("offset").and_then(|o| o.as_u64()).unwrap_or(0) as u32;
					let safe = method.get("safe").and_then(|s| s.as_bool()).unwrap_or(false);

					let mut parameters = Vec::new();
					if let Some(params) = method.get("parameters").and_then(|p| p.as_array()) {
						for param in params {
							let param_name = param
								.get("name")
								.and_then(|n| n.as_str())
								.unwrap_or("param")
								.to_string();
							let param_type = param
								.get("type")
								.and_then(|t| t.as_str())
								.unwrap_or("Any")
								.to_string();
							parameters.push(MethodParameter { name: param_name, param_type });
						}
					}

					let contract_method = ContractMethod {
						name: name.clone(),
						parameters,
						return_type,
						offset,
						safe,
					};

					self.methods.insert(name, contract_method);
				}
			}
		}
		Ok(())
	}

	/// Call a contract method with structured parameters
	async fn call_method(
		&self,
		method_name: &str,
		params: Vec<ContractParameter>,
	) -> Result<StackItem, Box<dyn std::error::Error>> {
		if let Some(method) = self.methods.get(method_name) {
			println!("   📞 Calling method: {}", method.name);
			println!("       Parameters: {}", params.len());
			println!("       Return type: {}", method.return_type);
			println!("       Safe: {}", method.safe);

			let result = self
				.client
				.invoke_function(&self.contract_hash, method_name.to_string(), params, None)
				.await?;

			if let Some(stack_item) = result.stack.first() {
				return Ok(stack_item.clone());
			}
		}
		Err(format!("Method {method_name} not found").into())
	}

	/// Convert a TokenTransfer struct to contract parameters
	fn token_transfer_to_params(
		&self,
		transfer: &TokenTransfer,
	) -> Result<Vec<ContractParameter>, Box<dyn std::error::Error>> {
		let from_hash = ScriptHash::from_address(&transfer.from)?;
		let to_hash = ScriptHash::from_address(&transfer.to)?;

		Ok(vec![
			ContractParameter::h160(&from_hash),
			ContractParameter::h160(&to_hash),
			ContractParameter::integer(transfer.amount as i64),
			match &transfer.data {
				Some(data) => ContractParameter::byte_array(data.clone()),
				None => ContractParameter::any(),
			},
		])
	}

	/// Convert a SwapOperation struct to contract parameters
	fn swap_operation_to_params(
		&self,
		swap: &SwapOperation,
	) -> Result<Vec<ContractParameter>, Box<dyn std::error::Error>> {
		let token_in_hash = ScriptHash::from_str(&swap.token_in)?;
		let token_out_hash = ScriptHash::from_str(&swap.token_out)?;
		let recipient_hash = ScriptHash::from_address(&swap.recipient)?;

		Ok(vec![
			ContractParameter::h160(&token_in_hash),
			ContractParameter::h160(&token_out_hash),
			ContractParameter::integer(swap.amount_in as i64),
			ContractParameter::integer(swap.min_amount_out as i64),
			ContractParameter::integer(swap.deadline as i64),
			ContractParameter::h160(&recipient_hash),
		])
	}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("📋 Neo N3 Contract Methods, ABI and Structured Data Example");
	println!("==========================================================");

	// 1. Connect to Neo N3 TestNet
	println!("\n1️⃣ Connecting to Neo N3 TestNet...");
	let provider = HttpProvider::new("https://testnet1.neo.org:443/")?;
	let client = RpcClient::new(provider);

	if let Ok(block_count) = client.get_block_count().await {
		println!("   ✅ Connected to TestNet");
		println!("   📦 Current block: {block_count}");
	}

	// 2. Set up contract ABI for GAS token (NEP-17)
	println!("\n2️⃣ Setting up contract ABI...");
	let gas_hash = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
	let mut gas_contract = ContractABI::new(gas_hash, client.clone());

	// Simulate loading ABI from contract manifest
	let gas_manifest = json!({
		"abi": {
			"methods": [
				{
					"name": "symbol",
					"parameters": [],
					"returntype": "String",
					"offset": 0,
					"safe": true
				},
				{
					"name": "decimals",
					"parameters": [],
					"returntype": "Integer",
					"offset": 7,
					"safe": true
				},
				{
					"name": "totalSupply",
					"parameters": [],
					"returntype": "Integer",
					"offset": 14,
					"safe": true
				},
				{
					"name": "balanceOf",
					"parameters": [
						{
							"name": "account",
							"type": "Hash160"
						}
					],
					"returntype": "Integer",
					"offset": 21,
					"safe": true
				},
				{
					"name": "transfer",
					"parameters": [
						{
							"name": "from",
							"type": "Hash160"
						},
						{
							"name": "to",
							"type": "Hash160"
						},
						{
							"name": "amount",
							"type": "Integer"
						},
						{
							"name": "data",
							"type": "Any"
						}
					],
					"returntype": "Boolean",
					"offset": 28,
					"safe": false
				}
			]
		}
	});

	gas_contract.load_from_manifest(&gas_manifest)?;
	println!("   ✅ Loaded {} methods from ABI", gas_contract.methods.len());

	// 3. Demonstrate structured method calls
	println!("\n3️⃣ Calling contract methods with ABI...");

	// Call symbol method
	let symbol_result = gas_contract.call_method("symbol", vec![]).await?;
	if let Some(symbol) = symbol_result.as_string() {
		println!("   💎 Token symbol: {symbol}");
	}

	// Call decimals method
	let decimals_result = gas_contract.call_method("decimals", vec![]).await?;
	if let Some(decimals) = decimals_result.as_int() {
		println!("   🔢 Token decimals: {decimals}");
	}

	// Call totalSupply method
	let supply_result = gas_contract.call_method("totalSupply", vec![]).await?;
	if let Some(supply) = supply_result.as_int() {
		println!("   📊 Total supply: {} GAS", supply as f64 / 100_000_000.0);
	}

	// 4. Demonstrate structured data usage
	println!("\n4️⃣ Working with structured data...");

	// Create a TokenTransfer struct
	let transfer = TokenTransfer {
		from: "NPvKVTGZapmFWABLsyvfreuqn73jCjJtN1".to_string(),
		to: "NTrezV3bgHEjFfWw3Jwz8XnCxwU8cJNTSi".to_string(),
		amount: 1_00000000, // 1 GAS
		data: Some(b"Hello Neo!".to_vec()),
	};

	println!("   📤 Transfer struct:");
	println!("       From: {}", transfer.from);
	println!("       To: {}", transfer.to);
	println!("       Amount: {} GAS", transfer.amount as f64 / 100_000_000.0);
	println!("       Data: {:?}", transfer.data);

	// Convert struct to contract parameters
	let transfer_params = gas_contract.token_transfer_to_params(&transfer)?;
	println!("   ✅ Converted to {} contract parameters", transfer_params.len());

	// 5. Demonstrate complex struct usage
	println!("\n5️⃣ Complex structured data example...");

	let swap = SwapOperation {
		token_in: "d2a4cff31913016155e38e474a2c06d08be276cf".to_string(), // GAS
		token_out: "ef4073a0f2b305a38ec4050e4d3d28bc40ea63f5".to_string(), // NEO
		amount_in: 10_00000000,                                           // 10 GAS
		min_amount_out: 1_00000000,                                       // 1 NEO minimum
		deadline: 1700000000,                                             // Unix timestamp
		recipient: "NPvKVTGZapmFWABLsyvfreuqn73jCjJtN1".to_string(),
	};

	println!("   🔄 Swap operation struct:");
	println!("       Token In: 0x{}", swap.token_in);
	println!("       Token Out: 0x{}", swap.token_out);
	println!("       Amount In: {} GAS", swap.amount_in as f64 / 100_000_000.0);
	println!("       Min Amount Out: {} NEO", swap.min_amount_out as f64 / 100_000_000.0);
	println!("       Deadline: {}", swap.deadline);
	println!("       Recipient: {}", swap.recipient);

	let swap_params = gas_contract.swap_operation_to_params(&swap)?;
	println!("   ✅ Converted to {} contract parameters", swap_params.len());

	// 6. Demonstrate parameter type handling
	println!("\n6️⃣ Parameter type demonstrations...");

	println!("   🔢 Integer parameter:");
	let int_param = ContractParameter::integer(42);
	println!("       Value: {int_param:?}");

	println!("   📝 String parameter:");
	let string_param = ContractParameter::string("Hello Neo N3!".to_string());
	println!("       Value: {string_param:?}");

	println!("   🔗 Hash160 parameter:");
	let address = "NPvKVTGZapmFWABLsyvfreuqn73jCjJtN1";
	let hash_param = ContractParameter::h160(&ScriptHash::from_address(address)?);
	println!("       Address: {address}");
	println!("       Hash: {hash_param:?}");

	println!("   📦 Array parameter:");
	let array_param = ContractParameter::array(vec![
		ContractParameter::integer(1),
		ContractParameter::integer(2),
		ContractParameter::integer(3),
	]);
	println!("       Value: {array_param:?}");

	println!("   🗂️ Map parameter:");
	let mut map = HashMap::new();
	map.insert(ContractParameter::string("key1".to_string()), ContractParameter::integer(100));
	map.insert(
		ContractParameter::string("key2".to_string()),
		ContractParameter::string("value2".to_string()),
	);
	let map_param = ContractParameter::map(ContractParameterMap(map));
	println!("       Value: {map_param:?}");

	// 7. Method signature analysis
	println!("\n7️⃣ Method signature analysis...");
	for (name, method) in &gas_contract.methods {
		println!("   📋 Method: {name}");
		println!("       Parameters: {}", method.parameters.len());
		for param in &method.parameters {
			println!("         • {} ({})", param.name, param.param_type);
		}
		println!("       Returns: {}", method.return_type);
		println!("       Safe: {}", method.safe);
		println!();
	}

	// 8. Real balance check with structured call
	println!("8️⃣ Real balance check using structured approach...");
	let test_address = "NPvKVTGZapmFWABLsyvfreuqn73jCjJtN1";
	let address_hash = ScriptHash::from_address(test_address)?;
	let balance_params = vec![ContractParameter::h160(&address_hash)];

	let balance_result = gas_contract.call_method("balanceOf", balance_params).await?;
	if let Some(balance) = balance_result.as_int() {
		println!("   💰 Balance for {}: {} GAS", test_address, balance as f64 / 100_000_000.0);
	}

	println!("\n✅ Contract methods, ABI and structured data example completed!");
	println!("💡 This demonstrates how to work with complex contract interactions");
	println!("   using structured data and ABI-based method calls in Neo N3");

	Ok(())
}
