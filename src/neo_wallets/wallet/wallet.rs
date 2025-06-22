use std::{collections::HashMap, fs::File, io::Write, path::PathBuf};

use primitive_types::H160;
use rayon::prelude::*;
use serde_derive::{Deserialize, Serialize};

use crate::{
	neo_builder::{AccountSigner, Transaction, TransactionBuilder, VerificationScript, Witness},
	neo_clients::{APITrait, JsonRpcProvider, ProviderError, RpcClient},
	neo_config::NeoConstants,
	neo_contract::ContractError,
	neo_crypto::{CryptoError, HashableForVec, KeyPair, Secp256r1Signature},
	neo_protocol::{Account, AccountTrait, UnclaimedGas},
	neo_types::{
		contract::ContractMethodToken,
		script_hash::ScriptHashExtension,
		serde_with_utils::{
			deserialize_hash_map_h160_account, deserialize_script_hash,
			serialize_hash_map_h160_account, serialize_script_hash,
		},
		AddressExtension, ScryptParamsDef,
	},
	neo_wallets::{NEP6Account, NEP6Contract, NEP6Parameter, Nep6Wallet, WalletError, WalletTrait},
};

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
		self.accounts
			.clone()
			.into_iter()
			.map(|(_k, v)| v.clone())
			.collect::<Vec<Self::Account>>()
	}

	fn default_account(&self) -> &Account {
		&self.accounts[&self.default_account]
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
		self.default_account = default_account.clone();
		if let Some(account) = self.accounts.get_mut(&self.default_account) {
			account.is_default = true;
		}
	}

	fn add_account(&mut self, account: Self::Account) {
		// let weak_self = Arc::new(&self);
		// account.set_wallet(Some(Arc::downgrade(weak_self)));
		self.accounts.insert(account.get_script_hash().clone(), account);
	}

	fn remove_account(&mut self, hash: &H160) -> Option<Self::Account> {
		self.accounts.remove(hash)
	}
}

impl Wallet {
	/// The default wallet name.
	pub const DEFAULT_WALLET_NAME: &'static str = "NeoWallet";
	/// The current wallet version.
	pub const CURRENT_VERSION: &'static str = "1.0";

	/// Creates a new wallet instance with a default account.
	pub fn new() -> Self {
		let account = match Account::create() {
			Ok(mut acc) => {
				acc.is_default = true;
				acc
			},
			Err(e) => {
				eprintln!("Failed to create account: {}", e);
				return Self::default();
			},
		};

		let mut accounts = HashMap::new();
		accounts.insert(account.address_or_scripthash.script_hash(), account.clone());
		Self {
			name: "NeoWallet".to_string(),
			version: "1.0".to_string(),
			scrypt_params: ScryptParamsDef::default(),
			accounts,
			default_account: account.clone().address_or_scripthash.script_hash(),
			extra: None,
		}
	}

	/// Creates a new wallet instance without any accounts.
	pub fn default() -> Self {
		Self {
			name: "NeoWallet".to_string(),
			version: "1.0".to_string(),
			scrypt_params: ScryptParamsDef::default(),
			accounts: HashMap::new(),
			default_account: H160::default(),
			extra: None,
		}
	}

	/// Converts the wallet to a NEP6Wallet format.
	pub fn to_nep6(&self) -> Result<Nep6Wallet, WalletError> {
		let accounts = self
			.accounts
			.clone()
			.into_iter()
			.filter_map(|(_, account)| match NEP6Account::from_account(&account) {
				Ok(nep6_account) => Some(nep6_account),
				Err(e) => {
					eprintln!("Failed to convert account to NEP6Account: {}", e);
					None
				},
			})
			.collect::<Vec<NEP6Account>>();

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
			.into_iter()
			.filter_map(|v| v.to_account().ok())
			.collect::<Vec<_>>();

		// Find default account or use first account
		let default_account_address =
			if let Some(account) = nep6.accounts().iter().find(|a| a.is_default) {
				account.address().clone()
			} else if let Some(account) = nep6.accounts().first() {
				eprintln!("No default account found, using first account");
				account.address().clone()
			} else {
				eprintln!("No accounts found, using empty address");
				String::new()
			};

		Ok(Self {
			name: nep6.name().clone(),
			version: nep6.version().clone(),
			scrypt_params: nep6.scrypt().clone(),
			accounts: accounts.into_iter().map(|a| (a.get_script_hash().clone(), a)).collect(),
			default_account: default_account_address.address_to_script_hash().map_err(|e| {
				WalletError::AccountState(format!(
					"Failed to convert address to script hash: {}",
					e
				))
			})?,
			extra: nep6.extra.clone(),
		})
	}

	// pub async fn get_nep17_balances(&self) -> Result<HashMap<H160, u32>, WalletError> {
	// 	let balances = HTTP_PROVIDER
	// 		.get_nep17_balances(self.get_script_hash().clone())
	// 		.await
	// 		.map_err(|e| WalletError::RpcError(format!("Failed to get NEP17 balances: {}", e)))?;
	// 	let mut nep17_balances = HashMap::new();
	// 	for balance in balances.balances {
	// 		nep17_balances.insert(balance.asset_hash, u32::from_str(&balance.amount).unwrap());
	// 	}
	// 	Ok(nep17_balances)
	// }

	pub fn from_account(account: &Account) -> Result<Wallet, WalletError> {
		let mut wallet: Wallet = Wallet::new();
		wallet.add_account(account.clone());
		wallet.set_default_account(account.get_script_hash());
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
	/// use NeoRust::prelude::{Account, Wallet};
	/// let account1 = Account::default();
	/// let account2 = Account::default();
	///
	/// let mut wallet = Wallet::from_accounts(vec![account1, account2]).unwrap();
	/// ```
	pub fn from_accounts(accounts: Vec<Account>) -> Result<Wallet, WalletError> {
		// for account in &accounts {
		// 	if account.wallet().is_some() {
		// 		return Err(WalletError::AccountState(format!(
		// 			"The account {} is already contained in a wallet. Please remove this account from its containing wallet before adding it to another wallet.",
		// 			account.address_or_scripthash.address()
		// 		)));
		// 	}
		// }

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

		// Write to file at path
		let mut file = File::create(path)
			.map_err(|e| WalletError::FileError(format!("Failed to create wallet file: {e}")))?;
		file.write_all(json.as_bytes())
			.map_err(|e| WalletError::FileError(format!("Failed to write wallet file: {e}")))?;

		Ok(())
	}

	pub fn get_account(&self, script_hash: &H160) -> Option<&Account> {
		self.accounts.get(script_hash)
	}

	pub fn remove_account(&mut self, script_hash: &H160) -> bool {
		self.accounts.remove(script_hash).is_some()
	}

	pub fn encrypt_accounts(&mut self, password: &str) {
		for account in self.accounts.values_mut() {
			// Only encrypt accounts that have a key pair
			if account.key_pair().is_some() {
				if let Err(e) = account.encrypt_private_key(password) {
					eprintln!(
						"Warning: Failed to encrypt private key for account {}: {}",
						account.get_address(),
						e
					);
				}
			}
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
	/// # let mut wallet = Wallet::new();
	/// // For wallets with many accounts, use parallel encryption
	/// wallet.encrypt_accounts_parallel("strong_password");
	/// ```
	pub fn encrypt_accounts_parallel(&mut self, password: &str) {
		// Collect errors in a thread-safe manner
		let errors: Vec<(String, String)> = self.accounts
			.par_iter_mut()
			.filter_map(|(_, account)| {
				// Only encrypt accounts that have a key pair
				if account.key_pair().is_some() {
					match account.encrypt_private_key(password) {
						Err(e) => Some((account.get_address(), e.to_string())),
						Ok(_) => None,
					}
				} else {
					None
				}
			})
			.collect();

		// Log any errors that occurred
		for (address, error) in errors {
			eprintln!(
				"Warning: Failed to encrypt private key for account {}: {}",
				address,
				error
			);
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
	/// # let mut wallet = Wallet::new();
	/// // Use 4 threads for encryption
	/// wallet.encrypt_accounts_parallel_with_threads("strong_password", 4);
	/// ```
	pub fn encrypt_accounts_parallel_with_threads(&mut self, password: &str, num_threads: usize) {
		// Create a custom thread pool with the specified number of threads
		let pool = rayon::ThreadPoolBuilder::new()
			.num_threads(num_threads)
			.build()
			.unwrap();

		pool.install(|| {
			self.encrypt_accounts_parallel(password);
		});
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
	/// # let mut wallet = Wallet::new();
	/// // Process accounts in batches of 50
	/// wallet.encrypt_accounts_batch_parallel("strong_password", 50);
	/// ```
	pub fn encrypt_accounts_batch_parallel(&mut self, password: &str, batch_size: usize) {
		use std::sync::{Arc, Mutex};
		
		// Collect accounts that need encryption along with their script hashes
		let accounts_to_encrypt: Vec<(H160, Account)> = self.accounts
			.iter()
			.filter(|(_, account)| account.key_pair().is_some())
			.map(|(hash, account)| (*hash, account.clone()))
			.collect();

		// Process in parallel batches and collect results
		let results: Arc<Mutex<Vec<(H160, Result<Account, String>)>>> = Arc::new(Mutex::new(Vec::new()));
		
		accounts_to_encrypt
			.par_chunks(batch_size)
			.for_each(|batch| {
				let batch_results: Vec<(H160, Result<Account, String>)> = batch
					.iter()
					.map(|(hash, account)| {
						let mut account_clone = account.clone();
						match account_clone.encrypt_private_key(password) {
							Ok(_) => (*hash, Ok(account_clone)),
							Err(e) => (*hash, Err(format!("{}: {}", account.get_address(), e))),
						}
					})
					.collect();
				
				results.lock().unwrap().extend(batch_results);
			});

		// Apply successful encryptions and collect errors
		let results = Arc::try_unwrap(results).unwrap().into_inner().unwrap();
		for (hash, result) in results {
			match result {
				Ok(encrypted_account) => {
					self.accounts.insert(hash, encrypted_account);
				}
				Err(error_msg) => {
					eprintln!("Warning: Failed to encrypt private key for account {}", error_msg);
				}
			}
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
	pub fn create(path: &PathBuf, password: &str) -> Result<Self, WalletError> {
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
	pub fn open(path: &PathBuf, password: &str) -> Result<Self, WalletError> {
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
		Ok(self.get_account(&account.get_script_hash()).unwrap())
	}

	/// Imports a private key in WIF format
	pub fn import_private_key(&mut self, wif: &str) -> Result<&Account, WalletError> {
		let key_pair = KeyPair::from_wif(wif)
			.map_err(|e| WalletError::AccountState(format!("Failed to import private key: {e}")))?;

		let account = Account::from_key_pair(key_pair, None, None)
			.map_err(|e| WalletError::ProviderError(e))?;
		self.add_account(account.clone());
		Ok(self.get_account(&account.get_script_hash()).unwrap())
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
			match account_clone.decrypt_private_key(password) {
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
		if !self.verify_password(current_password) {
			return Err(WalletError::AccountState("Invalid password".to_string()));
		}

		// First decrypt all accounts with the current password
		for account in self.accounts.values_mut() {
			if account.encrypted_private_key().is_some() && account.key_pair().is_none() {
				if let Err(e) = account.decrypt_private_key(current_password) {
					return Err(WalletError::DecryptionError(format!(
						"Failed to decrypt account {}: {}",
						account.get_address(),
						e
					)));
				}
			}
		}

		// Re-encrypt all accounts with the new password
		self.encrypt_accounts(new_password);

		Ok(())
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
	/// # let mut wallet = Wallet::new();
	/// wallet.change_password_parallel("old_password", "new_password").unwrap();
	/// ```
	pub fn change_password_parallel(
		&mut self,
		current_password: &str,
		new_password: &str,
	) -> Result<(), WalletError> {
		if !self.verify_password(current_password) {
			return Err(WalletError::AccountState("Invalid password".to_string()));
		}

		// Collect accounts that need decryption
		let accounts_to_decrypt: Vec<(H160, Account)> = self.accounts
			.iter()
			.filter(|(_, account)| account.encrypted_private_key().is_some() && account.key_pair().is_none())
			.map(|(hash, account)| (*hash, account.clone()))
			.collect();

		// Decrypt accounts in parallel
		let decrypted_results: Vec<(H160, Result<Account, String>)> = accounts_to_decrypt
			.into_par_iter()
			.map(|(hash, account)| {
				let mut account_clone = account.clone();
				match account_clone.decrypt_private_key(current_password) {
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
				self.accounts.insert(hash, decrypted_account);
			}
		}

		// Re-encrypt all accounts with the new password using parallel processing
		self.encrypt_accounts_parallel(new_password);

		Ok(())
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
	/// allowing for flexibility in the type of data that can be signed.
	///
	/// # Returns
	///
	/// A `Result` that, on success, contains the `Secp256r1Signature` of the message. On failure,
	/// it returns a `WalletError`, which could indicate issues like a missing key pair.
	///
	/// # Example
	///
	/// ```no_run
	/// # use NeoRust::prelude::Wallet;
	///  async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// # let wallet = Wallet::new();
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
			.clone()
			.key_pair()
			.clone()
			.ok_or_else(|| WalletError::NoKeyPair)?
			.private_key()
			.sign_tx(message_hash)
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
	/// # use NeoRust::prelude::{Transaction, Wallet};
	///  async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// # let wallet = Wallet::new();
	/// # let tx = Transaction::new();
	/// let witness = wallet.get_witness(&tx).await?;
	/// println!("Witness: {:?}", witness);
	/// # Ok(())
	/// # }
	/// ```
	pub async fn get_witness<'a, P: JsonRpcProvider + 'static>(
		&self,
		tx: &Transaction<'a, P>,
	) -> Result<Witness, WalletError> {
		let mut tx_with_chain = tx.clone();
		if tx_with_chain.network().is_none() {
			// in the case we don't have a network, let's use the signer network magic instead
			// tx_with_chain.set_network(Some(self.network()));
		}

		Witness::create(
			tx.get_hash_data().await?,
			&self.default_account().key_pair.clone().ok_or_else(|| WalletError::NoKeyPair)?,
		)
		.map_err(|_e| WalletError::NoKeyPair)
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
	fn address(&self) -> String {
		// Get the default account's address
		if let Some(account) = self.get_account(&self.default_account) {
			account.address_or_scripthash.address().clone()
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
	pub fn create_wallet(path: &PathBuf, password: &str) -> Result<Self, WalletError> {
		let mut wallet = Wallet::new();

		// Create a new account for the wallet
		let account = Account::create().map_err(|e| WalletError::ProviderError(e))?;
		wallet.add_account(account);

		// Encrypt the wallet with the provided password
		wallet.encrypt_accounts(password);

		// Save the wallet to the specified path
		wallet.save_to_file(path.clone())?;

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
	pub fn open_wallet(path: &PathBuf, password: &str) -> Result<Self, WalletError> {
		// Read the wallet file
		let wallet_json = std::fs::read_to_string(path)
			.map_err(|e| WalletError::FileError(format!("Failed to read wallet file: {e}")))?;

		// Parse the wallet JSON
		let nep6_wallet: Nep6Wallet = serde_json::from_str(&wallet_json).map_err(|e| {
			WalletError::DeserializationError(format!("Failed to parse wallet JSON: {e}"))
		})?;

		// Convert to Wallet
		let mut wallet = Wallet::from_nep6(nep6_wallet)?;

		// Verify the password by checking if we can decrypt any account
		let can_decrypt = wallet.verify_password(password);

		if !can_decrypt {
			return Err(WalletError::CryptoError(CryptoError::InvalidPassphrase(
				"Invalid password".to_string(),
			)));
		}

		Ok(wallet)
	}

	/// Gets all accounts in the wallet.
	///
	/// # Returns
	///
	/// A vector of references to all accounts in the wallet
	pub fn get_all_accounts(&self) -> Vec<&Account> {
		self.accounts.values().collect()
	}

	/// Creates a new account in the wallet.
	///
	/// # Returns
	///
	/// A `Result` containing the new account or a `WalletError`
	pub fn create_new_account(&mut self) -> Result<&Account, WalletError> {
		let account = Account::create().map_err(|e| WalletError::ProviderError(e))?;
		let script_hash = account.address_or_scripthash.script_hash();
		self.add_account(account);

		Ok(self.get_account(&script_hash).unwrap())
	}

	/// Imports a private key into the wallet.
	///
	/// # Arguments
	///
	/// * `private_key` - The private key to import
	///
	/// # Returns
	///
	/// A `Result` containing the imported account or a `WalletError`
	pub fn import_from_wif(&mut self, private_key: &str) -> Result<&Account, WalletError> {
		// Create a key pair from the private key
		let key_pair = KeyPair::from_wif(private_key).map_err(|e| WalletError::CryptoError(e))?;

		// Create an account from the key pair
		let account = Account::from_key_pair(key_pair, None, None)
			.map_err(|e| WalletError::AccountState(format!("Failed to create account: {e}")))?;
		let script_hash = account.address_or_scripthash.script_hash();

		// Add the account to the wallet
		self.add_account(account);

		Ok(self.get_account(&script_hash).unwrap())
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
				.map_err(|e| WalletError::ProviderError(e))?;

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
	/// # use NeoRust::prelude::{NeoConfig, NeoNetwork, Wallet};
	/// let mut wallet = Wallet::new();
	/// wallet = wallet.with_network(NeoNetwork::MainNet.to_magic());
	/// ```
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
		neo_config::TestConstants,
		neo_protocol::{Account, AccountTrait},
		neo_wallets::{Wallet, WalletTrait},
	};

	#[test]
	fn test_is_default() {
		let account = Account::from_address(TestConstants::DEFAULT_ACCOUNT_ADDRESS)
			.expect("Should be able to create account from valid address in test");
		let mut wallet: Wallet = Wallet::new();
		wallet.add_account(account.clone());

		assert!(!account.is_default);

		let hash = account.address_or_scripthash.script_hash();
		wallet.set_default_account(hash.clone());
		assert!(wallet.get_account(&hash).expect("Account should exist in wallet").is_default);
	}

	// #[test]
	// fn test_wallet_link() {
	// 	let account = Account::from_address(TestConstants::DEFAULT_ACCOUNT_ADDRESS)
	// 		.expect("Should be able to create account from valid address in test");
	// 	let wallet = Wallet::create().unwrap();
	//
	// 	assert!(account.wallet.is_none());
	//
	// 	wallet.add_account(account).unwrap();
	// 	assert_eq!(account.wallet.as_ref().unwrap().as_ptr(), wallet.as_ptr());
	// }

	#[test]
	fn test_create_default_wallet() {
		let wallet: Wallet = Wallet::default();

		assert_eq!(&wallet.name, "NeoWallet");
		assert_eq!(&wallet.version, Wallet::CURRENT_VERSION);
		assert_eq!(wallet.accounts.len(), 0usize);
	}

	#[test]
	fn test_create_wallet_with_accounts() {
		let account1 = Account::create().expect("Should be able to create account in test");
		let account2 = Account::create().expect("Should be able to create account in test");

		let wallet = Wallet::from_accounts(vec![account1.clone(), account2.clone()])
			.expect("Should be able to create wallet from accounts in test");

		assert_eq!(wallet.default_account(), &account1);
		assert_eq!(wallet.accounts.len(), 2);
		assert!(wallet
			.accounts
			.clone()
			.into_iter()
			.any(|(s, _)| s == account1.address_or_scripthash.script_hash()));
		assert!(wallet
			.accounts
			.clone()
			.into_iter()
			.any(|(s, _)| s == account2.address_or_scripthash.script_hash()));
	}

	#[test]
	fn test_is_default_account() {
		let account = Account::create().expect("Should be able to create account in test");
		let mut wallet = Wallet::from_accounts(vec![account.clone()])
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
		wallet.add_account(Account::create().expect("Should be able to create account in test"));

		assert!(wallet.accounts()[0].key_pair().is_some());
		assert!(wallet.accounts()[1].key_pair().is_some());

		wallet.encrypt_accounts("pw");

		assert!(wallet.accounts()[0].key_pair().is_none());
		assert!(wallet.accounts()[1].key_pair().is_none());
	}

	#[test]
	fn test_encrypt_wallet_parallel() {
		let mut wallet: Wallet = Wallet::new();
		// Add multiple accounts to test parallel processing
		for _ in 0..5 {
			wallet.add_account(Account::create().expect("Should be able to create account in test"));
		}

		// Verify all accounts have key pairs
		for account in wallet.accounts() {
			assert!(account.key_pair().is_some());
		}

		// Encrypt using parallel method
		wallet.encrypt_accounts_parallel("parallel_password");

		// Verify all accounts are now encrypted
		for account in wallet.accounts() {
			assert!(account.key_pair().is_none());
			assert!(account.encrypted_private_key().is_some());
		}
	}

	#[test]
	fn test_encrypt_wallet_batch_parallel() {
		let mut wallet: Wallet = Wallet::new();
		// Add many accounts to test batch processing
		for _ in 0..10 {
			wallet.add_account(Account::create().expect("Should be able to create account in test"));
		}

		// Verify all accounts have key pairs
		for account in wallet.accounts() {
			assert!(account.key_pair().is_some());
		}

		// Encrypt using batch parallel method with batch size of 3
		wallet.encrypt_accounts_batch_parallel("batch_password", 3);

		// Verify all accounts are now encrypted
		for account in wallet.accounts() {
			assert!(account.key_pair().is_none());
			assert!(account.encrypted_private_key().is_some());
		}
	}

	#[test]
	fn test_change_password_parallel() {
		let mut wallet = Wallet::new();
		// Add multiple accounts
		for _ in 0..5 {
			wallet.add_account(Account::create().expect("Should be able to create account in test"));
		}

		let old_password = "old_password";
		let new_password = "new_password";

		// Initially encrypt the wallet
		wallet.encrypt_accounts(old_password);

		// Verify initial encryption
		assert!(wallet.verify_password(old_password));
		assert!(!wallet.verify_password(new_password));

		// Change password using parallel method
		wallet.change_password_parallel(old_password, new_password)
			.expect("Password change should succeed");

		// Verify new password works
		assert!(!wallet.verify_password(old_password));
		assert!(wallet.verify_password(new_password));
	}

	#[test]
	fn test_verify_password() {
		let mut wallet = Wallet::new();
		let account = Account::create().unwrap();
		wallet.add_account(account.clone());

		// Initially, the account is not encrypted so verification should fail
		assert!(!wallet.verify_password("password123"));

		// Encrypt the account
		wallet.encrypt_accounts("password123");

		// Now verification should succeed with the correct password
		assert!(wallet.verify_password("password123"));

		// And fail with an incorrect password
		assert!(!wallet.verify_password("wrong_password"));
	}
}
