// SGX-Compatible Cryptographic Operations

#![cfg_attr(feature = "sgx", no_std)]

#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;

#[cfg(feature = "sgx")]
use sgx_tcrypto::*;
#[cfg(feature = "sgx")]
use sgx_types::*;

use super::SgxError;

#[cfg(not(feature = "sgx"))]
use sha2::{Sha256, Digest};
#[cfg(not(feature = "sgx"))]
use k256::ecdsa::{SigningKey, VerifyingKey, Signature};

/// SGX-compatible cryptographic operations
pub struct SgxCrypto {
    #[cfg(feature = "sgx")]
    context: SgxCryptoContext,
}

#[cfg(feature = "sgx")]
struct SgxCryptoContext {
    sealed_keys: Vec<SgxSealedKey>,
}

#[cfg(feature = "sgx")]
struct SgxSealedKey {
    key_id: [u8; 16],
    sealed_data: Vec<u8>,
}

impl SgxCrypto {
    /// Create a new SGX crypto instance
    pub fn new() -> Result<Self, SgxError> {
        #[cfg(feature = "sgx")]
        {
            Ok(Self {
                context: SgxCryptoContext {
                    sealed_keys: Vec::new(),
                },
            })
        }
        
        #[cfg(not(feature = "sgx"))]
        {
            Ok(Self {})
        }
    }
    
    /// SHA256 hash function (SGX-compatible)
    pub fn sha256(&self, data: &[u8]) -> Result<[u8; 32], SgxError> {
        #[cfg(feature = "sgx")]
        {
            let mut hash = [0u8; 32];
            let result = unsafe {
                sgx_sha256_msg(
                    data.as_ptr(),
                    data.len() as u32,
                    &mut hash as *mut _ as *mut sgx_sha256_hash_t,
                )
            };
            
            if result != sgx_status_t::SGX_SUCCESS {
                return Err(SgxError::CryptoError("SHA256 computation failed".into()));
            }
            
            Ok(hash)
        }
        
        #[cfg(not(feature = "sgx"))]
        {
            let mut hasher = Sha256::new();
            hasher.update(data);
            let result = hasher.finalize();
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&result);
            Ok(hash)
        }
    }
    
    /// ECDSA signing (SGX-compatible)
    pub fn sign_ecdsa(&self, message: &[u8], private_key: &[u8; 32]) -> Result<Vec<u8>, SgxError> {
        #[cfg(feature = "sgx")]
        {
            let mut signature = vec![0u8; 64];
            
            // Hash the message first
            let hash = self.sha256(message)?;
            
            // Sign using SGX crypto
            let result = unsafe {
                let ecc_handle = sgx_ecc256_open_context();
                if ecc_handle.is_null() {
                    return Err(SgxError::CryptoError("Failed to open ECC context".into()));
                }
                
                let mut ecc_signature = sgx_ec256_signature_t::default();
                let private = sgx_ec256_private_t { r: *private_key };
                
                let status = sgx_ecdsa_sign(
                    &hash as *const _ as *const u8,
                    32,
                    &private as *const _,
                    &mut ecc_signature as *mut _,
                    ecc_handle,
                );
                
                sgx_ecc256_close_context(ecc_handle);
                
                if status != sgx_status_t::SGX_SUCCESS {
                    return Err(SgxError::CryptoError("ECDSA signing failed".into()));
                }
                
                // Convert signature to bytes
                signature[..32].copy_from_slice(&ecc_signature.x);
                signature[32..].copy_from_slice(&ecc_signature.y);
                
                status
            };
            
            if result != sgx_status_t::SGX_SUCCESS {
                return Err(SgxError::CryptoError("ECDSA signing failed".into()));
            }
            
            Ok(signature)
        }
        
        #[cfg(not(feature = "sgx"))]
        {
            use k256::ecdsa::signature::Signer;
            let signing_key = SigningKey::from_bytes(private_key.into())
                .map_err(|e| SgxError::CryptoError(format!("Invalid private key: {}", e)))?;
            let signature: Signature = signing_key.sign(message);
            Ok(signature.to_bytes().to_vec())
        }
    }
    
    /// ECDSA verification (SGX-compatible)
    pub fn verify_ecdsa(&self, message: &[u8], signature: &[u8], public_key: &[u8]) -> Result<bool, SgxError> {
        #[cfg(feature = "sgx")]
        {
            // Hash the message first
            let hash = self.sha256(message)?;
            
            let result = unsafe {
                let ecc_handle = sgx_ecc256_open_context();
                if ecc_handle.is_null() {
                    return Err(SgxError::CryptoError("Failed to open ECC context".into()));
                }
                
                let mut ecc_signature = sgx_ec256_signature_t::default();
                ecc_signature.x.copy_from_slice(&signature[..32]);
                ecc_signature.y.copy_from_slice(&signature[32..64]);
                
                let mut public = sgx_ec256_public_t::default();
                public.gx.copy_from_slice(&public_key[..32]);
                public.gy.copy_from_slice(&public_key[32..64]);
                
                let mut valid: u8 = 0;
                let status = sgx_ecdsa_verify(
                    &hash as *const _ as *const u8,
                    32,
                    &public as *const _,
                    &ecc_signature as *const _,
                    &mut valid as *mut _,
                    ecc_handle,
                );
                
                sgx_ecc256_close_context(ecc_handle);
                
                if status != sgx_status_t::SGX_SUCCESS {
                    return Err(SgxError::CryptoError("ECDSA verification failed".into()));
                }
                
                Ok(valid == 1)
            };
            
            result
        }
        
        #[cfg(not(feature = "sgx"))]
        {
            use k256::ecdsa::signature::Verifier;
            let verifying_key = VerifyingKey::from_sec1_bytes(public_key)
                .map_err(|e| SgxError::CryptoError(format!("Invalid public key: {}", e)))?;
            let signature = Signature::from_slice(signature)
                .map_err(|e| SgxError::CryptoError(format!("Invalid signature: {}", e)))?;
            Ok(verifying_key.verify(message, &signature).is_ok())
        }
    }
    
    /// Generate random bytes (SGX-compatible)
    pub fn random_bytes(&self, size: usize) -> Result<Vec<u8>, SgxError> {
        let mut buffer = vec![0u8; size];
        
        #[cfg(feature = "sgx")]
        {
            let result = unsafe {
                sgx_read_rand(buffer.as_mut_ptr(), size)
            };
            
            if result != sgx_status_t::SGX_SUCCESS {
                return Err(SgxError::CryptoError("Random generation failed".into()));
            }
        }
        
        #[cfg(not(feature = "sgx"))]
        {
            use rand::RngCore;
            rand::thread_rng().fill_bytes(&mut buffer);
        }
        
        Ok(buffer)
    }
}

/// SGX Key Manager for sealed key storage
pub struct SgxKeyManager {
    #[cfg(feature = "sgx")]
    sealed_keys: Vec<SealedKey>,
}

#[cfg(feature = "sgx")]
struct SealedKey {
    key_id: [u8; 16],
    sealed_data: Vec<u8>,
    key_policy: sgx_attributes_t,
}

impl SgxKeyManager {
    /// Create a new key manager
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "sgx")]
            sealed_keys: Vec::new(),
        }
    }
    
    /// Seal a key for secure storage
    #[cfg(feature = "sgx")]
    pub fn seal_key(&mut self, key_id: &[u8; 16], key_data: &[u8]) -> Result<(), SgxError> {
        let sealed_size = unsafe {
            sgx_calc_sealed_data_size(0, key_data.len() as u32)
        };
        
        if sealed_size == u32::MAX {
            return Err(SgxError::CryptoError("Failed to calculate sealed size".into()));
        }
        
        let mut sealed_data = vec![0u8; sealed_size as usize];
        
        let result = unsafe {
            sgx_seal_data(
                0,
                core::ptr::null(),
                key_data.len() as u32,
                key_data.as_ptr(),
                sealed_size,
                sealed_data.as_mut_ptr() as *mut sgx_sealed_data_t,
            )
        };
        
        if result != sgx_status_t::SGX_SUCCESS {
            return Err(SgxError::CryptoError("Failed to seal key".into()));
        }
        
        self.sealed_keys.push(SealedKey {
            key_id: *key_id,
            sealed_data,
            key_policy: sgx_attributes_t::default(),
        });
        
        Ok(())
    }
    
    /// Unseal a previously sealed key
    #[cfg(feature = "sgx")]
    pub fn unseal_key(&self, key_id: &[u8; 16]) -> Result<Vec<u8>, SgxError> {
        let sealed_key = self.sealed_keys
            .iter()
            .find(|k| k.key_id == *key_id)
            .ok_or_else(|| SgxError::CryptoError("Key not found".into()))?;
        
        let unsealed_size = unsafe {
            sgx_get_encrypt_txt_len(sealed_key.sealed_data.as_ptr() as *const sgx_sealed_data_t)
        };
        
        let mut unsealed_data = vec![0u8; unsealed_size as usize];
        let mut mac_len = 0u32;
        
        let result = unsafe {
            sgx_unseal_data(
                sealed_key.sealed_data.as_ptr() as *const sgx_sealed_data_t,
                core::ptr::null_mut(),
                &mut mac_len,
                unsealed_data.as_mut_ptr(),
                &mut (unsealed_size as u32),
            )
        };
        
        if result != sgx_status_t::SGX_SUCCESS {
            return Err(SgxError::CryptoError("Failed to unseal key".into()));
        }
        
        Ok(unsealed_data)
    }
    
    #[cfg(not(feature = "sgx"))]
    pub fn seal_key(&mut self, _key_id: &[u8; 16], _key_data: &[u8]) -> Result<(), SgxError> {
        // No-op for non-SGX builds
        Ok(())
    }
    
    #[cfg(not(feature = "sgx"))]
    pub fn unseal_key(&self, _key_id: &[u8; 16]) -> Result<Vec<u8>, SgxError> {
        Err(SgxError::CryptoError("SGX not enabled".into()))
    }
}

/// Initialize crypto subsystem
pub fn init_crypto() -> Result<(), SgxError> {
    // Perform any necessary crypto initialization
    Ok(())
}