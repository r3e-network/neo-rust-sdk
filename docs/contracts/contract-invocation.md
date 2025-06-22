# Smart Contract Invocation

## Overview

Invoking smart contracts is a fundamental operation when interacting with the Neo N3 blockchain. The NeoRust SDK provides multiple approaches to call contract methods, both for state-changing operations (which require transactions) and read-only queries.

## Invocation Methods

### 1. Using Contract-Specific Interfaces

For common contracts like NEP-17 tokens or system contracts, the SDK provides dedicated interfaces:

```rust,no_run
use neo3::prelude::*;
use neo3::neo_contract::{FungibleTokenContract, GasToken, NeoToken};
use std::str::FromStr;

async fn token_interaction() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Neo N3
    let provider = HttpProvider::new("https://mainnet1.neo.org:443")?;
    let client = RpcClient::new(provider);
    
    // Using the GasToken interface (system contract)
    let gas_token = GasToken::new(&client);
    let symbol = gas_token.symbol().await?;
    let decimals = gas_token.decimals().await?;
    println!("GAS Token: {} with {} decimals", symbol, decimals);
    
    // Using the generic NEP-17 interface for any fungible token
    let flamingo_hash = ScriptHash::from_str("fb7f9d5188a8accb42fa8cacb4f5450cd5e0ac13")?;
    let flamingo = FungibleTokenContract::new(flamingo_hash, client.clone());
    
    let flm_symbol = flamingo.symbol().await?;
    let flm_decimals = flamingo.decimals().await?;
    println!("Flamingo Token: {} with {} decimals", flm_symbol, flm_decimals);
    
    Ok(())
}
```

### 2. Direct Script Invocation

For contracts without specific interfaces, you can build and invoke scripts directly:

```rust,no_run
use neo3::prelude::*;
use neo3::neo_builder::ScriptBuilder;
use std::str::FromStr;

async fn direct_invocation() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Neo N3
    let provider = HttpProvider::new("https://mainnet1.neo.org:443")?;
    let client = RpcClient::new(provider);
    
    // Contract script hash
    let contract_hash = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
    
    // Build a script to call a method
    let script = ScriptBuilder::build_contract_call(
        &contract_hash,
        "balanceOf",
        &[
            ContractParameter::hash160(
                ScriptHash::from_str("5c9c3a340f4c28262e7042b908b5f7e7a4bcd7e7")?
            ),
        ],
    )?;
    
    // Invoke the script
    let result = client.invoke_script(&script, None).await?;
    
    // Process the result
    if let Some(stack_item) = result.stack.first() {
        if let Some(value) = stack_item.as_integer() {
            println!("Balance: {}", value);
        }
    }
    
    Ok(())
}
```

### 3. RPC Method Invocation

You can also use the RPC client's convenience methods:

```rust,no_run
use neo3::prelude::*;
use std::str::FromStr;

async fn rpc_invocation() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Neo N3
    let provider = HttpProvider::new("https://mainnet1.neo.org:443")?;
    let client = RpcClient::new(provider);
    
    // Contract script hash
    let contract_hash = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
    
    // Call a contract method
    let result = client.invoke_function(
        &contract_hash,
        "symbol",
        &[],
        None,
    ).await?;
    
    // Process the result
    if let Some(stack_item) = result.stack.first() {
        if let Some(value) = stack_item.as_string() {
            println!("Token symbol: {}", value);
        }
    }
    
    Ok(())
}
```

## Read-Only vs. State-Changing Invocations

### Read-Only Invocations

Read-only invocations don't modify blockchain state and don't require transaction fees:

```rust,no_run
use neo3::prelude::*;
use neo3::neo_contract::NeoToken;

async fn read_only_example() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Neo N3
    let provider = HttpProvider::new("https://mainnet1.neo.org:443")?;
    let client = RpcClient::new(provider);
    
    // Create NeoToken instance
    let neo_token = NeoToken::new(&client);
    
    // Call read-only methods
    let total_supply = neo_token.total_supply().await?;
    let committee = neo_token.get_committee().await?;
    
    println!("NEO Total Supply: {}", total_supply);
    println!("Committee Members: {}", committee.len());
    
    Ok(())
}
```

### State-Changing Invocations

State-changing invocations modify blockchain state, require transaction fees, and must be signed:

```rust,no_run
use neo3::prelude::*;
use neo3::neo_contract::GasToken;
use std::str::FromStr;

async fn state_changing_example() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Neo N3
    let provider = HttpProvider::new("https://mainnet1.neo.org:443")?;
    let client = RpcClient::new(provider);
    
    // Load account with GAS
    let account = Account::from_wif("your-private-key-wif")?;
    
    // Create GasToken instance
    let gas_token = GasToken::new(&client);
    
    // Transfer GAS (state-changing operation)
    let recipient = ScriptHash::from_str("5c9c3a340f4c28262e7042b908b5f7e7a4bcd7e7")?;
    let amount = 1_0000_0000; // 1 GAS (with 8 decimals)
    
    let tx_hash = gas_token.transfer(
        &account,
        &recipient,
        amount,
        None, // No data
    ).await?;
    
    println!("Transfer transaction sent: {}", tx_hash);
    
    // Wait for transaction confirmation
    println!("Waiting for confirmation...");
    let tx_info = client.get_transaction(&tx_hash).await?;
    
    // Display transaction details
    println!("Transaction confirmed:");
    println!("Block Index: {}", tx_info.block_index);
    println!("System Fee: {}", tx_info.system_fee);
    println!("Network Fee: {}", tx_info.network_fee);
    
    Ok(())
}
```

## Advanced Invocation Features

### 1. Multi-Invocation Transactions

You can combine multiple contract calls in a single transaction:

```rust,no_run
use neo3::prelude::*;
use neo3::neo_builder::{ScriptBuilder, TransactionBuilder};
use std::str::FromStr;

async fn multi_invocation() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Neo N3
    let provider = HttpProvider::new("https://mainnet1.neo.org:443")?;
    let client = RpcClient::new(provider);
    
    // Load account with GAS
    let account = Account::from_wif("your-private-key-wif")?;
    
    // Contract script hashes for two tokens
    let token1_hash = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
    let token2_hash = ScriptHash::from_str("fb7f9d5188a8accb42fa8cacb4f5450cd5e0ac13")?;
    
    // Recipient
    let recipient = ScriptHash::from_str("5c9c3a340f4c28262e7042b908b5f7e7a4bcd7e7")?;
    
    // Build a script with multiple calls
    let mut script_builder = ScriptBuilder::new();
    
    // Add first token transfer
    script_builder.contract_call(
        &token1_hash,
        "transfer",
        &[
            ContractParameter::hash160(&account.get_script_hash()),
            ContractParameter::hash160(&recipient),
            ContractParameter::integer(1_0000_0000), // 1 Token1
            ContractParameter::any(None),
        ],
    )?;
    
    // Add second token transfer
    script_builder.contract_call(
        &token2_hash,
        "transfer",
        &[
            ContractParameter::hash160(&account.get_script_hash()),
            ContractParameter::hash160(&recipient),
            ContractParameter::integer(10_0000_0000), // 10 Token2
            ContractParameter::any(None),
        ],
    )?;
    
    // Build transaction
    let script = script_builder.to_bytes();
    let block_count = client.get_block_count().await?;
    
    let transaction = TransactionBuilder::new()
        .version(0)
        .nonce(rand::random::<u32>())
        .valid_until_block(block_count + 100)
        .script(script)
        .signers(vec![Signer::called_by_entry(account.get_script_hash())])
        .build();
    
    // Sign transaction
    let signed_tx = transaction.sign(&client, &account).await?;
    
    // Send transaction
    let tx_hash = client.send_raw_transaction(&signed_tx).await?;
    println!("Multi-invocation transaction sent: {}", tx_hash);
    
    Ok(())
}
```

### 2. Verifying Witness Scopes

Different witness scopes control how a signature can be used:

```rust,no_run
// Called By Entry scope - signature applies only for contract called directly
let signer = Signer::called_by_entry(account.get_script_hash());

// Global scope - signature applies for all contract calls
let signer = Signer::global(account.get_script_hash());

// Custom Groups scope - signature applies for specified contract groups
let signer = Signer::with_groups(
    account.get_script_hash(),
    vec!["group_public_key_1", "group_public_key_2"],
);

// Custom Contracts scope - signature applies for specified contracts
let signer = Signer::with_contracts(
    account.get_script_hash(),
    vec![contract_hash1, contract_hash2],
);
```

### 3. Test Invocation

Test invocation lets you see what would happen if a contract were called, without sending a transaction:

```rust,no_run
use neo3::prelude::*;
use neo3::neo_builder::ScriptBuilder;
use std::str::FromStr;

async fn test_invocation() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Neo N3
    let provider = HttpProvider::new("https://mainnet1.neo.org:443")?;
    let client = RpcClient::new(provider);
    
    // Contract script hash
    let token_hash = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
    
    // Account and recipient
    let account_hash = ScriptHash::from_str("0x1234567890123456789012345678901234567890")?;
    let recipient = ScriptHash::from_str("0x9876543210987654321098765432109876543210")?;
    
    // Build a script
    let script = ScriptBuilder::build_contract_call(
        &token_hash,
        "transfer",
        &[
            ContractParameter::hash160(&account_hash),
            ContractParameter::hash160(&recipient),
            ContractParameter::integer(1_0000_0000), // 1 Token
            ContractParameter::any(None),
        ],
    )?;
    
    // Test invoke the script
    let result = client.invoke_script(
        &script,
        Some(&[Signer::called_by_entry(account_hash)]),
    ).await?;
    
    // Check result state
    println!("Test Invocation Result:");
    println!("State: {}", result.state);
    println!("Gas Consumed: {}", result.gas_consumed);
    
    // Check for exceptions
    if result.exception.is_some() {
        println!("Exception: {}", result.exception.unwrap());
    }
    
    // Process result stack
    if let Some(stack_item) = result.stack.first() {
        if let Some(success) = stack_item.as_boolean() {
            println!("Would succeed: {}", success);
        }
    }
    
    Ok(())
}
```

### 4. Event Subscription

You can subscribe to contract events (requires a WebSocket connection):

```rust,no_run
use neo3::prelude::*;
use std::str::FromStr;

async fn event_subscription() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Neo N3 with WebSocket
    let provider = WebSocketProvider::new("wss://testnet1.neo.org:4443").await?;
    let client = RpcClient::new(provider);
    
    // Contract to monitor
    let contract_hash = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
    
    // Subscribe to contract events
    let mut events = client.subscribe_contract_event(&contract_hash).await?;
    
    println!("Listening for events from contract {}...", contract_hash);
    
    // Process events as they arrive
    while let Some(event) = events.next().await {
        println!("Event received: {} at block {}", event.event_name, event.block_index);
        
        // Process specific event types
        if event.event_name == "Transfer" {
            let from = event.state.get(0)
                .and_then(|item| item.as_hash160())
                .map(|h| h.to_string())
                .unwrap_or_default();
                
            let to = event.state.get(1)
                .and_then(|item| item.as_hash160())
                .map(|h| h.to_string())
                .unwrap_or_default();
                
            let amount = event.state.get(2)
                .and_then(|item| item.as_integer())
                .unwrap_or_default();
                
            println!("Transfer: {} tokens from {} to {}", amount, from, to);
        }
    }
    
    Ok(())
}
```

## Handling Contract Results

Contract invocations can return different types of values:

```rust,no_run
use neo3::prelude::*;
use std::str::FromStr;

async fn process_results() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Neo N3
    let provider = HttpProvider::new("https://mainnet1.neo.org:443")?;
    let client = RpcClient::new(provider);
    
    // Contract script hash
    let contract_hash = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
    
    // Invoke different methods
    let symbol_result = client.invoke_function(&contract_hash, "symbol", &[], None).await?;
    let decimals_result = client.invoke_function(&contract_hash, "decimals", &[], None).await?;
    let total_supply_result = client.invoke_function(&contract_hash, "totalSupply", &[], None).await?;
    
    // Process string result
    let symbol = symbol_result.stack.first()
        .and_then(|item| item.as_string())
        .unwrap_or_default();
    println!("Symbol: {}", symbol);
    
    // Process integer result
    let decimals = decimals_result.stack.first()
        .and_then(|item| item.as_integer())
        .unwrap_or_default();
    println!("Decimals: {}", decimals);
    
    // Process large integer result
    let total_supply = total_supply_result.stack.first()
        .and_then(|item| item.as_integer())
        .unwrap_or_default();
    println!("Total Supply: {}", total_supply);
    
    // Demonstrate processing other types of results
    let complex_result = client.invoke_function(
        &contract_hash,
        "complexMethod",
        &[],
        None,
    ).await?;
    
    if let Some(stack_item) = complex_result.stack.first() {
        match stack_item {
            StackItem::Boolean(value) => println!("Boolean: {}", value),
            StackItem::Integer(value) => println!("Integer: {}", value),
            StackItem::ByteString(bytes) => println!("ByteString: {:?}", bytes),
            StackItem::Array(items) => println!("Array with {} items", items.len()),
            StackItem::Map(map) => println!("Map with {} entries", map.len()),
            StackItem::Pointer(value) => println!("Pointer: {}", value),
            StackItem::InteropInterface => println!("InteropInterface"),
            StackItem::Any => println!("Any"),
            // Handle other types...
        }
    }
    
    Ok(())
}
```

## Best Practices

1. **Test Invocation First**: Before sending transactions, use test invocation to verify success
2. **Gas Estimation**: Use `gas_consumed` from test invocation to estimate fees
3. **Error Handling**: Always check for exceptions in invocation results
4. **Witness Scopes**: Use the most restrictive witness scope needed for your operation
5. **Batching**: Combine multiple operations in one transaction when possible
6. **Idempotency**: Design contract invocations to be idempotent when possible
7. **Timeouts**: Set appropriate timeouts for confirmation waiters
8. **Verification**: Verify transaction success by checking application logs