use crate::{ScryptParamsDef, neo_protocol::AccountTrait};
use primitive_types::H160;

/// Represents the core functionalities of a cryptocurrency wallet.
///
/// This trait defines the common operations that a cryptocurrency wallet should support,
/// including access to account details, wallet metadata (like name and version), and
/// scrypt parameters for key derivation. It also provides methods for account management,
/// such as adding, removing, and setting a default account.
///
/// # Type Parameters
///
/// - `Account`: The specific type of account managed by the wallet, constrained by
///   the `AccountTrait` to ensure it adheres to the expected interface for accounts.
///
/// # Required Methods
///
/// - `name`, `version`: Accessors for the wallet's metadata.
/// - `scrypt_params`: Access to the wallet's key derivation parameters.
/// - `accounts`: Lists all accounts stored within the wallet.
/// - `default_account`: Retrieves the wallet's default account.
/// - `set_name`, `set_version`, `set_scrypt_params`, `set_default_account`: Mutators for
///   the wallet's properties.
/// - `add_account`, `remove_account`: Methods for account management within the wallet.
///
/// # Example
///
/// Implementing the `WalletTrait` for a simple wallet:
///
/// ```rust,no_run
/// use neo3::neo_wallets::WalletTrait;
/// use neo3::neo_protocol::{Account, AccountTrait};
/// use neo3::ScryptParamsDef;
/// use primitive_types::H160;
///
/// // WalletTrait defines the interface for wallet implementations.
/// // See the Wallet struct for a concrete implementation.
/// fn example(wallet: &impl WalletTrait<Account = Account>) {
///     println!("Wallet: {}", wallet.name());
///     println!("Version: {}", wallet.version());
/// }
/// ```
///
/// This trait allows for the abstraction over different wallet implementations,
/// facilitating the use of different key storage mechanisms, account management strategies,
/// and cryptographic algorithms.
pub trait WalletTrait {
	/// The type of account managed by the wallet.
	type Account: AccountTrait;

	/// Returns the name of the wallet.
	fn name(&self) -> &String;

	/// Returns the version of the wallet.
	fn version(&self) -> &String;

	/// Returns the scrypt parameters used for key derivation.
	fn scrypt_params(&self) -> &ScryptParamsDef;

	/// Returns a list of accounts stored in the wallet.
	fn accounts(&self) -> Vec<Self::Account>;

	/// Returns a reference to the default account of the wallet.
	///
	/// # Returns
	///
	/// `Some(&Account)` if a default account is set and exists, `None` otherwise.
	///
	/// # Note
	///
	/// This method returns `Option` to avoid panics when the default account
	/// is not set or has been removed. Use `default_account_or_err()` if you
	/// need a `Result` with a descriptive error.
	fn default_account(&self) -> Option<&Self::Account>;

	/// Returns a reference to the default account, or an error if not available.
	///
	/// This is a convenience method that converts the `Option` from `default_account()`
	/// into a `Result` with a descriptive error message.
	///
	/// # Errors
	///
	/// Returns `WalletError::NoDefaultAccount` if no default account is set.
	fn default_account_or_err(&self) -> Result<&Self::Account, crate::neo_wallets::WalletError> {
		self.default_account().ok_or(crate::neo_wallets::WalletError::NoDefaultAccount)
	}

	/// Sets the name of the wallet.
	fn set_name(&mut self, name: String);

	/// Sets the version of the wallet.
	fn set_version(&mut self, version: String);

	/// Sets the scrypt parameters for the wallet.
	fn set_scrypt_params(&mut self, params: ScryptParamsDef);

	/// Sets the default account of the wallet.
	fn set_default_account(&mut self, default_account: H160);

	/// Adds a new account to the wallet.
	fn add_account(&mut self, account: Self::Account);

	/// Removes an account from the wallet by its hash.
	///
	/// Returns the removed account if it existed, or `None` otherwise.
	fn remove_account(&mut self, hash: &H160) -> Option<Self::Account>;
}
