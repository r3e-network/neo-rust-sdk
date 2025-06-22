# EVM Contracts on Neo X

## Overview

Neo X supports Ethereum Virtual Machine (EVM) compatible smart contracts, allowing developers to deploy and interact with contracts written in Solidity or other EVM-compatible languages. This document covers the process of working with EVM contracts on Neo X using the NeoRust SDK.

## Contract Deployment

### Deploying a New Contract

To deploy a new EVM contract on Neo X:

```rust,no_run
use neo3::prelude::*;
use neo3::neo_x::evm::*;

async fn deploy_contract() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Neo X
    let provider = NeoXProvider::new_http("https://rpc.neoX.io");
    
    // Load account
    let account = Account::from_private_key("0xprivate-key-hex")?;
    
    // Get the compiled contract bytecode
    let bytecode = "0x608060405234801561001057600080fd5b50610150806100206000396000f3fe608060405234801561001057600080fd5b50600436106100365760003560e01c80632e64cec11461003b5780636057361d14610059575b600080fd5b610043610075565b60405161005091906100d9565b60405180910390f35b610073600480360381019061006e919061009d565b61007e565b005b60008054905090565b8060008190555050565b60008135905061009781610103565b92915050565b6000602082840312156100b3576100b26100fe565b5b60006100c184828501610088565b91505092915050565b6100d3816100f4565b82525050565b60006020820190506100ee60008301846100ca565b92915050565b6000819050919050565b600080fd5b61010c816100f4565b811461011757600080fd5b5056fea2646970667358221220404e37f487a89a932dca5e7ddbe4e10b5a5826a70cddebd3d874726940a6160564736f6c63430008070033";
    
    // Deploy the contract (with constructor arguments if needed)
    let deploy_tx = NeoXTransaction::new()
        .data(bytecode)
        .gas_limit(3_000_000) // Sufficient gas for deployment
        .gas_price(20_000_000_000u64) // 20 Gwei
        .nonce(provider.get_transaction_count(account.address().to_eth_address(), None).await?)
        .chain_id(provider.get_chain_id().await?)
        .value(0) // No ETH being sent with deployment
        .build();
    
    // Sign and send the transaction
    let signed_tx = deploy_tx.sign(&account)?;
    let tx_hash = provider.send_raw_transaction(&signed_tx).await?;
    
    println!("Contract deployment transaction sent: {}", tx_hash);
    
    // Wait for deployment to be confirmed
    let receipt = provider.wait_for_transaction(&tx_hash, 60, 2).await?;
    
    if let Some(contract_address) = receipt.contract_address {
        println!("Contract deployed at: {}", contract_address);
    } else {
        println!("Deployment failed");
    }
    
    Ok(())
}
```

### Deploying with Constructor Arguments

If your contract requires constructor arguments:

```rust,no_run
// Prepare constructor arguments (example for a token contract)
let name = "MyToken";
let symbol = "MTK";
let decimals = 18u8;
let total_supply = 1_000_000_000_000_000_000_000_000u128; // 1 million tokens

// ABI encode the constructor arguments
let encoded_args = encode_constructor_args(&[
    name.to_string(),
    symbol.to_string(),
    decimals.to_string(),
    total_supply.to_string()
])?;

// Append encoded args to bytecode
let deploy_data = format!("{}{}", bytecode, encoded_args);

// Use deploy_data in the transaction
```

## Contract Interaction

### Reading Contract State

To call read-only (view/pure) functions:

```rust,no_run
use neo3::prelude::*;
use neo3::neo_x::evm::*;

async fn read_contract() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Neo X
    let provider = NeoXProvider::new_http("https://rpc.neoX.io");
    
    // Contract address
    let contract_address = "0x1234567890123456789012345678901234567890";
    
    // Create contract instance
    let contract = NeoXContract::new(contract_address, provider.clone());
    
    // Call a read method
    let balance = contract.call_read(
        "balanceOf",
        &["0x9876543210987654321098765432109876543210"],
    ).await?;
    
    // Process the result
    let balance_value = balance.as_u256()?;
    println!("Balance: {}", balance_value);
    
    // Call with multiple parameters
    let allowance = contract.call_read(
        "allowance",
        &[
            "0x9876543210987654321098765432109876543210",
            "0xabcdef1234567890abcdef1234567890abcdef12",
        ],
    ).await?;
    
    println!("Allowance: {}", allowance.as_u256()?);
    
    Ok(())
}
```

### Modifying Contract State

To call state-changing functions:

```rust,no_run
use neo3::prelude::*;
use neo3::neo_x::evm::*;

async fn write_contract() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Neo X
    let provider = NeoXProvider::new_http("https://rpc.neoX.io");
    
    // Load account
    let account = Account::from_private_key("0xprivate-key-hex")?;
    
    // Contract address
    let contract_address = "0x1234567890123456789012345678901234567890";
    
    // Create contract instance
    let contract = NeoXContract::new(contract_address, provider.clone());
    
    // Call a write method (ERC-20 transfer example)
    let recipient = "0x9876543210987654321098765432109876543210";
    let amount = 1_000_000_000_000_000_000u128; // 1 token with 18 decimals
    
    let options = CallOptions {
        gas_limit: Some(100_000),
        gas_price: Some(20_000_000_000u64), // 20 Gwei
        value: None, // No ETH being sent with call
    };
    
    let tx_hash = contract.call_write(
        &account,
        "transfer",
        &[recipient, amount.to_string()],
        Some(options),
    ).await?;
    
    println!("Transaction sent: {}", tx_hash);
    
    // Wait for transaction confirmation
    let receipt = provider.wait_for_transaction(&tx_hash, 60, 2).await?;
    println!("Transaction confirmed: {:?}", receipt);
    
    Ok(())
}
```

## Events and Logs

### Querying Contract Events

To query events emitted by contracts:

```rust,no_run
use neo3::prelude::*;
use neo3::neo_x::evm::*;

async fn query_events() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Neo X
    let provider = NeoXProvider::new_http("https://rpc.neoX.io");
    
    // Contract address
    let contract_address = "0x1234567890123456789012345678901234567890";
    
    // Create filter for Transfer events (ERC-20 example)
    let filter = EventFilter::new()
        .address(contract_address)
        .event("Transfer(address,address,uint256)")
        .from_block(1_000_000)
        .to_block(BlockId::Latest);
    
    // Query events
    let events = provider.get_logs(&filter).await?;
    
    // Process events
    for event in events {
        let from = event.topics.get(1).map(|t| format!("0x{}", hex::encode(&t[12..])));
        let to = event.topics.get(2).map(|t| format!("0x{}", hex::encode(&t[12..])));
        let amount = parse_event_data_as_uint(&event.data)?;
        
        println!("Transfer: {} from {} to {}", amount, from.unwrap_or_default(), to.unwrap_or_default());
    }
    
    Ok(())
}
```

## Advanced Contract Operations

### Working with Contract ABIs

For more robust contract interaction, you can use contract ABIs:

```rust,no_run
use neo3::prelude::*;
use neo3::neo_x::evm::*;

async fn use_contract_abi() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Neo X
    let provider = NeoXProvider::new_http("https://rpc.neoX.io");
    
    // Contract address
    let contract_address = "0x1234567890123456789012345678901234567890";
    
    // Define contract ABI (typically loaded from a JSON file)
    let abi = r#"[
        {
            "inputs": [
                {"internalType": "address", "name": "account", "type": "address"}
            ],
            "name": "balanceOf",
            "outputs": [
                {"internalType": "uint256", "name": "", "type": "uint256"}
            ],
            "stateMutability": "view",
            "type": "function"
        },
        {
            "inputs": [
                {"internalType": "address", "name": "to", "type": "address"},
                {"internalType": "uint256", "name": "amount", "type": "uint256"}
            ],
            "name": "transfer",
            "outputs": [
                {"internalType": "bool", "name": "", "type": "bool"}
            ],
            "stateMutability": "nonpayable",
            "type": "function"
        }
    ]"#;
    
    // Create contract with ABI
    let contract = NeoXContract::new_with_abi(contract_address, abi, provider.clone())?;
    
    // Call functions using the ABI
    let account_address = "0x9876543210987654321098765432109876543210";
    let balance = contract.call_abi_function(
        "balanceOf",
        &[account_address.into()],
        None,
    ).await?;
    
    println!("Balance: {}", balance.as_u256()?);
    
    Ok(())
}
```

### Interacting with Proxy Contracts

For upgradeable proxy contracts:

```rust,no_run
async fn proxy_contract_interaction() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Neo X
    let provider = NeoXProvider::new_http("https://rpc.neoX.io");
    
    // Proxy contract address
    let proxy_address = "0x1234567890123456789012345678901234567890";
    
    // Get implementation address
    let implementation = NeoXContract::new(proxy_address, provider.clone())
        .call_read("implementation", &[])
        .await?
        .as_address()?;
    
    println!("Implementation contract: {}", implementation);
    
    // Use implementation ABI with proxy address for correct interaction
    // This approach works with transparent proxies
    let abi = load_implementation_abi()?;
    let contract = NeoXContract::new_with_abi(proxy_address, abi, provider.clone())?;
    
    // Now interact with the proxy using the implementation's ABI
    let result = contract.call_abi_function("someFunction", &[param1, param2], None).await?;
    
    Ok(())
}
```

## Testing Contracts

### Using Local Development Networks

For testing before deploying to Neo X mainnet:

```rust,no_run
async fn test_with_local_node() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to local development node
    let provider = NeoXProvider::new_http("http://localhost:8545");
    
    // Use a development account
    let private_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
    let account = Account::from_private_key(private_key)?;
    
    // Deploy and test contracts as with mainnet
    // ...
    
    Ok(())
}
```