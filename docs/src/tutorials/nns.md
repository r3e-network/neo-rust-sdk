# Neo Name Service (NNS)

This tutorial covers working with the Neo Name Service (NNS) on the Neo N3 blockchain using the NeoRust SDK.

## Understanding NNS

The Neo Name Service (NNS) is a distributed, open-source naming system based on the Neo blockchain. It maps human-readable names to machine-readable identifiers such as Neo addresses, contract script hashes, and more. This makes it easier to work with blockchain addresses and resources.

## Key Concepts

- **Domain**: A human-readable name registered in the NNS (e.g., `example.neo`)
- **Record**: Data associated with a domain (e.g., address, text record, etc.)
- **TTL**: Time-to-live for a domain record
- **Owner**: The account that owns a domain and can manage its records
- **Resolver**: Contract that translates between domain names and addresses/resources

## Creating an NNS Instance

To interact with the NNS, you first need to create an NNS instance:

```rust,no_run
use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo N3 TestNet node
    let provider = Provider::new_http("https://testnet1.neo.coz.io:443");
    
    // Create an NNS instance
    let nns = NameService::new(provider.clone());
    
    Ok(())
}
```

## Checking Domain Availability

Before registering a domain, you should check if it's available:

```rust,no_run
use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo N3 TestNet node
    let provider = Provider::new_http("https://testnet1.neo.coz.io:443");
    
    // Create an NNS instance
    let nns = NameService::new(provider.clone());
    
    // Check if a domain is available
    let domain = "example.neo";
    let is_available = nns.is_available(domain).await?;
    
    if is_available {
        println!("Domain {} is available for registration", domain);
    } else {
        println!("Domain {} is already registered", domain);
    }
    
    Ok(())
}
```

## Registering a Domain

If a domain is available, you can register it:

```rust,no_run
use neo3::prelude::*;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo N3 TestNet node
    let provider = Provider::new_http("https://testnet1.neo.coz.io:443");
    
    // Load your wallet
    let wallet_path = Path::new("my-wallet.json");
    let password = "my-secure-password";
    let wallet = Wallet::load(wallet_path, password)?;
    
    // Get the account that will register the domain
    let account = wallet.default_account()?;
    
    // Create an NNS instance
    let nns = NameService::new(provider.clone());
    
    // Check if a domain is available
    let domain = "example.neo";
    let is_available = nns.is_available(domain).await?;
    
    if is_available {
        // Register the domain
        let registration_period = 1; // in years
        let txid = nns.register(account, domain, registration_period).await?;
        
        println!("Domain registration initiated with transaction ID: {}", txid);
        
        // Wait for the transaction to be confirmed
        let receipt = provider.wait_for_transaction(&txid, 60, 2).await?;
        println!("Domain registration confirmed: {:?}", receipt);
    } else {
        println!("Domain {} is already registered", domain);
    }
    
    Ok(())
}
```

## Setting Domain Records

Once you own a domain, you can set various records for it:

```rust,no_run
use neo3::prelude::*;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo N3 TestNet node
    let provider = Provider::new_http("https://testnet1.neo.coz.io:443");
    
    // Load your wallet
    let wallet_path = Path::new("my-wallet.json");
    let password = "my-secure-password";
    let wallet = Wallet::load(wallet_path, password)?;
    
    // Get the account that owns the domain
    let account = wallet.default_account()?;
    
    // Create an NNS instance
    let nns = NameService::new(provider.clone());
    
    // Domain name
    let domain = "example.neo";
    
    // Set an address record
    let address = account.address();
    let txid = nns.set_address(account, domain, address).await?;
    
    println!("Address record set with transaction ID: {}", txid);
    
    // Wait for the transaction to be confirmed
    let receipt = provider.wait_for_transaction(&txid, 60, 2).await?;
    println!("Address record confirmed: {:?}", receipt);
    
    // Set a text record
    let key = "email";
    let value = "contact@example.neo";
    let text_txid = nns.set_text(account, domain, key, value).await?;
    
    println!("Text record set with transaction ID: {}", text_txid);
    
    Ok(())
}
```

## Resolving Domain Records

You can resolve domain records to get the associated data:

```rust,no_run
use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo N3 TestNet node
    let provider = Provider::new_http("https://testnet1.neo.coz.io:443");
    
    // Create an NNS instance
    let nns = NameService::new(provider.clone());
    
    // Domain name
    let domain = "example.neo";
    
    // Resolve address
    let address = nns.resolve_address(domain).await?;
    
    if let Some(addr) = address {
        println!("Domain {} resolves to address: {}", domain, addr);
    } else {
        println!("No address record found for domain {}", domain);
    }
    
    // Resolve text record
    let key = "email";
    let text = nns.resolve_text(domain, key).await?;
    
    if let Some(value) = text {
        println!("Text record '{}' for domain {}: {}", key, domain, value);
    } else {
        println!("No text record '{}' found for domain {}", key, domain);
    }
    
    Ok(())
}
```

## Renewing a Domain

Domains need to be renewed periodically to maintain ownership:

```rust,no_run
use neo3::prelude::*;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo N3 TestNet node
    let provider = Provider::new_http("https://testnet1.neo.coz.io:443");
    
    // Load your wallet
    let wallet_path = Path::new("my-wallet.json");
    let password = "my-secure-password";
    let wallet = Wallet::load(wallet_path, password)?;
    
    // Get the account that owns the domain
    let account = wallet.default_account()?;
    
    // Create an NNS instance
    let nns = NameService::new(provider.clone());
    
    // Domain name
    let domain = "example.neo";
    
    // Check domain expiration
    let expiration = nns.get_expiration(domain).await?;
    
    if let Some(exp) = expiration {
        println!("Domain {} expires at: {}", domain, exp);
        
        // Renew the domain
        let renewal_period = 1; // in years
        let txid = nns.renew(account, domain, renewal_period).await?;
        
        println!("Domain renewal initiated with transaction ID: {}", txid);
        
        // Wait for the transaction to be confirmed
        let receipt = provider.wait_for_transaction(&txid, 60, 2).await?;
        println!("Domain renewal confirmed: {:?}", receipt);
    } else {
        println!("Domain {} is not registered", domain);
    }
    
    Ok(())
}
```

## Transferring Domain Ownership

You can transfer ownership of a domain to another address:

```rust,no_run
use neo3::prelude::*;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo N3 TestNet node
    let provider = Provider::new_http("https://testnet1.neo.coz.io:443");
    
    // Load your wallet
    let wallet_path = Path::new("my-wallet.json");
    let password = "my-secure-password";
    let wallet = Wallet::load(wallet_path, password)?;
    
    // Get the account that owns the domain
    let account = wallet.default_account()?;
    
    // Create an NNS instance
    let nns = NameService::new(provider.clone());
    
    // Domain name
    let domain = "example.neo";
    
    // New owner address
    let new_owner = "NZNos2WqTbu5oCgyfss9kUJgBXJqhuYAaj".parse::<Address>()?;
    
    // Transfer ownership
    let txid = nns.transfer(account, domain, new_owner).await?;
    
    println!("Domain transfer initiated with transaction ID: {}", txid);
    
    // Wait for the transaction to be confirmed
    let receipt = provider.wait_for_transaction(&txid, 60, 2).await?;
    println!("Domain transfer confirmed: {:?}", receipt);
    
    Ok(())
}
```

## Using NNS in Applications

You can integrate NNS resolution into your applications to allow users to use domain names instead of addresses:

```rust,no_run
use neo3::prelude::*;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo N3 TestNet node
    let provider = Provider::new_http("https://testnet1.neo.coz.io:443");
    
    // Load your wallet
    let wallet_path = Path::new("my-wallet.json");
    let password = "my-secure-password";
    let wallet = Wallet::load(wallet_path, password)?;
    
    // Get the account that will send tokens
    let account = wallet.default_account()?;
    
    // Create an NNS instance
    let nns = NameService::new(provider.clone());
    
    // Create a GAS token instance
    let gas_token = GasToken::new(provider.clone());
    
    // Domain or address input from user
    let recipient_input = "example.neo";
    
    // Determine if input is a domain or address
    let recipient_address = if recipient_input.ends_with(".neo") {
        // Resolve domain to address
        match nns.resolve_address(recipient_input).await? {
            Some(addr) => addr,
            None => {
                println!("Could not resolve domain {}", recipient_input);
                return Ok(());
            }
        }
    } else {
        // Parse as address directly
        recipient_input.parse::<Address>()?
    };
    
    // Amount to transfer
    let amount = 1_00000000; // 1 GAS (with 8 decimals)
    
    // Transfer GAS
    let txid = gas_token.transfer(account, recipient_address, amount, None).await?;
    println!("Transfer sent to {} with transaction ID: {}", recipient_input, txid);
    
    Ok(())
}
```

## Best Practices

1. **Check Domain Availability**: Always check if a domain is available before attempting to register it.
2. **Monitor Expiration**: Keep track of domain expiration dates and renew domains before they expire.
3. **Secure Ownership**: Ensure that the account owning valuable domains is properly secured.
4. **Validate Input**: When accepting domain names as input, validate them before attempting to resolve.
5. **Handle Resolution Failures**: Always handle cases where domain resolution fails gracefully.
6. **Test on TestNet**: Always test your NNS operations on TestNet before moving to MainNet.

<!-- toc -->
