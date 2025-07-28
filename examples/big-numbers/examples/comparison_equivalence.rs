/// Neo N3 Big Number Comparison and Equivalence Example
///
/// This example demonstrates how to compare and check equivalence of large numbers
/// in Neo N3 contexts, such as token amounts, contract values, and cryptographic operations.
use neo3::{
	neo_clients::{APITrait, HttpProvider, RpcClient},
	neo_types::{ContractParameter, ScriptHash},
};
use primitive_types::U256;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("🔢 Neo N3 Big Number Comparison and Equivalence Example");
	println!("======================================================");

	// 1. Basic U256 comparisons for Neo blockchain amounts
	println!("\n1️⃣ Basic U256 Comparisons for Blockchain Values");

	// NEO token total supply (100 million NEO)
	let neo_total_supply = U256::from(100_000_000u64) * U256::from(100_000_000u64); // 100M with 8 decimals
	let gas_amount = U256::from(50_000_000u64) * U256::from(100_000_000u64); // 50M GAS with 8 decimals

	println!("   💎 NEO Total Supply: {}", neo_total_supply);
	println!("   ⛽ GAS Amount: {}", gas_amount);

	// Compare token amounts
	if neo_total_supply > gas_amount {
		println!("   ✅ NEO total supply is greater than our GAS amount");
	}

	// 2. Token balance comparisons
	println!("\n2️⃣ Token Balance Comparisons");

	let user_balance = U256::from(1_000u64) * U256::from(100_000_000u64); // 1,000 tokens
	let transfer_amount = U256::from(500u64) * U256::from(100_000_000u64); // 500 tokens
	let minimum_balance = U256::from(100u64) * U256::from(100_000_000u64); // 100 tokens

	println!("   👤 User Balance: {} tokens", format_token_amount(user_balance));
	println!("   📤 Transfer Amount: {} tokens", format_token_amount(transfer_amount));
	println!("   💰 Minimum Balance: {} tokens", format_token_amount(minimum_balance));

	// Check if user can make the transfer
	if user_balance >= transfer_amount {
		let remaining_balance = user_balance - transfer_amount;
		println!("   ✅ Transfer possible");
		println!("   💵 Remaining balance: {} tokens", format_token_amount(remaining_balance));

		// Check if remaining balance meets minimum
		if remaining_balance >= minimum_balance {
			println!("   ✅ Minimum balance requirement satisfied");
		} else {
			println!("   ⚠️ Transfer would leave balance below minimum");
		}
	} else {
		println!("   ❌ Insufficient balance for transfer");
	}

	// 3. Real Neo N3 network token balance comparison
	println!("\n3️⃣ Real Neo N3 Network Balance Comparison");

	// Connect to TestNet
	let provider = HttpProvider::new("https://testnet1.neo.org:443/")?;
	let client = RpcClient::new(provider);

	if let Ok(block_count) = client.get_block_count().await {
		println!("   ✅ Connected to TestNet (Block: {})", block_count);

		// Get GAS total supply from network
		let gas_hash = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;

		match client.invoke_function(&gas_hash, "totalSupply".to_string(), vec![], None).await {
			Ok(result) => {
				if let Some(stack_item) = result.stack.first() {
					if let Some(total_supply) = stack_item.as_int() {
						let supply_u256 = U256::from(total_supply as u64);
						println!(
							"   ⛽ Actual GAS Total Supply: {}",
							format_token_amount(supply_u256)
						);

						// Compare with our calculated amounts
						if supply_u256 > gas_amount {
							println!("   📊 Actual supply is larger than our test amount");
						}

						// Check if supply exceeds certain thresholds
						let billion_tokens =
							U256::from(1_000_000_000u64) * U256::from(100_000_000u64);
						let million_tokens = U256::from(1_000_000u64) * U256::from(100_000_000u64);

						if supply_u256 >= billion_tokens {
							println!("   🔥 Supply exceeds 1 billion tokens");
						} else if supply_u256 >= million_tokens {
							println!("   💰 Supply exceeds 1 million tokens");
						}
					}
				}
			},
			Err(e) => println!("   ❌ Failed to get total supply: {}", e),
		}
	}

	// 4. Cryptographic hash comparisons
	println!("\n4️⃣ Cryptographic Hash Comparisons");

	// Simulate block hash comparisons
	let block_hash_1 =
		U256::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")?;
	let block_hash_2 =
		U256::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")?;
	let different_hash =
		U256::from_str("0xfedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321")?;

	println!("   🧮 Hash 1: 0x{:x}", block_hash_1);
	println!("   🧮 Hash 2: 0x{:x}", block_hash_2);
	println!("   🧮 Different: 0x{:x}", different_hash);

	// Hash equality check
	if block_hash_1 == block_hash_2 {
		println!("   ✅ Hash 1 and Hash 2 are identical");
	}

	if block_hash_1 != different_hash {
		println!("   ✅ Hash 1 and different hash are not equal");
	}

	// Hash ordering (useful for sorting)
	if block_hash_1 < different_hash {
		println!("   📊 Hash 1 is lexicographically smaller than different hash");
	}

	// 5. Network fee calculations
	println!("\n5️⃣ Network Fee Calculations");

	let base_fee = U256::from(1_000_000u64); // 0.01 GAS
	let size_fee_per_byte = U256::from(1_000u64); // 0.00001 GAS per byte
	let transaction_size = U256::from(250u64); // 250 bytes

	let size_fee = size_fee_per_byte * transaction_size;
	let total_fee = base_fee + size_fee;

	println!("   💵 Base Fee: {} GAS", format_gas_amount(base_fee));
	println!("   📏 Size Fee: {} GAS ({} bytes)", format_gas_amount(size_fee), transaction_size);
	println!("   💰 Total Fee: {} GAS", format_gas_amount(total_fee));

	// Fee threshold checks
	let max_acceptable_fee = U256::from(10_000_000u64); // 0.1 GAS
	let high_fee_threshold = U256::from(5_000_000u64); // 0.05 GAS

	if total_fee <= max_acceptable_fee {
		if total_fee > high_fee_threshold {
			println!("   ⚠️ Fee is high but acceptable");
		} else {
			println!("   ✅ Fee is reasonable");
		}
	} else {
		println!("   ❌ Fee exceeds maximum acceptable amount");
	}

	// 6. Zero and boundary value checks
	println!("\n6️⃣ Zero and Boundary Value Checks");

	let zero_amount = U256::zero();
	let max_uint256 = U256::max_value();
	let almost_max = max_uint256 - U256::one();

	println!("   🔢 Zero amount: {}", zero_amount);
	println!("   🔢 Max U256: {}", max_uint256);
	println!("   🔢 Almost max: {}", almost_max);

	// Zero checks
	if zero_amount.is_zero() {
		println!("   ✅ Zero amount is correctly identified as zero");
	}

	// Boundary checks
	if almost_max < max_uint256 {
		println!("   ✅ Almost max is correctly less than max");
	}

	// Overflow prevention check
	let large_number = U256::from(u64::MAX);
	if large_number < max_uint256 {
		println!("   ✅ u64::MAX fits safely in U256");
	}

	println!("\n✅ Big number comparison and equivalence example completed!");
	println!("💡 These patterns are essential for safe Neo N3 token and value operations");

	Ok(())
}

/// Format a token amount (assuming 8 decimal places like NEO/GAS)
fn format_token_amount(amount: U256) -> String {
	let divisor = U256::from(100_000_000u64);
	let integer_part = amount / divisor;
	let fractional_part = amount % divisor;

	if fractional_part.is_zero() {
		format!("{}", integer_part)
	} else {
		format!("{}.{:08}", integer_part, fractional_part.as_u64())
	}
}

/// Format a GAS amount with proper decimals
fn format_gas_amount(amount: U256) -> String {
	let gas_amount = amount.as_u64() as f64 / 100_000_000.0;
	format!("{:.8}", gas_amount)
}
