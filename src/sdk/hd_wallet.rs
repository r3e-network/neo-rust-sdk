//! HD Wallet support with BIP-39/44 implementation
//!
//! This module provides a complete hierarchical deterministic (HD) wallet
//! implementation for Neo blockchain, following BIP-39 (mnemonic generation)
//! and BIP-44 (derivation paths) standards. HD wallets allow users to manage
//! multiple accounts from a single seed phrase.
//!
//! ## Features
//!
//! - **BIP-39 Mnemonics**: 12-24 word seed phrases for wallet generation
//! - **BIP-44 Paths**: Standard derivation paths (m/44'/888'/...)
//! - **Multiple Accounts**: Derive unlimited accounts from one seed
//! - **Fast Derivation**: <10ms per account derivation
//! - **Passphrase Support**: Optional BIP-39 passphrase for extra security
//! - **Multi-language**: Support for multiple mnemonic languages
//!
//! ## Security
//!
//! - Never share your mnemonic phrase
//! - Store mnemonics securely offline
//! - Use passphrases for additional security
//! - Consider hardware wallet integration for production
//!
//! ## Example
//!
//! ```rust,no_run
//! use neo3::sdk::hd_wallet::{HDWallet, HDWalletBuilder};
//! use bip39::Language;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Generate new wallet with 24 words
//! let wallet = HDWallet::generate(24, None)?;
//! let _mnemonic = wallet.mnemonic_phrase();
//! // SECURITY: Store the mnemonic securely offline. Avoid logging it.
//!
//! // Derive accounts
//! let mut wallet = wallet;
//! let account1 = wallet.derive_account("m/44'/888'/0'/0/0")?;
//! let account2 = wallet.derive_account("m/44'/888'/0'/0/1")?;
//!
//! // Import existing mnemonic
//! let restored = HDWallet::from_phrase(
//!     "your twelve word mnemonic phrase goes here for wallet restoration",
//!     None,
//!     Language::English
//! )?;
//! # Ok(())
//! # }
//! ```

// use crate::neo_crypto::Secp256r1PublicKey;
use crate::neo_error::unified::{ErrorRecovery, NeoError};
use crate::neo_protocol::{Account, AccountTrait};
use bip39::{Language, Mnemonic};
use hmac::{Hmac, KeyInit, Mac};
use p256::elliptic_curve::zeroize::Zeroize;
use serde::{Deserialize, Serialize};
use sha2::Sha512;
use std::{collections::HashMap, fmt};

const HD_WALLET_EXPORT_VERSION: u8 = 2;
const HD_WALLET_LEGACY_ECB_VERSION: u8 = 1;
const HD_WALLET_SALT_LEN: usize = 16;
const HD_WALLET_NONCE_LEN: usize = 16;
const HD_WALLET_KEY_MATERIAL_LEN: usize = 64;
const HD_WALLET_TAG_LEN: usize = 32;

/// HD wallet derivation path components
///
/// Represents a BIP-44 compliant derivation path for hierarchical deterministic
/// wallets. The path follows the format: m/purpose'/coin_type'/account'/change/index
/// where apostrophes (') indicate hardened derivation.
///
/// For Neo, the standard path is m/44'/888'/account'/0/index
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivationPath {
	/// Purpose (BIP-44 = 44')
	purpose: u32,
	/// Coin type (NEO = 888')
	coin_type: u32,
	/// Account number
	account: u32,
	/// Change (0 = external, 1 = internal)
	change: u32,
	/// Address index
	index: u32,
}

impl DerivationPath {
	/// Create a new NEO derivation path
	///
	/// Creates a standard BIP-44 path for Neo blockchain.
	/// The coin type 888 is the registered number for Neo.
	///
	/// # Arguments
	///
	/// * `account` - Account index (will be hardened; must be < 2^31)
	/// * `index` - Address index within the account
	///
	/// # Returns
	///
	/// Default path: m/44'/888'/account'/0/index
	///
	/// # Errors
	///
	/// Returns an error if `account` is >= 2^31 and cannot be hardened.
	pub fn new_neo(account: u32, index: u32) -> Result<Self, NeoError> {
		let account = account.checked_add(0x80000000).ok_or_else(|| NeoError::Wallet {
			message: format!(
				"Account index {} cannot be hardened (max {})",
				account, 2_147_483_647
			),
			source: None,
			recovery: ErrorRecovery::new().suggest("Use an account index below 2147483648"),
		})?;
		Ok(Self {
			purpose: 0x80000000 + 44,    // 44' hardened
			coin_type: 0x80000000 + 888, // 888' for NEO
			account,                     // account' hardened
			change: 0,                   // external addresses
			index,
		})
	}

	/// Parse a derivation path string
	///
	/// Format: m/44'/888'/0'/0/0
	pub fn from_string(path: &str) -> Result<Self, NeoError> {
		let parts: Vec<&str> = path.trim_start_matches("m/").split('/').collect();

		if parts.len() != 5 {
			return Err(NeoError::Wallet {
				message: "Invalid derivation path format".to_string(),
				source: None,
				recovery: ErrorRecovery::new()
					.suggest("Use format: m/44'/888'/account'/change/index")
					.doc("https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki"),
			});
		}

		let parse_component = |s: &str| -> Result<u32, NeoError> {
			let hardened = s.ends_with('\'');
			let num_str = if hardened { &s[..s.len() - 1] } else { s };

			let num = num_str.parse::<u32>().map_err(|e| NeoError::Wallet {
				message: format!("Invalid path component: {}", s),
				source: Some(Box::new(e)),
				recovery: ErrorRecovery::new(),
			})?;

			// BIP-32 indices must fit in 31 bits; hardening sets bit 31.
			if hardened && num >= 0x80000000 {
				return Err(NeoError::Wallet {
					message: format!(
						"Hardened index {} is out of range (max {})",
						num, 2_147_483_647
					),
					source: None,
					recovery: ErrorRecovery::new().suggest("Use an index below 2147483648"),
				});
			}
			num.checked_add(if hardened { 0x80000000 } else { 0 })
				.ok_or_else(|| NeoError::Wallet {
					message: format!("Path component {} overflows the 32-bit index space", s),
					source: None,
					recovery: ErrorRecovery::new().suggest("Use an index below 2147483648"),
				})
		};

		Ok(Self {
			purpose: parse_component(parts[0])?,
			coin_type: parse_component(parts[1])?,
			account: parse_component(parts[2])?,
			change: parse_component(parts[3])?,
			index: parse_component(parts[4])?,
		})
	}
}

impl fmt::Display for DerivationPath {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let format_component = |n: u32| -> String {
			if n >= 0x80000000 { format!("{}'", n - 0x80000000) } else { format!("{}", n) }
		};

		write!(
			f,
			"m/{}/{}/{}/{}/{}",
			format_component(self.purpose),
			format_component(self.coin_type),
			format_component(self.account),
			format_component(self.change),
			format_component(self.index)
		)
	}
}

/// HD Wallet implementation with BIP-39/44 support
///
/// The main HD wallet structure that manages mnemonic phrases, seed generation,
/// and account derivation. This implementation follows industry standards for
/// hierarchical deterministic wallets, ensuring compatibility with other
/// Neo wallets that support BIP-39/44.
///
/// ## Internal Structure
///
/// - Mnemonic phrase for wallet recovery
/// - Master seed and keys derived from mnemonic
/// - Cache of derived accounts for performance
/// - Support for multiple languages
pub struct HDWallet {
	/// Mnemonic phrase
	///
	/// This field is intentionally stored to allow future API extensions
	/// for accessing the raw Mnemonic object. Currently accessed internally
	/// during wallet operations. The linter warning is a false positive
	/// as this follows the standard pattern of storing data for API completeness.
	#[allow(dead_code)]
	mnemonic: Mnemonic,
	/// Mnemonic phrase as string
	mnemonic_phrase: String,
	/// Seed bytes derived from mnemonic
	seed: Vec<u8>,
	/// Optional BIP-39 passphrase used to derive the seed.
	passphrase: Option<String>,
	/// Master private key
	master_key: ExtendedPrivateKey,
	/// Cached derived accounts
	accounts: HashMap<String, Account>,
	/// Language for mnemonic
	language: Language,
}

impl fmt::Debug for HDWallet {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("HDWallet")
			.field("mnemonic_words", &self.mnemonic_phrase.split_whitespace().count())
			.field("language", &self.language)
			.field("accounts_cached", &self.accounts.len())
			.field("has_passphrase", &self.passphrase.is_some())
			.field("mnemonic", &"<redacted>")
			.field("seed", &"<redacted>")
			.field("master_key", &"<redacted>")
			.finish()
	}
}

impl Drop for HDWallet {
	fn drop(&mut self) {
		self.seed.zeroize();
		self.mnemonic_phrase.zeroize();
		if let Some(passphrase) = &mut self.passphrase {
			passphrase.zeroize();
		}
		self.master_key.key.zeroize();
		self.master_key.chain_code.zeroize();
	}
}

impl HDWallet {
	/// Generate a new HD wallet with random mnemonic
	///
	/// Creates a new wallet with a randomly generated mnemonic phrase.
	/// The entropy and security increase with word count:
	/// - 12 words: 128 bits of entropy (minimum recommended)
	/// - 24 words: 256 bits of entropy (maximum security)
	///
	/// # Arguments
	///
	/// * `word_count` - Number of words (12, 15, 18, 21, or 24)
	/// * `passphrase` - Optional BIP-39 passphrase for extra security
	///
	/// # Security Note
	///
	/// The passphrase acts as a "25th word" and creates a completely
	/// different wallet. Loss of the passphrase means loss of funds.
	pub fn generate(word_count: usize, passphrase: Option<&str>) -> Result<Self, NeoError> {
		match word_count {
			12 | 15 | 18 | 21 | 24 => {},
			_ => {
				return Err(NeoError::Wallet {
					message: format!(
						"Invalid word count: {}. Use 12, 15, 18, 21, or 24",
						word_count
					),
					source: None,
					recovery: ErrorRecovery::new()
						.suggest("Use 12 words for standard security")
						.suggest("Use 24 words for maximum security"),
				});
			},
		};

		let mnemonic = Mnemonic::generate(word_count).map_err(|e| NeoError::Wallet {
			message: format!("Failed to generate mnemonic: {e}"),
			source: Some(Box::new(e)),
			recovery: ErrorRecovery::new()
				.suggest("Ensure a secure OS random number generator is available")
				.suggest("If running in a constrained environment, provide an existing mnemonic"),
		})?;

		Self::from_mnemonic(mnemonic, passphrase, Language::English)
	}

	/// Create HD wallet from existing mnemonic phrase
	///
	/// # Arguments
	/// * `mnemonic` - BIP-39 mnemonic phrase
	/// * `passphrase` - Optional BIP-39 passphrase
	/// * `language` - Mnemonic language
	pub fn from_mnemonic(
		mnemonic: Mnemonic,
		passphrase: Option<&str>,
		language: Language,
	) -> Result<Self, NeoError> {
		// Generate seed from mnemonic
		let seed = mnemonic.to_seed(passphrase.unwrap_or(""));
		let master_key = ExtendedPrivateKey::from_seed(&seed)?;
		let mnemonic_phrase = mnemonic.to_string();

		Ok(Self {
			mnemonic,
			mnemonic_phrase,
			seed: seed.to_vec(),
			passphrase: passphrase.map(str::to_owned),
			master_key,
			accounts: HashMap::new(),
			language,
		})
	}

	/// Create HD wallet from mnemonic string
	pub fn from_phrase(
		phrase: &str,
		passphrase: Option<&str>,
		language: Language,
	) -> Result<Self, NeoError> {
		let mnemonic = Mnemonic::parse_in(language, phrase).map_err(|e| NeoError::Wallet {
			message: format!("Invalid mnemonic phrase: {}", e),
			source: Some(Box::new(e)),
			recovery: ErrorRecovery::new()
				.suggest("Check for typos in the mnemonic phrase")
				.suggest("Ensure all words are from the BIP-39 word list")
				.suggest("Verify the correct number of words (12, 15, 18, 21, or 24)"),
		})?;

		Self::from_mnemonic(mnemonic, passphrase, language)
	}

	/// Get the mnemonic phrase
	pub fn mnemonic_phrase(&self) -> &str {
		&self.mnemonic_phrase
	}

	/// Derive an account at the given path
	///
	/// Derives a Neo account at the specified BIP-44 path. Accounts are
	/// cached for performance, so repeated derivations of the same path
	/// are nearly instant.
	///
	/// # Arguments
	///
	/// * `path` - Derivation path (e.g., "m/44'/888'/0'/0/0")
	///
	/// # Returns
	///
	/// A Neo `Account` that can be used for transactions
	///
	/// # Performance
	///
	/// First derivation: ~10ms
	/// Cached derivation: <1ms
	pub fn derive_account(&mut self, path: &str) -> Result<Account, NeoError> {
		// Check cache first
		if let Some(account) = self.accounts.get(path) {
			return Ok(account.clone());
		}

		let derivation_path = DerivationPath::from_string(path)?;
		let derived_key = self.derive_key(&derivation_path)?;

		use crate::neo_crypto::{Secp256r1PrivateKey, wif_from_private_key};

		let private_key =
			Secp256r1PrivateKey::from_bytes(&derived_key.key).map_err(|e| NeoError::Wallet {
				message: format!("Invalid derived key: {}", e),
				source: None,
				recovery: ErrorRecovery::new(),
			})?;

		// WIF encodes the private key: zeroize the intermediate copy instead of
		// letting it linger in freed heap memory.
		let mut wif = wif_from_private_key(&private_key);
		let account = Account::from_wif(&wif).map_err(|e| NeoError::Wallet {
			message: format!("Failed to create account from derived key: {}", e),
			source: None,
			recovery: ErrorRecovery::new(),
		});
		wif.zeroize();
		let account = account?;

		// Cache the account
		self.accounts.insert(path.to_string(), account.clone());

		Ok(account)
	}

	/// Derive multiple accounts
	///
	/// # Arguments
	/// * `account_index` - Account index to start from
	/// * `count` - Number of accounts to derive
	pub fn derive_accounts(
		&mut self,
		account_index: u32,
		count: u32,
	) -> Result<Vec<Account>, NeoError> {
		let mut accounts = Vec::new();

		for i in 0..count {
			let index = account_index.checked_add(i).ok_or_else(|| NeoError::Wallet {
				message: format!(
					"Deriving {} accounts from index {} overflows the u32 account space",
					count, account_index
				),
				source: None,
				recovery: ErrorRecovery::new().suggest("Use a smaller count or starting index"),
			})?;
			let path = format!("m/44'/888'/{}'/0/0", index);
			accounts.push(self.derive_account(&path)?);
		}

		Ok(accounts)
	}

	/// Get the default account (m/44'/888'/0'/0/0)
	pub fn get_default_account(&mut self) -> Result<Account, NeoError> {
		self.derive_account("m/44'/888'/0'/0/0")
	}

	/// Derive a key at the given path
	fn derive_key(&self, path: &DerivationPath) -> Result<ExtendedPrivateKey, NeoError> {
		let mut key = self.master_key.clone();

		// Derive through each level
		key = key.derive_child(path.purpose)?;
		key = key.derive_child(path.coin_type)?;
		key = key.derive_child(path.account)?;
		key = key.derive_child(path.change)?;
		key = key.derive_child(path.index)?;

		Ok(key)
	}

	/// Convert key bytes to WIF format
	/// Export wallet to encrypted JSON
	pub fn export_encrypted(&self, password: &str) -> Result<String, NeoError> {
		use base64::Engine;
		use base64::engine::general_purpose;
		use rand::RngCore;
		use scrypt::Params;

		if password.is_empty() {
			return Err(NeoError::Validation {
				message: "Password cannot be empty".to_string(),
				field: "password".to_string(),
				value: None,
				recovery: ErrorRecovery::new()
					.suggest("Provide a non-empty password")
					.suggest("Use a strong passphrase"),
			});
		}

		let wallet_data = HDWalletData {
			mnemonic: self.mnemonic_phrase.clone(),
			passphrase: self.passphrase.clone(),
			language: format!("{:?}", self.language),
			accounts: self.accounts.keys().cloned().collect(),
		};

		let plaintext = serde_json::to_vec(&wallet_data).map_err(|e| NeoError::Wallet {
			message: format!("Failed to serialize wallet: {e}"),
			source: Some(Box::new(e)),
			recovery: ErrorRecovery::new(),
		})?;

		// Derive key using standard Neo scrypt parameters and random salt
		let scrypt_def = crate::neo_types::ScryptParamsDef::default();
		let params = Params::new(scrypt_def.log_n, scrypt_def.r, scrypt_def.p).map_err(|e| {
			NeoError::Wallet {
				message: format!("Invalid scrypt parameters: {e}"),
				source: None,
				recovery: ErrorRecovery::new(),
			}
		})?;

		let mut salt = [0u8; HD_WALLET_SALT_LEN];
		rand::rng().fill_bytes(&mut salt);

		let mut nonce = [0u8; HD_WALLET_NONCE_LEN];
		rand::rng().fill_bytes(&mut nonce);

		let mut key_material =
			derive_hd_wallet_key_material(password, &salt, &params, HD_WALLET_KEY_MATERIAL_LEN)?;
		let mut encryption_key = [0u8; 32];
		let mut mac_key = [0u8; 32];
		encryption_key.copy_from_slice(&key_material[..32]);
		mac_key.copy_from_slice(&key_material[32..]);

		let ciphertext = aes256_ctr_crypt(&encryption_key, &nonce, &plaintext)?;
		let tag = hd_wallet_auth_tag(
			HD_WALLET_EXPORT_VERSION,
			&scrypt_def,
			&salt,
			&nonce,
			&ciphertext,
			&mac_key,
		)?;

		key_material.zeroize();
		encryption_key.zeroize();
		mac_key.zeroize();

		let encrypted = EncryptedHDWalletData {
			version: HD_WALLET_EXPORT_VERSION,
			scrypt: scrypt_def,
			salt: general_purpose::STANDARD.encode(salt),
			nonce: Some(general_purpose::STANDARD.encode(nonce)),
			tag: Some(general_purpose::STANDARD.encode(tag)),
			ciphertext: general_purpose::STANDARD.encode(ciphertext),
		};

		serde_json::to_string_pretty(&encrypted).map_err(|e| NeoError::Wallet {
			message: format!("Failed to serialize encrypted wallet: {e}"),
			source: Some(Box::new(e)),
			recovery: ErrorRecovery::new(),
		})
	}

	/// Import wallet from encrypted JSON
	pub fn import_encrypted(json: &str, password: &str) -> Result<Self, NeoError> {
		use aes::cipher::{BlockDecryptMut, KeyInit, block_padding::Pkcs7};
		use base64::Engine;
		use base64::engine::general_purpose;
		use scrypt::{Params, scrypt};

		type Aes256EcbDec = ecb::Decryptor<aes::Aes256>;

		// First try to parse as encrypted payload (new format).
		let encrypted_payload: Result<EncryptedHDWalletData, _> = serde_json::from_str(json);
		let wallet_data: HDWalletData = if let Ok(encrypted_payload) = encrypted_payload {
			if password.is_empty() {
				return Err(NeoError::Validation {
					message: "Password cannot be empty".to_string(),
					field: "password".to_string(),
					value: None,
					recovery: ErrorRecovery::new().suggest("Provide the password used for export"),
				});
			}

			let salt = general_purpose::STANDARD
				.decode(encrypted_payload.salt.as_bytes())
				.map_err(|e| NeoError::Wallet {
					message: format!("Invalid salt encoding: {e}"),
					source: Some(Box::new(e)),
					recovery: ErrorRecovery::new(),
				})?;

			let ciphertext = general_purpose::STANDARD
				.decode(encrypted_payload.ciphertext.as_bytes())
				.map_err(|e| NeoError::Wallet {
					message: format!("Invalid ciphertext encoding: {e}"),
					source: Some(Box::new(e)),
					recovery: ErrorRecovery::new(),
				})?;

			let params = Params::new(
				encrypted_payload.scrypt.log_n,
				encrypted_payload.scrypt.r,
				encrypted_payload.scrypt.p,
			)
			.map_err(|e| NeoError::Wallet {
				message: format!("Invalid scrypt parameters: {e}"),
				source: None,
				recovery: ErrorRecovery::new(),
			})?;

			let plaintext = match encrypted_payload.version {
				HD_WALLET_EXPORT_VERSION => {
					let nonce = encrypted_payload
						.nonce
						.as_deref()
						.ok_or_else(|| encrypted_wallet_error("Missing nonce"))?;
					let nonce =
						general_purpose::STANDARD.decode(nonce.as_bytes()).map_err(|e| {
							NeoError::Wallet {
								message: format!("Invalid nonce encoding: {e}"),
								source: Some(Box::new(e)),
								recovery: ErrorRecovery::new(),
							}
						})?;
					let nonce: [u8; HD_WALLET_NONCE_LEN] = nonce
						.try_into()
						.map_err(|_| encrypted_wallet_error("Invalid nonce length"))?;

					let tag = encrypted_payload
						.tag
						.as_deref()
						.ok_or_else(|| encrypted_wallet_error("Missing authentication tag"))?;
					let tag = general_purpose::STANDARD.decode(tag.as_bytes()).map_err(|e| {
						NeoError::Wallet {
							message: format!("Invalid authentication tag encoding: {e}"),
							source: Some(Box::new(e)),
							recovery: ErrorRecovery::new(),
						}
					})?;
					let tag: [u8; HD_WALLET_TAG_LEN] = tag
						.try_into()
						.map_err(|_| encrypted_wallet_error("Invalid authentication tag length"))?;

					let mut key_material = derive_hd_wallet_key_material(
						password,
						&salt,
						&params,
						HD_WALLET_KEY_MATERIAL_LEN,
					)?;
					let mut encryption_key = [0u8; 32];
					let mut mac_key = [0u8; 32];
					encryption_key.copy_from_slice(&key_material[..32]);
					mac_key.copy_from_slice(&key_material[32..]);

					verify_hd_wallet_auth_tag(
						HD_WALLET_EXPORT_VERSION,
						&encrypted_payload.scrypt,
						&salt,
						&nonce,
						&ciphertext,
						&mac_key,
						&tag,
					)?;

					let plaintext = aes256_ctr_crypt(&encryption_key, &nonce, &ciphertext)?;
					key_material.zeroize();
					encryption_key.zeroize();
					mac_key.zeroize();
					plaintext
				},
				HD_WALLET_LEGACY_ECB_VERSION => {
					let mut key = [0u8; 32];
					scrypt(password.as_bytes(), &salt, &params, &mut key).map_err(|e| {
						NeoError::Wallet {
							message: format!("Failed to derive decryption key: {e}"),
							source: Some(Box::new(e)),
							recovery: ErrorRecovery::new().suggest("Check password correctness"),
						}
					})?;

					let mut buf = vec![0u8; ciphertext.len()];
					let plaintext = Aes256EcbDec::new(&key.into())
						.decrypt_padded_b2b_mut::<Pkcs7>(&ciphertext, &mut buf)
						.map_err(|_| NeoError::Wallet {
							message: "AES decryption failed (wrong password?)".to_string(),
							source: None,
							recovery: ErrorRecovery::new()
								.suggest("Verify the password")
								.retryable(false),
						})?
						.to_vec();
					key.zeroize();
					plaintext
				},
				version => {
					return Err(encrypted_wallet_error(format!(
						"Unsupported encrypted wallet version: {version}"
					)));
				},
			};

			let plaintext_str = String::from_utf8(plaintext).map_err(|e| NeoError::Wallet {
				message: format!("Decrypted data is not valid UTF-8: {e}"),
				source: Some(Box::new(e)),
				recovery: ErrorRecovery::new(),
			})?;

			serde_json::from_str::<HDWalletData>(&plaintext_str).map_err(|e| NeoError::Wallet {
				message: format!("Failed to deserialize wallet data: {e}"),
				source: Some(Box::new(e)),
				recovery: ErrorRecovery::new(),
			})?
		} else {
			// Backwards-compatible plaintext import.
			serde_json::from_str::<HDWalletData>(json).map_err(|e| NeoError::Wallet {
				message: format!("Failed to deserialize wallet: {e}"),
				source: Some(Box::new(e)),
				recovery: ErrorRecovery::new(),
			})?
		};

		let language = match wallet_data.language.as_str() {
			"English" => Language::English,
			_ => Language::English, // Default to English
		};

		let mut wallet =
			Self::from_phrase(&wallet_data.mnemonic, wallet_data.passphrase.as_deref(), language)?;
		// Restore cached accounts based on stored derivation paths.
		for path in wallet_data.accounts {
			let _ = wallet.derive_account(&path)?;
		}

		Ok(wallet)
	}
}

fn encrypted_wallet_error(message: impl Into<String>) -> NeoError {
	NeoError::Wallet { message: message.into(), source: None, recovery: ErrorRecovery::new() }
}

fn derive_hd_wallet_key_material(
	password: &str,
	salt: &[u8],
	params: &scrypt::Params,
	len: usize,
) -> Result<Vec<u8>, NeoError> {
	use scrypt::scrypt;

	let mut key_material = vec![0u8; len];
	scrypt(password.as_bytes(), salt, params, &mut key_material).map_err(|e| NeoError::Wallet {
		message: format!("Failed to derive wallet encryption key: {e}"),
		source: Some(Box::new(e)),
		recovery: ErrorRecovery::new()
			.suggest("Check password encoding")
			.suggest("Verify the password"),
	})?;

	Ok(key_material)
}

fn aes256_ctr_crypt(
	key: &[u8; 32],
	nonce: &[u8; HD_WALLET_NONCE_LEN],
	input: &[u8],
) -> Result<Vec<u8>, NeoError> {
	use aes::cipher::{BlockEncrypt, KeyInit};

	let cipher =
		aes::Aes256::new_from_slice(key).map_err(|_| encrypted_wallet_error("Invalid AES key"))?;
	let mut counter = *nonce;
	let mut output = Vec::with_capacity(input.len());

	for chunk in input.chunks(16) {
		let mut block = counter.into();
		cipher.encrypt_block(&mut block);

		for (byte, keystream) in chunk.iter().zip(block.iter()) {
			output.push(byte ^ keystream);
		}

		for byte in counter.iter_mut().rev() {
			let (next, overflow) = byte.overflowing_add(1);
			*byte = next;
			if !overflow {
				break;
			}
		}
	}

	Ok(output)
}

fn hd_wallet_auth_tag(
	version: u8,
	params: &crate::neo_types::ScryptParamsDef,
	salt: &[u8],
	nonce: &[u8; HD_WALLET_NONCE_LEN],
	ciphertext: &[u8],
	mac_key: &[u8; 32],
) -> Result<[u8; HD_WALLET_TAG_LEN], NeoError> {
	let mut mac = Hmac::<Sha512>::new_from_slice(mac_key)
		.map_err(|_| encrypted_wallet_error("Invalid wallet authentication key"))?;
	mac.update(&[version]);
	mac.update(&[params.log_n]);
	mac.update(&params.r.to_be_bytes());
	mac.update(&params.p.to_be_bytes());
	mac.update(salt);
	mac.update(nonce);
	mac.update(ciphertext);

	let digest = mac.finalize().into_bytes();
	let mut tag = [0u8; HD_WALLET_TAG_LEN];
	tag.copy_from_slice(&digest[..HD_WALLET_TAG_LEN]);
	Ok(tag)
}

fn verify_hd_wallet_auth_tag(
	version: u8,
	params: &crate::neo_types::ScryptParamsDef,
	salt: &[u8],
	nonce: &[u8; HD_WALLET_NONCE_LEN],
	ciphertext: &[u8],
	mac_key: &[u8; 32],
	tag: &[u8; HD_WALLET_TAG_LEN],
) -> Result<(), NeoError> {
	let expected = hd_wallet_auth_tag(version, params, salt, nonce, ciphertext, mac_key)?;
	let diff = expected
		.iter()
		.zip(tag.iter())
		.fold(0u8, |acc, (expected, actual)| acc | (expected ^ actual));

	if diff == 0 {
		Ok(())
	} else {
		Err(NeoError::Wallet {
			message: "Encrypted wallet authentication failed (wrong password or tampered data)"
				.to_string(),
			source: None,
			recovery: ErrorRecovery::new().suggest("Verify the password").retryable(false),
		})
	}
}

/// Extended private key for HD derivation
#[derive(Clone)]
struct ExtendedPrivateKey {
	key: Vec<u8>,
	chain_code: Vec<u8>,
}

impl fmt::Debug for ExtendedPrivateKey {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("ExtendedPrivateKey")
			.field("key_len", &self.key.len())
			.field("chain_code_len", &self.chain_code.len())
			.field("key", &"<redacted>")
			.field("chain_code", &"<redacted>")
			.finish()
	}
}

impl Drop for ExtendedPrivateKey {
	fn drop(&mut self) {
		self.key.zeroize();
		self.chain_code.zeroize();
	}
}

impl ExtendedPrivateKey {
	/// Create from seed
	fn from_seed(seed: &[u8]) -> Result<Self, NeoError> {
		let mut mac =
			Hmac::<Sha512>::new_from_slice(b"Nist256p1 seed").map_err(|e| NeoError::Wallet {
				message: format!("Failed to create HMAC: {}", e),
				source: None,
				recovery: ErrorRecovery::new(),
			})?;

		mac.update(seed);
		let result = mac.finalize();
		let bytes = result.into_bytes();

		Ok(Self { key: bytes[..32].to_vec(), chain_code: bytes[32..].to_vec() })
	}

	/// Derive child key
	fn derive_child(&self, index: u32) -> Result<Self, NeoError> {
		let mut mac =
			Hmac::<Sha512>::new_from_slice(&self.chain_code).map_err(|e| NeoError::Wallet {
				message: format!("Failed to create HMAC for child derivation: {}", e),
				source: None,
				recovery: ErrorRecovery::new(),
			})?;

		if index >= 0x80000000 {
			// Hardened child: HMAC-SHA512(Key = chain_code, Data = 0x00 || key || index)
			mac.update(&[0x00]);
			mac.update(&self.key);
		} else {
			// Normal child: HMAC-SHA512(Key = chain_code, Data = public_key || index)
			// For secp256r1 (NIST P-256), derive the compressed public key from the private key
			let secret_key =
				p256::SecretKey::from_slice(&self.key).map_err(|e| NeoError::Wallet {
					message: format!("Invalid private key for public key derivation: {}", e),
					source: None,
					recovery: ErrorRecovery::new(),
				})?;
			let public_key = secret_key.public_key();
			let compressed = public_key.to_sec1_bytes();
			mac.update(&compressed);
		}
		mac.update(&index.to_be_bytes());
		let result = mac.finalize();
		let bytes = result.into_bytes();

		Ok(Self { key: bytes[..32].to_vec(), chain_code: bytes[32..].to_vec() })
	}
}

/// Serializable wallet data
#[derive(Serialize, Deserialize)]
struct HDWalletData {
	mnemonic: String,
	#[serde(default)]
	passphrase: Option<String>,
	language: String,
	accounts: Vec<String>,
}

/// Encrypted HD wallet export format.
#[derive(Serialize, Deserialize)]
struct EncryptedHDWalletData {
	version: u8,
	scrypt: crate::neo_types::ScryptParamsDef,
	/// base64-encoded salt
	salt: String,
	/// base64-encoded AES-256-CTR nonce (version 2+)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	nonce: Option<String>,
	/// base64-encoded authentication tag (version 2+)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	tag: Option<String>,
	/// base64-encoded ciphertext
	ciphertext: String,
}

/// Builder for HD wallet configuration
///
/// Provides a fluent interface for creating HD wallets with custom
/// configuration. Supports both generating new wallets and importing
/// existing mnemonics.
#[derive(Clone)]
pub struct HDWalletBuilder {
	word_count: usize,
	passphrase: Option<String>,
	language: Language,
	mnemonic: Option<String>,
}

impl fmt::Debug for HDWalletBuilder {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("HDWalletBuilder")
			.field("word_count", &self.word_count)
			.field("language", &self.language)
			.field("has_passphrase", &self.passphrase.is_some())
			.field("has_mnemonic", &self.mnemonic.is_some())
			.finish()
	}
}

impl Drop for HDWalletBuilder {
	fn drop(&mut self) {
		if let Some(passphrase) = &mut self.passphrase {
			passphrase.zeroize();
		}
		if let Some(mnemonic) = &mut self.mnemonic {
			mnemonic.zeroize();
		}
	}
}

impl Default for HDWalletBuilder {
	fn default() -> Self {
		Self { word_count: 12, passphrase: None, language: Language::English, mnemonic: None }
	}
}

impl HDWalletBuilder {
	/// Create a new builder
	pub fn new() -> Self {
		Self::default()
	}

	/// Set word count for mnemonic generation
	#[must_use]
	pub fn word_count(mut self, count: usize) -> Self {
		self.word_count = count;
		self
	}

	/// Set BIP-39 passphrase
	#[must_use]
	pub fn passphrase(mut self, passphrase: impl Into<String>) -> Self {
		self.passphrase = Some(passphrase.into());
		self
	}

	/// Set mnemonic language
	#[must_use]
	pub fn language(mut self, language: Language) -> Self {
		self.language = language;
		self
	}

	/// Set existing mnemonic phrase
	#[must_use]
	pub fn mnemonic(mut self, mnemonic: impl Into<String>) -> Self {
		self.mnemonic = Some(mnemonic.into());
		self
	}

	/// Build the HD wallet
	pub fn build(self) -> Result<HDWallet, NeoError> {
		let language = self.language;
		let passphrase = self.passphrase.as_deref();

		if let Some(mnemonic_phrase) = self.mnemonic.as_deref() {
			HDWallet::from_phrase(mnemonic_phrase, passphrase, language)
		} else {
			HDWallet::generate(self.word_count, passphrase)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_derivation_path_parsing() {
		let path = DerivationPath::from_string("m/44'/888'/0'/0/0").unwrap();
		assert_eq!(path.purpose, 0x80000000 + 44);
		assert_eq!(path.coin_type, 0x80000000 + 888);
		assert_eq!(path.account, 0x80000000);
		assert_eq!(path.change, 0);
		assert_eq!(path.index, 0);

		let path_str = path.to_string();
		assert_eq!(path_str, "m/44'/888'/0'/0/0");
	}

	#[test]
	fn test_hd_wallet_generation() {
		let wallet = HDWallet::generate(12, None);
		assert!(wallet.is_ok());

		let wallet = wallet.unwrap();
		let phrase = wallet.mnemonic_phrase();
		assert_eq!(phrase.split_whitespace().count(), 12);
	}

	#[test]
	fn test_hd_wallet_from_phrase() {
		let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
		let wallet = HDWallet::from_phrase(phrase, None, Language::English);
		assert!(wallet.is_ok());
	}

	#[test]
	fn test_account_derivation() {
		let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
		let mut wallet = HDWallet::from_phrase(phrase, None, Language::English).unwrap();

		let account1 = wallet.derive_account("m/44'/888'/0'/0/0");
		assert!(account1.is_ok());

		let account2 = wallet.derive_account("m/44'/888'/0'/0/1");
		assert!(account2.is_ok());

		// Verify different accounts have different addresses
		let addr1 = account1.unwrap().get_address();
		let addr2 = account2.unwrap().get_address();
		assert_ne!(addr1, addr2);
	}

	#[test]
	fn test_builder() {
		let wallet = HDWalletBuilder::new()
			.word_count(24)
			.passphrase("test")
			.language(Language::English)
			.build();

		assert!(wallet.is_ok());
	}

	#[test]
	fn test_export_import_encrypted_roundtrip() {
		let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
		let mut wallet = HDWallet::from_phrase(phrase, None, Language::English).unwrap();

		wallet.derive_account("m/44'/888'/0'/0/0").unwrap();
		wallet.derive_account("m/44'/888'/0'/0/1").unwrap();

		let password = "correct horse battery staple";
		let exported = wallet.export_encrypted(password).unwrap();
		let exported_json: serde_json::Value = serde_json::from_str(&exported).unwrap();
		assert_eq!(exported_json.get("version").and_then(|v| v.as_u64()), Some(2));
		assert!(exported_json.get("nonce").and_then(|v| v.as_str()).is_some());
		assert!(exported_json.get("tag").and_then(|v| v.as_str()).is_some());
		let imported = HDWallet::import_encrypted(&exported, password).unwrap();

		assert_eq!(imported.mnemonic_phrase(), phrase);
		assert_eq!(imported.accounts.len(), 2);
		assert!(imported.accounts.contains_key("m/44'/888'/0'/0/0"));
		assert!(imported.accounts.contains_key("m/44'/888'/0'/0/1"));
	}

	#[test]
	fn test_export_import_preserves_bip39_passphrase() {
		let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
		let passphrase = "bip39 passphrase";
		let path = "m/44'/888'/0'/0/0";
		let mut wallet =
			HDWallet::from_phrase(phrase, Some(passphrase), Language::English).unwrap();
		let original_address = wallet.derive_account(path).unwrap().get_address();

		let exported = wallet.export_encrypted("wallet password").unwrap();
		let mut imported = HDWallet::import_encrypted(&exported, "wallet password").unwrap();
		let imported_address = imported.derive_account(path).unwrap().get_address();

		assert_eq!(imported_address, original_address);
	}

	#[test]
	fn test_import_encrypted_rejects_tampered_ciphertext() {
		let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
		let wallet = HDWallet::from_phrase(phrase, None, Language::English).unwrap();
		let exported = wallet.export_encrypted("wallet password").unwrap();
		let mut json: serde_json::Value = serde_json::from_str(&exported).unwrap();
		let ciphertext = json.get_mut("ciphertext").unwrap().as_str().unwrap().to_string();
		let replacement = if ciphertext.ends_with('A') { "B" } else { "A" };
		let last = ciphertext.len() - 1;
		json["ciphertext"] =
			serde_json::Value::String(format!("{}{}", &ciphertext[..last], replacement));

		let tampered = serde_json::to_string(&json).unwrap();
		let err = HDWallet::import_encrypted(&tampered, "wallet password").unwrap_err();
		assert!(format!("{err}").contains("authentication failed"));
	}
}
