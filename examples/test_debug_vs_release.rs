use neo3::{
	neo_protocol::{Account, AccountTrait},
	neo_wallets::{Wallet, WalletTrait},
};
use std::time::Instant;

fn main() {
	#[cfg(debug_assertions)]
	println!("Running in DEBUG mode");

	#[cfg(not(debug_assertions))]
	println!("Running in RELEASE mode");

	let mut wallet = Wallet::new();

	// Create 50 accounts
	println!("\nCreating 50 accounts...");
	for _ in 0..50 {
		let account = Account::create().expect("Should create account");
		wallet.add_account(account);
	}

	println!("Starting encryption of 50 accounts...");
	let start = Instant::now();
	wallet.encrypt_accounts("test_password");
	let duration = start.elapsed();

	println!("\nResults:");
	println!("Total time: {:?}", duration);
	println!("Average per account: {:?}", duration / 50);

	if duration.as_secs() > 15 {
		println!(
			"\n❌ FAILED: Encryption took {} seconds, which exceeds the 15-second requirement",
			duration.as_secs()
		);

		// Calculate the factor by which we're off
		let factor = duration.as_secs_f64() / 15.0;
		println!("Performance is {:.1}x slower than required", factor);
	} else {
		println!("\n✅ PASSED: Encryption completed within 15 seconds");
	}
}
