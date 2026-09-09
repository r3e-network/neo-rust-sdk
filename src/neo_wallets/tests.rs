//! # Session Key Tests

#![cfg(test)]

use crate::{
    neo_builder::transaction::CallFlags,
    neo_clients::MockProvider,
    neo_types::TxHash,
    neo_wallets::{
        session_key::{derive_session_key, CallFlagsWrapper, SessionKeyConfig, SessionSigner},
        WalletError,
    },
};
use primitive_types::H256;

fn create_mock_provider() -> MockProvider {
    MockProvider::new()
}

fn create_test_private_key() -> crate::crypto::Secp256r1PrivateKey {
    let pk_bytes = [42u8; 32];
    crate::crypto::Secp256r1PrivateKey::from_bytes(&pk_bytes).unwrap()
}

fn create_sample_tx_hash() -> TxHash {
    let mut tx_hash = [0u8; 32];
    tx_hash.copy_from_slice(&[1u8; 32]);
    H256(tx_hash)
}

#[tokio::test]
async fn test_session_key_expiry() {
    let parent_key = create_test_private_key();
    
    let config = SessionKeyConfig {
        expiry_block: 1000,
        permissions: CallFlagsWrapper::from(CallFlags::All),
        max_spend_limit: None,
    };
    
    let (pubkey, _proof) = derive_session_key(&parent_key, &config).unwrap();
    let mut signer = SessionSigner::new(pubkey, parent_key.clone(), config);
    
    signer.update_block(500);
    assert!(!signer.is_expired());
    signer.validate(&create_sample_tx_hash()).unwrap();
    
    signer.update_block(999);
    assert!(!signer.is_expired());
    
    // Block at expiry should still be VALID
    signer.update_block(1000);
    assert!(!signer.is_expired());
    signer.validate(&create_sample_tx_hash()).unwrap();
    println!("✓ Session valid at expiry block");
    
    // Block after expiry should fail validation
    signer.update_block(1001);
    assert!(signer.is_expired());
    let err = signer.validate(&create_sample_tx_hash()).expect_err("Should fail after expiry");
    assert!(matches!(err, WalletError::AccountState(_)));
    println!("✓ Session expiry correctly enforced at block 1001");
}

#[tokio::test]
async fn test_session_key_perms_restricted() {
    let parent_key = create_test_private_key();
    
    let config = SessionKeyConfig {
        expiry_block: u32::MAX,
        permissions: CallFlagsWrapper::from(CallFlags::ReadOnly),
        max_spend_limit: None,
    };
    
    let (pubkey, _proof) = derive_session_key(&parent_key, &config).unwrap();
    let signer = SessionSigner::new(pubkey, parent_key.clone(), config);
    
    assert!(signer.allows(CallFlags::ReadOnly));
    assert!(!signer.allows(CallFlags::WriteStates));
    assert!(!signer.allows(CallFlags::States));
    
    println!("✓ Permission restrictions correctly enforced");
}

#[tokio::test]
async fn test_session_key_rotation() {
    let parent_key = create_test_private_key();
    
    let config1 = SessionKeyConfig {
        expiry_block: 5000,
        permissions: CallFlagsWrapper::from(CallFlags::ReadOnly),
        max_spend_limit: Some(1_000_000_000),
    };
    
    let (pubkey1, _proof) = derive_session_key(&parent_key, &config1).unwrap();
    let signer = SessionSigner::new(pubkey1, parent_key.clone(), config1);
    
    let config2 = SessionKeyConfig {
        expiry_block: 10000,
        permissions: CallFlagsWrapper::from(CallFlags::All),
        max_spend_limit: None,
    };
    
    let rotated = signer.rotate(config2.clone()).unwrap();
    
    assert_ne!(signer.pubkey, rotated.pubkey);
    assert_eq!(rotated.config.expiry_block, 10000);
    assert!(rotated.config.max_spend_limit.is_none());
    
    println!("✓ Session rotation works correctly");
}

#[tokio::test]
async fn test_session_key_spend_limit() {
    let parent_key = create_test_private_key();
    
    let config = SessionKeyConfig {
        expiry_block: u32::MAX,
        permissions: CallFlagsWrapper::from(CallFlags::All),
        max_spend_limit: Some(1_000_000_000),
    };
    
    let (pubkey, _proof) = derive_session_key(&parent_key, &config).unwrap();
    let signer = SessionSigner::new(pubkey, parent_key, config);
    
    assert!(signer.allows_spend(500_000_000));
    assert!(signer.allows_spend(1_000_000_000));
    assert!(!signer.allows_spend(1_000_000_001));
    assert!(!signer.allows_spend(5_000_000_000));
    
    // Test validate_tx_amount enforcement
    let tx_hash = create_sample_tx_hash();
    assert!(signer.validate_tx_amount(&tx_hash, 500_000_000).is_ok());
    assert!(signer.validate_tx_amount(&tx_hash, 1_000_000_000).is_ok());
    assert!(signer.validate_tx_amount(&tx_hash, 1_000_000_001).is_err());
    
    println!("✓ Spend limit correctly enforced via both allows_spend and validate_tx_amount");
}

#[tokio::test]
async fn test_verify_session_proof() {
    let parent_key = create_test_private_key();
    let config = SessionKeyConfig {
        expiry_block: u32::MAX,
        permissions: CallFlagsWrapper::from(CallFlags::All),
        max_spend_limit: None,
    };
    
    let (pubkey, proof) = derive_session_key(&parent_key, &config).unwrap();
    
    let tx_hash = create_sample_tx_hash();
    let result = crate::neo_wallets::verify_session_proof(&tx_hash, &pubkey, &proof);
    
    assert!(result.is_ok());
    println!("✓ Session proof verification interface works");
}

#[tokio::test]
async fn test_session_signer_address_generation() {
    let parent_key = create_test_private_key();
    let config = SessionKeyConfig {
        expiry_block: u32::MAX,
        permissions: CallFlagsWrapper::from(CallFlags::All),
        max_spend_limit: None,
    };
    
    let (pubkey, _proof) = derive_session_key(&parent_key, &config).unwrap();
    let signer = SessionSigner::new(pubkey, parent_key, config);
    
    let address = signer.address();
    
    assert!(address.starts_with('A') || address.starts_with('N'));
    assert!(address.len() > 20);
    
    println!("✓ Session address generation works");
}

#[tokio::test]
async fn test_session_config_defaults() {
    let default_config = SessionKeyConfig::default();
    assert_eq!(default_config.expiry_block, u32::MAX);
    
    let readonly = SessionKeyConfig::readonly(5000);
    assert_eq!(readonly.expiry_block, 5000);
    
    let transfer = SessionKeyConfig::transfer_only(10000, 5_000_000_000);
    assert_eq!(transfer.expiry_block, 10000);
    assert_eq!(transfer.max_spend_limit, Some(5_000_000_000));
    
    println!("✓ Session configuration factories work");
}

#[tokio::test]
async fn test_session_key_workflow_integration() {
    let _rpc_client = create_mock_provider();
    
    let parent_key = create_test_private_key();
    let session_config = SessionKeyConfig {
        expiry_block: 1000000,
        permissions: CallFlagsWrapper::from(CallFlags::ReadOnly),
        max_spend_limit: Some(100_000_000),
    };
    
    let (session_pubkey, _proof_data) = derive_session_key(&parent_key, &session_config).unwrap();
    
    let signer = SessionSigner::new(session_pubkey, parent_key, session_config)
        .with_current_block(500000);
    
    let tx_hash = create_sample_tx_hash();
    assert!(!signer.is_expired());
    assert!(signer.allows(CallFlags::ReadStates));
    assert!(signer.allows_spend(50_000_000));
    
    signer.with_current_block(1000001);
    assert!(signer.is_expired());
    
    println!("✓ Session workflow integration test passed");
}

#[tokio::test]
async fn test_derive_session_key_uniqueness() {
    let parent_key = create_test_private_key();
    let config = SessionKeyConfig {
        expiry_block: 10000,
        permissions: CallFlagsWrapper::from(CallFlags::All),
        max_spend_limit: Some(1_000_000_000),
    };
    
    let (pubkey1, proof1) = derive_session_key(&parent_key, &config).unwrap();
    let (pubkey2, proof2) = derive_session_key(&parent_key, &config).unwrap();
    
    assert_eq!(pubkey1, pubkey2);
    assert_eq!(proof1, proof2);
    
    let config2 = SessionKeyConfig {
        expiry_block: 20000,
        permissions: CallFlagsWrapper::from(CallFlags::All),
        max_spend_limit: Some(2_000_000_000),
    };
    
    let (pubkey3, _proof3) = derive_session_key(&parent_key, &config2).unwrap();
    assert_ne!(pubkey1, pubkey3);
    
    println!("✓ Session key derivation is deterministic");
}

mod crypto_tests {
    use super::*;
    
    #[test]
    fn test_call_flags_wrapper_conversion() {
        let wrapper = CallFlagsWrapper::from(CallFlags::ReadOnly);
        assert_eq!(wrapper.value(), 0b00000101);
        assert!(wrapper.allows_read_states());
        assert!(wrapper.allows_calls());
        assert!(!wrapper.allows_write_states());
        
        let all_wrapper = CallFlagsWrapper::from(CallFlags::All);
        assert_eq!(all_wrapper.value(), 0b00001111);
        
        println!("✓ CallFlagsWrapper conversion correct");
    }
}
