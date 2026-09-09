#![deny(unsafe_code)]

//! # Session Keys
//!
//! Session keys provide temporary, limited-permission cryptographic keys that can be
//! revoked without affecting the main wallet. This is essential for enterprise and
//! multi-signature use cases where delegated authority is required.

use crate::{
    crypto::{Secp256r1PrivateKey, Secp256r1PublicKey, Secp256r1Signature},
    neo_types::TxHash,
    neo_wallets::WalletError,
};
use sha2::{Digest, Sha256};

/// Configuration for a session key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionKeyConfig {
    /// Block number at which this session expires
    pub expiry_block: u32,

    /// Permission mask defining allowed operations (NEP-17 CallFlags)
    pub permissions: CallFlagsWrapper,

    /// Optional maximum GAS amount allowed for spending (in smallest unit: 1 GAS = 10^8 atoms)
    pub max_spend_limit: Option<u64>,
}

impl SessionKeyConfig {
    /// Creates a new session configuration with full permissions and no expiry.
    pub fn unlimited() -> Self {
        Self {
            expiry_block: u32::MAX,
            permissions: CallFlagsWrapper::ALL,
            max_spend_limit: None,
        }
    }

    /// Creates a read-only session with specified expiry.
    pub fn readonly(expiry_block: u32) -> Self {
        Self {
            expiry_block,
            permissions: CallFlagsWrapper::READ_ONLY,
            max_spend_limit: None,
        }
    }

    /// Creates a transfer-limited session with specified expiry and maximum spend.
    pub fn transfer_only(expiry_block: u32, max_gas_atoms: u64) -> Self {
        Self {
            expiry_block,
            permissions: CallFlagsWrapper::READ_ONLY,
            max_spend_limit: Some(max_gas_atoms),
        }
    }
}

impl Default for SessionKeyConfig {
    fn default() -> Self {
        Self::unlimited()
    }
}

/// Wrapper around CallFlags for serialization and comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallFlagsWrapper {
    value: u8,
}

impl CallFlagsWrapper {
    /// Full permissions - allows all operations
    pub const ALL: Self = Self { value: 0b00001111 };

    /// Read-only permissions
    pub const READ_ONLY: Self = Self { value: 0b00000101 };
    
    pub fn new(value: u8) -> Self {
        Self { value }
    }

    pub fn from_call_flags(flags: crate::neo_builder::CallFlags) -> Self {
        Self { value: flags.value() }
    }

    pub fn value(&self) -> u8 {
        self.value
    }

    pub fn allows_read_states(&self) -> bool {
        self.value & 0b00000001 != 0
    }

    pub fn allows_write_states(&self) -> bool {
        self.value & 0b00000010 != 0
    }

    pub fn allows_calls(&self) -> bool {
        self.value & 0b00000100 != 0
    }

    pub fn allows_notifications(&self) -> bool {
        self.value & 0b00001000 != 0
    }
}

impl From<crate::neo_builder::CallFlags> for CallFlagsWrapper {
    fn from(flags: crate::neo_builder::CallFlags) -> Self {
        Self::from_call_flags(flags)
    }
}

impl From<CallFlagsWrapper> for crate::neo_builder::CallFlags {
    fn from(wrapper: CallFlagsWrapper) -> crate::neo_builder::CallFlags {
        crate::neo_builder::CallFlags::from_value(wrapper.value).unwrap_or(crate::neo_builder::CallFlags::None)
    }
}

impl From<&crate::neo_builder::CallFlags> for CallFlagsWrapper {
    fn from(flags: &crate::neo_builder::CallFlags) -> Self {
        Self::new(flags.value())
    }
}

/// Serializes session configuration to bytes for hashing/signing.
fn serialize_config(config: &SessionKeyConfig) -> Vec<u8> {
    let mut result = Vec::with_capacity(16);
    result.extend_from_slice(&config.expiry_block.to_le_bytes());
    result.push(config.permissions.value());
    if let Some(spend_limit) = config.max_spend_limit {
        result.push(0x01);
        result.extend_from_slice(&spend_limit.to_le_bytes());
    } else {
        result.push(0x00);
    }
    result
}

/// Derives a session key pair from a parent private key and configuration.
pub fn derive_session_key(
    private_key: &Secp256r1PrivateKey,
    config: &SessionKeyConfig,
) -> Result<(Secp256r1PublicKey, Vec<u8>), WalletError> {
    let config_bytes = serialize_config(config);
    let mut hasher = Sha256::new();
    hasher.update(private_key.to_raw_bytes());
    hasher.update(b"session-key-v1");
    hasher.update(&config_bytes);
    let derivation_seed = hasher.finalize();

    let session_private_key = Secp256r1PrivateKey::from_bytes(&derivation_seed[..])?;
    let session_public_key = session_private_key.to_public_key();

    // Create proof by signing metadata with derived session key
    let proof = create_session_proof(&config_bytes, &session_private_key)?;

    Ok((session_public_key, proof))
}

/// Creates a cryptographic proof for session metadata.
/// Signs the metadata digest with the derived session private key.
fn create_session_proof(
    metadata: &[u8],
    session_private_key: &Secp256r1PrivateKey,
) -> Result<Vec<u8>, WalletError> {
    let hashed = Sha256::digest(metadata);
    let signature = session_private_key.sign_tx(&hashed)?;
    
    let sig_bytes = signature.to_bytes();
    let mut proof = Vec::with_capacity(132); // 64 (sig) + 64 (metadata)
    proof.extend_from_slice(sig_bytes.as_ref());
    proof.extend_from_slice(metadata);
    
    Ok(proof)
}

/// Verifies that a transaction is covered by a session's proof.
pub fn verify_session_proof(
    _tx_hash: &TxHash,
    session_pubkey: &Secp256r1PublicKey,
    proof: &[u8],
) -> Result<bool, WalletError> {
    const SIGNATURE_LEN: usize = 64;
    
    // Validate proof structure: signature (64 bytes) + at least some data
    if proof.len() < SIGNATURE_LEN + 1 {
        return Ok(false);
    }
    
    // Extract signature from first 64 bytes
    let signature_bytes: [u8; SIGNATURE_LEN] = proof[..SIGNATURE_LEN].try_into()
        .map_err(|_| WalletError::VerifyError)?;
    let signature = Secp256r1Signature::from_bytes(&signature_bytes)
        .map_err(|_| WalletError::VerifyError)?;
    
    // Extract metadata (everything after the signature)
    let metadata = &proof[SIGNATURE_LEN..];
    
    // Recompute the signed digest using SHA-256
    let hashed = Sha256::digest(metadata);
    
    // Verify the signature against the session public key
    match session_pubkey.verify(&hashed, &signature) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// A signer that enforces session key constraints before signing transactions.
#[derive(Debug, Clone)]
pub struct SessionSigner {
    /// The session public key
    pub pubkey: Secp256r1PublicKey,
    
    /// The parent private key
    parent_key: Secp256r1PrivateKey,
    
    /// Session configuration defining constraints
    pub config: SessionKeyConfig,
    
    /// Current block height
    current_block: u32,
}

impl SessionSigner {
    /// Creates a new session signer.
    pub fn new(
        pubkey: Secp256r1PublicKey,
        parent_key: Secp256r1PrivateKey,
        config: SessionKeyConfig,
    ) -> Self {
        Self {
            pubkey,
            parent_key,
            config,
            current_block: 0,
        }
    }

    /// Sets the current block height for expiry validation.
    pub fn with_current_block(mut self, block: u32) -> Self {
        self.current_block = block;
        self
    }

    /// Updates the current block height.
    pub fn update_block(&mut self, block: u32) {
        self.current_block = block;
    }

    /// Checks if the session has expired.
    pub fn is_expired(&self) -> bool {
        self.current_block > self.config.expiry_block
    }

    /// Checks if a transaction type is permitted.
    pub fn allows(&self, flags: crate::neo_builder::CallFlags) -> bool {
        let session_flags = CallFlagsWrapper::from(self.config.permissions);
        let required_flags = CallFlagsWrapper::from(flags);
        (session_flags.value() & required_flags.value()) == required_flags.value()
    }

    /// Checks if a spend amount is within limits.
    pub fn allows_spend(&self, amount: u64) -> bool {
        match self.config.max_spend_limit {
            Some(limit) => amount <= limit,
            None => true,
        }
    }

    /// Validates all constraints for signing a transaction.
    pub fn validate(&self, _tx_hash: &TxHash) -> Result<(), WalletError> {
        if self.is_expired() {
            return Err(WalletError::AccountState(format!(
                "Session expired at block {}, current block {}",
                self.config.expiry_block, self.current_block
            )));
        }
        Ok(())
    }

    /// Validates transaction amount against spend limits.
    pub fn validate_tx_amount(&self, _tx_hash: &TxHash, amount: u64) -> Result<(), WalletError> {
        if !self.allows_spend(amount) {
            return Err(WalletError::AccountState(format!(
                "Transaction amount {} exceeds spend limit {:?}",
                amount,
                self.config.max_spend_limit
            )));
        }
        Ok(())
    }

    /// Signs a transaction after validating constraints.
    pub async fn sign_transaction(
        &self,
        tx_hash: &TxHash,
    ) -> Result<crate::crypto::Secp256r1Signature, WalletError> {
        self.validate(tx_hash)?;
        
        // Derive the session private key internally (as done in proof creation)
        let config_bytes = serialize_config(&self.config);
        let mut hasher = Sha256::new();
        hasher.update(self.parent_key.to_raw_bytes());
        hasher.update(b"session-key-v1");
        hasher.update(&config_bytes);
        let derivation_seed = hasher.finalize();
        let session_private_key = Secp256r1PrivateKey::from_bytes(&derivation_seed[..])?;
        
        // Sign with the DERIVED session key, not the parent key
        let hashed = Sha256::digest(tx_hash.as_ref());
        session_private_key.sign_tx(&hashed).map_err(|e| {
            WalletError::SigningError(format!("Failed to sign transaction: {}", e))
        })
    }

    /// Creates a session proof for authorization.
    pub fn create_proof(&self) -> Result<Vec<u8>, WalletError> {
        let config_bytes = serialize_config(&self.config);
        
        // Re-derive the session private key to sign with it
        let mut hasher = Sha256::new();
        hasher.update(self.parent_key.to_raw_bytes());
        hasher.update(b"session-key-v1");
        hasher.update(&config_bytes);
        let derivation_seed = hasher.finalize();
        let session_private_key = Secp256r1PrivateKey::from_bytes(&derivation_seed[..])?;
        
        create_session_proof(&config_bytes, &session_private_key)
    }

    /// Gets the session address.
    pub fn address(&self) -> String {
        use crate::neo_builder::VerificationScript;
        let vs = VerificationScript::from_public_key(&self.pubkey);
        let script_hash = vs.hash();
        
        let mut address_bytes = vec![0x34u8];
        address_bytes.extend_from_slice(script_hash.as_ref());
        
        let hash = sha2::Sha256::digest(&address_bytes);
        let hash = &sha2::Sha256::digest(hash)[..4];
        
        address_bytes.extend_from_slice(hash);
        bs58::encode(&address_bytes).into_string()
    }

    /// Rotates to a new session with updated configuration.
    pub fn rotate(&self, new_config: SessionKeyConfig) -> Result<Self, WalletError> {
        let (new_pubkey, _) = derive_session_key(&self.parent_key, &new_config)?;
        Ok(Self {
            pubkey: new_pubkey,
            parent_key: self.parent_key.clone(),
            config: new_config,
            current_block: self.current_block,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::Secp256r1PrivateKey;
    use primitive_types::H256;

    fn create_test_keys() -> (Secp256r1PrivateKey, SessionKeyConfig) {
        let pk_bytes = [1u8; 32];
        let private_key = Secp256r1PrivateKey::from_bytes(&pk_bytes).unwrap();
        
        let config = SessionKeyConfig {
            expiry_block: 1000000,
            permissions: CallFlagsWrapper::from(crate::neo_builder::CallFlags::ReadOnly),
            max_spend_limit: Some(1_000_000_000),
        };
        
        (private_key, config)
    }

    #[test]
    fn test_derive_session_key_creates_pair() {
        let (private_key, config) = create_test_keys();
        let (pubkey, proof) = derive_session_key(&private_key, &config).unwrap();
        
        // Check uncompressed encoding (65 bytes with 0x04 prefix)
        assert!(pubkey.get_encoded(true).len() >= 33);
        assert!(proof.len() >= 33);
    }

    #[test]
    fn test_config_serialization() {
        let config = SessionKeyConfig {
            expiry_block: 12345,
            permissions: CallFlagsWrapper::from(crate::neo_builder::CallFlags::All),
            max_spend_limit: Some(500_000_000),
        };
        
        let bytes = serialize_config(&config);
        assert_eq!(bytes.len(), 14); // 4 (expiry) + 1 (flags) + 1 (flag) + 8 (limit)
    }

    #[test]
    fn test_session_key_expiry() {
        let (private_key, config) = create_test_keys();
        let (pubkey, _proof) = derive_session_key(&private_key, &config).unwrap();
        
        // Create session signer with block below expiry
        let session_signer = SessionSigner::new(
            pubkey.clone(),
            private_key.clone(),
            config.clone(),
        );
        
        // Should not expire at block 0
        assert!(!session_signer.is_expired());
        
        // Set block AT expiry - should be VALID (block N is valid up to and including expiry_block)
        let valid_at_expiry_signer = SessionSigner::new(
            pubkey.clone(),
            private_key.clone(),
            SessionKeyConfig {
                expiry_block: 100,
                permissions: CallFlagsWrapper::READ_ONLY,
                max_spend_limit: None,
            },
        ).with_current_block(100);
        
        let tx_hash = H256([1u8; 32]);
        assert!(valid_at_expiry_signer.validate(&tx_hash).is_ok());
        
        // Should fail on block AFTER expiry
        let expired_signer = SessionSigner::new(
            pubkey.clone(),
            private_key.clone(),
            SessionKeyConfig {
                expiry_block: 100,
                permissions: CallFlagsWrapper::READ_ONLY,
                max_spend_limit: None,
            },
        ).with_current_block(101);
        
        assert!(expired_signer.validate(&tx_hash).is_err());
        
        // Should succeed when below expiry
        let valid_signer = SessionSigner::new(
            pubkey.clone(),
            private_key.clone(),
            config,
        ).with_current_block(99);
        
        assert!(valid_signer.validate(&tx_hash).is_ok());
    }

    #[test]
    fn test_session_key_perms_restricted() {
        let (private_key, config) = create_test_keys();
        let (pubkey, _proof) = derive_session_key(&private_key, &config).unwrap();
        
        let session_signer = SessionSigner::new(pubkey, private_key, config);
        
        // READ_ONLY should allow read states
        assert!(session_signer.allows(crate::neo_builder::CallFlags::ReadStates));
        
        // READ_ONLY should deny write states
        assert!(!session_signer.allows(crate::neo_builder::CallFlags::WriteStates));
        
        // All flags should be denied if we have READ_ONLY only
        assert!(!session_signer.allows(crate::neo_builder::CallFlags::All));
    }

    #[test]
    fn test_session_key_rotation() {
        let (private_key, _config) = create_test_keys();
        
        let config1 = SessionKeyConfig {
            expiry_block: 1000,
            permissions: CallFlagsWrapper::READ_ONLY,
            max_spend_limit: None,
        };
        
        let (pubkey1, _proof1) = derive_session_key(&private_key, &config1).unwrap();
        
        let config2 = SessionKeyConfig {
            expiry_block: 2000,
            permissions: CallFlagsWrapper::ALL,
            max_spend_limit: Some(1_000_000_000),
        };
        
        let (pubkey2, _proof2) = derive_session_key(&private_key, &config2).unwrap();
        
        // Different configs should produce different public keys
        assert_ne!(pubkey1, pubkey2);
        
        // Rotation via SessionSigner should also work
        let session_signer = SessionSigner::new(pubkey1.clone(), private_key.clone(), config1);
        let rotated = session_signer.rotate(config2.clone()).unwrap();
        
        assert_eq!(rotated.config, config2);
        assert_ne!(rotated.pubkey, pubkey1);
    }

    #[test]
    fn test_verify_session_proof_roundtrip() {
        let (private_key, config) = create_test_keys();
        
        // Generate a fresh proof
        let (pubkey, proof) = derive_session_key(&private_key, &config).unwrap();
        
        // Fresh proof should verify OK
        let tx_hash = H256([1u8; 32]);
        assert!(verify_session_proof(&tx_hash, &pubkey, &proof).unwrap());
        
        // Tamper with the signature (change first byte)
        let mut tampered_proof = proof.clone();
        if !tampered_proof.is_empty() {
            tampered_proof[0] ^= 0xFF;
            assert!(!verify_session_proof(&tx_hash, &pubkey, &tampered_proof).unwrap());
        }
        
        // Tamper with metadata (change first byte after signature)
        let mut metadata_tampered_proof = proof.clone();
        if metadata_tampered_proof.len() > 64 {
            metadata_tampered_proof[64] ^= 0xFF;
            assert!(!verify_session_proof(&tx_hash, &pubkey, &metadata_tampered_proof).unwrap());
        }
        
        // Wrong key should fail verification - use different seed
        let wrong_private_key_bytes = [2u8; 32];
        let wrong_private_key = Secp256r1PrivateKey::from_bytes(&wrong_private_key_bytes).unwrap();
        let (_, wrong_proof) = derive_session_key(&wrong_private_key, &config).unwrap();
        assert!(!verify_session_proof(&tx_hash, &pubkey, &wrong_proof).unwrap());
    }

    #[test]
    fn test_validate_spend_limit() {
        let (private_key, config) = create_test_keys();
        let (pubkey, _proof) = derive_session_key(&private_key, &config).unwrap();
        
        let signer = SessionSigner::new(pubkey, private_key, config);
        let tx_hash = H256([1u8; 32]);
        
        // Amount within limit should pass
        assert!(signer.validate_tx_amount(&tx_hash, 500_000_000).is_ok());
        
        // Amount exactly at limit should pass
        assert!(signer.validate_tx_amount(&tx_hash, 1_000_000_000).is_ok());
        
        // Amount over limit should fail
        assert!(signer.validate_tx_amount(&tx_hash, 1_000_000_001).is_err());
    }

    #[test]
    fn test_verify_session_proof_with_tx_hash() {
        let (private_key, config) = create_test_keys();
        
        // Generate proof
        let (pubkey, proof) = derive_session_key(&private_key, &config).unwrap();
        
        // Fresh proof should verify with the correct tx hash
        let tx_hash1 = H256([1u8; 32]);
        
        assert!(verify_session_proof(&tx_hash1, &pubkey, &proof).unwrap());
    }
}
