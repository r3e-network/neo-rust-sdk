# Neo X Integration

This tutorial covers working with Neo X, an EVM-compatible chain maintained by Neo, using the NeoRust SDK.

## Understanding Neo X

Neo X is an EVM-compatible chain maintained by the Neo ecosystem. It provides Ethereum compatibility while leveraging Neo's infrastructure and security. Key features include:

- **EVM Compatibility**: Run Ethereum smart contracts and use Ethereum tools
- **Bridge Functionality**: Transfer tokens between Neo N3 and Neo X
- **Shared Security**: Benefit from Neo's consensus mechanism
- **Cross-Chain Interoperability**: Interact with both Neo and Ethereum ecosystems

## Setting Up Neo X Provider

To interact with Neo X, you first need to create a Neo X provider:

```rust,no_run
use neo3::prelude::*;
use neo3::neo_clients::{HttpProvider, RpcClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo X node
    // Note: Neo X provider functionality is currently under development
    let http_provider = HttpProvider::new_http("https://rpc.neoX.io");
    let rpc_client = RpcClient::new(http_provider);
    
    // Example of getting block information from Neo X
    // This demonstrates how to interact with Neo X RPC endpoints
    
    Ok(())
}
```

## Creating and Sending Neo X Transactions

You can create and send transactions on the Neo X chain:

```rust,no_run
use neo3::prelude::*;
use neo3::neo_clients::{HttpProvider, RpcClient};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo X node
    let provider = NeoXProvider::new_http("https://rpc.neoX.io");
    
    // Load your wallet
    let wallet_path = Path::new("my-wallet.json");
    let password = "my-secure-password";
    let wallet = Wallet::load(wallet_path, password)?;
    
    // Get the account that will send the transaction
    let account = wallet.default_account()?;
    
    // Create a transaction
    let transaction = NeoXTransaction::new()
        .to("0x1234567890123456789012345678901234567890")
        .value(1_000_000_000_000_000_000u128) // 1 ETH in wei
        .gas_price(20_000_000_000u64) // 20 Gwei
        .gas_limit(21_000u64)
        .nonce(provider.get_transaction_count(account.address().to_eth_address(), None).await?)
        .chain_id(provider.get_chain_id().await?)
        .build();
    
    // Sign the transaction
    let signed_tx = transaction.sign(account)?;
    
    // Send the transaction
    let txid = provider.send_raw_transaction(&signed_tx).await?;
    println!("Transaction sent with ID: {}", txid);
    
    // Wait for the transaction to be confirmed
    let receipt = provider.wait_for_transaction(&txid, 60, 2).await?;
    println!("Transaction confirmed: {:?}", receipt);
    
    Ok(())
}
```

## Interacting with EVM Smart Contracts

You can interact with EVM smart contracts on Neo X:

```rust,no_run
use neo3::prelude::*;
use neo3::neo_clients::{HttpProvider, RpcClient};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo X node
    let provider = NeoXProvider::new_http("https://rpc.neoX.io");
    
    // Load your wallet
    let wallet_path = Path::new("my-wallet.json");
    let password = "my-secure-password";
    let wallet = Wallet::load(wallet_path, password)?;
    
    // Get the account that will interact with the contract
    let account = wallet.default_account()?;
    
    // ERC-20 token contract address
    let contract_address = "0x1234567890123456789012345678901234567890";
    
    // Create a contract instance
    let contract = NeoXContract::new(contract_address, provider.clone());
    
    // Call a read-only method (balanceOf)
    let balance = contract.call_read(
        "balanceOf",
        &[account.address().to_eth_address()],
    ).await?;
    
    println!("Token balance: {}", balance.as_u256().unwrap_or_default());
    
    // Call a state-changing method (transfer)
    let recipient = "0x0987654321098765432109876543210987654321";
    let amount = 1_000_000_000_000_000_000u128; // 1 token with 18 decimals
    
    let tx = contract.call_write(
        account,
        "transfer",
        &[recipient, amount.to_string()],
        None,
    ).await?;
    
    println!("Transfer transaction sent with ID: {}", tx);
    
    Ok(())
}
```

## Using the Neo X Bridge

The Neo X Bridge allows you to transfer tokens between Neo N3 and Neo X:

```rust,no_run
use neo3::prelude::*;
use neo3::neo_x::bridge::*;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Neo N3 and Neo X nodes
    let neo_provider = Provider::new_http("https://mainnet1.neo.coz.io:443");
    let neox_provider = NeoXProvider::new_http("https://rpc.neoX.io");
    
    // Load your wallet
    let wallet_path = Path::new("my-wallet.json");
    let password = "my-secure-password";
    let wallet = Wallet::load(wallet_path, password)?;
    
    // Get the account
    let account = wallet.default_account()?;
    
    // Create a bridge contract instance
    let bridge = NeoXBridgeContract::new(neo_provider.clone(), neox_provider.clone());
    
    // Bridge GAS from Neo N3 to Neo X
    let amount = 1_00000000; // 1 GAS (with 8 decimals)
    
    let txid = bridge.bridge_to_neox(
        account,
        BridgeToken::Gas,
        amount,
        account.address().to_eth_address(),
    ).await?;
    
    println!("Bridge transaction sent with ID: {}", txid);
    
    // Wait for the transaction to be confirmed and processed by the bridge
    println!("Waiting for bridge processing (this may take several minutes)...");
    let receipt = neo_provider.wait_for_transaction(&txid, 300, 2).await?;
    println!("Bridge transaction confirmed on Neo N3: {:?}", receipt);
    
    // Check if tokens were received on Neo X
    // Note: There might be a delay before tokens appear on Neo X
    let erc20_address = bridge.get_neox_token_address(BridgeToken::Gas).await?;
    let contract = NeoXContract::new(erc20_address, neox_provider.clone());
    
    let balance = contract.call_read(
        "balanceOf",
        &[account.address().to_eth_address()],
    ).await?;
    
    println!("Bridged GAS balance on Neo X: {}", balance.as_u256().unwrap_or_default());
    
    Ok(())
}
```

## Bridging Tokens from Neo X to Neo N3

You can also bridge tokens from Neo X back to Neo N3:

```rust,no_run
use neo3::prelude::*;
use neo3::neo_x::bridge::*;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Neo N3 and Neo X nodes
    let neo_provider = Provider::new_http("https://mainnet1.neo.coz.io:443");
    let neox_provider = NeoXProvider::new_http("https://rpc.neoX.io");
    
    // Load your wallet
    let wallet_path = Path::new("my-wallet.json");
    let password = "my-secure-password";
    let wallet = Wallet::load(wallet_path, password)?;
    
    // Get the account
    let account = wallet.default_account()?;
    
    // Create a bridge contract instance
    let bridge = NeoXBridgeContract::new(neo_provider.clone(), neox_provider.clone());
    
    // Bridge GAS from Neo X to Neo N3
    let amount = 1_000_000_000_000_000_000u128; // 1 GAS (with 18 decimals on Neo X)
    
    let txid = bridge.bridge_to_neo(
        account,
        BridgeToken::Gas,
        amount,
        account.address(),
    ).await?;
    
    println!("Bridge transaction sent with ID: {}", txid);
    
    // Wait for the transaction to be confirmed and processed by the bridge
    println!("Waiting for bridge processing (this may take several minutes)...");
    let receipt = neox_provider.wait_for_transaction(&txid, 300, 2).await?;
    println!("Bridge transaction confirmed on Neo X: {:?}", receipt);
    
    // Check if tokens were received on Neo N3
    // Note: There might be a delay before tokens appear on Neo N3
    let gas_token = GasToken::new(neo_provider.clone());
    let balance = gas_token.balance_of(account.address()).await?;
    
    println!("GAS balance on Neo N3: {}", balance);
    
    Ok(())
}
```

## Monitoring Bridge Events

You can monitor bridge events to track token transfers between chains:

```rust,no_run
use neo3::prelude::*;
use neo3::neo_x::bridge::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Neo N3 and Neo X nodes with WebSocket support
    let neo_provider = Provider::new_ws("wss://mainnet1.neo.coz.io:4443/ws").await?;
    let neox_provider = NeoXProvider::new_ws("wss://ws.neoX.io").await?;
    
    // Create a bridge contract instance
    let bridge = NeoXBridgeContract::new(neo_provider.clone(), neox_provider.clone());
    
    // Subscribe to bridge events on Neo N3
    let mut neo_events = bridge.subscribe_neo_events().await?;
    
    println!("Listening for bridge events on Neo N3...");
    
    // Process Neo N3 events in a separate task
    tokio::spawn(async move {
        while let Some(event) = neo_events.next().await {
            println!("Neo N3 Bridge Event: {:?}", event);
            
            if event.event_name == "TokensLocked" {
                if let Some(from) = event.state.get(0) {
                    if let Some(to) = event.state.get(1) {
                        if let Some(amount) = event.state.get(2) {
                            if let Some(token) = event.state.get(3) {
                                println!("Tokens Locked: {} {} from {} to {}", 
                                    amount.as_integer().unwrap_or_default(),
                                    token.as_string().unwrap_or_default(),
                                    from.as_address().map(|a| a.to_string()).unwrap_or_default(),
                                    to.as_string().unwrap_or_default()
                                );
                            }
                        }
                    }
                }
            }
        }
    });
    
    // Subscribe to bridge events on Neo X
    let mut neox_events = bridge.subscribe_neox_events().await?;
    
    println!("Listening for bridge events on Neo X...");
    
    // Process Neo X events
    while let Some(event) = neox_events.next().await {
        println!("Neo X Bridge Event: {:?}", event);
        
        if event.event_name == "TokensUnlocked" {
            println!("Tokens Unlocked: from {} to {} amount {}", 
                event.get_param("from").unwrap_or_default(),
                event.get_param("to").unwrap_or_default(),
                event.get_param("amount").unwrap_or_default()
            );
        }
    }
    
    Ok(())
}
```

## Best Practices

1. **Gas Management**: Be aware of gas costs on Neo X, which follow Ethereum's gas model.
2. **Bridge Delays**: Expect delays when bridging tokens between chains, as cross-chain operations require confirmations on both chains.
3. **Address Formats**: Remember that Neo N3 and Neo X use different address formats. Use the appropriate conversion methods.
4. **Security**: Always verify addresses and amounts before sending transactions or bridging tokens.
5. **Testing**: Test bridge operations with small amounts before transferring larger values.
6. **Error Handling**: Implement proper error handling for both Neo N3 and Neo X operations.
7. **Monitoring**: Set up monitoring for bridge events to track the status of cross-chain transfers.

<!-- toc -->
