// SGX Secure Storage

#![cfg_attr(feature = "sgx", no_std)]

#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;

#[cfg(feature = "sgx")]
use sgx_types::*;

use super::SgxError;

/// Secure storage for SGX enclaves
pub struct SecureStorage {
    #[cfg(feature = "sgx")]
    sealed_items: Vec<SealedItem>,
}

#[cfg(feature = "sgx")]
struct SealedItem {
    key_id: [u8; 32],
    sealed_data: Vec<u8>,
    timestamp: u64,
}

impl SecureStorage {
    /// Create new secure storage instance
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "sgx")]
            sealed_items: Vec::new(),
        }
    }
    
    /// Store data securely with sealing
    #[cfg(feature = "sgx")]
    pub fn store(&mut self, key_id: &[u8; 32], data: &[u8]) -> Result<(), SgxError> {
        // Calculate sealed data size
        let sealed_size = unsafe {
            sgx_calc_sealed_data_size(0, data.len() as u32)
        };
        
        if sealed_size == u32::MAX {
            return Err(SgxError::MemoryError("Failed to calculate sealed size".into()));
        }
        
        // Seal the data
        let mut sealed_data = vec![0u8; sealed_size as usize];
        
        let result = unsafe {
            sgx_seal_data(
                0,
                std::ptr::null(),
                data.len() as u32,
                data.as_ptr(),
                sealed_size,
                sealed_data.as_mut_ptr() as *mut sgx_sealed_data_t,
            )
        };
        
        if result != sgx_status_t::SGX_SUCCESS {
            return Err(SgxError::MemoryError("Failed to seal data".into()));
        }
        
        // Store sealed item
        let item = SealedItem {
            key_id: *key_id,
            sealed_data,
            timestamp: get_timestamp(),
        };
        
        // Remove old version if exists
        self.sealed_items.retain(|item| item.key_id != *key_id);
        self.sealed_items.push(item);
        
        // Optionally persist to untrusted storage via OCALL
        self.persist_to_disk(key_id, &sealed_data)?;
        
        Ok(())
    }
    
    #[cfg(not(feature = "sgx"))]
    pub fn store(&mut self, _key_id: &[u8; 32], _data: &[u8]) -> Result<(), SgxError> {
        // Simulated storage for non-SGX builds
        Ok(())
    }
    
    /// Retrieve and unseal data
    #[cfg(feature = "sgx")]
    pub fn retrieve(&self, key_id: &[u8; 32]) -> Result<Vec<u8>, SgxError> {
        // Find sealed item
        let item = self.sealed_items
            .iter()
            .find(|item| item.key_id == *key_id)
            .ok_or_else(|| SgxError::MemoryError("Key not found".into()))?;
        
        // Get unsealed size
        let unsealed_size = unsafe {
            sgx_get_encrypt_txt_len(item.sealed_data.as_ptr() as *const sgx_sealed_data_t)
        };
        
        // Unseal the data
        let mut unsealed_data = vec![0u8; unsealed_size as usize];
        let mut mac_len = 0u32;
        
        let result = unsafe {
            sgx_unseal_data(
                item.sealed_data.as_ptr() as *const sgx_sealed_data_t,
                std::ptr::null_mut(),
                &mut mac_len,
                unsealed_data.as_mut_ptr(),
                &mut (unsealed_size as u32),
            )
        };
        
        if result != sgx_status_t::SGX_SUCCESS {
            return Err(SgxError::MemoryError("Failed to unseal data".into()));
        }
        
        Ok(unsealed_data)
    }
    
    #[cfg(not(feature = "sgx"))]
    pub fn retrieve(&self, _key_id: &[u8; 32]) -> Result<Vec<u8>, SgxError> {
        // Return dummy data for non-SGX builds
        Ok(vec![0u8; 32])
    }
    
    /// Delete stored data
    #[cfg(feature = "sgx")]
    pub fn delete(&mut self, key_id: &[u8; 32]) -> Result<(), SgxError> {
        self.sealed_items.retain(|item| item.key_id != *key_id);
        self.delete_from_disk(key_id)?;
        Ok(())
    }
    
    #[cfg(not(feature = "sgx"))]
    pub fn delete(&mut self, _key_id: &[u8; 32]) -> Result<(), SgxError> {
        Ok(())
    }
    
    /// List all stored keys
    #[cfg(feature = "sgx")]
    pub fn list_keys(&self) -> Vec<[u8; 32]> {
        self.sealed_items
            .iter()
            .map(|item| item.key_id)
            .collect()
    }
    
    #[cfg(not(feature = "sgx"))]
    pub fn list_keys(&self) -> Vec<[u8; 32]> {
        Vec::new()
    }
    
    /// Persist sealed data to disk via OCALL
    #[cfg(feature = "sgx")]
    fn persist_to_disk(&self, key_id: &[u8; 32], sealed_data: &[u8]) -> Result<(), SgxError> {
        let result = unsafe {
            ocall_secure_save(
                sealed_data.as_ptr(),
                sealed_data.len(),
                key_id.as_ptr(),
            )
        };
        
        if result != sgx_status_t::SGX_SUCCESS {
            return Err(SgxError::MemoryError("Failed to persist data".into()));
        }
        
        Ok(())
    }
    
    #[cfg(not(feature = "sgx"))]
    fn persist_to_disk(&self, _key_id: &[u8; 32], _sealed_data: &[u8]) -> Result<(), SgxError> {
        Ok(())
    }
    
    /// Delete from disk via OCALL
    #[cfg(feature = "sgx")]
    fn delete_from_disk(&self, key_id: &[u8; 32]) -> Result<(), SgxError> {
        // Call OCALL to delete file
        // Implementation depends on untrusted side
        Ok(())
    }
    
    #[cfg(not(feature = "sgx"))]
    fn delete_from_disk(&self, _key_id: &[u8; 32]) -> Result<(), SgxError> {
        Ok(())
    }
}

/// Get current timestamp
#[cfg(feature = "sgx")]
fn get_timestamp() -> u64 {
    // Use SGX monotonic counter for timestamp
    let mut counter_value = 0u32;
    let mut counter_uuid = sgx_mc_uuid_t { counter_id: [0; 16] };
    
    let result = unsafe {
        sgx_create_monotonic_counter(&mut counter_uuid, &mut counter_value)
    };
    
    if result == sgx_status_t::SGX_SUCCESS {
        counter_value as u64
    } else {
        // Fallback: use a pseudo-timestamp based on sealed data count
        unsafe {
            static mut COUNTER: u64 = 0;
            COUNTER += 1;
            COUNTER
        }
    }
}

#[cfg(not(feature = "sgx"))]
fn get_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(feature = "sgx")]
extern "C" {
    fn ocall_secure_save(
        data: *const u8,
        data_len: usize,
        key_id: *const u8,
    ) -> sgx_status_t;
    
    fn ocall_secure_load(
        key_id: *const u8,
        data: *mut u8,
        data_len: usize,
        actual_data_len: *mut usize,
    ) -> sgx_status_t;
    
    fn sgx_create_monotonic_counter(
        counter_uuid: *mut sgx_mc_uuid_t,
        counter_value: *mut u32,
    ) -> sgx_status_t;
}