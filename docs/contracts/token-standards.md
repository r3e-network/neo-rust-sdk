# Neo N3 Token Standards

## Overview

Neo N3 provides standard interfaces for token contracts, similar to Ethereum's ERC standards. These standards ensure that tokens on the Neo blockchain maintain consistent behavior and compatibility with wallets, exchanges, and other applications.

## NEP-17: Fungible Tokens

NEP-17 is the standard for fungible tokens on Neo N3, similar to Ethereum's ERC-20 standard. Fungible tokens are interchangeable with each other, like currencies.

### Standard Interface

| Method | Parameters | Return Type | Description |
| ------ | ---------- | ----------- | ----------- |
| `symbol` | None | String | Returns the token symbol |
| `decimals` | None | Integer | Returns the number of decimal places |
| `totalSupply` | None | Integer | Returns the total token supply |
| `balanceOf` | Hash160 (account) | Integer | Returns the token balance for an account |
| `transfer` | Hash160 (from), Hash160 (to), Integer (amount), Any (data) | Boolean | Transfers tokens between accounts |

### Usage with NeoRust

```rust,no_run
use neo3::prelude::*;
use neo3::neo_contract::FungibleTokenContract;
use std::str::FromStr;

async fn nep17_example() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Neo N3
    let provider = HttpProvider::new("https://mainnet1.neo.org:443")?;
    let client = RpcClient::new(provider);
    
    // Create a reference to a NEP-17 token (GAS token used as example)
    let token_hash = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
    let token = FungibleTokenContract::new(token_hash, client.clone());
    
    // Get token information
    let symbol = token.symbol().await?;
    let decimals = token.decimals().await?;
    let total_supply = token.total_supply().await?;
    
    println!("Token: {} (Decimals: {})", symbol, decimals);
    println!("Total Supply: {}", total_supply);
    
    // Get balance for an account
    let account_hash = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
    let balance = token.balance_of(&account_hash).await?;
    println!("Account Balance: {}", balance);
    
    // Transfer tokens (requires account with private key)
    // let account = Account::from_wif("your-wif-here")?;
    // let recipient = ScriptHash::from_str("5c9c3a340f4c28262e7042b908b5f7e7a4bcd7e7")?;
    // let amount = 1_0000_0000; // 1 GAS (with 8 decimals)
    // let tx_hash = token.transfer(&account, &recipient, amount, None).await?;
    // println!("Transfer Transaction: {}", tx_hash);
    
    Ok(())
}
```

### Built-in NEP-17 Tokens

Neo N3 includes two native NEP-17 tokens:

#### NEO Token

The NEO token represents ownership in the Neo network and grants voting rights:

```rust,no_run
let neo_token = NeoToken::new(&client);
let balance = neo_token.balance_of(&account_hash).await?;
let voting_power = neo_token.get_unclaimed_gas(&account_hash, block_height).await?;
```

#### GAS Token

The GAS token is used to pay for transaction fees and smart contract execution:

```rust,no_run
let gas_token = GasToken::new(&client);
let balance = gas_token.balance_of(&account_hash).await?;
```

## NEP-11: Non-Fungible Tokens

NEP-11 is the standard for non-fungible tokens on Neo N3, similar to Ethereum's ERC-721 standard. Non-fungible tokens represent unique assets.

### Standard Interface

| Method | Parameters | Return Type | Description |
| ------ | ---------- | ----------- | ----------- |
| `symbol` | None | String | Returns the token symbol |
| `totalSupply` | None | Integer | Returns total number of tokens |
| `balanceOf` | Hash160 (account) | Integer | Returns number of tokens owned by account |
| `ownerOf` | ByteArray (tokenId) | Hash160 | Returns the owner of a token |
| `tokens` | None | Iterator | Returns an iterator of all token IDs |
| `tokensOf` | Hash160 (account) | Iterator | Returns an iterator of token IDs owned by account |
| `transfer` | Hash160 (from), Hash160 (to), ByteArray (tokenId), Any (data) | Boolean | Transfers a token |
| `properties` | ByteArray (tokenId) | Map | Returns token properties |

### Usage with NeoRust

```rust,no_run
use neo3::prelude::*;
use neo3::neo_contract::NftContract;
use std::str::FromStr;

async fn nep11_example() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Neo N3
    let provider = HttpProvider::new("https://mainnet1.neo.org:443")?;
    let client = RpcClient::new(provider);
    
    // Create a reference to a NEP-11 token
    let nft_hash = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
    let nft = NftContract::new(nft_hash, client.clone());
    
    // Get token information
    let symbol = nft.symbol().await?;
    let total_supply = nft.total_supply().await?;
    
    println!("NFT: {}", symbol);
    println!("Total Supply: {}", total_supply);
    
    // Get token owner
    let token_id = "token123".as_bytes().to_vec();
    if let Ok(owner) = nft.owner_of(&token_id).await {
        println!("Token Owner: {}", owner);
        
        // Get token properties
        let properties = nft.properties(&token_id).await?;
        println!("Token Properties: {:?}", properties);
    }
    
    // Get tokens for an account
    let account_hash = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
    let tokens = nft.tokens_of(&account_hash).await?;
    println!("Account has {} tokens", tokens.len());
    
    for token_id in tokens {
        println!("Token ID: {:?}", token_id);
    }
    
    // Transfer a token (requires account with private key)
    // let account = Account::from_wif("your-wif-here")?;
    // let recipient = ScriptHash::from_str("5c9c3a340f4c28262e7042b908b5f7e7a4bcd7e7")?;
    // let token_id = "token123".as_bytes().to_vec();
    // let tx_hash = nft.transfer(&account, &recipient, &token_id, None).await?;
    // println!("Transfer Transaction: {}", tx_hash);
    
    Ok(())
}
```

## NEP-11 Divisible Tokens

NEP-11 supports divisible NFTs, which can be partially owned by multiple accounts. These add additional methods to the standard:

| Method | Parameters | Return Type | Description |
| ------ | ---------- | ----------- | ----------- |
| `balanceOf` | Hash160 (account), ByteArray (tokenId) | Integer | Returns account's balance of a specific token |
| `transfer` | Hash160 (from), Hash160 (to), Integer (amount), ByteArray (tokenId), Any (data) | Boolean | Transfers a portion of a token |

## Events

Both NEP-17 and NEP-11 standards define events that must be emitted by compliant contracts:

### NEP-17 Events

```
Transfer(Hash160 from, Hash160 to, Integer amount)
```

### NEP-11 Events

```
Transfer(Hash160 from, Hash160 to, ByteArray tokenId, Any data)
```

## Custom Token Implementation

To implement a custom token contract, you'll need to develop a Neo N3 smart contract that complies with the relevant NEP standard. This requires:

1. Implementing all standard methods defined by the NEP
2. Emitting the required events
3. Handling token transfers correctly, including access control
4. Properly managing token balances and supply

## Common Operations

### Sending Tokens

```rust,no_run
// For NEP-17
let tx_hash = token.transfer(&account, &recipient, amount, None).await?;

// For NEP-11
let tx_hash = nft.transfer(&account, &recipient, &token_id, None).await?;
```

### Checking Token Balances

```rust,no_run
// For NEP-17
let balance = token.balance_of(&account_hash).await?;

// For NEP-11 (number of tokens owned)
let balance = nft.balance_of(&account_hash).await?;

// For divisible NEP-11 (balance of specific token)
let balance = nft.balance_of_token(&account_hash, &token_id).await?;
```

### Finding Token Information

```rust,no_run
// For both NEP-17 and NEP-11
let symbol = token.symbol().await?;
let total_supply = token.total_supply().await?;

// For NEP-17 only
let decimals = token.decimals().await?;

// For NEP-11 only
let properties = nft.properties(&token_id).await?;
```