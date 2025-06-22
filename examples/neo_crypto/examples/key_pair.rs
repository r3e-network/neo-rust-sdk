use neo3::{
	neo_crypto::{HashableForVec, KeyPair},
	neo_protocol::{Account, AccountTrait},
};

/// Production-ready comprehensive example demonstrating Neo N3 key pair operations
/// including generation, import, export, signing, and verification
fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("🔐 Neo N3 Comprehensive Key Pair Management Example");
	println!("===================================================");

	// 1. Generate multiple random key pairs for different purposes
	println!("\n1. 🎲 Generating random key pairs...");
	let key_pair_1 = KeyPair::new_random();
	let key_pair_2 = KeyPair::new_random();
	println!("✅ Generated 2 random key pairs successfully");

	// 2. Create accounts from key pairs
	println!("\n2. 👤 Creating Neo accounts from key pairs...");
	let account_1 = Account::from_key_pair(key_pair_1.clone(), None, None)?;
	let account_2 = Account::from_key_pair(key_pair_2.clone(), None, None)?;
	println!("✅ Created Neo accounts successfully");

	// 3. Display comprehensive key pair information
	println!("\n3. 🔍 Examining key pair details...");
	display_key_pair_info("Account 1", &key_pair_1, &account_1);
	display_key_pair_info("Account 2", &key_pair_2, &account_2);

	// 4. Export keys in different formats
	println!("\n4. 📤 Exporting keys in various formats...");
	export_key_examples(&account_1)?;

	// 5. Import keys from different formats
	println!("\n5. 📥 Importing keys from various formats...");
	import_key_examples()?;

	// 6. Digital signature demonstration
	println!("\n6. ✍️ Digital signature demonstration...");
	signature_verification_example(&key_pair_1)?;

	// 7. Multi-signature setup demonstration
	println!("\n7. 🔗 Multi-signature setup demonstration...");
	multisig_example(&[&key_pair_1, &key_pair_2])?;

	// 8. Security best practices demonstration
	println!("\n8. 🛡️ Security best practices...");
	security_best_practices();

	println!("\n🎉 Comprehensive key pair example completed successfully!");
	println!("💡 This example demonstrates production-ready key management patterns");

	Ok(())
}

/// Display comprehensive information about a key pair and account
fn display_key_pair_info(label: &str, key_pair: &KeyPair, account: &Account) {
	println!("\n   📋 {label}:");
	println!("      🔑 Public Key:    {}", hex::encode(key_pair.public_key.get_encoded(true)));
	println!(
		"      🗝️  Private Key:   {}...{} (truncated for security)",
		&hex::encode(key_pair.private_key.to_raw_bytes())[..8],
		&hex::encode(key_pair.private_key.to_raw_bytes())[56..]
	);
	println!("      #️⃣  Script Hash:   {}", key_pair.get_script_hash());
	println!("      🏠 Address:       {}", account.get_address());
	println!("      🆔 Script Hash:   {:?}", account.get_script_hash());
}

/// Demonstrate key export in various formats
fn export_key_examples(account: &Account) -> Result<(), Box<dyn std::error::Error>> {
	// Export as WIF (Wallet Import Format) - using key_pair
	if let Some(key_pair) = &account.key_pair {
		let wif = key_pair.export_as_wif();
		println!("   📋 WIF Export:     {}...{} (truncated)", &wif[..10], &wif[wif.len() - 6..]);
	}

	// Export public key in different formats
	if let Some(public_key) = account.get_public_key() {
		let public_key_hex = hex::encode(public_key.get_encoded(true));
		println!(
			"   🔑 Public Key:     {}...{}",
			&public_key_hex[..16],
			&public_key_hex[public_key_hex.len() - 8..]
		);
	}

	println!("   🔒 NEP-2 Export:   Available through wallet encryption features");

	Ok(())
}

/// Demonstrate key import from various formats
fn import_key_examples() -> Result<(), Box<dyn std::error::Error>> {
	// Generate a test key for import demonstration
	let test_account = Account::create()?;
	let test_wif = if let Some(key_pair) = &test_account.key_pair {
		key_pair.export_as_wif()
	} else {
		return Err("Test account has no key pair".into());
	};

	// Import from WIF
	let imported_account = Account::from_wif(&test_wif)?;
	println!("   ✅ WIF Import:     Successfully imported account from WIF");
	println!("      🏠 Address:       {}", imported_account.get_address());

	// Verify they're the same account
	if test_account.get_address() == imported_account.get_address() {
		println!("   ✅ Verification:   WIF import produces identical account");
	}

	println!("   🔒 NEP-2 Import:   Available through wallet decryption features");

	Ok(())
}

/// Demonstrate digital signature creation and verification
fn signature_verification_example(key_pair: &KeyPair) -> Result<(), Box<dyn std::error::Error>> {
	let message = b"Hello Neo N3 Blockchain! This is a test message for signature verification.";

	// Hash the message (Neo uses SHA256 for message hashing)
	let message_hash = message.hash256();

	// Create signature using private key
	let signature = key_pair.private_key.sign_prehash(&message_hash)?;
	let signature_bytes = signature.to_bytes();

	println!("   ✍️  Message:        \"{}\"", String::from_utf8_lossy(message));
	println!(
		"   📝 Signature:      {}...{}",
		hex::encode(&signature_bytes[..8]),
		hex::encode(&signature_bytes[signature_bytes.len() - 8..])
	);

	// Verify signature using public key
	let is_valid = key_pair.public_key.verify(&message_hash, &signature).is_ok();
	println!(
		"   {} Verification:   Signature is {}",
		if is_valid { "✅" } else { "❌" },
		if is_valid { "VALID" } else { "INVALID" }
	);

	// Test with tampered message
	let tampered_message =
		b"Hello Neo N3 Blockchain! This is a TAMPERED message for signature verification.";
	let tampered_hash = tampered_message.hash256();
	let is_tampered_valid = key_pair.public_key.verify(&tampered_hash, &signature).is_ok();
	println!(
		"   {} Tamper Test:    Tampered message signature is {}",
		if !is_tampered_valid { "✅" } else { "❌" },
		if is_tampered_valid { "VALID" } else { "INVALID" }
	);

	Ok(())
}

/// Demonstrate multi-signature setup
fn multisig_example(key_pairs: &[&KeyPair]) -> Result<(), Box<dyn std::error::Error>> {
	let threshold = 2; // Require 2 out of 3 signatures
	let total_signers = key_pairs.len();

	println!("   🔗 Setup:          {threshold}-of-{total_signers} multi-signature configuration");

	// Create accounts for each key pair
	let accounts: Result<Vec<Account>, _> = key_pairs
		.iter()
		.map(|kp| Account::from_key_pair((*kp).clone(), None, None))
		.collect();
	let accounts = accounts?;

	// Display participant addresses
	for (i, account) in accounts.iter().enumerate() {
		println!("   👤 Signer {}:       {}", i + 1, account.get_address());
	}

	// Create multi-signature address
	let mut public_keys: Vec<_> = accounts.iter().filter_map(|a| a.get_public_key()).collect();
	let multisig_account = Account::multi_sig_from_public_keys(&mut public_keys, threshold)?;
	println!("   🏠 MultiSig Addr:  {}", multisig_account.get_address());
	println!("   ✅ Configuration:  Multi-signature account created successfully");

	Ok(())
}

/// Display security best practices
fn security_best_practices() {
	println!("   🛡️ Security Best Practices:");
	println!("      • Never expose private keys in production logs");
	println!("      • Use hardware wallets for high-value accounts");
	println!("      • Always verify signatures before processing transactions");
	println!("      • Use strong, unique passwords for key encryption");
	println!("      • Regularly backup encrypted key files");
	println!("      • Implement proper key rotation policies");
	println!("      • Use multi-signature for critical operations");
	println!("      • Keep software and dependencies updated");
}
