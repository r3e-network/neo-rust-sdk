# NEP-17 Tokens

This tutorial covers working with NEP-17 tokens on the Neo N3 blockchain using the NeoRust SDK.

## Understanding NEP-17

NEP-17 is Neo's token standard, similar to Ethereum's ERC-20. It defines a standard interface for fungible tokens on the Neo blockchain. NEP-17 tokens have the following key methods:

- `symbol`: Returns the token's symbol
- `decimals`: Returns the number of decimal places the token uses
- `totalSupply`: Returns the total token supply
- `balanceOf`: Returns the token balance of a specific address
- `transfer`: Transfers tokens from one address to another

## Creating a NEP-17 Token Instance

To interact with a NEP-17 token, you first need to create a token instance:

```rust,edition2021
use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo N3 TestNet node
    let provider = Provider::new_http("https://testnet1.neo.coz.io:443");
    
    // NEP-17 token contract hash (e.g., GAS token)
    let gas_hash = "0xd2a4cff31913016155e38e474a2c06d08be276cf".parse::<ScriptHash>()?;
    
    // Create a NEP-17 token instance
    let gas_token = Nep17Contract::new(gas_hash, provider.clone());
    
    Ok(())
}
```

## Getting Token Information

You can retrieve basic information about a token:

```rust,edition2021
use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo N3 TestNet node
    let provider = Provider::new_http("https://testnet1.neo.coz.io:443");
    
    // NEP-17 token contract hash (e.g., GAS token)
    let gas_hash = "0xd2a4cff31913016155e38e474a2c06d08be276cf".parse::<ScriptHash>()?;
    
    // Create a NEP-17 token instance
    let gas_token = Nep17Contract::new(gas_hash, provider.clone());
    
    // Get token information
    let symbol = gas_token.symbol().await?;
    let decimals = gas_token.decimals().await?;
    let total_supply = gas_token.total_supply().await?;
    
    println!("Token: {} (Decimals: {})", symbol, decimals);
    println!("Total Supply: {}", total_supply);
    
    Ok(())
}
```

## Checking Token Balance

You can check the token balance of an address:

```rust,edition2021
use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo N3 TestNet node
    let provider = Provider::new_http("https://testnet1.neo.coz.io:443");
    
    // NEP-17 token contract hash (e.g., GAS token)
    let gas_hash = "0xd2a4cff31913016155e38e474a2c06d08be276cf".parse::<ScriptHash>()?;
    
    // Create a NEP-17 token instance
    let gas_token = Nep17Contract::new(gas_hash, provider.clone());
    
    // Address to check balance for
    let address = "NZNos2WqTbu5oCgyfss9kUJgBXJqhuYAaj".parse::<Address>()?;
    
    // Get token balance
    let balance = gas_token.balance_of(address).await?;
    
    println!("Balance: {}", balance);
    
    // For better display, consider the token's decimals
    let decimals = gas_token.decimals().await?;
    let formatted_balance = balance as f64 / 10f64.powi(decimals as i32);
    
    println!("Formatted Balance: {} {}", formatted_balance, gas_token.symbol().await?);
    
    Ok(())
}
```

## Transferring Tokens

You can transfer tokens from one address to another:

```rust,edition2021
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
    
    // Get the account that will send the tokens
    let account = wallet.default_account()?;
    
    // NEP-17 token contract hash (e.g., GAS token)
    let gas_hash = "0xd2a4cff31913016155e38e474a2c06d08be276cf".parse::<ScriptHash>()?;
    
    // Create a NEP-17 token instance
    let gas_token = Nep17Contract::new(gas_hash, provider.clone());
    
    // Recipient address
    let recipient = "NZNos2WqTbu5oCgyfss9kUJgBXJqhuYAaj".parse::<Address>()?;
    
    // Amount to transfer (considering decimals)
    let decimals = gas_token.decimals().await?;
    let amount = 1 * 10i64.pow(decimals as u32); // 1 token with proper decimal places
    
    // Transfer tokens
    let txid = gas_token.transfer(account, recipient, amount, None).await?;
    
    println!("Transfer sent with transaction ID: {}", txid);
    
    // Wait for the transaction to be confirmed
    let receipt = provider.wait_for_transaction(&txid, 60, 2).await?;
    println!("Transaction confirmed: {:?}", receipt);
    
    Ok(())
}
```

## Working with NEO and GAS Tokens

The NeoRust SDK provides specialized classes for the native NEO and GAS tokens:

```rust,edition2021
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
    
    // Create NEO token instance
    let neo_token = NeoToken::new(provider.clone());
    
    // Create GAS token instance
    let gas_token = GasToken::new(provider.clone());
    
    // Get NEO balance
    let neo_balance = neo_token.balance_of(account.address()).await?;
    println!("NEO Balance: {}", neo_balance);
    
    // Get GAS balance
    let gas_balance = gas_token.balance_of(account.address()).await?;
    println!("GAS Balance: {}", gas_balance);
    
    // Transfer NEO
    let recipient = "NZNos2WqTbu5oCgyfss9kUJgBXJqhuYAaj".parse::<Address>()?;
    let neo_amount = 1_00000000; // 1 NEO (with 8 decimals)
    
    let neo_txid = neo_token.transfer(account, recipient, neo_amount, None).await?;
    println!("NEO transfer sent with transaction ID: {}", neo_txid);
    
    // Transfer GAS
    let gas_amount = 1_00000000; // 1 GAS (with 8 decimals)
    
    let gas_txid = gas_token.transfer(account, recipient, gas_amount, None).await?;
    println!("GAS transfer sent with transaction ID: {}", gas_txid);
    
    Ok(())
}
```

## Monitoring Token Transfers

You can monitor token transfers by subscribing to the Transfer event:

```rust,edition2021
use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo N3 TestNet node with WebSocket support
    let provider = Provider::new_ws("wss://testnet1.neo.coz.io:4443/ws").await?;
    
    // NEP-17 token contract hash (e.g., GAS token)
    let gas_hash = "0xd2a4cff31913016155e38e474a2c06d08be276cf".parse::<ScriptHash>()?;
    
    // Subscribe to Transfer events
    let mut events = provider.subscribe_contract_event(gas_hash).await?;
    
    println!("Listening for token transfers...");
    
    // Process events as they arrive
    while let Some(event) = events.next().await {
        if event.event_name == "Transfer" {
            if let Some(from) = event.state.get(0) {
                if let Some(to) = event.state.get(1) {
                    if let Some(amount) = event.state.get(2) {
                        println!("Transfer: {} tokens from {} to {}", 
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

## Working with Famous Neo N3 Contracts

The NeoRust SDK provides direct support for several famous Neo N3 contracts:

### Flamingo Finance

```rust,edition2021
use neo3::prelude::*;
use neo3::neo_contract::famous::flamingo::*;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo N3 MainNet node
    let provider = Provider::new_http("https://mainnet1.neo.coz.io:443");
    
    // Load your wallet
    let wallet_path = Path::new("my-wallet.json");
    let password = "my-secure-password";
    let wallet = Wallet::load(wallet_path, password)?;
    
    // Get the account
    let account = wallet.default_account()?;
    
    // Create Flamingo Finance instance
    let flamingo = FlamingoFinance::new(provider.clone());
    
    // Get FLM token balance
    let flm_balance = flamingo.flm_token().balance_of(account.address()).await?;
    println!("FLM Balance: {}", flm_balance);
    
    // Get liquidity pool information
    let pool_info = flamingo.get_pool_info(FlamingoPool::NeoGas).await?;
    println!("Pool Info: {:?}", pool_info);
    
    // Add liquidity to a pool
    let neo_amount = 1_00000000; // 1 NEO
    let gas_amount = 1_00000000; // 1 GAS
    
    let add_liquidity_txid = flamingo.add_liquidity(
        account,
        FlamingoPool::NeoGas,
        neo_amount,
        gas_amount,
        None,
    ).await?;
    
    println!("Add Liquidity transaction ID: {}", add_liquidity_txid);
    
    Ok(())
}
```

### NeoburgerNeo

```rust,edition2021
use neo3::prelude::*;
use neo3::neo_contract::famous::neoburger::*;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo N3 MainNet node
    let provider = Provider::new_http("https://mainnet1.neo.coz.io:443");
    
    // Load your wallet
    let wallet_path = Path::new("my-wallet.json");
    let password = "my-secure-password";
    let wallet = Wallet::load(wallet_path, password)?;
    
    // Get the account
    let account = wallet.default_account()?;
    
    // Create NeoburgerNeo instance
    let neoburger = NeoburgerNeo::new(provider.clone());
    
    // Get bNEO token balance
    let bneo_balance = neoburger.bneo_token().balance_of(account.address()).await?;
    println!("bNEO Balance: {}", bneo_balance);
    
    // Wrap NEO to get bNEO
    let neo_amount = 1_00000000; // 1 NEO
    let wrap_txid = neoburger.wrap_neo(account, neo_amount).await?;
    println!("Wrap NEO transaction ID: {}", wrap_txid);
    
    // Unwrap bNEO to get NEO
    let bneo_amount = 1_00000000; // 1 bNEO
    let unwrap_txid = neoburger.unwrap_bneo(account, bneo_amount).await?;
    println!("Unwrap bNEO transaction ID: {}", unwrap_txid);
    
    Ok(())
}
```

## Best Practices

1. **Check Balances Before Transfers**: Always check that an account has sufficient balance before attempting a transfer.
2. **Consider Decimals**: Remember to account for token decimals when displaying balances or specifying transfer amounts.
3. **Wait for Confirmations**: Always wait for transaction confirmations before considering a transfer complete.
4. **Error Handling**: Implement proper error handling for token operations.
5. **Gas Costs**: Be aware of the gas costs associated with token transfers and other operations.
6. **Test on TestNet**: Always test your token operations on TestNet before moving to MainNet.

<!-- toc -->
