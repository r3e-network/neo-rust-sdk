# Neo N3 Wallets

## Overview

Wallet management is a critical component of the Neo N3 ecosystem, providing secure access to accounts, assets, and blockchain operations. The NeoRust SDK offers comprehensive wallet functionality with a focus on security, usability, and extensibility.

## Key Features

- **Account Management**: Create, import, and manage Neo N3 accounts
- **Key Security**: Securely store and handle private keys
- **Transaction Signing**: Sign transactions for broadcasting to the Neo N3 network
- **Message Signing**: Create and verify cryptographic signatures for off-chain authentication
- **Hardware Integration**: Support for hardware wallets like Ledger
- **BIP-39 Support**: Mnemonic phrase generation and recovery
- **NEP-6 Compatibility**: Standard Neo wallet format support

## Documentation Sections

- [Message Signing](message-signing.md): Comprehensive guide to cryptographic message signing

## Implementation Components

The wallet system in NeoRust includes several key components:

1. **WalletSigner**: Core wallet functionality for signing messages and transactions
2. **Account**: Represents a Neo N3 account with address and key information
3. **KeyPair**: Manages public-private key pairs for cryptographic operations
4. **LedgerWallet**: Integration with Ledger hardware wallets
5. **BIP39 Support**: Utilities for mnemonic phrase handling

## Example Usage

```rust,no_run
use neo3::prelude::*;

// Create a new random key pair
let key_pair = KeyPair::new_random()?;

// Create a wallet signer
let wallet = WalletSigner::new_with_signer(key_pair.clone(), key_pair.get_address());

// Sign a message
let message = b"Hello, Neo!";
let signature = wallet.sign_message(message).await?;

// Use the wallet to sign a transaction
let signed_tx = transaction.sign_with(&wallet).await?;
```

## Best Practices

- Use hardware wallets for high-value accounts
- Maintain secure backups of private keys or mnemonic phrases
- Validate addresses before signing transactions
- Consider multi-signature setups for enhanced security
- Keep wallet software updated to address security vulnerabilities

## Related Modules

- [neo_crypto](../crypto/README.md): Cryptographic primitives used by wallet functions
- [neo_protocol](../protocol/README.md): Core protocol implementations
- [neo_types](../types/README.md): Type definitions for Neo N3