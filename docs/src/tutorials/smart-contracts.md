# Smart Contracts

This tutorial covers working with smart contracts on the Neo N3 blockchain using the NeoRust SDK.

## Understanding Neo Smart Contracts

Neo N3 smart contracts are written in a variety of languages including C#, Python, Go, and TypeScript, and are compiled to NeoVM bytecode. The NeoRust SDK provides tools for deploying and interacting with these contracts.

## Deploying a Smart Contract

To deploy a smart contract, you need the contract's NEF (Neo Executable Format) file and its manifest:

```rust,no_run
use neo3::prelude::*;
use std::path::Path;
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo N3 TestNet node
    let provider = Provider::new_http("https://testnet1.neo.coz.io:443");
    
    // Load your wallet
    let wallet_path = Path::new("my-wallet.json");
    let password = "my-secure-password";
    let wallet = Wallet::load(wallet_path, password)?;
    
    // Get the account that will deploy the contract
    let account = wallet.default_account()?;
    
    // Read the NEF file and manifest
    let nef_bytes = fs::read("path/to/contract.nef")?;
    let manifest_bytes = fs::read("path/to/contract.manifest.json")?;
    
    // Create a transaction to deploy the contract
    let transaction = TransactionBuilder::new()
        .version(0)
        .nonce(rand::random::<u32>())
        .valid_until_block(provider.get_block_count().await? + 100)
        .deploy_contract(&nef_bytes, &manifest_bytes)
        .sign(account)?
        .build();
    
    // Send the transaction
    let txid = provider.send_raw_transaction(&transaction).await?;
    println!("Contract deployed with transaction ID: {}", txid);
    
    // Wait for the transaction to be confirmed
    let receipt = provider.wait_for_transaction(&txid, 60, 2).await?;
    println!("Transaction confirmed: {:?}", receipt);
    
    Ok(())
}
```

## Invoking a Smart Contract

Once a contract is deployed, you can invoke its methods:

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
    
    // Get the account that will invoke the contract
    let account = wallet.default_account()?;
    
    // Contract script hash (address)
    let contract_hash = "0x1234567890abcdef1234567890abcdef12345678".parse::<ScriptHash>()?;
    
    // Create a transaction to invoke the contract
    let transaction = TransactionBuilder::new()
        .version(0)
        .nonce(rand::random::<u32>())
        .valid_until_block(provider.get_block_count().await? + 100)
        .script(
            ScriptBuilder::new()
                .contract_call(
                    contract_hash,
                    "transfer",
                    &[
                        ContractParameter::hash160(account.address().script_hash()),
                        ContractParameter::hash160("NZNos2WqTbu5oCgyfss9kUJgBXJqhuYAaj".parse::<Address>()?),
                        ContractParameter::integer(1000),
                        ContractParameter::any(None),
                    ],
                )
                .to_array()
        )
        .sign(account)?
        .build();
    
    // Send the transaction
    let txid = provider.send_raw_transaction(&transaction).await?;
    println!("Contract invoked with transaction ID: {}", txid);
    
    // Wait for the transaction to be confirmed
    let receipt = provider.wait_for_transaction(&txid, 60, 2).await?;
    println!("Transaction confirmed: {:?}", receipt);
    
    Ok(())
}
```

## Reading Contract State

You can read the state of a contract without sending a transaction:

```rust,no_run
use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo N3 TestNet node
    let provider = Provider::new_http("https://testnet1.neo.coz.io:443");
    
    // Contract script hash (address)
    let contract_hash = "0x1234567890abcdef1234567890abcdef12345678".parse::<ScriptHash>()?;
    
    // Address to check balance for
    let address = "NZNos2WqTbu5oCgyfss9kUJgBXJqhuYAaj".parse::<Address>()?;
    
    // Invoke the contract's balanceOf method
    let result = provider.invoke_function(
        contract_hash,
        "balanceOf",
        &[ContractParameter::hash160(address.script_hash())],
        None,
    ).await?;
    
    // Parse the result
    if let Some(stack) = result.stack.first() {
        if let Some(value) = stack.as_integer() {
            println!("Balance: {}", value);
        }
    }
    
    Ok(())
}
```

## Working with NEP-17 Tokens

NEP-17 is Neo's token standard, similar to Ethereum's ERC-20. The NeoRust SDK provides a convenient way to work with NEP-17 tokens:

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
    
    // Get the account
    let account = wallet.default_account()?;
    
    // NEP-17 token contract hash
    let token_hash = "0x1234567890abcdef1234567890abcdef12345678".parse::<ScriptHash>()?;
    
    // Create a NEP-17 token instance
    let token = Nep17Contract::new(token_hash, provider.clone());
    
    // Get token information
    let symbol = token.symbol().await?;
    let decimals = token.decimals().await?;
    let total_supply = token.total_supply().await?;
    
    println!("Token: {} (Decimals: {})", symbol, decimals);
    println!("Total Supply: {}", total_supply);
    
    // Get account balance
    let balance = token.balance_of(account.address()).await?;
    println!("Your Balance: {}", balance);
    
    // Transfer tokens
    let recipient = "NZNos2WqTbu5oCgyfss9kUJgBXJqhuYAaj".parse::<Address>()?;
    let amount = 100;
    let txid = token.transfer(account, recipient, amount, None).await?;
    
    println!("Transfer sent with transaction ID: {}", txid);
    
    Ok(())
}
```

## Contract Events

Neo smart contracts can emit events that you can listen for:

```rust,no_run
use neo3::prelude::*;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo N3 TestNet node with WebSocket support
    let provider = Provider::new_ws("wss://testnet1.neo.coz.io:4443/ws").await?;
    
    // Contract script hash (address)
    let contract_hash = "0x1234567890abcdef1234567890abcdef12345678".parse::<ScriptHash>()?;
    
    // Subscribe to contract events
    let mut events = provider.subscribe_contract_event(contract_hash).await?;
    
    println!("Listening for events from contract {}...", contract_hash);
    
    // Process events as they arrive
    while let Some(event) = events.next().await {
        println!("Event received: {:?}", event);
        
        // Process specific event types
        if event.event_name == "Transfer" {
            if let Some(from) = event.state.get(0) {
                if let Some(to) = event.state.get(1) {
                    if let Some(amount) = event.state.get(2) {
                        println!("Transfer: {} from {} to {}", 
                            amount.as_integer().unwrap_or_default(),
                            from.as_address().map(|a| a.to_string()).unwrap_or_default(),
                            to.as_address().map(|a| a.to_string()).unwrap_or_default()
                        );
                    }
                }
            }
        }
    }
    
    Ok(())
}
```

## Best Practices

1. **Test on TestNet First**: Always test your contract interactions on TestNet before moving to MainNet.
2. **Gas Estimation**: Use the `estimate_gas` method to estimate the gas cost of your transactions.
3. **Error Handling**: Implement proper error handling for contract interactions.
4. **Event Monitoring**: Set up event monitoring for important contract events.
5. **Security**: Carefully review contract code and parameters before deployment or interaction.

<!-- toc -->
