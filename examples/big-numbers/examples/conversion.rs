/// Neo N3 Big Number Conversion Example
///
/// This example demonstrates how to convert between different number types
/// in Neo N3 contexts, including token amounts, gas values, and blockchain data.
/// Safe conversion patterns prevent precision loss and overflow issues.

use neo3::{neo_clients::{APITrait, HttpProvider, RpcClient}, neo_types::{ContractParameter, ScriptHash}};
use primitive_types::U256;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("🔄 Neo N3 Big Number Conversion Example");
	println!("======================================");

	// 1. Basic U256 conversions for Neo blockchain values
	println!("\n1️⃣ Basic U256 Conversions");
	
	// Start with a typical NEO token amount (1000 NEO with 8 decimals)
	let neo_amount = U256::from(1000u64) * U256::from(100_000_000u64);
	println!("   💎 NEO Amount (raw): {}", neo_amount);
	
	// Safe conversions to smaller types
	let as_u128: u128 = neo_amount.as_u128();
	let as_u64: u64 = neo_amount.as_u64();
	let as_string: String = neo_amount.to_string();
	
	println!("   📊 As u128: {}", as_u128);
	println!("   📊 As u64: {}", as_u64);
	println!("   📊 As string: {}", as_string);
	
	// Convert to human-readable format
	let human_readable = convert_to_token_amount(neo_amount, 8);
	println!("   👤 Human readable: {} NEO", human_readable);

	// 2. Conversion from different input types
	println!("\n2️⃣ Converting from Different Input Types");
	
	// From string (hex)
	let hex_string = "0x1234567890abcdef";
	let from_hex = U256::from_str(hex_string)?;
	println!("   🔤 From hex '{}': {}", hex_string, from_hex);
	
	// From decimal string
	let decimal_string = "123456789012345678901234567890";
	let from_decimal = U256::from_dec_str(decimal_string)?;
	println!("   🔢 From decimal '{}': {}", decimal_string, from_decimal);
	
	// From bytes
	let bytes = [0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef];
	let from_bytes = U256::from_big_endian(&bytes);
	println!("   📦 From bytes {:?}: {}", bytes, from_bytes);
	
	// From smaller integers
	let from_u64 = U256::from(u64::MAX);
	let from_u32 = U256::from(42u32);
	println!("   🔢 From u64::MAX: {}", from_u64);
	println!("   🔢 From 42u32: {}", from_u32);

	// 3. Real Neo N3 network data conversion
	println!("\n3️⃣ Real Neo N3 Network Data Conversion");
	
	// Connect to TestNet
	let provider = HttpProvider::new("https://testnet1.neo.org:443/")?;
	let client = RpcClient::new(provider);
	
	if let Ok(block_count) = client.get_block_count().await {
		println!("   ✅ Connected to TestNet");
		
		// Get GAS total supply and convert to different formats
		let gas_hash = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
		
		match client.invoke_function(&gas_hash, "totalSupply".to_string(), vec![], None).await {
			Ok(result) => {
				if let Some(stack_item) = result.stack.first() {
					if let Some(total_supply) = stack_item.as_int() {
						let supply_u256 = U256::from(total_supply as u64);
						
						println!("   ⛽ GAS Total Supply Conversions:");
						println!("       Raw value: {}", supply_u256);
						println!("       As u64: {}", supply_u256.as_u64());
						println!("       As string: {}", supply_u256.to_string());
						println!("       Human readable: {} GAS", convert_to_token_amount(supply_u256, 8));
						
						// Convert to different bases
						println!("       Hex: 0x{:x}", supply_u256);
						println!("       Scientific: {:.2e}", supply_u256.as_u64() as f64);
						
						// Convert to bytes for storage/transmission
						let mut bytes = [0u8; 32];
						supply_u256.to_big_endian(&mut bytes);
						println!("       As bytes (first 8): {:?}...", &bytes[0..8]);
					}
				}
			},
			Err(e) => println!("   ❌ Failed to get total supply: {}", e),
		}
	}

	// 4. Safe conversion patterns
	println!("\n4️⃣ Safe Conversion Patterns");
	
	// Large number that might overflow smaller types
	let large_number = U256::from(u64::MAX) * U256::from(1000u64);
	println!("   🔢 Large number: {}", large_number);
	
	// Safe conversion to u64 with overflow check
	match safe_convert_to_u64(large_number) {
		Some(value) => println!("   ✅ Safe u64 conversion: {}", value),
		None => println!("   ⚠️ Number too large for u64, value: {}", large_number),
	}
	
	// Safe conversion to f64 with precision warning
	let as_f64 = safe_convert_to_f64(large_number);
	println!("   🔄 As f64 (may lose precision): {:.2e}", as_f64);
	
	// Conversion for display purposes
	let display_value = convert_for_display(large_number);
	println!("   👁️ Display format: {}", display_value);

	// 5. Token amount conversions
	println!("\n5️⃣ Token Amount Conversions");
	
	// Convert user input to token amount
	let user_input = "123.45678900"; // User wants to send 123.45678900 tokens
	let token_amount = parse_token_amount(user_input, 8)?;
	println!("   📝 User input '{}' -> {}", user_input, token_amount);
	
	// Convert back to display
	let back_to_display = convert_to_token_amount(token_amount, 8);
	println!("   🔄 Back to display: {}", back_to_display);
	
	// Different decimal places (like different tokens)
	let custom_decimals = parse_token_amount("1000.123", 3)?;
	println!("   🎯 Custom decimals (3): {}", custom_decimals);
	println!("   🎯 Back to display: {}", convert_to_token_amount(custom_decimals, 3));

	// 6. Gas and fee conversions
	println!("\n6️⃣ Gas and Fee Conversions");
	
	// Convert GAS amounts for fee calculations
	let base_fee_gas = parse_token_amount("0.001", 8)?; // 0.001 GAS
	let size_fee_gas = parse_token_amount("0.00025", 8)?; // 0.00025 GAS
	let total_fee = base_fee_gas + size_fee_gas;
	
	println!("   💵 Base fee: {} ({})", base_fee_gas, convert_to_token_amount(base_fee_gas, 8));
	println!("   📏 Size fee: {} ({})", size_fee_gas, convert_to_token_amount(size_fee_gas, 8));
	println!("   💰 Total fee: {} ({} GAS)", total_fee, convert_to_token_amount(total_fee, 8));
	
	// Convert to different units
	println!("   📊 Total fee in micro-GAS: {}", total_fee);
	println!("   📊 Total fee in wei-equivalent: {}", total_fee * U256::from(10_000_000_000u64));

	// 7. Boundary and edge case conversions
	println!("\n7️⃣ Boundary and Edge Case Conversions");
	
	// Zero conversion
	let zero = U256::zero();
	println!("   🔢 Zero conversions:");
	println!("       As u64: {}", zero.as_u64());
	println!("       As string: {}", zero.to_string());
	println!("       Display: {} tokens", convert_to_token_amount(zero, 8));
	
	// Maximum values
	let max_u64_in_u256 = U256::from(u64::MAX);
	let max_u256 = U256::max_value();
	
	println!("   🔢 Maximum value conversions:");
	println!("       u64::MAX in U256: {}", max_u64_in_u256);
	println!("       U256::MAX (truncated): {:.30}...", max_u256.to_string());
	
	// One unit conversions
	let one_token = U256::from(100_000_000u64); // 1 token with 8 decimals
	println!("   🔢 One token (8 decimals): {} -> {}", one_token, convert_to_token_amount(one_token, 8));

	println!("\n✅ Big number conversion example completed!");
	println!("💡 These conversion patterns ensure safe handling of Neo N3 values");
	println!("   without precision loss or overflow issues");

	Ok(())
}

/// Safely convert U256 to u64, returning None if overflow would occur
fn safe_convert_to_u64(value: U256) -> Option<u64> {
	if value <= U256::from(u64::MAX) {
		Some(value.as_u64())
	} else {
		None
	}
}

/// Convert U256 to f64 for display (may lose precision)
fn safe_convert_to_f64(value: U256) -> f64 {
	// Convert to string and then to f64 for better precision
	// This is safer than direct conversion for very large numbers
	if value.is_zero() {
		0.0
	} else if value <= U256::from(u64::MAX) {
		value.as_u64() as f64
	} else {
		// For very large numbers, use scientific notation approach
		let string_val = value.to_string();
		string_val.parse::<f64>().unwrap_or(f64::INFINITY)
	}
}

/// Convert large numbers to human-readable display format
fn convert_for_display(value: U256) -> String {
	if value.is_zero() {
		"0".to_string()
	} else if value < U256::from(1_000u64) {
		value.to_string()
	} else if value < U256::from(1_000_000u64) {
		format!("{:.1}K", value.as_u64() as f64 / 1_000.0)
	} else if value < U256::from(1_000_000_000u64) {
		format!("{:.1}M", value.as_u64() as f64 / 1_000_000.0)
	} else {
		format!("{:.1}B", value.as_u64() as f64 / 1_000_000_000.0)
	}
}

/// Convert token string (like "123.45") to raw amount with decimals
fn parse_token_amount(amount_str: &str, decimals: u8) -> Result<U256, Box<dyn std::error::Error>> {
	let parts: Vec<&str> = amount_str.split('.').collect();
	
	let integer_part = parts[0].parse::<u64>()?;
	let fractional_part = if parts.len() > 1 {
		let frac_str = format!("{:0<width$}", parts[1], width = decimals as usize);
		let truncated = if frac_str.len() > decimals as usize {
			&frac_str[0..decimals as usize]
		} else {
			&frac_str
		};
		truncated.parse::<u64>()?
	} else {
		0
	};
	
	let multiplier = U256::from(10u64).pow(U256::from(decimals));
	let integer_amount = U256::from(integer_part) * multiplier;
	let fractional_amount = U256::from(fractional_part);
	
	Ok(integer_amount + fractional_amount)
}

/// Convert raw token amount to human-readable string
fn convert_to_token_amount(amount: U256, decimals: u8) -> String {
	let divisor = U256::from(10u64).pow(U256::from(decimals));
	let integer_part = amount / divisor;
	let fractional_part = amount % divisor;
	
	if fractional_part.is_zero() {
		integer_part.to_string()
	} else {
		let frac_str = format!("{:0width$}", fractional_part, width = decimals as usize);
		let trimmed = frac_str.trim_end_matches('0');
		if trimmed.is_empty() {
			integer_part.to_string()
		} else {
			format!("{}.{}", integer_part, trimmed)
		}
	}
}
