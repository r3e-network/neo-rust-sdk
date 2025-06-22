# Neo X Bridge

## Overview

The Neo X Bridge enables cross-chain transfers of assets between Neo N3 and Neo X. This bridging functionality allows users to leverage both ecosystems while maintaining access to their assets.

## How the Bridge Works

The Neo X Bridge operates through a set of smart contracts deployed on both Neo N3 and Neo X. These contracts work together to:

1. Lock assets on the source chain
2. Mint equivalent tokens on the destination chain
3. Track and verify cross-chain transfers
4. Process withdrawal requests

## Supported Tokens

The bridge supports the following token types:

| Token | Neo N3 Format | Neo X Format |
|-------|--------------|--------------|
| GAS   | Native GAS   | ERC-20       |
| NEO   | Native NEO   | ERC-20       |
| NEP-17 Tokens | NEP-17 | ERC-20 |

## Bridge Operations

### Bridging from Neo N3 to Neo X

When bridging assets from Neo N3 to Neo X:

```rust,no_run
use neo3::prelude::*;
use neo3::neo_x::bridge::*;

async fn bridge_to_neox() -> Result<(), Box<dyn std::error::Error>> {
    // Create providers for both chains
    let neo_provider = Provider::new_http("https://mainnet1.neo.coz.io:443");
    let neox_provider = NeoXProvider::new_http("https://rpc.neoX.io");
    
    // Load account
    let account = Account::from_wif("your-private-key-wif")?;
    
    // Create bridge contract instance
    let bridge = NeoXBridgeContract::new(neo_provider.clone(), neox_provider.clone());
    
    // Bridge GAS to Neo X
    let amount = 1_00000000; // 1 GAS (8 decimals)
    let destination_address = "0x1234567890123456789012345678901234567890"; // Ethereum-format address
    
    let txid = bridge.bridge_to_neox(
        &account,
        BridgeToken::Gas,
        amount,
        destination_address,
    ).await?;
    
    println!("Bridge transaction initiated: {}", txid);
    
    Ok(())
}
```

### Bridging from Neo X to Neo N3

When bridging assets from Neo X to Neo N3:

```rust,no_run
use neo3::prelude::*;
use neo3::neo_x::bridge::*;

async fn bridge_to_neo() -> Result<(), Box<dyn std::error::Error>> {
    // Create providers for both chains
    let neo_provider = Provider::new_http("https://mainnet1.neo.coz.io:443");
    let neox_provider = NeoXProvider::new_http("https://rpc.neoX.io");
    
    // Load account (with Ethereum-compatible private key)
    let account = Account::from_ethereum_key("0xprivate-key-hex");
    
    // Create bridge contract instance
    let bridge = NeoXBridgeContract::new(neo_provider.clone(), neox_provider.clone());
    
    // Bridge GAS back to Neo N3
    let amount = 1_000_000_000_000_000_000u128; // 1 GAS (18 decimals on Neo X)
    let destination_address = "Neo1AbcDefGhiJklMnoPqrsTuvWxYz12345"; // Neo N3 address
    
    let txid = bridge.bridge_to_neo(
        &account,
        BridgeToken::Gas,
        amount,
        destination_address,
    ).await?;
    
    println!("Bridge transaction initiated: {}", txid);
    
    Ok(())
}
```

## Checking Bridge Status

You can monitor the status of your bridge transactions:

```rust,no_run
async fn check_bridge_status(txid: &str) -> Result<(), Box<dyn std::error::Error>> {
    let neo_provider = Provider::new_http("https://mainnet1.neo.coz.io:443");
    let neox_provider = NeoXProvider::new_http("https://rpc.neoX.io");
    
    let bridge = NeoXBridgeContract::new(neo_provider.clone(), neox_provider.clone());
    
    // Check if a transaction has been processed
    let status = bridge.get_transaction_status(txid).await?;
    println!("Bridge transaction status: {:?}", status);
    
    // For Neo N3 to Neo X transfers
    if let Some(neo_x_hash) = bridge.get_neox_transaction_hash(txid).await? {
        println!("Corresponding Neo X transaction: {}", neo_x_hash);
    }
    
    // For Neo X to Neo N3 transfers
    if let Some(neo_hash) = bridge.get_neo_transaction_hash(txid).await? {
        println!("Corresponding Neo N3 transaction: {}", neo_hash);
    }
    
    Ok(())
}
```

## Security Considerations

When using the Neo X Bridge:

1. **Verification**: Always verify destination addresses before initiating bridge transfers
2. **Waiting Time**: Bridge operations typically take several minutes to complete
3. **Gas Costs**: Ensure sufficient GAS/ETH for transaction fees on both chains
4. **Transaction Limits**: Be aware of any bridge limits for maximum transaction amounts
5. **Contract Addresses**: Only use official bridge contracts (verify addresses on Neo documentation)

## Emergency Procedures

If you encounter issues with your bridge transaction:

1. **Check Status**: Use the bridge status API to verify transaction status
2. **Transaction ID**: Keep both source chain and destination chain transaction IDs
3. **Support Channels**: Contact Neo support through official channels
4. **Recovery**: In some cases, emergency procedures may be available for stuck transactions

## Advanced Bridge Operations

### Custom Token Registration

For projects wanting to bridge custom tokens between Neo N3 and Neo X:

```rust,no_run
async fn register_custom_token() -> Result<(), Box<dyn std::error::Error>> {
    let neo_provider = Provider::new_http("https://mainnet1.neo.coz.io:443");
    let neox_provider = NeoXProvider::new_http("https://rpc.neoX.io");
    
    let bridge = NeoXBridgeContract::new(neo_provider.clone(), neox_provider.clone());
    
    // Register a custom NEP-17 token for bridging
    let admin_account = Account::from_wif("admin-private-key-wif")?;
    let nep17_contract = "0x1234567890123456789012345678901234567890"; // NEP-17 contract hash
    
    let txid = bridge.register_token(
        &admin_account,
        nep17_contract,
        "TOKEN", // Symbol
        8, // Decimals
    ).await?;
    
    println!("Token registration initiated: {}", txid);
    
    Ok(())
}
```

### Bridge Fee Management

Bridge operations may include fees:

```rust,no_run
async fn check_bridge_fees() -> Result<(), Box<dyn std::error::Error>> {
    let neo_provider = Provider::new_http("https://mainnet1.neo.coz.io:443");
    let neox_provider = NeoXProvider::new_http("https://rpc.neoX.io");
    
    let bridge = NeoXBridgeContract::new(neo_provider.clone(), neox_provider.clone());
    
    // Get current bridge fees
    let fees = bridge.get_bridge_fees().await?;
    
    println!("Current bridge fees:");
    println!("Neo N3 to Neo X: {}", fees.neo_to_neox);
    println!("Neo X to Neo N3: {}", fees.neox_to_neo);
    
    Ok(())
}
```