# Smart Contract Deployment

## Overview

Deploying a smart contract to the Neo N3 blockchain involves preparing the contract files, creating a deployment transaction, and submitting it to the network. The NeoRust SDK provides utilities to make this process straightforward.

## Prerequisites

Before deploying a contract, you need:

1. **Contract Files**: 
   - The **NEF** (Neo Executable Format) file containing the compiled contract bytecode
   - The **Manifest** JSON file describing the contract's interface and permissions

2. **Account with GAS**:
   - A Neo N3 account with sufficient GAS to pay for deployment fees

3. **Neo N3 Node Connection**:
   - A connection to a Neo N3 RPC node (TestNet recommended for initial testing)

## Contract Deployment Process

### 1. Load Contract Files

First, load your contract files:

```rust,no_run
use neo3::prelude::*;
use neo3::neo_types::{ContractManifest, NefFile};
use std::fs;

// Read NEF and manifest files
let nef_bytes = fs::read("path/to/contract.nef")?;
let manifest_json = fs::read_to_string("path/to/contract.manifest.json")?;

// Parse contract files
let nef = NefFile::from_bytes(&nef_bytes)?;
let manifest = ContractManifest::from_json(&manifest_json)?;
```

### 2. Set Up Connection and Account

Next, set up your connection to Neo N3 and prepare your account:

```rust,no_run
// Connect to Neo N3 TestNet
let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
let client = RpcClient::new(provider);

// Load your account with GAS
let account = Account::from_wif("your-private-key-wif")?;

// Check GAS balance before deployment
let gas_token = GasToken::new(&client);
let gas_balance = gas_token.balance_of(&account.get_script_hash()).await?;
println!("GAS Balance: {}", gas_balance);
```

### 3. Deploy the Contract

Use the ContractManagement system contract to deploy your contract:

```rust,no_run
use neo3::neo_contract::ContractManagement;

// Create contract management instance
let contract_mgmt = ContractManagement::new(&client);

// Deploy the contract
println!("Deploying contract...");
let result = contract_mgmt.deploy(
    &nef,
    &manifest,
    None, // Optional deployment data
    &account,
).await?;

// Display deployment results
let contract_hash = result.script_hash;
println!("Contract deployed successfully!");
println!("Contract hash: {}", contract_hash);
```

### 4. Verify Deployment

After deployment, verify that your contract is available on the blockchain:

```rust,no_run
// Get contract details
let contract_state = contract_mgmt.get_contract(&contract_hash).await?;

// Display contract information
println!("Contract ID: {}", contract_state.id);
println!("Contract Update Counter: {}", contract_state.update_counter);
println!("Contract Author: {}", contract_state.manifest.author);
```

## Complete Deployment Example

Here's a complete example of contract deployment:

```rust,no_run
use neo3::prelude::*;
use neo3::neo_contract::ContractManagement;
use neo3::neo_types::{ContractManifest, NefFile};
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Neo N3 Contract Deployment Example");
    println!("=================================");
    
    // Step 1: Set up connection to Neo N3 TestNet
    println!("\nConnecting to Neo N3 TestNet...");
    let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
    let client = RpcClient::new(provider);
    let block_count = client.get_block_count().await?;
    println!("Connected to Neo N3 TestNet at block height: {}", block_count);
    
    // Step 2: Load account with GAS
    println!("\nLoading account...");
    let account = Account::from_wif("your-private-key-wif")?;
    println!("Account loaded: {}", account.get_address());
    
    // Check GAS balance
    let gas_token = GasToken::new(&client);
    let gas_balance = gas_token.balance_of(&account.get_script_hash()).await?;
    println!("GAS Balance: {}", gas_balance);
    
    if gas_balance < 10_00000000 { // 10 GAS
        println!("Warning: Low GAS balance. You may need more GAS to deploy a contract.");
    }
    
    // Step 3: Load contract files
    println!("\nLoading contract files...");
    let nef_bytes = fs::read("path/to/contract.nef")?;
    let manifest_json = fs::read_to_string("path/to/contract.manifest.json")?;
    
    // Parse contract files
    let nef = NefFile::from_bytes(&nef_bytes)?;
    let manifest = ContractManifest::from_json(&manifest_json)?;
    println!("Contract files loaded successfully");
    println!("Contract name: {}", manifest.name);
    println!("Contract features: {:?}", manifest.features);
    
    // Step 4: Deploy the contract
    println!("\nDeploying contract...");
    
    // Estimate deployment costs
    let contract_mgmt = ContractManagement::new(&client);
    let deployment_fee = contract_mgmt.estimate_deployment_fee(&nef, &manifest).await?;
    println!("Estimated deployment fee: {} GAS", deployment_fee as f64 / 100000000.0);
    
    // Confirm deployment
    println!("\nProceed with deployment? This will cost approximately {} GAS [y/N]", 
             deployment_fee as f64 / 100000000.0);
    
    // In a real application, you would get user confirmation here
    // For this example, we assume "yes"
    
    // Deploy the contract
    let result = contract_mgmt.deploy(
        &nef,
        &manifest,
        None, // No deployment data
        &account,
    ).await?;
    
    // Display deployment results
    let contract_hash = result.script_hash;
    println!("\nContract deployed successfully!");
    println!("Contract hash: {}", contract_hash);
    println!("Transaction ID: {}", result.tx_id);
    
    // Step 5: Verify deployment
    println!("\nVerifying contract deployment...");
    let contract_state = contract_mgmt.get_contract(&contract_hash).await?;
    
    println!("Contract verified on blockchain:");
    println!("Contract ID: {}", contract_state.id);
    println!("Contract Update Counter: {}", contract_state.update_counter);
    println!("Contract Author: {}", contract_state.manifest.author);
    
    println!("\nDeployment completed successfully!");
    Ok(())
}
```

## Updating an Existing Contract

Neo N3 allows you to update existing contracts while preserving their state:

```rust,no_run
// Update an existing contract
let update_result = contract_mgmt.update(
    &contract_hash,
    &new_nef,
    &new_manifest,
    None, // No data
    &account,
).await?;

println!("Contract updated successfully!");
println!("Transaction ID: {}", update_result.tx_id);
```

## Destroying a Contract

You can also permanently destroy a contract:

```rust,no_run
// Destroy an existing contract
let destroy_result = contract_mgmt.destroy(
    &contract_hash,
    &account,
).await?;

println!("Contract destroyed successfully!");
println!("Transaction ID: {}", destroy_result);
```

## Contract Manifest Structure

The contract manifest is a crucial part of contract deployment. It defines:

- The contract's name and supported features
- ABI (methods, events, and parameters)
- Permissions (what the contract can access)
- Safe methods (which methods can be called without verification)
- Other metadata (author, description, etc.)

A simple example manifest:

```json
{
  "name": "ExampleContract",
  "groups": [],
  "features": {},
  "abi": {
    "methods": [
      {
        "name": "transfer",
        "parameters": [
          { "name": "from", "type": "Hash160" },
          { "name": "to", "type": "Hash160" },
          { "name": "amount", "type": "Integer" }
        ],
        "returntype": "Boolean",
        "offset": 0,
        "safe": false
      },
      {
        "name": "balanceOf",
        "parameters": [
          { "name": "account", "type": "Hash160" }
        ],
        "returntype": "Integer",
        "offset": 0,
        "safe": true
      }
    ],
    "events": [
      {
        "name": "Transfer",
        "parameters": [
          { "name": "from", "type": "Hash160" },
          { "name": "to", "type": "Hash160" },
          { "name": "amount", "type": "Integer" }
        ]
      }
    ]
  },
  "permissions": [
    { "contract": "*", "methods": "*" }
  ],
  "trusts": [],
  "safemethods": ["balanceOf"],
  "extra": {
    "author": "Neo Developer",
    "email": "dev@neo.org",
    "description": "Example NEP-17 Token Contract"
  }
}
```

## Deployment Best Practices

1. **Test on TestNet first**: Always deploy to TestNet before MainNet
2. **Verify contract hash**: Double-check contract hash after deployment
3. **Estimate fees**: Use `estimate_deployment_fee` to know costs in advance
4. **Secure private keys**: Never expose the private key in code
5. **Contract security**: Audit your contract before deployment
6. **Permissions**: Use the minimum required permissions in the manifest
7. **Documentation**: Document your contract's methods and expected behaviors