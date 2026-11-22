#![allow(missing_docs)]

use std::{
	collections::HashMap,
	io::Write,
	path::{Path, PathBuf},
};

#[cfg(not(unix))]
use std::fs::File;
#[cfg(unix)]
use std::{fs::OpenOptions, os::unix::fs::OpenOptionsExt};

use primitive_types::H160;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
	neo_builder::{Transaction, TransactionBuilder, Witness},
	neo_clients::{APITrait, JsonRpcProvider, ProviderError, RpcClient},
	neo_config::NeoConstants,
	neo_crypto::{CryptoError, HashableForVec, KeyPair, Secp256r1Signature},
	neo_protocol::{Account, AccountTrait, UnclaimedGas},
	neo_types::{
		ScryptParamsDef,
		script_hash::ScriptHashExtension,
		serde_with_utils::{
			deserialize_hash_map_h160_account, deserialize_script_hash,
			serialize_hash_map_h160_account, serialize_script_hash,
		},
	},
	neo_wallets::{NEP6Account, Nep6Wallet, WalletError, WalletTrait},
};
use scrypt::Params;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
	pub name: String,
	pub version: String,
	pub scrypt_params: ScryptParamsDef,
	#[serde(deserialize_with = "deserialize_hash_map_h160_account")]
	#[serde(serialize_with = "serialize_hash_map_h160_account")]
	pub accounts: HashMap<H160, Account>,
	#[serde(deserialize_with = "deserialize_script_hash")]
	#[serde(serialize_with = "serialize_script_hash")]
	pub(crate) default_account: H160,
	/// Additional wallet metadata stored as key-value pairs
	#[serde(skip_serializing_if = "Option::is_none")]
	pub extra: Option<HashMap<String, String>>,
}

impl Default for Wallet {
	fn default() -> Self {
		Self {
			name: Wallet::DEFAULT_WALLET_NAME.to_string(),
			version: Wallet::CURRENT_VERSION.to_string(),
			scrypt_params: ScryptParamsDef::default(),
			accounts: HashMap::new(),
			default_account: H160::default(),
			extra: None,
		}
	}
}

impl WalletTrait for Wallet {
	type Account = Account;

	fn name(&self) -> &String {
		&self.name
	}

	fn version(&self) -> &String {
		&self.version
	}

	fn scrypt_params(&self) -> &ScryptParamsDef {
		&self.scrypt_params
	}

	fn accounts(&self) -> Vec<Self::Account> {
		self.accounts.values().cloned().collect::<Vec<Self::Account>>()
	}

	fn default_account(&self) -> Option<&Account> {
		self.accounts.get(&self.default_account)
	}

	fn set_name(&mut self, name: String) {
		self.name = name;
	}

	fn set_version(&mut self, version: String) {
		self.version = version;
	}

	fn set_scrypt_params(&mut self, params: ScryptParamsDef) {
		self.scrypt_params = params;
	}

	fn set_default_account(&mut self, default_account: H160) {
		self.default_account = default_account;
		self.sync_default_account_flags();
	}

	fn add_account(&mut self, account: Self::Account) {
		// let weak_self = Arc::new(&self);
		// account.set_wallet(Some(Arc::downgrade(weak_self)));
		let script_hash = account.get_script_hash();
		let was_empty = self.accounts.is_empty();

		self.accounts.insert(script_hash, account);
		if was_empty {
			self.default_account = script_hash;
		}
		self.sync_default_account_flags();
	}

	fn remove_account(&mut self, hash: &H160) -> Option<Self::Account> {
		let removed = self.accounts.remove(hash);
		if removed.is_some() {
			if self.default_account == *hash {
				self.promote_default_account();
			} else {
				self.sync_default_account_flags();
			}
		}
		removed
	}
}

impl Wallet {
	/// The default wallet name.
	pub const DEFAULT_WALLET_NAME: &'static str = "NeoWallet";
	/// The current wallet version.
	pub const CURRENT_VERSION: &'static str = "1.0";

	fn sync_default_account_flags(&mut self) {
		for (hash, account) in self.accounts.iter_mut() {
			account.is_default = *hash == self.default_account;
		}
	}

	fn promote_default_account(&mut self) {
		self.default_account = self
			.accounts
			.keys()
			.copied()
			.min_by_key(|hash| hash.to_fixed_bytes())
			.unwrap_or_default();
		self.sync_default_account_flags();
	}

	fn effective_scrypt_params(&self) -> Params {
		Params::new(self.scrypt_params.log_n, self.scrypt_params.r, self.scrypt_params.p)
			.unwrap_or_else(|e| {
				tracing::warn!(
					error = %e,
					log_n = self.scrypt_params.log_n,
					r = self.scrypt_params.r,
					p = self.scrypt_params.p,
					"Invalid scrypt params; falling back to Neo defaults"
				);
				Params::new(
					NeoConstants::SCRYPT_LOG_N,
					NeoConstants::SCRYPT_R,
					NeoConstants::SCRYPT_P,
				)
				.unwrap_or_else(|e| {
					tracing::error!(
						error = %e,
						"Neo default scrypt parameters are invalid; falling back to scrypt recommended params"
					);
					Params::RECOMMENDED
				})
			})
	}

	/// Creates a new wallet instance with a default account.
	///
	/// This convenience constructor panics if account generation fails.
	/// Use [`Wallet::try_new`] when you need the failure surfaced to the caller.
	pub fn new() -> Self {
		Self::try_new_with_account_factory(Account::create).unwrap_or_else(|e| {
			panic!(
				"failed to create default wallet; use Wallet::try_new for fallible handling: {e}"
			)
		})
	}

	/// Creates a new wallet instance with a generated default account.
	pub fn try_new() -> Result<Self, WalletError> {
		Self::try_new_with_account_factory(Account::create)
	}

	fn try_new_with_account_factory<F>(create_account: F) -> Result<Self, WalletError>
	where
		F: FnOnce() -> Result<Account, ProviderError>,
	{
		let mut account = create_account().map_err(WalletError::ProviderError)?;
		account.is_default = true;

		let default_account_hash = account.address_or_scripthash.script_hash();
		let mut accounts = HashMap::new();
		accounts.insert(default_account_hash, account);
		Ok(Self {
			name: Self::DEFAULT_WALLET_NAME.to_string(),
			version: Self::CURRENT_VERSION.to_string(),
			scrypt_params: ScryptParamsDef::default(),
			accounts,
			default_account: default_account_hash,
			extra: None,
		})
	}

	/// Converts the wallet to a NEP6Wallet format.
	pub fn to_nep6(&self) -> Result<Nep6Wallet, WalletError> {
		let accounts = self
			.accounts
			.values()
			.map(NEP6Account::from_account)
			.collect::<Result<Vec<NEP6Account>, _>>()?;

		Ok(Nep6Wallet {
			name: self.name.clone(),
			version: self.version.clone(),
			scrypt: self.scrypt_params.clone(),
			accounts,
			extra: None,
		})
	}

	/// Creates a wallet from a NEP6Wallet format.
	pub fn from_nep6(nep6: Nep6Wallet) -> Result<Self, WalletError> {
		let accounts = nep6
			.accounts()
			.iter()
			.map(NEP6Account::to_account)
			.collect::<Result<Vec<_>, _>>()?;

		if accounts.is_empty() {
			tracing::warn!("No accounts found in NEP6 wallet");
			return Err(WalletError::NoAccounts);
		}

		let default_account =
			if let Some(account) = accounts.iter().find(|account| account.is_default) {
				account.get_script_hash()
			} else {
				tracing::warn!("No default account found, using first account");
				accounts[0].get_script_hash()
			};

		let mut wallet = Self {
			name: nep6.name().clone(),
			version: nep6.version().clone(),
			scrypt_params: nep6.scrypt().clone(),
			accounts: accounts
				.into_iter()
				.map(|account| (account.get_script_hash(), account))
				.collect(),
			default_account,
			extra: nep6.extra.clone(),
		};
		wallet.sync_default_account_flags();
		Ok(wallet)
	}

	pub fn from_account(account: &Account) -> Result<Wallet, WalletError> {
		let mut wallet: Wallet = Wallet::default();
		wallet.add_account(account.clone());
		Ok(wallet)
	}

	/// Adds the given accounts to this wallet, if it doesn't contain an account with the same script hash (address).
	///
	/// # Parameters
	///
	/// * `accounts` - The accounts to add
	///
	/// # Returns
	///
	/// Returns the mutable wallet reference if the accounts were successfully added, or a `WalletError` if an account is already contained in another wallet.
	///
	/// # Errors
	///
	/// Returns a `WalletError::IllegalArgument` error if an account is already contained in another wallet.
	///
	/// # Example
	///
	/// ```
	///
	/// use neo3::prelude::*;
	/// use neo3::neo_protocol::AccountTrait;
	/// let account1 = protocol::Account::create().unwrap();
	/// let account2 = protocol::Account::create().unwrap();
	///
	/// let mut wallet = wallets::Wallet::from_accounts(vec![account1, account2]).unwrap();
	/// ```
	pub fn from_accounts(accounts: Vec<Account>) -> Result<Wallet, WalletError> {
		let mut wallet: Wallet = Wallet::default();
		for account in &accounts {
			wallet.add_account(account.clone());
			// account.wallet = Some(self);
		}
		if let Some(first_account) = accounts.first() {
			wallet.set_default_account(first_account.get_script_hash());
		} else {
			return Err(WalletError::NoAccounts);
		}
		Ok(wallet)
	}

	pub fn save_to_file(&self, path: PathBuf) -> Result<(), WalletError> {
		// Convert wallet to NEP6
		let nep6 = self.to_nep6()?;

		// Encode as JSON
		let json = serde_json::to_string(&nep6).map_err(|e| {
			WalletError::AccountState(format!("Failed to serialize wallet to JSON: {e}"))
		})?;

		// Write to file at path. NEP-6 wallets hold encrypted key material, so
		// the file is created owner-only (0600) on Unix instead of the default
		// world-readable mode.
		#[cfg(unix)]
		let mut file = OpenOptions::new()
			.write(true)
			.create(true)
			.truncate(true)
			.mode(0o600)
			.open(&path)
			.map_err(|e| WalletError::FileError(format!("Failed to create wallet file: {e}")))?;
		#[cfg(not(unix))]
		let mut file = File::create(&path)
			.map_err(|e| WalletError::FileError(format!("Failed to create wallet file: {e}")))?;

		file.write_all(json.as_bytes())
			.map_err(|e| WalletError::FileError(format!("Failed to write wallet file: {e}")))?;

		Ok(())
	}

	pub fn get_account(&self, script_hash: &H160) -> Option<&Account> {
		self.accounts.get(script_hash)
	}

	pub fn remove_account(&mut self, script_hash: &H160) -> bool {
		WalletTrait::remove_account(self, script_hash).is_some()
	}

	pub fn encrypt_accounts(&mut self, password: &str) -> Result<(), WalletError> {
		let params = self.effective_scrypt_params();
		let mut errors = Vec::new();

		for account in self.accounts.values_mut() {
			// Only encrypt accounts that have a key pair
			if account.key_pair().is_some()
				&& let Err(e) = account.encrypt_private_key_with_params(password, &params)
			{
				tracing::warn!(
					address = %account.get_address(),
					error = %e,
					"Failed to encrypt private key for account"
				);
				errors.push(format!("{}: {e}", account.get_address()));
			}
		}

		if errors.is_empty() {
			Ok(())
		} else {
			Err(WalletError::AccountState(format!(
				"Failed to encrypt {} account(s): {}",
				errors.len(),
				errors.join("; ")
			)))
		}
	}

	/// Encrypts all accounts in the wallet using parallel processing.
	///
	/// This method provides significant performance improvements when dealing with
	/// wallets containing many accounts by leveraging Rayon's parallel iteration.
	/// The encryption of each account is independent and CPU-intensive (due to
	/// scrypt key derivation), making it ideal for parallelization.
	///
	/// # Arguments
	///
	/// * `password` - The password to use for encrypting all accounts
	///
	/// # Performance Notes
	///
	/// - Uses Rayon's work-stealing thread pool for optimal CPU utilization
	/// - Each account encryption is processed in parallel
	/// - Thread pool size automatically adjusts to available CPU cores
	/// - Performance gains scale with the number of accounts and CPU cores
	///
	/// # Example
	///
	/// ```no_run
	/// # use neo3::prelude::*;
	/// # let mut wallet = wallets::Wallet::new();
	/// // For wallets with many accounts, use parallel encryption
	/// wallet.encrypt_accounts_parallel("strong_password").unwrap();
	/// ```
	pub fn encrypt_accounts_parallel(&mut self, password: &str) -> Result<(), WalletError> {
		let params = self.effective_scrypt_params();

		// Collect errors in a thread-safe manner
		let errors: Vec<(String, String)> = self
			.accounts
			.par_iter_mut()
			.filter_map(|(_, account)| {
				// Only encrypt accounts that have a key pair
				if account.key_pair().is_some() {
					match account.encrypt_private_key_with_params(password, &params) {
						Err(e) => Some((account.get_address(), e.to_string())),
						Ok(_) => None,
					}
				} else {
					None
				}
			})
			.collect();

		// Log any errors that occurred
		for (address, error) in &errors {
			tracing::warn!(address = %address, error = %error, "Failed to encrypt private key");
		}

		if errors.is_empty() {
			Ok(())
		} else {
			Err(WalletError::AccountState(format!(
				"Failed to encrypt {} account(s): {}",
				errors.len(),
				errors
					.into_iter()
					.map(|(address, error)| format!("{address}: {error}"))
					.collect::<Vec<_>>()
					.join("; ")
			)))
		}
	}

	/// Encrypts accounts in parallel with custom thread pool configuration.
	///
	/// This method allows fine-tuning of the parallel encryption process by
	/// configuring the number of threads used. This can be useful in scenarios
	/// where you want to limit CPU usage or optimize for specific hardware.
	///
	/// # Arguments
	///
	/// * `password` - The password to use for encrypting all accounts
	/// * `num_threads` - The number of threads to use for parallel processing
	///
	/// # Example
	///
	/// ```no_run
	/// # use neo3::prelude::*;
	/// # let mut wallet = wallets::Wallet::new();
	/// // Use 4 threads for encryption
	/// wallet.encrypt_accounts_parallel_with_threads("strong_password", 4).unwrap();
	/// ```
	pub fn encrypt_accounts_parallel_with_threads(
		&mut self,
		password: &str,
		num_threads: usize,
	) -> Result<(), WalletError> {
		// Create a custom thread pool with the specified number of threads
		let pool = match rayon::ThreadPoolBuilder::new().num_threads(num_threads).build() {
			Ok(pool) => pool,
			Err(err) => {
				tracing::warn!(
					threads = num_threads,
					error = %err,
					"Failed to build custom rayon thread pool; falling back to default pool"
				);
				return self.encrypt_accounts_parallel(password);
			},
		};

		pool.install(|| self.encrypt_accounts_parallel(password))
	}

	/// Encrypts accounts in parallel using batch processing.
	///
	/// This method processes accounts in batches, which can be more efficient
	/// for very large wallets by reducing overhead and improving cache locality.
	/// It uses a different approach than the standard parallel method by collecting
	/// account data first to avoid mutable borrow conflicts.
	///
	/// # Arguments
	///
	/// * `password` - The password to use for encrypting all accounts
	/// * `batch_size` - The number of accounts to process in each batch
	///
	/// # Example
	///
	/// ```no_run
	/// # use neo3::prelude::*;
	/// # let mut wallet = wallets::Wallet::new();
	/// // Process accounts in batches of 50
	/// wallet.encrypt_accounts_batch_parallel("strong_password", 50).unwrap();
	/// ```
	pub fn encrypt_accounts_batch_parallel(
		&mut self,
		password: &str,
		batch_size: usize,
	) -> Result<(), WalletError> {
		use std::sync::{Arc, Mutex};

		let params = self.effective_scrypt_params();

		// Collect accounts that need encryption along with their script hashes
		let accounts_to_encrypt: Vec<(H160, Account)> = self
			.accounts
			.iter()
			.filter(|(_, account)| account.key_pair().is_some())
			.map(|(hash, account)| (*hash, account.clone()))
			.collect();

		// Process in parallel batches and collect results
		let results: Arc<Mutex<Vec<(H160, Result<Account, String>)>>> =
			Arc::new(Mutex::new(Vec::new()));

		accounts_to_encrypt.par_chunks(batch_size).for_each(|batch| {
			let batch_results: Vec<(H160, Result<Account, String>)> = batch
				.iter()
				.map(|(hash, account)| {
					let mut account_clone = account.clone();
					match account_clone.encrypt_private_key_with_params(password, &params) {
						Ok(_) => (*hash, Ok(account_clone)),
						Err(e) => (*hash, Err(format!("{}: {}", account.get_address(), e))),
					}
				})
				.collect();

			results.lock().unwrap_or_else(|e| e.into_inner()).extend(batch_results);
		});

		// Apply successful encryptions and collect errors
		let results = match Arc::try_unwrap(results) {
			Ok(mutex) => mutex.into_inner().unwrap_or_else(|e| e.into_inner()),
			Err(arc) => arc.lock().unwrap_or_else(|e| e.into_inner()).clone(),
		};
		let mut errors = Vec::new();
		for (hash, result) in results {
			match result {
				Ok(encrypted_account) => {
					self.accounts.insert(hash, encrypted_account);
				},
				Err(error_msg) => {
					tracing::warn!("Failed to encrypt private key for account {}", error_msg);
					errors.push(error_msg);
				},
			}
		}

		if errors.is_empty() {
			Ok(())
		} else {
			Err(WalletError::AccountState(format!(
				"Failed to encrypt {} account(s): {}",
				errors.len(),
				errors.join("; ")
			)))
		}
	}

	/// Creates a new wallet and saves it to the specified path
	///
	/// This method has been renamed to `create_wallet` for clarity.
	/// Please use `create_wallet` instead.
	///
	/// # Arguments
	///
	/// * `path` - The file path where the wallet will be saved
	/// * `password` - The password to encrypt the wallet
	///
	/// # Returns
	///
	/// A `Result` containing the new wallet or a `WalletError`
	#[deprecated(since = "0.1.0", note = "Please use `create_wallet` instead")]
	pub fn create(path: &Path, password: &str) -> Result<Self, WalletError> {
		Self::create_wallet(path, password)
	}

	/// Opens a wallet from the specified path
	///
	/// This method has been renamed to `open_wallet` for clarity.
	/// Please use `open_wallet` instead.
	///
	/// # Arguments
	///
	/// * `path` - The file path of the wallet to open
	/// * `password` - The password to decrypt the wallet
	///
	/// # Returns
	///
	/// A `Result` containing the opened wallet or a `WalletError`
	#[deprecated(since = "0.1.0", note = "Please use `open_wallet` instead")]
	pub fn open(path: &Path, password: &str) -> Result<Self, WalletError> {
		Self::open_wallet(path, password)
	}

	/// Returns all accounts in the wallet
	pub fn get_accounts(&self) -> Vec<&Account> {
		self.accounts.values().collect()
	}

	/// Creates a new account in the wallet
	pub fn create_account(&mut self) -> Result<&Account, WalletError> {
		let account = Account::create()?;
		self.add_account(account.clone());
		self.get_account(&account.get_script_hash()).ok_or_else(|| {
			WalletError::AccountState("Account was added but could not be retrieved".to_string())
		})
	}

	/// Imports a private key in WIF format
	pub fn import_private_key(&mut self, wif: &str) -> Result<&Account, WalletError> {
		let key_pair = KeyPair::from_wif(wif)
			.map_err(|e| WalletError::AccountState(format!("Failed to import private key: {e}")))?;

		let account =
			Account::from_key_pair(key_pair, None, None).map_err(WalletError::ProviderError)?;
		self.add_account(account.clone());
		self.get_account(&account.get_script_hash()).ok_or_else(|| {
			WalletError::AccountState("Account was added but could not be retrieved".to_string())
		})
	}

	/// Verifies if the provided password is correct by attempting to decrypt any encrypted account
	///
	/// This function checks if the provided password can successfully decrypt at least one
	/// of the encrypted private keys in the wallet. If at least one account can be decrypted,
	/// the password is considered valid.
	///
	/// Returns true if the password is correct, false otherwise.
	pub fn verify_password(&self, password: &str) -> bool {
		// If there are no accounts, we can't verify the password
		if self.accounts.is_empty() {
			return false;
		}

		let params = self.effective_scrypt_params();

		// Try to decrypt any account with the provided password
		for account in self.accounts.values() {
			// Skip accounts that don't have an encrypted private key
			if account.encrypted_private_key().is_none() {
				continue;
			}

			// Skip accounts that already have a key pair (already decrypted)
			if account.key_pair().is_some() {
				continue;
			}

			// Try to decrypt the account's private key
			let mut account_clone = account.clone();
			match account_clone.decrypt_private_key_with_params(password, &params) {
				Ok(_) => return true, // Password decrypted successfully
				Err(_) => continue,   // Try the next account
			}
		}

		// If we get here, none of the accounts could be decrypted with the provided password
		false
	}

	/// Changes the wallet password
	pub fn change_password(
		&mut self,
		current_password: &str,
		new_password: &str,
	) -> Result<(), WalletError> {
		if new_password.is_empty() {
			return Err(WalletError::InvalidPassword);
		}

		if !self.verify_password(current_password) {
			return Err(WalletError::AccountState("Invalid password".to_string()));
		}

		let params = self.effective_scrypt_params();
		let mut updated_accounts = self.accounts.clone();

		// First decrypt all accounts with the current password
		for account in updated_accounts.values_mut() {
			if account.encrypted_private_key().is_some()
				&& account.key_pair().is_none()
				&& let Err(e) = account.decrypt_private_key_with_params(current_password, &params)
			{
				return Err(WalletError::DecryptionError(format!(
					"Failed to decrypt account {}: {}",
					account.get_address(),
					e
				)));
			}
		}

		// Re-encrypt all accounts with the new password
		let mut errors = Vec::new();
		for account in updated_accounts.values_mut() {
			if account.key_pair().is_some()
				&& let Err(e) = account.encrypt_private_key_with_params(new_password, &params)
			{
				errors.push(format!("{}: {e}", account.get_address()));
			}
		}

		if errors.is_empty() {
			self.accounts = updated_accounts;
			Ok(())
		} else {
			Err(WalletError::AccountState(format!(
				"Failed to encrypt {} account(s): {}",
				errors.len(),
				errors.join("; ")
			)))
		}
	}

	/// Changes the wallet password using parallel processing.
	///
	/// This method provides better performance for wallets with many accounts
	/// by parallelizing both the decryption and re-encryption processes.
	///
	/// # Arguments
	///
	/// * `current_password` - The current password of the wallet
	/// * `new_password` - The new password to set
	///
	/// # Returns
	///
	/// A `Result` indicating success or containing a `WalletError` on failure
	///
	/// # Example
	///
	/// ```no_run
	/// # use neo3::prelude::*;
	/// # let mut wallet = wallets::Wallet::new();
	/// wallet.change_password_parallel("old_password", "new_password").unwrap();
	/// ```
	pub fn change_password_parallel(
		&mut self,
		current_password: &str,
		new_password: &str,
	) -> Result<(), WalletError> {
		if new_password.is_empty() {
			return Err(WalletError::InvalidPassword);
		}

		if !self.verify_password(current_password) {
			return Err(WalletError::AccountState("Invalid password".to_string()));
		}

		let params = self.effective_scrypt_params();
		let mut updated_accounts = self.accounts.clone();

		// Collect accounts that need decryption
		let accounts_to_decrypt: Vec<(H160, Account)> = updated_accounts
			.iter()
			.filter(|(_, account)| {
				account.encrypted_private_key().is_some() && account.key_pair().is_none()
			})
			.map(|(hash, account)| (*hash, account.clone()))
			.collect();

		// Decrypt accounts in parallel
		let decrypted_results: Vec<(H160, Result<Account, String>)> = accounts_to_decrypt
			.into_par_iter()
			.map(|(hash, account)| {
				let mut account_clone = account.clone();
				match account_clone.decrypt_private_key_with_params(current_password, &params) {
					Ok(_) => (hash, Ok(account_clone)),
					Err(e) => (hash, Err(format!("{}: {}", account.get_address(), e))),
				}
			})
			.collect();

		// Check for decryption errors
		for (_, result) in &decrypted_results {
			if let Err(error_msg) = result {
				return Err(WalletError::DecryptionError(format!(
					"Failed to decrypt account {}",
					error_msg
				)));
			}
		}

		// Apply successful decryptions
		for (hash, result) in decrypted_results {
			if let Ok(decrypted_account) = result {
				updated_accounts.insert(hash, decrypted_account);
			}
		}

		// Re-encrypt all accounts with the new password using parallel processing
		let errors: Vec<(String, String)> = updated_accounts
			.par_iter_mut()
			.filter_map(|(_, account)| {
				if account.key_pair().is_some() {
					match account.encrypt_private_key_with_params(new_password, &params) {
						Err(e) => Some((account.get_address(), e.to_string())),
						Ok(_) => None,
					}
				} else {
					None
				}
			})
			.collect();

		if errors.is_empty() {
			self.accounts = updated_accounts;
			Ok(())
		} else {
			Err(WalletError::AccountState(format!(
				"Failed to encrypt {} account(s): {}",
				errors.len(),
				errors
					.into_iter()
					.map(|(address, error)| format!("{address}: {error}"))
					.collect::<Vec<_>>()
					.join("; ")
			)))
		}
	}

	/// Gets the unclaimed GAS for all accounts in the wallet
	pub async fn get_unclaimed_gas<P>(&self, rpc_client: &P) -> Result<UnclaimedGas, WalletError>
	where
		P: JsonRpcProvider + APITrait + 'static,
		<P as APITrait>::Error: Into<ProviderError>,
	{
		let mut total_unclaimed = UnclaimedGas::default();

		for account in self.get_accounts() {
			let script_hash = account.get_script_hash();
			let unclaimed = rpc_client
				.get_unclaimed_gas(script_hash)
				.await
				.map_err(|e| WalletError::ProviderError(e.into()))?;

			total_unclaimed += unclaimed;
		}

		Ok(total_unclaimed)
	}
}

impl Wallet {
	/// Signs a given message using the default account's private key.
	///
	/// This method computes the SHA-256 hash of the input message and then signs it
	/// using the ECDSA Secp256r1 algorithm. It's primarily used for generating signatures
	/// that can prove ownership of an address or for other cryptographic verifications.
	///
	/// # Parameters
	///
	/// - `message`: The message to be signed. This can be any data that implements `AsRef<[u8]>`,
	///   allowing for flexibility in the type of data that can be signed.
	///
	/// # Returns
	///
	/// A `Result` that, on success, contains the `Secp256r1Signature` of the message. On failure,
	/// it returns a `WalletError`, which could indicate issues like a missing key pair.
	///
	/// # Example
	///
	/// ```no_run
	/// # use neo3::prelude::*;
	///  async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// # let wallet = wallets::Wallet::new();
	/// let message = "Hello, world!";
	/// let signature = wallet.sign_message(message).await?;
	/// println!("Signed message: {:?}", signature);
	/// # Ok(())
	/// # }
	/// ```
	pub async fn sign_message<S: Send + Sync + AsRef<[u8]>>(
		&self,
		message: S,
	) -> Result<Secp256r1Signature, WalletError> {
		let message = message.as_ref();
		let binding = message.hash256();
		let message_hash = binding.as_slice();
		self.default_account()
			.ok_or(WalletError::NoDefaultAccount)?
			.key_pair()
			.clone()
			.ok_or(WalletError::NoKeyPair)?
			.sign(message_hash)
			.map_err(|_e| WalletError::NoKeyPair)
	}

	/// Generates a witness for a transaction using the default account's key pair.
	///
	/// This method is used to attach a signature to a transaction, proving that the
	/// transaction was authorized by the owner of the default account. It's an essential
	/// step in transaction validation for blockchain systems.
	///
	/// # Parameters
	///
	/// - `tx`: A reference to the transaction that needs a witness.
	///
	/// # Returns
	///
	/// A `Result` that, on success, contains the `Witness` for the given transaction.
	/// On failure, it returns a `WalletError`, which could be due to issues like a missing
	/// key pair.
	///
	/// # Example
	///
	/// ```no_run
	/// # use neo3::prelude::*;
	///  async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// # let wallet = wallets::Wallet::new();
	/// # let provider = providers::HttpProvider::new("http://localhost:10332")?;
	/// # let client = providers::RpcClient::new(provider);
	/// # let mut tx_builder = builder::TransactionBuilder::with_client(&client);
	/// # let tx = tx_builder.get_unsigned_tx().await?;
	/// let witness = wallet.get_witness(&tx).await?;
	/// println!("Witness: {:?}", witness);
	/// # Ok(())
	/// # }
	/// ```
	pub async fn get_witness<'a, P: JsonRpcProvider + 'static>(
		&self,
		tx: &Transaction<'a, P>,
	) -> Result<Witness, WalletError> {
		let tx_with_chain = tx;
		if tx_with_chain.network().is_none() {
			// in the case we don't have a network, let's use the signer network magic instead
			// tx_with_chain.set_network(Some(self.network()));
		}

		let account = self.default_account().ok_or(WalletError::NoDefaultAccount)?;
		let key_pair = account.key_pair.clone().ok_or(WalletError::NoKeyPair)?;
		Witness::create(tx.get_hash_data().await?, &key_pair).map_err(|_e| WalletError::NoKeyPair)
	}

	/// Signs a transaction using the specified account.
	///
	/// # Arguments
	///
	/// * `tx_builder` - The transaction builder containing the transaction to sign
	/// * `account_address` - The address of the account to use for signing
	/// * `password` - The password to decrypt the account's private key if needed
	///
	/// # Returns
	///
	/// A `Result` containing the signed transaction or a `WalletError`
	pub async fn sign_transaction<'a, P>(
		&self,
		tx_builder: &'a mut TransactionBuilder<'a, P>,
		account_address: &str,
		password: &str,
	) -> Result<Transaction<'a, P>, WalletError>
	where
		P: JsonRpcProvider + 'static,
	{
		// Get the account from the wallet
		let script_hash = H160::from_address(account_address)
			.map_err(|e| WalletError::AccountState(format!("Invalid address: {e}")))?;

		let account = self.get_account(&script_hash).ok_or_else(|| {
			WalletError::AccountState(format!("Account not found: {account_address}"))
		})?;

		// Ensure the account has a key pair or can be decrypted
		let key_pair = match account.key_pair() {
			Some(kp) => kp.clone(),
			None => {
				// Try to decrypt the account with the provided password
				let mut account_clone = account.clone();
				account_clone.decrypt_private_key(password).map_err(|e| {
					WalletError::DecryptionError(format!("Failed to decrypt account: {e}"))
				})?;

				match account_clone.key_pair() {
					Some(kp) => kp.clone(),
					None => return Err(WalletError::NoKeyPair),
				}
			},
		};

		// Build the transaction
		let mut tx = tx_builder.get_unsigned_tx().await?;

		// Create a witness for the transaction
		let witness = Witness::create(tx.get_hash_data().await?, &key_pair)
			.map_err(|e| WalletError::SigningError(format!("Failed to create witness: {e}")))?;

		// Add the witness to the transaction
		tx.add_witness(witness);

		Ok(tx)
	}

	/// Returns the address of the wallet's default account.
	///
	/// This method provides access to the blockchain address associated with the
	/// wallet's default account, which is typically used as the sender address in
	/// transactions.
	///
	/// # Returns
	///
	/// The `Address` of the wallet's default account.
	#[allow(dead_code)]
	fn address(&self) -> String {
		// Get the default account's address
		if let Some(account) = self.get_account(&self.default_account) {
			account.address_or_scripthash.address()
		} else {
			// Return a default address if no default account exists
			H160::default().to_address()
		}
	}

	/// Creates a new wallet with the specified path and password.
	///
	/// # Arguments
	///
	/// * `path` - The file path where the wallet will be saved
	/// * `password` - The password to encrypt the wallet
	///
	/// # Returns
	///
	/// A `Result` containing the new wallet or a `WalletError`
	pub fn create_wallet(path: &Path, password: &str) -> Result<Self, WalletError> {
		let mut wallet = Wallet::default();

		// Create a new account for the wallet
		let account = Account::create().map_err(WalletError::ProviderError)?;
		wallet.add_account(account);

		// Encrypt the wallet with the provided password
		wallet.encrypt_accounts(password)?;

		// Save the wallet to the specified path
		wallet.save_to_file(path.to_path_buf())?;

		Ok(wallet)
	}

	/// Opens an existing wallet from the specified path with the given password.
	///
	/// # Arguments
	///
	/// * `path` - The file path of the wallet to open
	/// * `password` - The password to decrypt the wallet
	///
	/// # Returns
	///
	/// A `Result` containing the opened wallet or a `WalletError`
	pub fn open_wallet(path: &Path, password: &str) -> Result<Self, WalletError> {
		// Read the wallet file
		let wallet_json = std::fs::read_to_string(path)
			.map_err(|e| WalletError::FileError(format!("Failed to read wallet file: {e}")))?;

		// Parse the wallet JSON
		let nep6_wallet: Nep6Wallet = serde_json::from_str(&wallet_json).map_err(|e| {
			WalletError::DeserializationError(format!("Failed to parse wallet JSON: {e}"))
		})?;

		// Convert to Wallet
		let wallet = Wallet::from_nep6(nep6_wallet)?;

		// Verify the password by checking if we can decrypt any account
		let can_decrypt = wallet.verify_password(password);

		if !can_decrypt {
			return Err(WalletError::CryptoError(CryptoError::InvalidPassphrase(
				"Invalid password".to_string(),
			)));
		}

		Ok(wallet)
	}

	/// Returns all accounts in the wallet.
	///
	/// This is equivalent to [`get_accounts`](Self::get_accounts).
	/// Prefer `get_accounts` for consistency.
	#[deprecated(since = "1.0.8", note = "Use `get_accounts` instead")]
	pub fn get_all_accounts(&self) -> Vec<&Account> {
		self.accounts.values().collect()
	}

	/// Creates a new account in the wallet.
	///
	/// This is equivalent to [`create_account`](Self::create_account).
	/// Prefer `create_account` for consistency.
	#[deprecated(since = "1.0.8", note = "Use `create_account` instead")]
	pub fn create_new_account(&mut self) -> Result<&Account, WalletError> {
		let account = Account::create().map_err(WalletError::ProviderError)?;
		let script_hash = account.address_or_scripthash.script_hash();
		self.add_account(account);

		self.get_account(&script_hash).ok_or_else(|| {
			WalletError::AccountState("Account was added but could not be retrieved".to_string())
		})
	}

	/// Imports a private key in WIF format.
	///
	/// This is equivalent to [`import_private_key`](Self::import_private_key).
	/// Prefer `import_private_key` for consistency.
	#[deprecated(since = "1.0.8", note = "Use `import_private_key` instead")]
	pub fn import_from_wif(&mut self, private_key: &str) -> Result<&Account, WalletError> {
		// Create a key pair from the private key
		let key_pair = KeyPair::from_wif(private_key).map_err(WalletError::CryptoError)?;

		// Create an account from the key pair
		let account = Account::from_key_pair(key_pair, None, None)
			.map_err(|e| WalletError::AccountState(format!("Failed to create account: {e}")))?;
		let script_hash = account.address_or_scripthash.script_hash();

		// Add the account to the wallet
		self.add_account(account);

		self.get_account(&script_hash).ok_or_else(|| {
			WalletError::AccountState("Account was added but could not be retrieved".to_string())
		})
	}

	/// Gets the unclaimed GAS for the wallet as a float value.
	///
	/// # Arguments
	///
	/// * `rpc_client` - The RPC client to use for the query
	///
	/// # Returns
	///
	/// A `Result` containing the unclaimed GAS amount as a float or a `WalletError`
	pub async fn get_unclaimed_gas_as_float<P>(
		&self,
		rpc_client: &RpcClient<P>,
	) -> Result<f64, WalletError>
	where
		P: JsonRpcProvider + 'static,
	{
		let mut total_unclaimed = 0.0;

		// Get unclaimed GAS for each account
		for account in self.accounts.values() {
			let script_hash = account.address_or_scripthash.script_hash();

			// Query the RPC client for unclaimed GAS
			let unclaimed = rpc_client
				.get_unclaimed_gas(script_hash)
				.await
				.map_err(WalletError::ProviderError)?;

			// Add to the total
			total_unclaimed += unclaimed.unclaimed.parse::<f64>().unwrap_or(0.0);
		}

		Ok(total_unclaimed)
	}

	/// Retrieves the network ID associated with the wallet.
	///
	/// This network ID is used for network-specific operations, such as signing
	/// transactions with EIP-155 to prevent replay attacks across chains.
	///
	/// # Returns
	///
	/// The network ID as a `u32`.
	#[allow(dead_code)]
	fn network(&self) -> u32 {
		// Default to MainNet if not specified
		self.extra
			.as_ref()
			.and_then(|extra| {
				extra
					.get("network")
					.map(|n| n.parse::<u32>().unwrap_or(NeoConstants::MAGIC_NUMBER_MAINNET))
			})
			.unwrap_or(NeoConstants::MAGIC_NUMBER_MAINNET)
	}

	//// Sets the network magic (ID) for the wallet.
	///
	/// This method configures the wallet to operate within a specific blockchain
	/// network by setting the network magic (ID), which is essential for correctly
	/// signing transactions.
	///
	/// # Parameters
	///
	/// - `network`: The network ID to set for the wallet.
	///
	/// # Returns
	///
	/// The modified `Wallet` instance with the new network ID set.
	///
	/// # Example
	///
	/// ```no_run
	/// # use neo3::prelude::*;
	/// let mut wallet = wallets::Wallet::new();
	/// wallet = wallet.with_network(0x334F454E); // MainNet magic number
	/// ```
	#[must_use]
	pub fn with_network(mut self, network: u32) -> Self {
		let mut extra = self.extra.unwrap_or_default();
		extra.insert("network".to_string(), network.to_string());
		self.extra = Some(extra);
		self
	}
}

#[cfg(test)]
mod tests {
	use crate::{
		ScryptParamsDef,
		neo_clients::ProviderError,
		neo_config::TestConstants,
		neo_protocol::{Account, AccountTrait},
		neo_wallets::{NEP6Account, Nep6Wallet, Wallet, WalletError, WalletTrait},
	};
	use primitive_types::H160;
	use tempfile::tempdir;

	fn apply_fast_scrypt(wallet: &mut Wallet) {
		wallet.set_scrypt_params(ScryptParamsDef { log_n: 10, r: 8, p: 1 });
	}

	#[test]
	fn test_is_default() {
		let account = Account::from_address(TestConstants::DEFAULT_ACCOUNT_ADDRESS)
			.expect("Should be able to create account from valid address in test");
		let mut wallet: Wallet = Wallet::new();
		wallet.add_account(account.clone());

		assert!(!account.is_default);

		let hash = account.address_or_scripthash.script_hash();
		wallet.set_default_account(hash);
		assert!(wallet.get_account(&hash).expect("Account should exist in wallet").is_default);
	}

	#[test]
	fn test_create_default_wallet() {
		let wallet: Wallet = Wallet::default();

		assert_eq!(&wallet.name, "NeoWallet");
		assert_eq!(&wallet.version, Wallet::CURRENT_VERSION);
		assert_eq!(wallet.accounts.len(), 0usize);
	}

	#[test]
	fn test_try_new_creates_single_default_account() {
		let wallet = Wallet::try_new().expect("Should create wallet with default account");

		assert_eq!(wallet.accounts.len(), 1);
		let account = wallet.default_account().expect("Wallet should have a default account");
		assert!(account.is_default);
	}

	#[test]
	#[should_panic(
		expected = "failed to create default wallet; use Wallet::try_new for fallible handling"
	)]
	fn test_new_panics_when_account_creation_fails() {
		let _ = Wallet::try_new_with_account_factory(|| {
			Err(ProviderError::CustomError("boom".to_string()))
		})
		.unwrap_or_else(|e| {
			panic!(
				"failed to create default wallet; use Wallet::try_new for fallible handling: {e}"
			)
		});
	}

	#[test]
	fn test_create_wallet_with_accounts() {
		let account1 = Account::create().expect("Should be able to create account in test");
		let account2 = Account::create().expect("Should be able to create account in test");

		let wallet = Wallet::from_accounts(vec![account1.clone(), account2.clone()])
			.expect("Should be able to create wallet from accounts in test");

		assert_eq!(wallet.default_account(), Some(&account1));
		assert_eq!(wallet.accounts.len(), 2);
		assert!(
			wallet
				.accounts
				.values()
				.any(|a| a.get_script_hash() == account1.address_or_scripthash.script_hash())
		);
		assert!(
			wallet
				.accounts
				.values()
				.any(|a| a.get_script_hash() == account2.address_or_scripthash.script_hash())
		);
	}

	#[test]
	fn test_from_account_keeps_only_supplied_account() {
		let account = Account::create().expect("Should be able to create account in test");

		let wallet =
			Wallet::from_account(&account).expect("Should be able to create wallet from account");

		assert_eq!(wallet.accounts.len(), 1);
		assert_eq!(wallet.default_account(), Some(&account));
		assert_eq!(wallet.get_account(&account.get_script_hash()), Some(&account));
	}

	#[test]
	fn test_add_account_to_empty_wallet_sets_default_account() {
		let account = Account::create().expect("Should be able to create account in test");
		let mut wallet = Wallet::default();

		wallet.add_account(account.clone());

		assert_eq!(wallet.default_account(), Some(&account));
		assert!(
			wallet
				.get_account(&account.get_script_hash())
				.expect("Account should exist")
				.is_default
		);
	}

	#[test]
	fn test_set_default_account_with_unknown_hash_leaves_no_default() {
		let account1 = Account::create().expect("Should be able to create account in test");
		let account2 = Account::create().expect("Should be able to create account in test");
		let mut wallet = Wallet::from_accounts(vec![account1.clone(), account2.clone()])
			.expect("Should be able to create wallet from accounts in test");

		wallet.set_default_account(H160::repeat_byte(0xff));

		assert_eq!(wallet.default_account(), None);
		assert!(!wallet.accounts.values().any(|account| account.is_default));
	}

	#[test]
	fn test_is_default_account() {
		let account = Account::create().expect("Should be able to create account in test");
		let wallet = Wallet::from_accounts(vec![account.clone()])
			.expect("Should be able to create wallet from accounts in test");

		assert_eq!(wallet.default_account, account.get_script_hash());
	}

	#[test]
	fn test_add_account() {
		let account = Account::create().expect("Should be able to create account in test");
		let mut wallet: Wallet = Wallet::new();

		wallet.add_account(account.clone());

		assert_eq!(wallet.accounts.len(), 2);
		assert_eq!(
			wallet.get_account(&account.address_or_scripthash.script_hash()),
			Some(&account)
		);
	}

	#[test]
	fn test_encrypt_wallet() {
		let mut wallet: Wallet = Wallet::new();
		apply_fast_scrypt(&mut wallet);
		wallet.add_account(Account::create().expect("Should be able to create account in test"));

		assert!(wallet.accounts()[0].key_pair().is_some());
		assert!(wallet.accounts()[1].key_pair().is_some());

		wallet.encrypt_accounts("pw").unwrap();

		assert!(wallet.accounts()[0].key_pair().is_none());
		assert!(wallet.accounts()[1].key_pair().is_none());
	}

	#[test]
	fn test_encrypt_wallet_parallel() {
		let mut wallet: Wallet = Wallet::new();
		apply_fast_scrypt(&mut wallet);
		// Add multiple accounts to test parallel processing
		for _ in 0..5 {
			wallet
				.add_account(Account::create().expect("Should be able to create account in test"));
		}

		// Verify all accounts have key pairs
		for account in wallet.accounts() {
			assert!(account.key_pair().is_some());
		}

		// Encrypt using parallel method
		wallet.encrypt_accounts_parallel("parallel_password").unwrap();

		// Verify all accounts are now encrypted
		for account in wallet.accounts() {
			assert!(account.key_pair().is_none());
			assert!(account.encrypted_private_key().is_some());
		}
	}

	#[test]
	fn test_encrypt_wallet_batch_parallel() {
		let mut wallet: Wallet = Wallet::new();
		apply_fast_scrypt(&mut wallet);
		// Add many accounts to test batch processing
		for _ in 0..10 {
			wallet
				.add_account(Account::create().expect("Should be able to create account in test"));
		}

		// Verify all accounts have key pairs
		for account in wallet.accounts() {
			assert!(account.key_pair().is_some());
		}

		// Encrypt using batch parallel method with batch size of 3
		wallet.encrypt_accounts_batch_parallel("batch_password", 3).unwrap();

		// Verify all accounts are now encrypted
		for account in wallet.accounts() {
			assert!(account.key_pair().is_none());
			assert!(account.encrypted_private_key().is_some());
		}
	}

	#[test]
	fn test_change_password_parallel() {
		let mut wallet = Wallet::new();
		apply_fast_scrypt(&mut wallet);
		// Add multiple accounts
		for _ in 0..5 {
			wallet
				.add_account(Account::create().expect("Should be able to create account in test"));
		}

		let old_password = "old_password";
		let new_password = "new_password";

		// Initially encrypt the wallet
		wallet.encrypt_accounts(old_password).unwrap();

		// Verify initial encryption
		assert!(wallet.verify_password(old_password));
		assert!(!wallet.verify_password(new_password));

		// Change password using parallel method
		wallet
			.change_password_parallel(old_password, new_password)
			.expect("Password change should succeed");

		// Verify new password works
		assert!(!wallet.verify_password(old_password));
		assert!(wallet.verify_password(new_password));
	}

	#[test]
	fn test_change_password_rejects_empty_new_password_without_mutating_wallet() {
		let mut wallet = Wallet::new();
		apply_fast_scrypt(&mut wallet);

		let old_password = "old_password";
		wallet.encrypt_accounts(old_password).unwrap();
		let encrypted_before: Vec<_> = wallet
			.accounts()
			.iter()
			.map(|account| account.encrypted_private_key().clone())
			.collect();

		let err = wallet.change_password(old_password, "").unwrap_err();
		assert!(matches!(err, WalletError::InvalidPassword));
		assert!(wallet.verify_password(old_password));
		assert_eq!(
			encrypted_before,
			wallet
				.accounts()
				.iter()
				.map(|account| account.encrypted_private_key().clone())
				.collect::<Vec<_>>()
		);
	}

	#[test]
	fn test_to_nep6_rejects_unencrypted_accounts_instead_of_dropping_them() {
		let wallet = Wallet::try_new().expect("Should create wallet with default account");

		let result = wallet.to_nep6();
		assert!(matches!(
			result,
			Err(WalletError::AccountState(message))
				if message.contains("not encrypted")
		));
	}

	#[test]
	fn test_save_to_file_rejects_unencrypted_wallet() {
		let temp_dir = tempdir().expect("Should create temp dir");
		let path = temp_dir.path().join("wallet.json");
		let wallet = Wallet::try_new().expect("Should create wallet with default account");

		let result = wallet.save_to_file(path.clone());
		assert!(matches!(
			result,
			Err(WalletError::AccountState(message))
				if message.contains("not encrypted")
		));
		assert!(!path.exists());
	}

	#[test]
	fn test_from_nep6_rejects_empty_wallet() {
		let nep6_wallet = Nep6Wallet::new(
			"Empty".to_string(),
			Wallet::CURRENT_VERSION.to_string(),
			ScryptParamsDef::default(),
			vec![],
			None,
		);

		let err = Wallet::from_nep6(nep6_wallet).unwrap_err();
		assert!(matches!(err, WalletError::NoAccounts));
	}

	#[test]
	fn test_from_nep6_surfaces_invalid_account_errors() {
		let nep6_wallet = Nep6Wallet::new(
			"Invalid".to_string(),
			Wallet::CURRENT_VERSION.to_string(),
			ScryptParamsDef::default(),
			vec![NEP6Account::new(String::new(), None, true, false, None, None, None)],
			None,
		);

		let err = Wallet::from_nep6(nep6_wallet).unwrap_err();
		assert!(matches!(
			err,
			WalletError::AccountState(message)
				if message.contains("missing both address and verification script")
		));
	}

	#[test]
	fn test_create_wallet_creates_single_encrypted_default_account() {
		let temp_dir = tempdir().expect("Should create temp dir");
		let path = temp_dir.path().join("wallet.json");

		let wallet =
			Wallet::create_wallet(&path, "password123").expect("Should create wallet on disk");

		assert!(path.exists());
		assert_eq!(wallet.accounts.len(), 1);
		let account = wallet.default_account().expect("Wallet should have a default account");
		assert!(account.is_default);
		assert!(account.key_pair().is_none());
		assert!(account.encrypted_private_key().is_some());
	}

	#[test]
	fn test_verify_password() {
		let mut wallet = Wallet::new();
		apply_fast_scrypt(&mut wallet);
		let account = Account::create().unwrap();
		wallet.add_account(account.clone());

		// Initially, the account is not encrypted so verification should fail
		assert!(!wallet.verify_password("password123"));

		// Encrypt the account
		wallet.encrypt_accounts("password123").unwrap();

		// Now verification should succeed with the correct password
		assert!(wallet.verify_password("password123"));

		// And fail with an incorrect password
		assert!(!wallet.verify_password("wrong_password"));
	}

	#[test]
	fn test_remove_default_account_promotes_deterministic_remaining_account() {
		let account1 = Account::create().expect("Should be able to create account in test");
		let account2 = Account::create().expect("Should be able to create account in test");
		let account3 = Account::create().expect("Should be able to create account in test");
		let mut wallet =
			Wallet::from_accounts(vec![account1.clone(), account2.clone(), account3.clone()])
				.expect("Should be able to create wallet from accounts in test");
		let expected_hash = [account2.get_script_hash(), account3.get_script_hash()]
			.into_iter()
			.min_by_key(|hash| hash.to_fixed_bytes())
			.expect("remaining accounts should not be empty");

		assert!(wallet.remove_account(&account1.get_script_hash()));

		assert_eq!(
			wallet.default_account().map(|account| account.get_script_hash()),
			Some(expected_hash)
		);
		assert!(wallet.get_account(&expected_hash).expect("Account should exist").is_default);
	}

	#[test]
	fn test_remove_default_account_promotes_remaining_account() {
		let account1 = Account::create().expect("Should be able to create account in test");
		let account2 = Account::create().expect("Should be able to create account in test");
		let mut wallet = Wallet::from_accounts(vec![account1.clone(), account2.clone()])
			.expect("Should be able to create wallet from accounts in test");

		assert!(wallet.remove_account(&account1.get_script_hash()));

		assert_eq!(wallet.accounts.len(), 1);
		assert_eq!(wallet.default_account(), Some(&account2));
		assert!(
			wallet
				.get_account(&account2.get_script_hash())
				.expect("Account should exist")
				.is_default
		);
	}
}
