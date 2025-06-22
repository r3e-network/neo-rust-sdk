# Wallet Management

This tutorial covers wallet management with the NeoRust SDK, including creating, loading, and using wallets.

## Creating a New Wallet

```rust,edition2021
use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a new wallet with a password
    let password = "my-secure-password";
    let wallet = Wallet::new("my-wallet", password)?;
    
    // Generate a new account
    let account = wallet.create_account()?;
    println!("New account address: {}", account.get_address());
    
    // Save the wallet to a file
    wallet.save("my-wallet.json")?;
    
    Ok(())
}
```

## Loading an Existing Wallet

```rust,edition2021
use neo3::prelude::*;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load a wallet from a file
    let wallet_path = Path::new("my-wallet.json");
    let password = "my-secure-password";
    let wallet = Wallet::load(wallet_path, Some(password))?;
    
    // Get the default account
    let account = wallet.default_account()?;
    println!("Default account address: {}", account.get_address());
    
    Ok(())
}
```

## Importing a Private Key

```rust,edition2021
use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a new wallet
    let password = "my-secure-password";
    let mut wallet = Wallet::new("my-wallet", password)?;
    
    // Import a private key
    let private_key = "your-private-key-here";
    let account = wallet.import_private_key(private_key, password)?;
    println!("Imported account address: {}", account.get_address());
    
    // Save the wallet
    wallet.save("my-wallet.json")?;
    
    Ok(())
}
```

## Signing a Message

```rust,edition2021
use neo3::prelude::*;
use neo3::neo_wallets::WalletSigner;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a key pair
    let key_pair = KeyPair::new_random()?;
    
    // Create a wallet signer
    let wallet = WalletSigner::new_with_signer(key_pair.clone(), key_pair.get_address());
    
    // Sign a message
    let message = b"Hello, Neo!";
    let signature = wallet.sign_message(message).await?;
    println!("Signature: {:?}", signature);
    
    // For more information on message signing, see the dedicated documentation
    // in the /docs/wallets/message-signing.md file
    
    Ok(())
}
```

## Wallet Backup and Recovery

```rust,edition2021
use neo3::prelude::*;
use neo3::neo_wallets::bip39_account::create_account_from_mnemonic;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a mnemonic phrase for backup
    let mnemonic = "liberty village rhythm couch december axis barely model flag gym tortoise must";
    
    // Create an account from the mnemonic
    let account = create_account_from_mnemonic(mnemonic)?;
    println!("Account address: {}", account.get_address());
    
    // Later, recover the account from the same mnemonic
    let recovered_account = create_account_from_mnemonic(mnemonic)?;
    println!("Recovered account address: {}", recovered_account.get_address());
    
    assert_eq!(account.get_address(), recovered_account.get_address());
    
    Ok(())
}
```

## Using Hardware Wallets

If you have enabled the `ledger` feature, you can use hardware wallets for enhanced security:

```rust,edition2021
use neo3::prelude::*;
use neo3::neo_wallets::{HDPath, LedgerWallet, WalletSigner};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Define an HD derivation path
    let hd_path = HDPath::new(44, 888, 0, 0, 0)?;
    
    // Initialize the Ledger wallet (will prompt the user on the device)
    let ledger = LedgerWallet::new(hd_path, None).await?;
    
    // Get the address from the hardware wallet
    let address = ledger.get_address();
    println!("Ledger wallet address: {}", address);
    
    // Create a wallet signer that uses the hardware wallet
    let signer = WalletSigner::new_with_signer(ledger, address);
    
    // Sign a transaction or message using the hardware wallet
    // (will require physical confirmation on the device)
    let message = b"Hello, Neo!";
    let signature = signer.sign_message(message).await?;
    
    Ok(())
}
```

## Advanced Message Signing

For detailed information about message signing capabilities, including use cases and security considerations, see the message signing examples in the examples directory.

## Best Practices

1. **Secure Password Storage**: Never hardcode passwords in your application.
2. **Regular Backups**: Always backup your wallet's mnemonic phrase or private keys.
3. **Verify Addresses**: Always verify addresses before sending transactions.
4. **Use Hardware Wallets**: For production applications, consider using hardware wallets via the `ledger` feature.
5. **Multiple Signers**: Consider using multi-signature setups for high-value wallets.

<!-- toc -->
