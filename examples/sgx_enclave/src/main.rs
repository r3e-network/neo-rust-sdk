// Untrusted application that uses the SGX enclave

use sgx_types::*;
use sgx_urts::SgxEnclave;
use std::fs;
use std::path::Path;

static ENCLAVE_FILE: &str = "enclave.signed.so";

extern "C" {
    fn enclave_init(eid: sgx_enclave_id_t, retval: *mut sgx_status_t) -> sgx_status_t;
    
    fn neo_sign_transaction(
        eid: sgx_enclave_id_t,
        retval: *mut sgx_status_t,
        data: *const u8,
        data_len: usize,
        signature: *mut u8,
    ) -> sgx_status_t;
    
    fn neo_verify_signature(
        eid: sgx_enclave_id_t,
        retval: *mut sgx_status_t,
        data: *const u8,
        data_len: usize,
        signature: *const u8,
        public_key: *const u8,
        valid: *mut u8,
    ) -> sgx_status_t;
    
    fn neo_calculate_hash(
        eid: sgx_enclave_id_t,
        retval: *mut sgx_status_t,
        data: *const u8,
        data_len: usize,
        hash: *mut u8,
    ) -> sgx_status_t;
    
    fn neo_generate_keypair(
        eid: sgx_enclave_id_t,
        retval: *mut sgx_status_t,
        private_key: *mut u8,
        public_key: *mut u8,
    ) -> sgx_status_t;
}

fn init_enclave() -> Result<SgxEnclave, sgx_status_t> {
    let mut launch_token: sgx_launch_token_t = [0; 1024];
    let mut launch_token_updated: i32 = 0;
    
    let debug = 1; // Enable debug mode for development
    
    let enclave = SgxEnclave::create(
        ENCLAVE_FILE,
        debug,
        &mut launch_token,
        &mut launch_token_updated,
        std::ptr::null_mut(),
    )?;
    
    // Initialize the enclave
    let mut retval = sgx_status_t::SGX_SUCCESS;
    let result = unsafe { enclave_init(enclave.geteid(), &mut retval) };
    
    if result != sgx_status_t::SGX_SUCCESS {
        return Err(result);
    }
    
    if retval != sgx_status_t::SGX_SUCCESS {
        return Err(retval);
    }
    
    Ok(enclave)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Neo SGX Enclave Example");
    println!("========================");
    
    // Initialize the enclave
    println!("\n1. Initializing SGX enclave...");
    let enclave = init_enclave().map_err(|e| format!("Failed to initialize enclave: {:?}", e))?;
    println!("   ✓ Enclave initialized successfully (ID: {})", enclave.geteid());
    
    // Generate a keypair inside the enclave
    println!("\n2. Generating keypair inside enclave...");
    let mut private_key = [0u8; 32];
    let mut public_key = [0u8; 64];
    let mut retval = sgx_status_t::SGX_SUCCESS;
    
    let result = unsafe {
        neo_generate_keypair(
            enclave.geteid(),
            &mut retval,
            private_key.as_mut_ptr(),
            public_key.as_mut_ptr(),
        )
    };
    
    if result != sgx_status_t::SGX_SUCCESS || retval != sgx_status_t::SGX_SUCCESS {
        return Err(format!("Failed to generate keypair: {:?}/{:?}", result, retval).into());
    }
    
    println!("   ✓ Keypair generated");
    println!("   Private key: {}", hex::encode(&private_key[..8]) + "...");
    println!("   Public key: {}", hex::encode(&public_key[..8]) + "...");
    
    // Sign a transaction inside the enclave
    println!("\n3. Signing transaction inside enclave...");
    let transaction_data = b"NEO transaction data to sign";
    let mut signature = [0u8; 64];
    
    let result = unsafe {
        neo_sign_transaction(
            enclave.geteid(),
            &mut retval,
            transaction_data.as_ptr(),
            transaction_data.len(),
            signature.as_mut_ptr(),
        )
    };
    
    if result != sgx_status_t::SGX_SUCCESS || retval != sgx_status_t::SGX_SUCCESS {
        return Err(format!("Failed to sign transaction: {:?}/{:?}", result, retval).into());
    }
    
    println!("   ✓ Transaction signed");
    println!("   Signature: {}", hex::encode(&signature[..8]) + "...");
    
    // Verify the signature inside the enclave
    println!("\n4. Verifying signature inside enclave...");
    let mut valid = 0u8;
    
    let result = unsafe {
        neo_verify_signature(
            enclave.geteid(),
            &mut retval,
            transaction_data.as_ptr(),
            transaction_data.len(),
            signature.as_ptr(),
            public_key.as_ptr(),
            &mut valid,
        )
    };
    
    if result != sgx_status_t::SGX_SUCCESS || retval != sgx_status_t::SGX_SUCCESS {
        return Err(format!("Failed to verify signature: {:?}/{:?}", result, retval).into());
    }
    
    println!("   ✓ Signature verification: {}", if valid == 1 { "VALID" } else { "INVALID" });
    
    // Calculate hash inside the enclave
    println!("\n5. Calculating hash inside enclave...");
    let data_to_hash = b"Neo blockchain data";
    let mut hash = [0u8; 32];
    
    let result = unsafe {
        neo_calculate_hash(
            enclave.geteid(),
            &mut retval,
            data_to_hash.as_ptr(),
            data_to_hash.len(),
            hash.as_mut_ptr(),
        )
    };
    
    if result != sgx_status_t::SGX_SUCCESS || retval != sgx_status_t::SGX_SUCCESS {
        return Err(format!("Failed to calculate hash: {:?}/{:?}", result, retval).into());
    }
    
    println!("   ✓ Hash calculated");
    println!("   SHA256: {}", hex::encode(hash));
    
    // Destroy the enclave
    println!("\n6. Destroying enclave...");
    enclave.destroy();
    println!("   ✓ Enclave destroyed");
    
    println!("\n✅ All SGX operations completed successfully!");
    
    Ok(())
}

// Hex encoding utility
mod hex {
    pub fn encode(data: impl AsRef<[u8]>) -> String {
        data.as_ref()
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect()
    }
}