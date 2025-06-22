use neo3::{
	neo_crypto::KeyPair,
	neo_protocol::{Account, AccountTrait},
	neo_wallets::{Wallet, WalletTrait},
};
use std::time::Instant;

fn main() {
	println!("Analyzing wallet encryption performance bottleneck...\n");

	// First, let's time individual operations
	println!("1. Testing individual operation times:");

	// Time account creation
	let start = Instant::now();
	let account = Account::create().expect("Should create account");
	let create_time = start.elapsed();
	println!("   Account creation: {create_time:?}");

	// Time key pair generation
	let start = Instant::now();
	let _key_pair = KeyPair::new_random();
	let key_gen_time = start.elapsed();
	println!("   Key pair generation: {key_gen_time:?}");

	// Time single account encryption
	let mut single_wallet = Wallet::new();
	single_wallet.add_account(account);
	let start = Instant::now();
	single_wallet.encrypt_accounts("test_password");
	let single_encrypt_time = start.elapsed();
	println!("   Single account encryption: {single_encrypt_time:?}");

	println!("\n2. Testing parallel vs sequential encryption:");

	// Create accounts for testing
	let mut accounts = Vec::new();
	for _ in 0..10 {
		accounts.push(Account::create().expect("Should create account"));
	}

	// Test sequential encryption (current implementation)
	let mut seq_wallet = Wallet::new();
	for acc in &accounts {
		seq_wallet.add_account(acc.clone());
	}

	let start = Instant::now();
	seq_wallet.encrypt_accounts("test_password");
	let seq_time = start.elapsed();
	println!("   Sequential encryption (10 accounts): {seq_time:?}");
	println!("   Average per account: {:?}", seq_time / 10);

	// Analyze memory usage pattern
	println!("\n3. Memory and resource analysis:");
	println!("   Scrypt parameters:");
	println!("   - N (iterations): 2^14 = 16384");
	println!("   - r (block size): 8");
	println!("   - p (parallelization): 8");
	println!("   - Memory usage per operation: ~16MB");
	println!("   - Total for 50 accounts: ~800MB sequential");

	// Test with reduced parameters for comparison
	println!("\n4. Performance extrapolation:");
	let accounts_50_time = seq_time / 10 * 50;
	println!("   Estimated time for 50 accounts: {accounts_50_time:?}");

	if accounts_50_time.as_secs() > 15 {
		println!("\n⚠️  Performance issue detected!");
		println!("   Current implementation would take {accounts_50_time:?} for 50 accounts");
		println!("   This exceeds the 15-second requirement.");

		println!("\n5. Possible optimizations:");
		println!("   a) Parallel encryption using rayon or tokio");
		println!("   b) Batch key derivation to reuse scrypt state");
		println!("   c) Consider alternative encryption schemes");
		println!("   d) Implement progressive encryption with progress reporting");
	}
}
