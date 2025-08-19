# NeoRust API Documentation v0.4.4

## Table of Contents
1. [Core Components](#core-components)
2. [Client APIs](#client-apis)
3. [Transaction Builder](#transaction-builder)
4. [Wallet Management](#wallet-management)
5. [Smart Contracts](#smart-contracts)
6. [Rate Limiting](#rate-limiting)
7. [Gas Estimation](#gas-estimation)
8. [Error Handling](#error-handling)

## Core Components

### ProductionNeoClient

The production-ready client with enterprise features for interacting with Neo N3 blockchain.

```rust
use neo3::prelude::*;
use neo3::neo_clients::{ProductionNeoClient, RateLimitPreset};

/// Create a new production client with rate limiting
pub async fn create_client() -> Result<ProductionNeoClient, ClientError> {
    ProductionNeoClient::new(
        "https://mainnet.neo.org",
        RateLimitPreset::Standard, // 10 requests per second
    ).await
}
```

#### Configuration Options

| Preset | Requests/Second | Use Case |
|--------|----------------|----------|
| `Conservative` | 5 | Public nodes, shared infrastructure |
| `Standard` | 10 | Default for most applications |
| `Performance` | 20 | Private nodes, high-throughput |
| `Aggressive` | 50 | Local nodes, testing |
| `Custom(rate)` | Variable | Custom rate limiting |

### RPC Methods

#### Block Operations

```rust
// Get current block height
let height = client.get_block_count().await?;

// Get block by index
let block = client.get_block(BlockId::Height(100000)).await?;

// Get block with transactions
let block_verbose = client.get_block_verbose(BlockId::Height(100000)).await?;

// Get block header
let header = client.get_block_header(BlockId::Hash(hash)).await?;

// Get best block hash
let best_hash = client.get_best_block_hash().await?;
```

#### Transaction Operations

```rust
// Send raw transaction
let tx_hash = client.send_raw_transaction(raw_tx).await?;

// Get transaction details
let tx = client.get_transaction(tx_hash).await?;

// Get transaction height
let height = client.get_transaction_height(tx_hash).await?;

// Get raw transaction
let raw_tx = client.get_raw_transaction(tx_hash).await?;

// Get application log
let log = client.get_application_log(tx_hash).await?;
```

#### Account Operations

```rust
// Get account state
let account = client.get_account_state(address).await?;

// Get NEP-17 balances
let balances = client.get_nep17_balances(address).await?;

// Get NEP-11 balances
let nft_balances = client.get_nep11_balances(address).await?;

// Get unclaimed GAS
let gas = client.get_unclaimed_gas(address).await?;
```

#### Smart Contract Operations

```rust
// Invoke contract for reading (no fees)
let result = client.invoke_function(
    contract_hash,
    "balanceOf",
    vec![address.to_array()],
).await?;

// Invoke script (for testing)
let result = client.invoke_script(
    script_bytes,
    signers,
).await?;

// Get contract state
let contract = client.get_contract_state(contract_hash).await?;

// Get storage value
let value = client.get_storage(contract_hash, key).await?;
```

#### Network Operations

```rust
// Get connection count
let count = client.get_connection_count().await?;

// Get peers
let peers = client.get_peers().await?;

// Get version
let version = client.get_version().await?;

// Get network fee
let fee = client.get_network_fee(transaction).await?;
```

## Transaction Builder

### Basic Transaction

```rust
use neo3::neo_builder::TransactionBuilder;
use neo3::neo_types::{Signer, WitnessScope};

let mut builder = TransactionBuilder::new();

// Configure transaction
builder
    .set_script(script_bytes)
    .set_system_fee(1000000) // 0.01 GAS
    .add_signer(Signer {
        account: sender_account.get_script_hash(),
        scopes: WitnessScope::CalledByEntry,
        allowed_contracts: vec![],
        allowed_groups: vec![],
        rules: vec![],
    })
    .set_network_fee(500000); // 0.005 GAS

// Build transaction
let transaction = builder.build()?;

// Sign transaction
let signed_tx = transaction.sign(&private_key)?;

// Send transaction
let tx_hash = client.send_raw_transaction(signed_tx).await?;
```

### NEP-17 Token Transfer

```rust
use neo3::neo_builder::Nep17TransferBuilder;

let transfer = Nep17TransferBuilder::new()
    .token(NEO_TOKEN_HASH)
    .from(sender_address)
    .to(recipient_address)
    .amount(1000000000) // 10 NEO (8 decimals)
    .build()?;

// Sign and send
let signed_tx = transfer.sign(&private_key)?;
let tx_hash = client.send_raw_transaction(signed_tx).await?;
```

## Wallet Management

### Create Wallet

```rust
use neo3::neo_wallets::{Wallet, Account};

// Create new wallet
let mut wallet = Wallet::new("MyWallet")?;

// Generate new account
let account = Account::create()?;
wallet.add_account(account);

// Save encrypted wallet (NEP-6)
wallet.save_to_file("wallet.json", "password")?;
```

### Import Wallet

```rust
// Load from file
let wallet = Wallet::from_file("wallet.json", "password")?;

// Import from WIF
let account = Account::from_wif("L1QqQJnpBwbsPGAuutuzPTac8piqvbR1HRjrY5qHup48TBCBFe4g")?;

// Import from private key
let private_key = PrivateKey::from_bytes(&key_bytes)?;
let account = Account::from_private_key(private_key)?;
```

### Multi-Signature Account

```rust
use neo3::neo_crypto::PublicKey;

// Create 2-of-3 multisig
let pubkeys = vec![pubkey1, pubkey2, pubkey3];
let multisig_account = Account::create_multisig(2, pubkeys)?;

// Get multisig address
let address = multisig_account.get_address();
```

## Smart Contracts

### Contract Invocation

```rust
use neo3::neo_builder::ContractInvoker;
use neo3::neo_vm::StackItem;

// Create invoker
let invoker = ContractInvoker::new(contract_hash, &client);

// Call contract method
let result = invoker
    .function("transfer")
    .param(from_address)
    .param(to_address)
    .param(amount)
    .param(data)
    .test_invoke()
    .await?;

// Parse result
if let StackItem::Boolean(success) = result.stack[0] {
    println!("Transfer successful: {}", success);
}
```

### Contract Deployment

```rust
use neo3::neo_builder::ContractDeployer;

let deployer = ContractDeployer::new()
    .nef_file(nef_bytes)
    .manifest(manifest_json)
    .data(deployment_data)
    .build()?;

// Deploy contract
let tx_hash = client.deploy_contract(deployer).await?;
```

## Rate Limiting

### Configuration

```rust
use neo3::neo_clients::{RateLimiter, RateLimitConfig};
use std::time::Duration;

// Create custom rate limiter
let config = RateLimitConfig {
    requests_per_second: 15,
    burst_size: 30,
    refill_interval: Duration::from_millis(100),
};

let rate_limiter = RateLimiter::with_config(config);

// Apply to client
let client = ProductionNeoClient::with_rate_limiter(
    "https://mainnet.neo.org",
    rate_limiter,
).await?;
```

### Usage Patterns

```rust
// Automatic rate limiting on all calls
let result = client.get_block_count().await?; // Automatically rate limited

// Batch operations with rate limiting
let mut handles = vec![];
for height in 0..100 {
    let client = client.clone();
    handles.push(tokio::spawn(async move {
        client.get_block(BlockId::Height(height)).await
    }));
}

// All requests are automatically rate limited
let blocks: Vec<Block> = futures::future::join_all(handles)
    .await
    .into_iter()
    .filter_map(Result::ok)
    .collect();
```

## Gas Estimation

### Real-Time Gas Estimation

```rust
use neo3::neo_builder::GasEstimator;

// Estimate gas for transaction
let estimator = GasEstimator::new(&client);

let estimated_gas = estimator
    .estimate_gas(script_bytes, signers)
    .await?;

println!("Estimated system fee: {} GAS", estimated_gas as f64 / 100_000_000.0);

// Add 10% buffer for safety
let system_fee = (estimated_gas as f64 * 1.1) as i64;
```

### Gas Optimization

```rust
// Use gas estimation in transaction builder
let mut builder = TransactionBuilder::new();

// Automatically estimate and set gas
builder
    .set_script(script_bytes)
    .estimate_and_set_gas(&client)
    .await?
    .add_signer(signer)
    .build()?;
```

## Error Handling

### Error Types

```rust
use neo3::prelude::*;

match client.get_block_count().await {
    Ok(count) => println!("Block height: {}", count),
    Err(e) => match e {
        ClientError::Network(err) => {
            eprintln!("Network error: {}", err);
            // Implement retry logic
        },
        ClientError::RateLimited => {
            eprintln!("Rate limited, waiting...");
            tokio::time::sleep(Duration::from_secs(1)).await;
        },
        ClientError::InvalidResponse(msg) => {
            eprintln!("Invalid response: {}", msg);
        },
        ClientError::Timeout => {
            eprintln!("Request timeout");
            // Implement circuit breaker
        },
        _ => eprintln!("Unexpected error: {}", e),
    }
}
```

### Retry Logic

```rust
use neo3::neo_clients::RetryPolicy;

// Configure retry policy
let retry_policy = RetryPolicy {
    max_retries: 3,
    initial_delay: Duration::from_millis(100),
    max_delay: Duration::from_secs(5),
    exponential_base: 2,
};

// Apply to client
let client = ProductionNeoClient::with_retry(
    "https://mainnet.neo.org",
    retry_policy,
).await?;
```

### Circuit Breaker

```rust
use neo3::neo_clients::CircuitBreaker;

// Configure circuit breaker
let circuit_breaker = CircuitBreaker::new()
    .failure_threshold(5)
    .success_threshold(2)
    .timeout(Duration::from_secs(30));

// Client with circuit breaker
let client = ProductionNeoClient::with_circuit_breaker(
    "https://mainnet.neo.org",
    circuit_breaker,
).await?;
```

## Advanced Features

### Connection Pooling

```rust
use neo3::neo_clients::ConnectionPoolConfig;

let pool_config = ConnectionPoolConfig {
    min_idle: 5,
    max_size: 20,
    idle_timeout: Duration::from_secs(300),
    connection_timeout: Duration::from_secs(10),
};

let client = ProductionNeoClient::with_pool(
    "https://mainnet.neo.org",
    pool_config,
).await?;
```

### Monitoring and Metrics

```rust
use neo3::neo_monitoring::{MetricsCollector, PrometheusExporter};

// Setup metrics collection
let metrics = MetricsCollector::new();
let client = ProductionNeoClient::with_metrics(
    "https://mainnet.neo.org",
    metrics.clone(),
).await?;

// Export metrics
let exporter = PrometheusExporter::new(metrics);
exporter.serve(9090).await?;
```

### Batch Operations

```rust
use neo3::neo_clients::BatchClient;

let batch_client = BatchClient::new(client);

// Batch multiple operations
let results = batch_client
    .add_request(|c| c.get_block_count())
    .add_request(|c| c.get_best_block_hash())
    .add_request(|c| c.get_version())
    .execute()
    .await?;

let block_count = results[0].as_u64()?;
let best_hash = results[1].as_hash()?;
let version = results[2].as_version()?;
```

## Performance Tips

1. **Use Connection Pooling**: Reuse connections for better performance
2. **Enable Rate Limiting**: Prevent overwhelming nodes
3. **Batch Operations**: Group multiple requests when possible
4. **Cache Results**: Cache immutable data like historical blocks
5. **Use Circuit Breakers**: Prevent cascade failures
6. **Monitor Metrics**: Track performance and identify bottlenecks
7. **Optimize Gas Usage**: Use real-time estimation to avoid overpaying

## Migration Guide

### From v0.4.3 to v0.4.4

```rust
// Old (v0.4.3)
let client = Client::new("https://mainnet.neo.org", None, None).await?;

// New (v0.4.4)
let client = ProductionNeoClient::new(
    "https://mainnet.neo.org",
    RateLimitPreset::Standard,
).await?;

// Old gas estimation (v0.4.3)
let gas = 1000000; // Fixed estimate

// New gas estimation (v0.4.4)
let gas = GasEstimator::new(&client)
    .estimate_gas(script, signers)
    .await?;
```

## Support

For issues, questions, or contributions:
- GitHub: https://github.com/R3E-Network/NeoRust
- Documentation: https://docs.rs/neo3
- Examples: See `/examples` directory in repository

---

*Generated for NeoRust v0.4.4 - Production Ready*