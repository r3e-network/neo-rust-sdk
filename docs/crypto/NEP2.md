# NEP2 Implementation

NEP2 (Neo Extended Protocol 2) is a standard for encrypting and decrypting private keys in the Neo blockchain.

## Overview

The NEP2 standard provides a secure method to store private keys by encrypting them with a user passphrase. This enhances security by ensuring that even if the encrypted key is obtained by an attacker, it cannot be used without the passphrase.

## Structure

A NEP2 encrypted private key has the following format:

1. A prefix identifying it as a NEP2 encrypted key (always starts with "6P")
2. A one-byte flag indicating the version (0x01)
3. A two-byte flag for compression/encryption (0x42, 0xE0)
4. A four-byte address hash for verification
5. Encrypted private key data (32 bytes)
6. A four-byte Base58Check checksum

## Encryption Process

1. Calculate the address hash from the private key's corresponding address
2. Derive an encryption key using scrypt with the passphrase and address hash
3. Split the derived key into two 32-byte halves
4. XOR the private key with the first half
5. Encrypt the result with AES-256-ECB using the second half as the key
6. Assemble the final data structure with flags, address hash, and encrypted data
7. Encode using Base58Check

## Decryption Process

1. Decode the NEP2 string using Base58Check
2. Validate the format, version, and flags
3. Extract the address hash and encrypted data
4. Derive a decryption key using scrypt with the passphrase and address hash
5. Split the derived key into two 32-byte halves
6. Decrypt the encrypted data with AES-256-ECB using the second half
7. XOR the result with the first half to get the original private key
8. Verify that the address hash matches the one calculated from the decrypted key

## Security Considerations

- The NEP2 standard uses scrypt as a key derivation function, which is designed to be computationally intensive, making brute force attacks difficult
- The standard parameters (N=16384, r=8, p=8) provide a good balance between security and performance
- The address hash verification ensures that the decryption was successful with the correct password
- Always use strong passphrases to protect against dictionary attacks

## Usage in NeoRust

```rust,no_run
use NeoRust::prelude::{KeyPair, NEP2};
use p256::elliptic_curve::rand_core::OsRng;
use NeoRust::prelude::Secp256r1PrivateKey;

// Generate a key pair
let key_pair = KeyPair::from_secret_key(&Secp256r1PrivateKey::random(&mut OsRng));

// Encrypt the key pair
let encrypted = NEP2::encrypt("my-secure-password", &key_pair).expect("Encryption failed");

// Decrypt the key pair
let decrypted_key_pair = NEP2::decrypt("my-secure-password", &encrypted).expect("Decryption failed");
```

## Advanced Usage with Custom Parameters

```rust,no_run
use NeoRust::prelude::{KeyPair, NEP2};
use p256::elliptic_curve::rand_core::OsRng;
use scrypt::Params;
use NeoRust::prelude::Secp256r1PrivateKey;

// Generate a key pair
let key_pair = KeyPair::from_secret_key(&Secp256r1PrivateKey::random(&mut OsRng));

// Custom scrypt parameters (more secure but slower)
let params = Params::new(15, 8, 8, 32).unwrap();

// Encrypt with custom parameters
let encrypted = NEP2::encrypt_with_params("my-secure-password", &key_pair, params.clone())
    .expect("Encryption failed");

// Decrypt with the same parameters
let decrypted_key_pair = NEP2::decrypt_with_params("my-secure-password", &encrypted, params)
    .expect("Decryption failed");
```

## Error Handling

The NEP2 implementation provides detailed error information through the `Nep2Error` enum:

```rust,no_run
pub enum Nep2Error {
    InvalidPassphrase(String),
    InvalidFormat(String),
    InvalidPrivateKey(String),
    EncryptionError(String),
    DecryptionError(String),
    VerificationFailed(String),
    ScryptError(String),
    Base58Error(String),
}
```

This allows for proper error handling in applications using the NEP2 functionality.