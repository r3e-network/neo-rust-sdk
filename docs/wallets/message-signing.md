# Message Signing in Neo N3

## Overview

Message signing is a cryptographic process that enables users to prove ownership of a private key without revealing it. In the Neo N3 ecosystem, message signing serves several critical functions:

- Authenticating users for dApps and services
- Verifying identity on-chain
- Authorizing off-chain operations
- Proving ownership of an address

## How It Works

The message signing process in Neo N3 follows these steps:

1. The account owner creates a signature using their private key
2. Anyone with the public key can verify the signature
3. Verification confirms the message was signed by the private key owner
4. The signature cannot be reused for different messages

## Implementation in NeoRust

NeoRust provides comprehensive support for message signing through the `WalletSigner` struct:

```rust,no_run
use neo3::prelude::*;
use neo3::neo_wallets::WalletSigner;

async fn sign_and_verify_message() -> Result<(), Box<dyn std::error::Error>> {
    // Create or load a key pair
    let key_pair = KeyPair::new_random()?;
    
    // Create a wallet signer
    let wallet = WalletSigner::new_with_signer(key_pair.clone(), key_pair.get_address());
    
    // Message to sign
    let message = b"Hello, Neo!";
    
    // Sign the message
    let signature = wallet.sign_message(message).await?;
    println!("Signature: {:?}", signature);
    
    // Verify the signature
    // Implementation details for verification may vary based on your use case
    
    Ok(())
}
```

## Advanced Usage

### Custom Signers

The `WalletSigner` struct is designed to work with any implementation of the `PrehashSigner` trait, making it flexible for various key management solutions:

```rust,no_run
// Using a hardware wallet (when 'ledger' feature is enabled)
let ledger_wallet = LedgerWallet::new(hdpath, address).await?;
let signer = WalletSigner::new_with_signer(ledger_wallet, address);

// Sign message with hardware wallet
let signature = signer.sign_message(message).await?;
```

### Security Considerations

When implementing message signing in your applications:

1. Always hash the message before signing to prevent length-extension attacks
2. Include domain separation in your message format to prevent signature reuse across contexts
3. Consider including a timestamp or nonce to prevent replay attacks
4. For critical applications, use hardware wallets when possible

## Integration with dApps

When building dApps that require authentication:

```rust,no_run
async fn authenticate_user(message: &[u8], signature: Signature, claimed_address: &Address) -> bool {
    // Implement verification logic
    // This would typically involve recovering the public key from the signature
    // and comparing it to the claimed address
    
    // Return true if authentication succeeds, false otherwise
    true
}
```

## Common Use Cases

### Authentication

```rust,no_run
// Generate a challenge for the user to sign
let challenge = format!("Login to ExampleDApp at {}", chrono::Utc::now());

// User signs the challenge with their wallet
let signature = wallet.sign_message(challenge.as_bytes()).await?;

// Server verifies the signature to authenticate the user
```

### Document Signing

```rust,no_run
// Hash a document
let document_hash = document.hash256();

// Sign the document hash
let signature = wallet.sign_message(&document_hash).await?;

// Store the document, hash, and signature for later verification
```

### Authorization

```rust,no_run
// Create a specific permission request
let permission = format!("Allow ExampleDApp to trade up to 100 GAS until {}", expiry_time);

// User signs the permission
let signature = wallet.sign_message(permission.as_bytes()).await?;

// The signature serves as proof of authorization
```

## Related Components

- `KeyPair`: Manages the cryptographic key pair
- `Transaction`: Uses similar signing mechanisms for blockchain transactions
- `Account`: Represents Neo N3 accounts and their capabilities

For more advanced cryptography features, see the [Cryptography](../crypto/README.md) documentation.