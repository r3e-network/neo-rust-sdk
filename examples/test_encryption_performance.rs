use neo3::{
	neo_protocol::{Account, AccountTrait},
	neo_wallets::{Wallet, WalletTrait},
};
use std::time::Instant;

fn main() {
	println!("Testing wallet encryption performance...");

	// Test with different numbers of accounts
	for account_count in [1, 5, 10, 20, 50] {
		let mut wallet = Wallet::new();

		// Create accounts
		for _ in 0..account_count {
			let account = Account::create().expect("Should create account");
			wallet.add_account(account);
		}

		println!("\nEncrypting {account_count} accounts...");
		let start = Instant::now();
		wallet.encrypt_accounts("test_password");
		let duration = start.elapsed();

		println!("Time taken: {duration:?}");
		println!("Average per account: {:?}", duration / account_count as u32);
	}
}
