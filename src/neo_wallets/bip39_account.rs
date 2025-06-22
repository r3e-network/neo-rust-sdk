use crate::{
	crypto::KeyPair,
	neo_protocol::{Account, AccountTrait},
};
use bip39::{Language, Mnemonic};
use sha2::{Digest, Sha256};

/// A BIP-39 compatible neo account that uses mnemonic phrases for key generation and recovery.
///
/// This implementation follows the BIP-39 standard for generating and recovering neo accounts using
/// mnemonic phrases. The account can be created with a new random mnemonic or recovered from an
/// existing mnemonic phrase.
///
/// # Examples
///
/// ## Creating a new account
/// ```
/// use neo3::prelude::Bip39Account;
///
/// // Create a new account with a password
/// let password = "your_secure_password";
/// let account = Bip39Account::create(password).unwrap();
///
/// // The account will have a randomly generated 24-word mnemonic
/// println!("Mnemonic: {}", account.mnemonic);
/// ```
///
/// ## Recovering an existing account
/// ```
/// use neo3::prelude::Bip39Account;
///
/// // Recover an account using an existing mnemonic and password
/// let mnemonic = "word1 word2 ... word24"; // Your 24 word mnemonic
/// let password = "your_secure_password";
/// let recovered = Bip39Account::from_bip39_mnemonic(password, mnemonic).unwrap();
/// ```
#[derive(Debug)]
pub struct Bip39Account {
	/// The underlying neo account
	account: Account,

	/// Generated BIP-39 mnemonic for the account
	mnemonic: String,
}

impl Bip39Account {
	/// Creates a new BIP-39 compatible neo account with a randomly generated mnemonic.
	///
	/// The private key for the wallet is calculated using:
	/// `Key = SHA-256(BIP_39_SEED(mnemonic, password))`
	///
	/// The password is used as a BIP-39 passphrase and is required to recover the account later.
	/// The same password must be provided during recovery to generate the same keys.
	///
	/// # Arguments
	/// * `password` - The passphrase used in BIP-39 seed generation. This must be saved to recover the account.
	///
	/// # Returns
	/// A Result containing the new Bip39Account or an error if creation fails.
	///
	/// # Example
	/// ```
	/// use neo3::prelude::Bip39Account;
	///
	/// let account = Bip39Account::create("my secure password").unwrap();
	/// // Save the mnemonic securely
	/// let mnemonic = account.mnemonic.clone();
	/// ```
	pub fn create(password: &str) -> Result<Self, Box<dyn std::error::Error>> {
		let mut rng = bip39::rand::thread_rng();
		let mnemonic =
			Mnemonic::generate_in_with(&mut rng, Language::English, 24).map_err(|e| {
				Box::<dyn std::error::Error>::from(format!("Failed to generate mnemonic: {e}"))
			})?;
		let seed = mnemonic.to_seed(password);

		let mut hasher = Sha256::new();
		hasher.update(&seed);
		let private_key = hasher.finalize();

		let key_pair = KeyPair::from_private_key(private_key.as_ref()).map_err(|e| {
			Box::<dyn std::error::Error>::from(format!("Failed to create key pair: {e}"))
		})?;
		let account = Account::from_key_pair(key_pair.clone(), None, None).map_err(|e| {
			Box::<dyn std::error::Error>::from(format!(
				"Failed to create account from key pair: {}",
				e
			))
		})?;

		Ok(Self { account, mnemonic: mnemonic.to_string() })
	}

	/// Recovers a neo account from an existing BIP-39 mnemonic phrase and password.
	///
	/// This method will reconstruct the exact same neo account if provided with the same
	/// mnemonic and password combination that was used to create the original account.
	///
	/// # Arguments
	/// * `password` - The same passphrase that was used when generating the original account
	/// * `mnemonic` - The 24-word mnemonic phrase from the original account
	///
	/// # Returns
	/// A Result containing the recovered Bip39Account or an error if recovery fails
	///
	/// # Example
	/// ```
	/// use neo3::prelude::Bip39Account;
	///
	/// let mnemonic = "word1 word2 ... word24"; // Your saved 24-word mnemonic
	/// let password = "your_secure_password";      // Original password used
	/// let account = Bip39Account::from_bip39_mnemonic(password, mnemonic).unwrap();
	/// ```
	pub fn from_bip39_mnemonic(
		password: &str,
		mnemonic: &str,
	) -> Result<Self, Box<dyn std::error::Error>> {
		let mnemonic = Mnemonic::parse_in(Language::English, mnemonic)?;
		let seed = mnemonic.to_seed(password);

		let mut hasher = Sha256::new();
		hasher.update(&seed);
		let private_key = hasher.finalize();

		let key_pair = KeyPair::from_private_key(private_key.as_ref()).map_err(|e| {
			Box::<dyn std::error::Error>::from(format!("Failed to create key pair: {e}"))
		})?;
		let account = Account::from_key_pair(key_pair.clone(), None, None).map_err(|e| {
			Box::<dyn std::error::Error>::from(format!(
				"Failed to create account from key pair: {}",
				e
			))
		})?;

		Ok(Self { account, mnemonic: mnemonic.to_string() })
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_create_bip39_account() {
		let password = std::env::var("TEST_PASSWORD").unwrap_or_else(|_| "test_password".to_string());
		let account =
			Bip39Account::create(password).expect("Should be able to create Bip39Account in test");

		// Check that mnemonic is 24 words
		assert_eq!(account.mnemonic.split_whitespace().count(), 24);

		// Verify account was created with valid key pair
		assert!(account.account.key_pair().is_some());
	}

	#[test]
	fn test_recover_from_mnemonic() {
		let password = std::env::var("TEST_PASSWORD").unwrap_or_else(|_| "test_password".to_string());
		let original =
			Bip39Account::create(password).expect("Should be able to create Bip39Account in test");
		let mnemonic = original.mnemonic.clone();

		// Recover account using mnemonic
		let recovered = Bip39Account::from_bip39_mnemonic(password, &mnemonic)
			.expect("Should be able to recover Bip39Account from mnemonic in test");

		// Verify recovered account matches original
		assert_eq!(original.account.get_script_hash(), recovered.account.get_script_hash());
		assert_eq!(original.mnemonic, recovered.mnemonic);
	}

	#[test]
	fn test_invalid_mnemonic() {
		let result = Bip39Account::from_bip39_mnemonic("password", "invalid mnemonic phrase");
		assert!(result.is_err());
	}

	#[test]
	fn test_different_passwords_different_accounts() {
		let account1 = Bip39Account::create("password1")
			.expect("Should be able to create Bip39Account in test");
		let account2 = Bip39Account::create("password2")
			.expect("Should be able to create Bip39Account in test");

		assert_ne!(account1.account.get_script_hash(), account2.account.get_script_hash());
	}

	#[test]
	fn test_generate_and_recover_bip39_account() {
		let password = std::env::var("TEST_PASSWORD").unwrap_or_else(|_| "test_password".to_string());
		let account1 =
			Bip39Account::create(password).expect("Should be able to create Bip39Account in test");
		let account2 = Bip39Account::from_bip39_mnemonic(password, &account1.mnemonic)
			.expect("Should be able to recover Bip39Account from mnemonic in test");

		assert_eq!(account1.account.get_address(), account2.account.get_address());
		assert!(account1.account.key_pair().is_some());
		assert_eq!(account1.account.key_pair(), account2.account.key_pair());
		assert_eq!(account1.mnemonic, account2.mnemonic);
		assert!(!account1.mnemonic.is_empty());
	}
}
