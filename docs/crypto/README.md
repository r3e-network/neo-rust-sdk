# Neo N3 Cryptography

## Overview

The cryptography module in the NeoRust SDK provides comprehensive cryptographic utilities for working with the Neo N3 blockchain. It implements the core security primitives necessary for blockchain operations, including key management, hashing, signature creation and verification, and encrypted key storage.

## Key Features

- **Key Pair Management**: Creation and management of ECDSA key pairs on the secp256r1 curve
- **Hashing Functions**: SHA-256, RIPEMD-160, and other cryptographic hash functions
- **Digital Signatures**: Creation and verification of cryptographic signatures
- **NEP-2 Standard**: Private key encryption and decryption according to the NEP-2 standard
- **Base58 Encoding/Decoding**: Utilities for Base58 and Base58Check encoding
- **WIF (Wallet Import Format)**: Import and export of private keys in WIF format
- **Secure Random Number Generation**: Cryptographically secure random number generation

## Documentation Sections

- [NEP-2 Standard](NEP2.md): Implementation of the NEP-2 encrypted key format

## Implementation Components

The cryptography system in NeoRust includes several key components:

1. **KeyPair**: Manages ECDSA key pairs for cryptographic operations
2. **Hash**: Cryptographic hash functions and utilities
3. **Base58Helper**: Encoding and decoding utilities
4. **NEP2**: Implementation of the NEP-2 standard for encrypted private keys
5. **WIF**: Utilities for Wallet Import Format handling

## Example Usage

```rust,no_run
use neo3::prelude::*;

// Create a new random key pair
let key_pair = KeyPair::new_random()?;
println!("Public key: {}", key_pair.public_key());
println!("Private key: {}", key_pair.private_key());

// Sign and verify data
let data = b"Hello, Neo!";
let signature = key_pair.sign(data)?;
let is_valid = key_pair.verify_signature(data, &signature)?;
assert!(is_valid);

// Work with NEP-2 encrypted keys
let encrypted = NEP2::encrypt("my-secure-password", &key_pair)?;
let decrypted_key_pair = NEP2::decrypt("my-secure-password", &encrypted)?;
```

## Security Considerations

- The NEP-2 standard uses scrypt as a key derivation function to resist brute force attacks
- Proper handling of private keys is critical; they should never be exposed or stored insecurely
- Random number generation is a cornerstone of cryptographic security
- Signature verification is essential before accepting any signed message or transaction

## Related Modules

- [neo_wallets](../wallets/README.md): Wallet management built on cryptography primitives
- [neo_protocol](../protocol/README.md): Core protocol implementations
- [neo_types](../types/README.md): Type definitions for Neo N3