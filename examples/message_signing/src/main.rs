/// This example demonstrates message signing in Neo N3.
use neo3::{
	neo_crypto::{HashableForVec, KeyPair},
	neo_protocol::{Account, AccountTrait},
	neo_wallets::{Wallet, WalletTrait},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("Neo N3 Message Signing Example");
	println!("==============================");

	println!("\nIn Neo N3, message signing is used to prove ownership of a private key");
	println!("without revealing the key itself. Common use cases include:");
	println!("- Authentication for dApps and services");
	println!("- Verifying identity on-chain");
	println!("- Authorizing off-chain operations");
	println!("- Proving ownership of an address");

	// Create a random key pair for demonstration
	println!("\nStep 1: Creating a key pair");
	let key_pair = KeyPair::new_random();
	let account = Account::from_key_pair(key_pair.clone(), None, None)?;
	let address = account.get_address();
	println!("Generated address: {address}");
	println!("Public key: {}", hex::encode(key_pair.public_key.get_encoded(true)));

	// Create a wallet and add the account
	println!("\nStep 2: Creating a wallet with the account");
	let mut wallet = Wallet::new();
	wallet.add_account(account.clone());
	wallet.set_default_account(account.get_script_hash());

	// Message to sign
	println!("\nStep 3: Preparing a message to sign");
	let message = b"Hello, Neo N3!";
	println!("Message: {}", String::from_utf8_lossy(message));

	// Sign the message
	println!("\nStep 4: Signing the message");

	// Hash the message (Neo uses SHA256 for message signing)
	let message_hash = message.hash256();
	println!("Message hash: {}", hex::encode(&message_hash));

	// Sign with the private key
	let signature = key_pair.private_key.sign_prehash(&message_hash)?;
	let signature_bytes = signature.to_bytes();
	println!("Signature created successfully");
	println!("Signature (hex): {}", hex::encode(signature_bytes));
	println!("Signature length: {} bytes", signature_bytes.len());

	println!("\nStep 5: Verifying the signature");

	// Verify the signature using the public key
	let is_valid = key_pair.public_key.verify(&message_hash, &signature).is_ok();
	println!("Signature verification: {}", if is_valid { "✅ VALID" } else { "❌ INVALID" });

	// Demonstrate verification with a tampered message
	println!("\nStep 6: Testing with tampered message");
	let tampered_message = b"Hello, Neo N3! (tampered)";
	let tampered_hash = tampered_message.hash256();
	let is_tampered_valid = key_pair.public_key.verify(&tampered_hash, &signature).is_ok();
	println!(
		"Tampered message verification: {}",
		if is_tampered_valid { "✅ VALID" } else { "❌ INVALID" }
	);

	println!("\nIn a production application, verification process:");
	println!("1. Receive the message, signature, and claimed address");
	println!("2. Hash the message using the same algorithm (SHA256)");
	println!("3. Verify the signature using the public key");
	println!("4. Ensure the public key corresponds to the claimed address");

	// Demonstrate a more complex message signing scenario
	println!("\nStep 7: Signing structured data");
	let structured_message = format!(
		"Neo N3 Signed Message:\nTimestamp: {}\nNonce: {}\nAction: Transfer 100 GAS\nTo: {}",
		123456789, // Example timestamp
		42,        // Example nonce
		"NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc"
	);
	println!("Structured message:\n{structured_message}");

	let structured_hash = structured_message.as_bytes().hash256();
	let structured_signature = key_pair.private_key.sign_prehash(&structured_hash)?;
	println!("\nStructured message signature: {}", hex::encode(structured_signature.to_bytes()));

	println!("\n🔒 Security Best Practices:");
	println!("   • Always hash messages before signing (prevents length extension attacks)");
	println!("   • Include domain separation in your message format");
	println!("   • Add timestamps and nonces to prevent replay attacks");
	println!("   • Use hardware wallets for high-value operations");
	println!("   • Never sign raw transaction data without understanding it");
	println!("   • Implement signature expiration for time-sensitive operations");

	println!("\nMessage signing example completed!");
	Ok(())
}
