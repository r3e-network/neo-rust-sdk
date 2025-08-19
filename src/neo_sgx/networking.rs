// SGX-Safe Networking Layer

#![cfg_attr(feature = "sgx", no_std)]

#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;

#[cfg(feature = "sgx")]
use sgx_types::*;

use super::SgxError;

/// SGX-safe networking interface
pub struct SgxNetworking {
    #[cfg(feature = "sgx")]
    secure_channels: Vec<SecureChannel>,
}

/// Secure channel for encrypted communication
pub struct SecureChannel {
    channel_id: [u8; 16],
    remote_public_key: Option<[u8; 64]>,
    session_key: Option<[u8; 32]>,
    nonce: u64,
}

impl SgxNetworking {
    /// Create a new networking instance
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "sgx")]
            secure_channels: Vec::new(),
        }
    }
    
    /// Establish a secure channel with remote party
    #[cfg(feature = "sgx")]
    pub fn establish_channel(&mut self, remote_id: &[u8; 16]) -> Result<SecureChannel, SgxError> {
        use super::crypto::SgxCrypto;
        
        let crypto = SgxCrypto::new()?;
        
        // Generate ephemeral key pair for DH
        let private_key = crypto.random_bytes(32)?;
        let mut private_key_array = [0u8; 32];
        private_key_array.copy_from_slice(&private_key);
        
        // Create channel
        let channel = SecureChannel {
            channel_id: *remote_id,
            remote_public_key: None,
            session_key: None,
            nonce: 0,
        };
        
        self.secure_channels.push(channel.clone());
        Ok(channel)
    }
    
    #[cfg(not(feature = "sgx"))]
    pub fn establish_channel(&mut self, remote_id: &[u8; 16]) -> Result<SecureChannel, SgxError> {
        Ok(SecureChannel {
            channel_id: *remote_id,
            remote_public_key: None,
            session_key: None,
            nonce: 0,
        })
    }
    
    /// Send data through secure channel
    pub fn send_secure(&mut self, channel: &mut SecureChannel, data: &[u8]) -> Result<Vec<u8>, SgxError> {
        // Encrypt data with session key
        let encrypted = self.encrypt_message(channel, data)?;
        
        // Increment nonce
        channel.nonce += 1;
        
        Ok(encrypted)
    }
    
    /// Receive data through secure channel
    pub fn receive_secure(&mut self, channel: &mut SecureChannel, encrypted_data: &[u8]) -> Result<Vec<u8>, SgxError> {
        // Decrypt data with session key
        let decrypted = self.decrypt_message(channel, encrypted_data)?;
        
        // Increment nonce
        channel.nonce += 1;
        
        Ok(decrypted)
    }
    
    /// Encrypt message for secure channel
    fn encrypt_message(&self, channel: &SecureChannel, data: &[u8]) -> Result<Vec<u8>, SgxError> {
        let session_key = channel.session_key
            .ok_or_else(|| SgxError::NetworkError("No session key established".into()))?;
        
        #[cfg(feature = "sgx")]
        {
            use sgx_tcrypto::*;
            
            let mut encrypted = vec![0u8; data.len() + 16]; // Add space for MAC
            let mut mac = sgx_aes_gcm_128bit_tag_t::default();
            
            // Create IV from nonce
            let mut iv = [0u8; 12];
            iv[..8].copy_from_slice(&channel.nonce.to_le_bytes());
            
            let result = unsafe {
                sgx_rijndael128GCM_encrypt(
                    &session_key as *const _ as *const sgx_aes_gcm_128bit_key_t,
                    data.as_ptr(),
                    data.len() as u32,
                    encrypted.as_mut_ptr(),
                    &iv as *const u8,
                    12,
                    std::ptr::null(),
                    0,
                    &mut mac as *mut _,
                )
            };
            
            if result != sgx_status_t::SGX_SUCCESS {
                return Err(SgxError::NetworkError("Encryption failed".into()));
            }
            
            // Append MAC to encrypted data
            encrypted.extend_from_slice(&mac);
            Ok(encrypted)
        }
        
        #[cfg(not(feature = "sgx"))]
        {
            // Simple XOR for non-SGX builds (not secure, just for testing)
            let mut encrypted = data.to_vec();
            for (i, byte) in encrypted.iter_mut().enumerate() {
                *byte ^= session_key[i % 32];
            }
            Ok(encrypted)
        }
    }
    
    /// Decrypt message from secure channel
    fn decrypt_message(&self, channel: &SecureChannel, encrypted_data: &[u8]) -> Result<Vec<u8>, SgxError> {
        let session_key = channel.session_key
            .ok_or_else(|| SgxError::NetworkError("No session key established".into()))?;
        
        #[cfg(feature = "sgx")]
        {
            use sgx_tcrypto::*;
            
            if encrypted_data.len() < 16 {
                return Err(SgxError::NetworkError("Invalid encrypted data".into()));
            }
            
            let data_len = encrypted_data.len() - 16;
            let mut decrypted = vec![0u8; data_len];
            
            // Extract MAC from end of data
            let mut mac = sgx_aes_gcm_128bit_tag_t::default();
            mac.copy_from_slice(&encrypted_data[data_len..]);
            
            // Create IV from nonce
            let mut iv = [0u8; 12];
            iv[..8].copy_from_slice(&channel.nonce.to_le_bytes());
            
            let result = unsafe {
                sgx_rijndael128GCM_decrypt(
                    &session_key as *const _ as *const sgx_aes_gcm_128bit_key_t,
                    encrypted_data.as_ptr(),
                    data_len as u32,
                    decrypted.as_mut_ptr(),
                    &iv as *const u8,
                    12,
                    std::ptr::null(),
                    0,
                    &mac as *const _,
                )
            };
            
            if result != sgx_status_t::SGX_SUCCESS {
                return Err(SgxError::NetworkError("Decryption failed".into()));
            }
            
            Ok(decrypted)
        }
        
        #[cfg(not(feature = "sgx"))]
        {
            // Simple XOR for non-SGX builds (not secure, just for testing)
            let mut decrypted = encrypted_data.to_vec();
            for (i, byte) in decrypted.iter_mut().enumerate() {
                *byte ^= session_key[i % 32];
            }
            Ok(decrypted)
        }
    }
}

impl SecureChannel {
    /// Clone the secure channel (for internal use)
    #[cfg(feature = "sgx")]
    fn clone(&self) -> Self {
        Self {
            channel_id: self.channel_id,
            remote_public_key: self.remote_public_key,
            session_key: self.session_key,
            nonce: self.nonce,
        }
    }
    
    /// Complete handshake with remote public key
    pub fn complete_handshake(&mut self, remote_public_key: &[u8; 64]) -> Result<(), SgxError> {
        #[cfg(feature = "sgx")]
        {
            use super::crypto::SgxCrypto;
            
            self.remote_public_key = Some(*remote_public_key);
            
            // Derive session key using ECDH
            let crypto = SgxCrypto::new()?;
            let shared_secret = crypto.random_bytes(32)?; // Placeholder for ECDH
            
            let mut session_key = [0u8; 32];
            session_key.copy_from_slice(&shared_secret);
            self.session_key = Some(session_key);
            
            Ok(())
        }
        
        #[cfg(not(feature = "sgx"))]
        {
            self.remote_public_key = Some(*remote_public_key);
            // Generate dummy session key for testing
            let mut session_key = [0u8; 32];
            for (i, byte) in session_key.iter_mut().enumerate() {
                *byte = (i as u8) ^ remote_public_key[i % 64];
            }
            self.session_key = Some(session_key);
            Ok(())
        }
    }
    
    /// Check if channel is established
    pub fn is_established(&self) -> bool {
        self.session_key.is_some()
    }
}

/// OCALL wrapper for network requests
#[cfg(feature = "sgx")]
pub fn ocall_network_request(request: &[u8]) -> Result<Vec<u8>, SgxError> {
    let mut response = vec![0u8; 4096];
    let mut actual_len = 0usize;
    
    let result = unsafe {
        ocall_neo_rpc_request(
            request.as_ptr(),
            request.len(),
            response.as_mut_ptr(),
            response.len(),
            &mut actual_len as *mut _,
        )
    };
    
    if result != sgx_status_t::SGX_SUCCESS {
        return Err(SgxError::NetworkError("OCALL failed".into()));
    }
    
    response.truncate(actual_len);
    Ok(response)
}

#[cfg(feature = "sgx")]
extern "C" {
    fn ocall_neo_rpc_request(
        request: *const u8,
        request_len: usize,
        response: *mut u8,
        response_len: usize,
        actual_response_len: *mut usize,
    ) -> sgx_status_t;
}

/// Simulated network request for non-SGX builds
#[cfg(not(feature = "sgx"))]
pub fn ocall_network_request(_request: &[u8]) -> Result<Vec<u8>, SgxError> {
    // Return dummy response for testing
    Ok(b"OK".to_vec())
}