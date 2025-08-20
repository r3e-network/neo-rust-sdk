//! HD Wallet support with BIP-39/44 implementation
//! 
//! Provides hierarchical deterministic wallet functionality for Neo,
//! including mnemonic generation, key derivation, and account management.

use crate::neo_error::unified::{NeoError, ErrorRecovery};
use crate::neo_protocol::{Account, AccountTrait};
use crate::neo_crypto::{KeyPair, Secp256r1PublicKey};
use bip39::{Mnemonic, Language};
use hmac::{Hmac, Mac};
use sha2::Sha512;
use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

/// HD wallet derivation path components
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
    /// Default path: m/44'/888'/0'/0/0
    pub fn new_neo(account: u32, index: u32) -> Self {
        Self {
            purpose: 0x80000000 + 44,  // 44' hardened
            coin_type: 0x80000000 + 888, // 888' for NEO
            account: 0x80000000 + account, // account' hardened
            change: 0, // external addresses
            index,
        }
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
            let num_str = if hardened { &s[..s.len()-1] } else { s };
            
            let num = num_str.parse::<u32>().map_err(|e| NeoError::Wallet {
                message: format!("Invalid path component: {}", s),
                source: Some(Box::new(e)),
                recovery: ErrorRecovery::new(),
            })?;

            Ok(if hardened { 0x80000000 + num } else { num })
        };

        Ok(Self {
            purpose: parse_component(parts[0])?,
            coin_type: parse_component(parts[1])?,
            account: parse_component(parts[2])?,
            change: parse_component(parts[3])?,
            index: parse_component(parts[4])?,
        })
    }

    /// Convert to string representation
    pub fn to_string(&self) -> String {
        let format_component = |n: u32| -> String {
            if n >= 0x80000000 {
                format!("{}'", n - 0x80000000)
            } else {
                format!("{}", n)
            }
        };

        format!("m/{}/{}/{}/{}/{}",
            format_component(self.purpose),
            format_component(self.coin_type),
            format_component(self.account),
            format_component(self.change),
            format_component(self.index))
    }
}

/// HD Wallet implementation with BIP-39/44 support
pub struct HDWallet {
    /// Mnemonic phrase
    mnemonic: Mnemonic,
    /// Mnemonic phrase as string
    mnemonic_phrase: String,
    /// Seed bytes derived from mnemonic
    seed: Vec<u8>,
    /// Master private key
    master_key: ExtendedPrivateKey,
    /// Cached derived accounts
    accounts: HashMap<String, Account>,
    /// Language for mnemonic
    language: Language,
}

impl HDWallet {
    /// Generate a new HD wallet with random mnemonic
    /// 
    /// # Arguments
    /// * `word_count` - Number of words (12, 15, 18, 21, or 24)
    /// * `passphrase` - Optional BIP-39 passphrase
    pub fn generate(word_count: usize, passphrase: Option<&str>) -> Result<Self, NeoError> {
        let entropy_bits = match word_count {
            12 => 128,
            15 => 160,
            18 => 192,
            21 => 224,
            24 => 256,
            _ => return Err(NeoError::Wallet {
                message: format!("Invalid word count: {}. Use 12, 15, 18, 21, or 24", word_count),
                source: None,
                recovery: ErrorRecovery::new()
                    .suggest("Use 12 words for standard security")
                    .suggest("Use 24 words for maximum security"),
            }),
        };

        let mnemonic = Mnemonic::generate(entropy_bits).map_err(|e| NeoError::Wallet {
            message: format!("Failed to generate mnemonic: {}", e),
            source: Some(Box::new(e)),
            recovery: ErrorRecovery::new(),
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
    /// # Arguments
    /// * `path` - Derivation path (e.g., "m/44'/888'/0'/0/0")
    pub fn derive_account(&mut self, path: &str) -> Result<Account, NeoError> {
        // Check cache first
        if let Some(account) = self.accounts.get(path) {
            return Ok(account.clone());
        }

        let derivation_path = DerivationPath::from_string(path)?;
        let derived_key = self.derive_key(&derivation_path)?;
        
        // Convert to Neo account using WIF
        // For now, we'll create an account from the derived key bytes
        // In a full implementation, we'd use the proper Secp256r1 private key
        use crate::neo_crypto::wif_from_private_key;
        
        // Create a WIF from the derived key (simplified - in production use proper conversion)
        let wif = self.key_to_wif(&derived_key.key)?;
        let account = Account::from_wif(&wif).map_err(|e| NeoError::Wallet {
            message: format!("Failed to create account from derived key: {}", e),
            source: None,
            recovery: ErrorRecovery::new(),
        })?;

        // Cache the account
        self.accounts.insert(path.to_string(), account.clone());

        Ok(account)
    }

    /// Derive multiple accounts
    /// 
    /// # Arguments
    /// * `account_index` - Account index to start from
    /// * `count` - Number of accounts to derive
    pub fn derive_accounts(&mut self, account_index: u32, count: u32) -> Result<Vec<Account>, NeoError> {
        let mut accounts = Vec::new();
        
        for i in 0..count {
            let path = format!("m/44'/888'/{}'/0/0", account_index + i);
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
    fn key_to_wif(&self, key_bytes: &[u8]) -> Result<String, NeoError> {
        use crate::neo_crypto::base58check_encode;
        
        if key_bytes.len() != 32 {
            return Err(NeoError::Wallet {
                message: format!("Invalid key length: {} (expected 32)", key_bytes.len()),
                source: None,
                recovery: ErrorRecovery::new(),
            });
        }

        // Build WIF: 0x80 + key + 0x01 (for compressed)
        // base58check_encode handles the checksum
        let mut wif_bytes = Vec::with_capacity(34);
        wif_bytes.push(0x80); // MainNet prefix
        wif_bytes.extend_from_slice(key_bytes);
        wif_bytes.push(0x01); // Compressed flag
        
        Ok(base58check_encode(&wif_bytes))
    }

    /// Export wallet to encrypted JSON
    pub fn export_encrypted(&self, _password: &str) -> Result<String, NeoError> {
        let wallet_data = HDWalletData {
            mnemonic: self.mnemonic_phrase.clone(),
            language: format!("{:?}", self.language),
            accounts: self.accounts.keys().cloned().collect(),
        };

        // Encrypt with password (simplified - in production use proper encryption)
        let json = serde_json::to_string_pretty(&wallet_data).map_err(|e| NeoError::Wallet {
            message: format!("Failed to serialize wallet: {}", e),
            source: Some(Box::new(e)),
            recovery: ErrorRecovery::new(),
        })?;

        // TODO: Implement proper encryption with scrypt + AES
        Ok(json)
    }

    /// Import wallet from encrypted JSON
    pub fn import_encrypted(json: &str, _password: &str) -> Result<Self, NeoError> {
        // TODO: Implement proper decryption
        let wallet_data: HDWalletData = serde_json::from_str(json).map_err(|e| NeoError::Wallet {
            message: format!("Failed to deserialize wallet: {}", e),
            source: Some(Box::new(e)),
            recovery: ErrorRecovery::new(),
        })?;

        let language = match wallet_data.language.as_str() {
            "English" => Language::English,
            _ => Language::English, // Default to English
        };

        Self::from_phrase(&wallet_data.mnemonic, None, language)
    }
}

/// Extended private key for HD derivation
#[derive(Clone)]
struct ExtendedPrivateKey {
    key: Vec<u8>,
    chain_code: Vec<u8>,
}

impl ExtendedPrivateKey {
    /// Create from seed
    fn from_seed(seed: &[u8]) -> Result<Self, NeoError> {
        let mut mac = Hmac::<Sha512>::new_from_slice(b"Bitcoin seed")
            .map_err(|e| NeoError::Wallet {
                message: format!("Failed to create HMAC: {}", e),
                source: None,
                recovery: ErrorRecovery::new(),
            })?;
        
        mac.update(seed);
        let result = mac.finalize();
        let bytes = result.into_bytes();

        Ok(Self {
            key: bytes[..32].to_vec(),
            chain_code: bytes[32..].to_vec(),
        })
    }

    /// Derive child key
    fn derive_child(&self, index: u32) -> Result<Self, NeoError> {
        let mut mac = Hmac::<Sha512>::new_from_slice(&self.chain_code)
            .map_err(|e| NeoError::Wallet {
                message: format!("Failed to create HMAC for child derivation: {}", e),
                source: None,
                recovery: ErrorRecovery::new(),
            })?;

        if index >= 0x80000000 {
            // Hardened derivation
            mac.update(&[0x00]);
            mac.update(&self.key);
        } else {
            // Non-hardened derivation (requires public key)
            // For simplicity, using hardened only for now
            return Err(NeoError::Wallet {
                message: "Non-hardened derivation not yet implemented".to_string(),
                source: None,
                recovery: ErrorRecovery::new()
                    .suggest("Use hardened derivation (index >= 0x80000000)"),
            });
        }

        mac.update(&index.to_be_bytes());
        let result = mac.finalize();
        let bytes = result.into_bytes();

        Ok(Self {
            key: bytes[..32].to_vec(),
            chain_code: bytes[32..].to_vec(),
        })
    }
}

/// Serializable wallet data
#[derive(Serialize, Deserialize)]
struct HDWalletData {
    mnemonic: String,
    language: String,
    accounts: Vec<String>,
}

/// Builder for HD wallet configuration
pub struct HDWalletBuilder {
    word_count: usize,
    passphrase: Option<String>,
    language: Language,
    mnemonic: Option<String>,
}

impl Default for HDWalletBuilder {
    fn default() -> Self {
        Self {
            word_count: 12,
            passphrase: None,
            language: Language::English,
            mnemonic: None,
        }
    }
}

impl HDWalletBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set word count for mnemonic generation
    pub fn word_count(mut self, count: usize) -> Self {
        self.word_count = count;
        self
    }

    /// Set BIP-39 passphrase
    pub fn passphrase(mut self, passphrase: impl Into<String>) -> Self {
        self.passphrase = Some(passphrase.into());
        self
    }

    /// Set mnemonic language
    pub fn language(mut self, language: Language) -> Self {
        self.language = language;
        self
    }

    /// Set existing mnemonic phrase
    pub fn mnemonic(mut self, mnemonic: impl Into<String>) -> Self {
        self.mnemonic = Some(mnemonic.into());
        self
    }

    /// Build the HD wallet
    pub fn build(self) -> Result<HDWallet, NeoError> {
        if let Some(mnemonic_phrase) = self.mnemonic {
            HDWallet::from_phrase(
                &mnemonic_phrase,
                self.passphrase.as_deref(),
                self.language,
            )
        } else {
            HDWallet::generate(self.word_count, self.passphrase.as_deref())
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
}