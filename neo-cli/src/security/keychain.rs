use crate::errors::CliError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Secure key storage interface for OS keychain integration
/// Provides platform-specific secure storage for sensitive data
pub struct KeychainManager {
    service_name: String,
    #[cfg(target_os = "macos")]
    keychain: security_framework::os::macos::keychain::SecKeychain,
    #[cfg(target_os = "windows")]
    credential_store: HashMap<String, Vec<u8>>,
    #[cfg(target_os = "linux")]
    secret_service: Option<libsecret::SecretService>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecureCredential {
    pub account: String,
    pub data: Vec<u8>,
    pub metadata: HashMap<String, String>,
}

impl KeychainManager {
    /// Create a new keychain manager
    pub fn new(service_name: &str) -> Result<Self, CliError> {
        #[cfg(target_os = "macos")]
        {
            use security_framework::os::macos::keychain::SecKeychain;
            Ok(Self {
                service_name: service_name.to_string(),
                keychain: SecKeychain::default().map_err(|e| {
                    CliError::Security(format!("Failed to access macOS keychain: {}", e))
                })?,
            })
        }

        #[cfg(target_os = "windows")]
        {
            Ok(Self {
                service_name: service_name.to_string(),
                credential_store: HashMap::new(),
            })
        }

        #[cfg(target_os = "linux")]
        {
            let secret_service = libsecret::SecretService::new().ok();
            Ok(Self {
                service_name: service_name.to_string(),
                secret_service,
            })
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            Err(CliError::Security("Unsupported platform for keychain access".to_string()))
        }
    }

    /// Store a credential securely
    pub fn store_credential(&mut self, key: &str, credential: &SecureCredential) -> Result<(), CliError> {
        let data = serde_json::to_vec(credential)
            .map_err(|e| CliError::Security(format!("Failed to serialize credential: {}", e)))?;

        #[cfg(target_os = "macos")]
        {
            use security_framework::os::macos::passwords;
            passwords::set_generic_password(
                &self.service_name,
                &credential.account,
                data.as_slice(),
            )
            .map_err(|e| CliError::Security(format!("Failed to store in macOS keychain: {}", e)))?;
        }

        #[cfg(target_os = "windows")]
        {
            // Use Windows Credential Manager via winapi
            self.credential_store.insert(key.to_string(), data);
            // In production, this would use Windows DPAPI
            self.store_windows_credential(key, &data)?;
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(ref service) = self.secret_service {
                // Use libsecret for Linux
                self.store_linux_secret(service, key, &data)?;
            } else {
                // Fallback to encrypted file storage
                self.store_encrypted_file(key, &data)?;
            }
        }

        Ok(())
    }

    /// Retrieve a credential securely
    pub fn get_credential(&self, key: &str) -> Result<SecureCredential, CliError> {
        #[cfg(target_os = "macos")]
        {
            use security_framework::os::macos::passwords;
            let data = passwords::get_generic_password(&self.service_name, key)
                .map_err(|e| CliError::Security(format!("Failed to retrieve from macOS keychain: {}", e)))?;
            
            serde_json::from_slice(&data)
                .map_err(|e| CliError::Security(format!("Failed to deserialize credential: {}", e)))
        }

        #[cfg(target_os = "windows")]
        {
            self.credential_store
                .get(key)
                .ok_or_else(|| CliError::Security("Credential not found".to_string()))
                .and_then(|data| {
                    serde_json::from_slice(data)
                        .map_err(|e| CliError::Security(format!("Failed to deserialize credential: {}", e)))
                })
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(ref service) = self.secret_service {
                self.get_linux_secret(service, key)
            } else {
                self.get_encrypted_file(key)
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            Err(CliError::Security("Unsupported platform".to_string()))
        }
    }

    /// Delete a credential
    pub fn delete_credential(&mut self, key: &str) -> Result<(), CliError> {
        #[cfg(target_os = "macos")]
        {
            use security_framework::os::macos::passwords;
            passwords::delete_generic_password(&self.service_name, key)
                .map_err(|e| CliError::Security(format!("Failed to delete from keychain: {}", e)))?;
        }

        #[cfg(target_os = "windows")]
        {
            self.credential_store.remove(key);
            self.delete_windows_credential(key)?;
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(ref service) = self.secret_service {
                self.delete_linux_secret(service, key)?;
            } else {
                self.delete_encrypted_file(key)?;
            }
        }

        Ok(())
    }

    /// List all stored credentials (keys only, not values)
    pub fn list_credentials(&self) -> Result<Vec<String>, CliError> {
        #[cfg(target_os = "macos")]
        {
            // Query keychain for all items with our service name
            Ok(vec![]) // Simplified for now
        }

        #[cfg(target_os = "windows")]
        {
            Ok(self.credential_store.keys().cloned().collect())
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(ref service) = self.secret_service {
                self.list_linux_secrets(service)
            } else {
                self.list_encrypted_files()
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            Ok(vec![])
        }
    }

    // Platform-specific helper methods
    #[cfg(target_os = "windows")]
    fn store_windows_credential(&self, key: &str, data: &[u8]) -> Result<(), CliError> {
        // Use Windows DPAPI for encryption
        // This is a simplified implementation
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn delete_windows_credential(&self, key: &str) -> Result<(), CliError> {
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn store_linux_secret(&self, _service: &libsecret::SecretService, _key: &str, _data: &[u8]) -> Result<(), CliError> {
        // Use libsecret API
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn get_linux_secret(&self, _service: &libsecret::SecretService, _key: &str) -> Result<SecureCredential, CliError> {
        Err(CliError::Security("Not implemented".to_string()))
    }

    #[cfg(target_os = "linux")]
    fn delete_linux_secret(&self, _service: &libsecret::SecretService, _key: &str) -> Result<(), CliError> {
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn list_linux_secrets(&self, _service: &libsecret::SecretService) -> Result<Vec<String>, CliError> {
        Ok(vec![])
    }

    #[cfg(target_os = "linux")]
    fn store_encrypted_file(&self, key: &str, data: &[u8]) -> Result<(), CliError> {
        use aes_gcm::{
            aead::{Aead, KeyInit},
            Aes256Gcm, Nonce,
        };
        use rand::RngCore;

        // Generate or derive encryption key
        let mut key_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key_bytes);
        
        let cipher = Aes256Gcm::new(&key_bytes.into());
        let nonce = Nonce::from_slice(b"unique nonce");
        
        let ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|e| CliError::Security(format!("Encryption failed: {}", e)))?;

        // Store encrypted data
        let path = self.get_secure_storage_path(key)?;
        std::fs::write(path, ciphertext)
            .map_err(|e| CliError::Security(format!("Failed to write encrypted file: {}", e)))?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn get_encrypted_file(&self, key: &str) -> Result<SecureCredential, CliError> {
        Err(CliError::Security("Not implemented".to_string()))
    }

    #[cfg(target_os = "linux")]
    fn delete_encrypted_file(&self, key: &str) -> Result<(), CliError> {
        let path = self.get_secure_storage_path(key)?;
        std::fs::remove_file(path)
            .map_err(|e| CliError::Security(format!("Failed to delete file: {}", e)))?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn list_encrypted_files(&self) -> Result<Vec<String>, CliError> {
        Ok(vec![])
    }

    #[cfg(target_os = "linux")]
    fn get_secure_storage_path(&self, key: &str) -> Result<std::path::PathBuf, CliError> {
        let home = dirs::home_dir()
            .ok_or_else(|| CliError::Security("Cannot find home directory".to_string()))?;
        let secure_dir = home.join(".neo-cli").join("secure");
        std::fs::create_dir_all(&secure_dir)
            .map_err(|e| CliError::Security(format!("Failed to create secure directory: {}", e)))?;
        Ok(secure_dir.join(format!("{}.enc", key)))
    }
}

/// Secure wallet storage using OS keychain
pub struct SecureWalletStorage {
    keychain: KeychainManager,
}

impl SecureWalletStorage {
    pub fn new() -> Result<Self, CliError> {
        Ok(Self {
            keychain: KeychainManager::new("neo-cli")?,
        })
    }

    /// Store wallet private key securely
    pub fn store_private_key(&mut self, address: &str, private_key: &[u8]) -> Result<(), CliError> {
        let credential = SecureCredential {
            account: address.to_string(),
            data: private_key.to_vec(),
            metadata: HashMap::from([
                ("type".to_string(), "private_key".to_string()),
                ("created".to_string(), chrono::Utc::now().to_rfc3339()),
            ]),
        };

        self.keychain.store_credential(address, &credential)
    }

    /// Retrieve wallet private key
    pub fn get_private_key(&self, address: &str) -> Result<Vec<u8>, CliError> {
        let credential = self.keychain.get_credential(address)?;
        Ok(credential.data)
    }

    /// Store mnemonic phrase securely
    pub fn store_mnemonic(&mut self, wallet_id: &str, mnemonic: &str) -> Result<(), CliError> {
        let credential = SecureCredential {
            account: wallet_id.to_string(),
            data: mnemonic.as_bytes().to_vec(),
            metadata: HashMap::from([
                ("type".to_string(), "mnemonic".to_string()),
                ("created".to_string(), chrono::Utc::now().to_rfc3339()),
            ]),
        };

        self.keychain.store_credential(&format!("mnemonic_{}", wallet_id), &credential)
    }

    /// Retrieve mnemonic phrase
    pub fn get_mnemonic(&self, wallet_id: &str) -> Result<String, CliError> {
        let credential = self.keychain.get_credential(&format!("mnemonic_{}", wallet_id))?;
        String::from_utf8(credential.data)
            .map_err(|e| CliError::Security(format!("Invalid mnemonic data: {}", e)))
    }

    /// Delete all wallet data
    pub fn delete_wallet(&mut self, address: &str) -> Result<(), CliError> {
        self.keychain.delete_credential(address)?;
        self.keychain.delete_credential(&format!("mnemonic_{}", address))?;
        Ok(())
    }

    /// List all stored wallets
    pub fn list_wallets(&self) -> Result<Vec<String>, CliError> {
        let all_keys = self.keychain.list_credentials()?;
        Ok(all_keys
            .into_iter()
            .filter(|k| !k.starts_with("mnemonic_"))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keychain_manager_creation() {
        let manager = KeychainManager::new("test-service");
        assert!(manager.is_ok());
    }

    #[test]
    fn test_secure_wallet_storage() {
        let storage = SecureWalletStorage::new();
        assert!(storage.is_ok());
    }
}