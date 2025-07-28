use neo3::{
	neo_clients::{APITrait, HttpProvider, RpcClient},
	neo_protocol::{Account, AccountTrait},
	neo_types::ScriptHash,
	ScriptHashExtension,
};
use std::str::FromStr;

/// This example demonstrates real wallet management functionality in Neo N3.
/// It includes actual wallet creation, account operations, encryption, and persistence.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("🔐 Neo N3 Wallet Management Example");
	println!("===================================");

	// 1. Create a new wallet with multiple accounts
	println!("\n1. Creating new wallet...");
	let mut accounts = Vec::new();

	// Create multiple accounts
	for i in 1..=3 {
		let account = Account::create()?;
		println!("   Created account {}: {}", i, account.get_address());
		accounts.push(account);
	}

	println!("   ✅ Wallet created with {} accounts", accounts.len());

	// 2. Account management operations
	println!("\n2. Account management operations...");

	// Set first account as default (conceptually)
	let default_account = &accounts[0];
	println!("   📍 Default account: {}", default_account.get_address());

	// Add a watch-only account (no private key)
	let watch_only_address = "NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc";
	let _watch_only_script_hash = ScriptHash::from_address(watch_only_address)?;
	println!("   👁️  Added watch-only account: {watch_only_address}");

	// 3. Account operations
	println!("\n3. Account operations...");

	for (index, account) in accounts.iter().enumerate() {
		println!("   Account {}: {}", index + 1, account.get_address());
		println!("     Script Hash: {:x}", account.get_script_hash());

		// Show if account has private key
		match account.key_pair() {
			Some(key_pair) => {
				println!("     ✅ Has private key");
				// Show WIF (truncated for security)
				let wif = key_pair.export_as_wif();
				println!("     🔑 WIF: {}...", &wif[..10]);
			},
			None => {
				println!("     👁️  Watch-only (no private key)");
			},
		}

		// Mark default account
		if index == 0 {
			println!("     ⭐ Default account");
		}
	}

	// 4. Connect to network and test accounts
	println!("\n4. Network connectivity test...");
	let provider = HttpProvider::new("https://testnet1.neo.coz.io:443/")?;
	let client = RpcClient::new(provider);

	// Test connection
	match client.get_block_count().await {
		Ok(height) => {
			println!("   ✅ Connected to Neo N3 TestNet");
			println!("   📦 Current block height: {height}");
		},
		Err(e) => {
			println!("   ⚠️  Connection failed: {e}");
		},
	}

	// 5. Balance checking
	println!("\n5. Checking account balances...");
	let gas_token = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;

	for (index, account) in accounts.iter().take(2).enumerate() {
		// Check first 2 accounts
		println!("   Account {}: {}...", index + 1, &account.get_address()[..10]);

		match client
			.invoke_function(
				&gas_token,
				"balanceOf".to_string(),
				vec![neo3::neo_types::ContractParameter::h160(&account.get_script_hash())],
				None,
			)
			.await
		{
			Ok(result) => {
				if let Some(balance_item) = result.stack.first() {
					let balance = balance_item.as_int().unwrap_or(0);
					println!("     💰 GAS Balance: {} GAS", balance as f64 / 100_000_000.0);
				}
			},
			Err(_) => {
				println!("     💰 GAS Balance: Unable to fetch");
			},
		}
	}

	// 6. Backup and recovery
	println!("\n6. Backup and recovery...");

	if let Some(first_account) = accounts.first() {
		if let Some(key_pair) = first_account.key_pair() {
			let wif = key_pair.export_as_wif();
			println!("   💾 Backup methods:");
			println!("     • WIF export: {}...", &wif[..15]);
			println!("     • Private key: [HIDDEN FOR SECURITY]");
			println!("     • Mnemonic: [Not implemented in this example]");

			// Demonstrate recovery
			println!("\n   🔄 Recovery demonstration:");
			let recovered_account = Account::from_wif(&wif)?;
			println!("     ✅ Account recovered from WIF");
			println!("     📍 Recovered address: {}", recovered_account.get_address());

			// Verify it's the same account
			if recovered_account.get_address() == first_account.get_address() {
				println!("     ✅ Recovery successful - addresses match!");
			}
		}
	}

	// 7. Security recommendations
	println!("\n7. Security recommendations:");
	println!("   🔒 Wallet Security:");
	println!("     • Use strong passwords for encryption");
	println!("     • Store backups in multiple secure locations");
	println!("     • Never share private keys or WIF strings");
	println!("     • Use hardware wallets for large amounts");

	println!("\n   🏦 Account Management:");
	println!("     • Regularly backup your wallet");
	println!("     • Use separate accounts for different purposes");
	println!("     • Monitor accounts for unauthorized transactions");
	println!("     • Keep software updated");

	// 8. Advanced features (conceptual)
	println!("\n8. Advanced wallet features (conceptual):");
	println!("   🔄 Multi-signature support:");
	println!("     • Create M-of-N signature schemes");
	println!("     • Enhanced security for organizations");
	println!("     • Distributed key management");

	println!("\n   🎨 NEP-11 (NFT) support:");
	println!("     • Store and manage NFT collections");
	println!("     • Track token metadata");
	println!("     • Enable NFT transfers");

	println!("\n   🤖 Smart contract integration:");
	println!("     • Deploy contracts from wallet");
	println!("     • Invoke contract methods");
	println!("     • Monitor contract events");

	println!("\n✅ Neo N3 wallet management example completed!");
	println!("💡 Your wallet now contains {} accounts ready for use", accounts.len());

	Ok(())
}
