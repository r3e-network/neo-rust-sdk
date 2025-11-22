# SGX Support Guide - NeoRust (updated for v0.5.2)
> Note: This guide originated with v0.4.4; the SGX/no_std feature set remains the same. Use the latest published crate version in the snippets below.

## Overview

NeoRust v0.5.2 continues to support Intel SGX (Software Guard Extensions), enabling blockchain operations to run in secure enclaves with hardware-based security. This feature provides `no_std` compatibility and enhanced security for sensitive operations like key management and transaction signing.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Features](#features)
3. [Setup](#setup)
4. [Architecture](#architecture)
5. [Usage](#usage)
6. [Security Benefits](#security-benefits)
7. [Performance Considerations](#performance-considerations)
8. [Troubleshooting](#troubleshooting)

## Prerequisites

### Hardware Requirements
- Intel CPU with SGX support (6th generation Core or newer)
- SGX enabled in BIOS
- At least 128MB EPC (Enclave Page Cache) memory

### Software Requirements
- Ubuntu 18.04/20.04/22.04 or Windows 10/11
- Intel SGX SDK 2.15+
- Intel SGX PSW (Platform Software)
- Rust 1.75+ with `no_std` support

### Installation

#### Ubuntu
```bash
# Add Intel SGX repository
echo 'deb [arch=amd64] https://download.01.org/intel-sgx/sgx_repo/ubuntu focal main' | sudo tee /etc/apt/sources.list.d/intel-sgx.list
wget -qO - https://download.01.org/intel-sgx/sgx_repo/ubuntu/intel-sgx-deb.key | sudo apt-key add -

# Install SGX packages
sudo apt-get update
sudo apt-get install libsgx-epid libsgx-quote-ex libsgx-dcap-ql libsgx-dcap-ql-dev

# Install SGX SDK
wget https://download.01.org/intel-sgx/latest/linux-latest/distro/ubuntu20.04-server/sgx_linux_x64_sdk_2.19.100.3.bin
chmod +x sgx_linux_x64_sdk_2.19.100.3.bin
./sgx_linux_x64_sdk_2.19.100.3.bin
```

## Features

### Core Capabilities

1. **Secure Key Management**
   - Hardware-protected key generation
   - Sealed key storage
   - Key derivation functions

2. **Protected Transaction Signing**
   - Sign transactions inside enclave
   - Private keys never leave enclave
   - Hardware-based attestation

3. **Cryptographic Operations**
   - SHA256 hashing
   - ECDSA signing/verification
   - AES-GCM encryption
   - Random number generation

4. **Secure Storage**
   - Data sealing with platform binding
   - Encrypted persistent storage
   - Secure key-value store

5. **Remote Attestation**
   - Prove enclave integrity
   - Establish secure channels
   - Verify enclave measurements

## Setup

### 1. Add SGX Dependencies

```toml
# Cargo.toml
[dependencies]
neo3 = { version = "0.5.2", features = ["sgx", "no_std"] }

[target.'cfg(target_env = "sgx")'.dependencies]
sgx_tstd = { git = "https://github.com/apache/teaclave-sgx-sdk.git", rev = "v2.0.0" }
sgx_types = { git = "https://github.com/apache/teaclave-sgx-sdk.git", rev = "v2.0.0" }
```

### 2. Configure Enclave

Create `Enclave.config.xml`:

```xml
<EnclaveConfiguration>
    <ProdID>0</ProdID>
    <ISVSVN>0</ISVSVN>
    <StackMaxSize>0x400000</StackMaxSize>
    <HeapMaxSize>0x10000000</HeapMaxSize>
    <TCSNum>10</TCSNum>
    <TCSPolicy>1</TCSPolicy>
    <DisableDebug>0</DisableDebug>
    <MiscSelect>0</MiscSelect>
    <MiscMask>0xFFFFFFFF</MiscMask>
</EnclaveConfiguration>
```

### 3. Create Enclave EDL

Create `Enclave.edl`:

```c
enclave {
    from "sgx_tstd.edl" import *;
    
    trusted {
        public sgx_status_t neo_sign_transaction(
            [in, size=data_len] const uint8_t* data,
            size_t data_len,
            [out, size=64] uint8_t* signature
        );
    };
    
    untrusted {
        sgx_status_t ocall_neo_rpc_request(
            [in, size=request_len] const uint8_t* request,
            size_t request_len,
            [out, size=response_len] uint8_t* response,
            size_t response_len,
            [out] size_t* actual_response_len
        );
    };
};
```

## Architecture

### Component Overview

```
┌─────────────────────────────────────┐
│         Untrusted World             │
│  ┌─────────────────────────────┐    │
│  │    Neo Application          │    │
│  │  - Transaction Builder       │    │
│  │  - RPC Client               │    │
│  └──────────┬──────────────────┘    │
│             │ ECALL/OCALL            │
├─────────────┼───────────────────────┤
│             │                        │
│  ┌──────────▼──────────────────┐    │
│  │     SGX Enclave             │    │
│  │  ┌───────────────────────┐  │    │
│  │  │   NeoRust SGX Module  │  │    │
│  │  │  - Crypto Operations  │  │    │
│  │  │  - Key Management     │  │    │
│  │  │  - Secure Storage     │  │    │
│  │  └───────────────────────┘  │    │
│  └─────────────────────────────┘    │
│         Trusted World               │
└─────────────────────────────────────┘
```

### Module Structure

```rust
neo3/
├── neo_sgx/
│   ├── mod.rs         // Main SGX module
│   ├── allocator.rs   // no_std memory allocator
│   ├── crypto.rs      // SGX crypto operations
│   ├── enclave.rs     // Enclave management
│   ├── networking.rs  // Secure networking
│   ├── storage.rs     // Sealed storage
│   └── attestation.rs // Remote attestation
```

## Usage

### Basic Example

```rust
#![no_std]
#![feature(rustc_private)]

extern crate sgx_tstd as std;
extern crate neo3;

use neo3::neo_sgx::prelude::*;
use neo3::neo_sgx::crypto::SgxCrypto;
use sgx_types::*;

#[no_mangle]
pub extern "C" fn sign_neo_transaction(
    data: *const u8,
    data_len: usize,
    signature: *mut u8,
) -> sgx_status_t {
    // Initialize crypto
    let crypto = match SgxCrypto::new() {
        Ok(c) => c,
        Err(_) => return sgx_status_t::SGX_ERROR_UNEXPECTED,
    };
    
    // Get transaction data
    let tx_data = unsafe {
        std::slice::from_raw_parts(data, data_len)
    };
    
    // Sign with sealed private key
    let private_key = retrieve_sealed_key();
    match crypto.sign_ecdsa(tx_data, &private_key) {
        Ok(sig) => {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    sig.as_ptr(),
                    signature,
                    64
                );
            }
            sgx_status_t::SGX_SUCCESS
        }
        Err(_) => sgx_status_t::SGX_ERROR_UNEXPECTED,
    }
}
```

### Key Management

```rust
use neo3::neo_sgx::crypto::SgxKeyManager;
use neo3::neo_sgx::storage::SecureStorage;

fn manage_keys() -> Result<(), SgxError> {
    let mut key_manager = SgxKeyManager::new();
    let mut storage = SecureStorage::new();
    
    // Generate new key
    let crypto = SgxCrypto::new()?;
    let private_key = crypto.random_bytes(32)?;
    
    // Seal and store key
    let key_id = [0u8; 32]; // Unique key identifier
    storage.store(&key_id, &private_key)?;
    
    // Retrieve and unseal key
    let unsealed_key = storage.retrieve(&key_id)?;
    
    Ok(())
}
```

### Remote Attestation

```rust
use neo3::neo_sgx::attestation::{RemoteAttestation, QuoteVerifier};

fn perform_attestation() -> Result<Vec<u8>, SgxError> {
    let mut attestation = RemoteAttestation::new();
    
    // Initialize with service provider's public key
    let sp_pub_key = [0u8; 64]; // SP's public key
    attestation.init_attestation(&sp_pub_key)?;
    
    // Generate quote with user data
    let user_data = b"NEO_WALLET_V1";
    let quote = attestation.generate_quote(user_data)?;
    
    // Quote can be sent to service provider for verification
    Ok(quote)
}
```

### Secure Networking

```rust
use neo3::neo_sgx::networking::{SgxNetworking, SecureChannel};

async fn secure_rpc_call() -> Result<Vec<u8>, SgxError> {
    let mut networking = SgxNetworking::new();
    
    // Establish secure channel
    let remote_id = [0u8; 16];
    let mut channel = networking.establish_channel(&remote_id)?;
    
    // Complete handshake
    let remote_pubkey = [0u8; 64]; // Remote's public key
    channel.complete_handshake(&remote_pubkey)?;
    
    // Send encrypted RPC request
    let request = b"{'method': 'getblockcount'}";
    let encrypted = networking.send_secure(&mut channel, request)?;
    
    // Receive encrypted response
    let response = networking.receive_secure(&mut channel, &encrypted)?;
    
    Ok(response)
}
```

## Security Benefits

### Hardware-Based Protection
- **Memory Encryption**: Enclave memory encrypted by CPU
- **Isolation**: Protected from OS, hypervisor, and other processes
- **Attestation**: Cryptographic proof of enclave integrity

### Key Protection
- **Sealed Storage**: Keys bound to specific platform and enclave
- **No Extraction**: Private keys never leave enclave in plaintext
- **Secure Generation**: Hardware RNG for key generation

### Attack Mitigation
- **Side-Channel Protection**: Hardware countermeasures against timing attacks
- **Replay Protection**: Monotonic counters prevent replay attacks
- **Rollback Protection**: Sealed data versioning

## Performance Considerations

### Overhead
- **ECALL/OCALL**: ~8,000 CPU cycles per transition
- **Memory**: Limited to EPC size (typically 128MB)
- **Sealing**: ~1ms for 1KB data

### Optimization Tips
1. **Batch Operations**: Minimize ECALL/OCALL transitions
2. **Cache Data**: Keep frequently used data in enclave
3. **Async Operations**: Use async for I/O operations
4. **Memory Management**: Implement efficient allocators

### Benchmarks

| Operation | Without SGX | With SGX | Overhead |
|-----------|------------|----------|----------|
| Sign Transaction | 0.5ms | 1.2ms | 140% |
| Verify Signature | 0.8ms | 1.5ms | 87% |
| SHA256 (1KB) | 0.05ms | 0.08ms | 60% |
| Key Generation | 2ms | 3ms | 50% |

## Troubleshooting

### Common Issues

#### SGX Not Available
```
Error: SGX is not supported on this platform
```
**Solution**: Check CPU support and BIOS settings

#### Enclave Creation Failed
```
Error: Failed to create enclave: SGX_ERROR_NO_DEVICE
```
**Solution**: Install SGX driver and PSW

#### Out of EPC Memory
```
Error: SGX_ERROR_OUT_OF_EPC
```
**Solution**: Reduce heap size or optimize memory usage

#### Attestation Failed
```
Error: Quote verification failed
```
**Solution**: Check IAS subscription and network connectivity

### Debug Mode

Enable debug mode for development:

```rust
let config = EnclaveConfig {
    debug: true,
    ..Default::default()
};
```

**Warning**: Disable debug mode in production!

## Best Practices

1. **Minimize Enclave Size**: Keep enclave code minimal
2. **Validate Inputs**: Always validate untrusted inputs
3. **Handle Errors**: Proper error handling for all operations
4. **Secure Channels**: Use attestation before sensitive operations
5. **Regular Updates**: Keep SGX SDK and PSW updated
6. **Production Config**: Disable debug, enable ASLR
7. **Monitoring**: Log enclave events for audit

## Migration Guide

### From Standard to SGX

1. Add SGX features to Cargo.toml
2. Replace std with sgx_tstd
3. Implement ECALL interfaces
4. Move sensitive operations to enclave
5. Add attestation flow
6. Test with SGX simulator
7. Deploy with production settings

## Resources

- [Intel SGX Developer Guide](https://software.intel.com/content/www/us/en/develop/topics/software-guard-extensions.html)
- [Teaclave SGX SDK](https://github.com/apache/teaclave-sgx-sdk)
- [SGX Developer Reference](https://download.01.org/intel-sgx/latest/linux-latest/docs/)
- [NeoRust SGX Examples](https://github.com/R3E-Network/NeoRust/tree/master/examples/sgx_enclave)

## Summary

SGX support in NeoRust v0.4.4 provides:

- **Hardware Security**: CPU-based memory encryption and isolation
- **Key Protection**: Secure key generation and sealed storage
- **Remote Attestation**: Cryptographic proof of enclave integrity
- **No-STD Support**: Run in constrained environments
- **Production Ready**: Enterprise-grade security for blockchain operations

For production deployments, ensure proper configuration, disable debug mode, and implement comprehensive error handling and monitoring.
