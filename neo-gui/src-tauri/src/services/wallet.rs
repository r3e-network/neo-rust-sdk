use neo3::{
	neo_clients::ProductionRpcClient,
	neo_error::{Neo3Error, Neo3Result},
	neo_wallets::Wallet,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use uuid;

/// Wallet service for managing Neo wallets
#[derive(Clone)]
pub struct WalletService {
	wallets: Arc<RwLock<HashMap<String, Wallet>>>,
	current_wallet: Arc<RwLock<Option<Wallet>>>,
}

impl WalletService {
	pub fn new() -> Self {
		Self {
			wallets: Arc::new(RwLock::new(HashMap::new())),
			current_wallet: Arc::new(RwLock::new(None)),
		}
	}

	/// Create a new wallet
	pub async fn create_wallet(&self, name: &str, password: &str) -> Neo3Result<String> {
		let mut wallet = Wallet::new();
		wallet.name = name.to_string();
		wallet.version = "1.0".to_string();

		// The wallet already has a default account from new()
		// Encrypt all accounts with the password
		wallet.encrypt_accounts(password);

		let wallet_id = uuid::Uuid::new_v4().to_string();
		let wallet_path = format!("{}.json", name.to_lowercase().replace(" ", "_"));

		// Save to file
		use std::path::PathBuf;
		wallet
			.save_to_file(PathBuf::from(&wallet_path))
			.map_err(|e| Neo3Error::Generic { message: format!("Failed to save wallet: {}", e) })?;

		// Store in memory
		let mut wallets = self.wallets.write().await;
		wallets.insert(wallet_id.clone(), wallet.clone());

		// Set as current wallet
		*self.current_wallet.write().await = Some(wallet);

		Ok(wallet_id)
	}

	/// Open an existing wallet
	pub async fn open_wallet(&self, path: &str, password: &str) -> Neo3Result<String> {
		// Load wallet from file
		use std::path::PathBuf;
		let wallet = Wallet::open_wallet(&PathBuf::from(path), password)
			.map_err(|e| Neo3Error::Generic { message: format!("Failed to open wallet: {}", e) })?;

		let wallet_id = uuid::Uuid::new_v4().to_string();

		// Store in memory
		let mut wallets = self.wallets.write().await;
		wallets.insert(wallet_id.clone(), wallet.clone());

		// Set as current wallet
		*self.current_wallet.write().await = Some(wallet);

		Ok(wallet_id)
	}

	/// Get wallet balance
	pub async fn get_balance(
		&self,
		wallet_id: &str,
		rpc_client: Option<&ProductionRpcClient>,
	) -> Neo3Result<String> {
		let wallets = self.wallets.read().await;
		let wallet = wallets.get(wallet_id).ok_or_else(|| Neo3Error::Generic {
			message: format!("Wallet not found: {}", wallet_id),
		})?;

		if let Some(client) = rpc_client {
			// Get real balances from blockchain
			let mut balances = HashMap::new();

			for account in wallet.get_accounts() {
				let address = account.get_address();

				// Get NEP-17 token balances
				let nep17_balances = client.get_nep17_balances(address.clone()).await?;

				let mut account_balance = HashMap::new();
				account_balance.insert("address", account.get_address());
				account_balance.insert("balances", serde_json::to_string(&nep17_balances)?);

				balances.insert(account.get_address(), account_balance);
			}

			Ok(serde_json::to_string(&balances)?)
		} else {
			// No RPC client, return empty balances
			Err(Neo3Error::Config("No RPC client provided for balance query".to_string()))
		}
	}

	/// Get current wallet
	pub async fn get_current_wallet(&self) -> Arc<RwLock<Option<Wallet>>> {
		self.current_wallet.clone()
	}

	/// Send transaction
	pub async fn send_transaction(
		&self,
		_wallet_id: &str,
		_from: &str,
		_to: &str,
		_asset: &str,
		_amount: &str,
	) -> Neo3Result<String> {
		// This method is deprecated and should use the TransactionService instead
		// Return an error directing users to use the proper transaction service
		Err(Neo3Error::Generic {
			message: "Use TransactionService.send_transaction() instead of WalletService.send_transaction()".to_string()
		})
	}

	/// Get transaction history
	pub async fn get_transaction_history(
		&self,
		_wallet_id: &str,
		_address: Option<&str>,
	) -> Neo3Result<Vec<serde_json::Value>> {
		// Transaction history requires blockchain indexing service integration
		// Professional implementation uses external indexer or comprehensive blockchain scanning
		// Neo N3 RPC doesn't provide transaction history by address natively

		Err(Neo3Error::Generic {
			message: "Transaction history by address requires external indexer service. Use TransactionService.get_transaction_history() or integrate with a blockchain explorer API.".to_string()
		})
	}

	/// List all wallets
	pub async fn list_wallets(&self) -> Vec<String> {
		let wallets = self.wallets.read().await;
		wallets.keys().cloned().collect()
	}

	/// Close wallet
	pub async fn close_wallet(&self, wallet_id: &str) -> Neo3Result<()> {
		let mut wallets = self.wallets.write().await;
		wallets.remove(wallet_id);
		Ok(())
	}
}

impl Default for WalletService {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_wallet_service_creation() {
		let service = WalletService::new();
		let wallets = service.list_wallets().await;
		assert!(wallets.is_empty());
	}

	#[tokio::test]
	async fn test_create_wallet() {
		let service = WalletService::new();
		let wallet_id = service.create_wallet("test_wallet", "password123").await.unwrap();
		assert!(!wallet_id.is_empty());

		let wallets = service.list_wallets().await;
		assert_eq!(wallets.len(), 1);
		assert!(wallets.contains(&wallet_id));
	}

	#[tokio::test]
	async fn test_close_wallet() {
		let service = WalletService::new();
		let wallet_id = service.create_wallet("test_wallet", "password123").await.unwrap();

		service.close_wallet(&wallet_id).await.unwrap();
		let wallets = service.list_wallets().await;
		assert!(wallets.is_empty());
	}
}
