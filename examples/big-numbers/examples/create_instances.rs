use neo3::prelude::*;

/// Neo N3 Big Numbers Example
///
/// This example demonstrates working with large numbers in Neo N3 contexts,
/// particularly for token amounts, balances, and calculations.
fn main() {
	println!("🔢 Neo N3 Big Numbers Example");
	println!("=============================");

	// 1. Creating U256 instances for Neo N3 token amounts
	println!("\n1. Creating U256 instances:");

	// From decimal strings (common for user input)
	let a = U256::from_dec_str("42").unwrap();
	println!("   From decimal string '42': {a}");
	assert_eq!(format!("{a:?}"), "42");

	// From hex strings (common for contract values)
	let b = U256::from_str_radix("2a", 16).unwrap(); // 42 in hex
	println!("   From hex string '2a': {b}");
	assert_eq!(b, U256::from(42));

	// From numbers
	let c = U256::from(42_u8);
	println!("   From u8 value 42: {c}");
	assert_eq!(format!("{c:?}"), "42");

	let d = U256::from(42_u16);
	println!("   From u16 value 42: {d}");
	assert_eq!(format!("{d:?}"), "42");

	let e = U256::from(42_u32);
	println!("   From u32 value 42: {e}");
	assert_eq!(format!("{e:?}"), "42");

	let f = U256::from(42_u64);
	println!("   From u64 value 42: {f}");
	assert_eq!(format!("{f:?}"), "42");

	// From hex literal
	let g = U256::from(0x2a);
	println!("   From hex literal 0x2a: {g}");
	assert_eq!(format!("{g:?}"), "42");

	// Using Into trait
	let h: U256 = 42.into();
	println!("   Using Into trait: {h}");
	assert_eq!(format!("{h:?}"), "42");

	// 2. Neo N3 token amount examples
	println!("\n2. Neo N3 Token Amount Examples:");

	// GAS amounts (8 decimals)
	let one_gas = U256::from(100_000_000u64); // 1 GAS
	println!("   1 GAS (base units): {one_gas}");

	let gas_amount = U256::from_dec_str("1000000000").unwrap(); // 10 GAS
	println!("   10 GAS (base units): {gas_amount}");

	// NEO amounts (indivisible)
	let neo_amount = U256::from(50); // 50 NEO
	println!("   50 NEO: {neo_amount}");

	// Large token supply
	let total_supply = U256::from_dec_str("100000000000000000000000000").unwrap();
	println!("   Large token supply: {total_supply}");

	// 3. Arithmetic operations
	println!("\n3. Arithmetic Operations:");

	let x = U256::from(1000);
	let y = U256::from(500);

	let sum = x + y;
	println!("   {x} + {y} = {sum}");

	let difference = x - y;
	println!("   {x} - {y} = {difference}");

	let product = x * y;
	println!("   {x} * {y} = {product}");

	let quotient = x / y;
	println!("   {x} / {y} = {quotient}");

	let remainder = x % y;
	println!("   {x} % {y} = {remainder}");

	// 4. Common Neo N3 calculations
	println!("\n4. Common Neo N3 Calculations:");

	// Calculate percentage
	let balance = U256::from(1_500_000_000u64); // 15 GAS
	let total = U256::from(10_000_000_000u64); // 100 GAS
	let percentage = (balance * U256::from(10000)) / total; // Basis points
	println!(
		"   Balance percentage: {}.{}%",
		percentage / U256::from(100),
		percentage % U256::from(100)
	);

	// Calculate fees
	let transaction_amount = U256::from(1_000_000_000u64); // 10 GAS
	let fee_rate = U256::from(30); // 0.3% in basis points
	let fee = (transaction_amount * fee_rate) / U256::from(10000);
	println!("   Transaction fee (0.3%): {fee} base units");

	// Convert to human-readable
	let human_fee = fee.as_u64() as f64 / 100_000_000.0;
	println!("   Transaction fee: {human_fee} GAS");

	// 5. Comparison operations
	println!("\n5. Comparison Operations:");

	let amount1 = U256::from(1_000_000);
	let amount2 = U256::from(2_000_000);

	let eq_result = amount1 == amount2;
	println!("   {amount1} == {amount2}: {eq_result}");
	let lt_result = amount1 < amount2;
	println!("   {amount1} < {amount2}: {lt_result}");
	let gt_result = amount1 > amount2;
	println!("   {amount1} > {amount2}: {gt_result}");
	let le_result = amount1 <= amount2;
	println!("   {amount1} <= {amount2}: {le_result}");
	let ge_result = amount1 >= amount2;
	println!("   {amount1} >= {amount2}: {ge_result}");

	// 6. Working with maximum values
	println!("\n6. Working with Maximum Values:");

	let max_u256 = U256::MAX;
	println!("   U256::MAX: {max_u256}");

	let zero = U256::zero();
	println!("   U256::zero(): {zero}");

	let one = U256::one();
	println!("   U256::one(): {one}");

	// 7. Bit operations
	println!("\n7. Bit Operations:");

	let bits = U256::from(0b1010); // Binary 1010 (decimal 10)
	println!("   Binary 1010: {bits}");

	let shifted_left = bits << 2; // Shift left by 2 positions
	println!("   Left shift by 2: {shifted_left}");

	let shifted_right = bits >> 1; // Shift right by 1 position
	println!("   Right shift by 1: {shifted_right}");

	// 8. Converting back to primitives
	println!("\n8. Converting Back to Primitives:");

	let large_number = U256::from(42_000_000_000u64);

	// Safe conversion
	if large_number <= U256::from(u64::MAX) {
		let as_u64 = large_number.as_u64();
		println!("   As u64: {as_u64}");
	}

	// Get low 128 bits
	let low_128 = large_number.low_u128();
	println!("   Low 128 bits: {low_128}");

	// Check if it fits in u32
	if large_number <= U256::from(u32::MAX) {
		let as_u32 = large_number.as_u32();
		println!("   As u32: {as_u32}");
	} else {
		println!("   Too large for u32");
	}

	// 9. Error handling
	println!("\n9. Error Handling:");

	// Overflow protection
	let large1 = U256::MAX - U256::from(10);
	let large2 = U256::from(20);

	match large1.checked_add(large2) {
		Some(result) => println!("   Addition result: {result}"),
		None => println!("   Addition would overflow"),
	}

	// Division by zero protection
	let dividend = U256::from(100);
	let divisor = U256::zero();

	match dividend.checked_div(divisor) {
		Some(result) => println!("   Division result: {result}"),
		None => println!("   Division by zero prevented"),
	}

	// 10. Formatting and display
	println!("\n10. Formatting and Display:");

	let number = U256::from_dec_str("123456789012345678901234567890").unwrap();
	println!("   Decimal: {number}");
	println!("   Debug: {number:?}");
	println!("   Hex (lowercase): {number:x}");
	println!("   Hex (uppercase): {number:X}");
	// Note: U256 doesn't implement Binary trait directly
	// Convert to string representation for binary display
	println!("   Binary representation would require custom formatting");

	println!("\n🎉 Neo N3 big numbers example completed!");
	println!("💡 Key takeaways:");
	println!("   • Use U256 for large token amounts and calculations");
	println!("   • Always consider decimal places when working with token amounts");
	println!("   • Use checked arithmetic to prevent overflows");
	println!("   • Convert safely between U256 and primitive types");
	println!("   • Handle edge cases like division by zero");
}
