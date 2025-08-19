# NeoRust SDK - API Specification v0.4.4

## Table of Contents
1. [Core APIs](#core-apis)
2. [Transaction APIs](#transaction-apis)
3. [Wallet APIs](#wallet-apis)
4. [Smart Contract APIs](#smart-contract-apis)
5. [Network APIs](#network-apis)
6. [Cryptography APIs](#cryptography-apis)
7. [Error Handling](#error-handling)
8. [Rate Limiting](#rate-limiting)

## Core APIs

### Client Initialization

```rust
use neo3::prelude::*;
use neo3::neo_clients::{HttpProvider, RpcClient, APITrait};

// Basic client creation
let provider = HttpProvider::new("https://mainnet1.neo.org:443")?;
let client = RpcClient::new(provider);

// Production client with advanced features
let config = ProductionClientConfig {
    pool_config: PoolConfig {
        max_connections: 20,
        min_idle: 5,
        max_idle_time: Duration::from_secs(300),
    },
    cache_config: CacheConfig {
        max_entries: 10000,
        default_ttl: Duration::from_secs(30),
    },
    circuit_breaker_config: CircuitBreakerConfig {
        failure_threshold: 5,
        timeout: Duration::from_secs(60),
    },
    enable_logging: true,
    enable_metrics: true,
};
let client = ProductionRpcClient::new(endpoint, config);
```

### Blockchain Query APIs

```rust
// Get blockchain information
async fn get_block_count(&self) -> Result<u32, Error>;
async fn get_best_block_hash(&self) -> Result<H256, Error>;
async fn get_block(&self, hash: H256, full_tx: bool) -> Result<Block, Error>;
async fn get_block_by_index(&self, index: u32, full_tx: bool) -> Result<Block, Error>;
async fn get_block_header(&self, hash: H256) -> Result<BlockHeader, Error>;
async fn get_transaction(&self, hash: H256) -> Result<Transaction, Error>;
async fn get_raw_transaction(&self, hash: H256) -> Result<String, Error>;

// Example usage
let block_count = client.get_block_count().await?;
let latest_block = client.get_block_by_index(block_count - 1, true).await?;
println!("Latest block: {:?}", latest_block);
```

## Transaction APIs

### Transaction Building

```rust
use neo3::neo_builder::{TransactionBuilder, ScriptBuilder, Signer};

// Create transaction builder
let mut builder = TransactionBuilder::with_client(&client);

// Build script
let script = ScriptBuilder::new()
    .contract_call(
        &contract_hash,
        "transfer",
        &[
            ContractParameter::h160(&from),
            ContractParameter::h160(&to),
            ContractParameter::integer(amount),
            ContractParameter::any(),
        ],
        None,
    )?
    .to_bytes();

// Configure transaction
builder
    .set_script(Some(script))
    .set_signers(vec![Signer::called_by_entry(&account)?])
    .valid_until_block(block_count + 5760)?
    .set_system_fee(1000000)?
    .set_network_fee(500000)?;

// Sign and send
let mut tx = builder.sign().await?;
let result = tx.send_tx().await?;
```

### Gas Estimation

```rust
use neo3::neo_builder::GasEstimator;

// Real-time gas estimation
let gas = GasEstimator::estimate_gas_realtime(
    &client,
    &script,
    signers,
).await?;

// With safety margin (15%)
let safe_gas = GasEstimator::estimate_gas_with_margin(
    &client,
    &script,
    signers,
    15,
).await?;

// Batch estimation
let scripts = vec![
    (script1.as_slice(), signers1),
    (script2.as_slice(), signers2),
];
let estimates = GasEstimator::batch_estimate_gas(&client, scripts).await?;
```

## Wallet APIs

### Account Management

```rust
use neo3::neo_protocol::{Account, AccountTrait};
use neo3::neo_wallets::{Wallet, WalletTrait};

// Create new account
let account = Account::create()?;
println!("Address: {}", account.get_address());
println!("WIF: {}", account.export()?);

// Import from WIF
let account = Account::from_wif("L1QqQJnpBwbsPGAuutuzPTac8piqvbR1HRjrY5qHup48TBCBFe4g")?;

// Import from private key
let account = Account::from_private_key(&private_key_bytes)?;

// Multi-signature account
let pub_keys = vec![pub_key1, pub_key2, pub_key3];
let multi_sig = Account::create_multi_sig(2, &pub_keys)?;
```

### Wallet Operations

```rust
// Create wallet
let mut wallet = Wallet::new();
wallet.name = Some("MyWallet".to_string());

// Add accounts
wallet.add_account(account1);
wallet.add_account(account2);
wallet.set_default_account(0)?;

// Encrypt wallet (NEP-2)
wallet.encrypt_accounts("strong_password")?;

// Save to file
wallet.save_to_file("wallet.json")?;

// Load from file
let wallet = Wallet::from_file("wallet.json")?;
wallet.decrypt_accounts("strong_password")?;

// Sign message
let signature = wallet.sign_message(
    &wallet.get_default_account()?,
    b"Hello, Neo!"
)?;
```

## Smart Contract APIs

### Contract Invocation

```rust
use neo3::neo_contract::{SmartContract, ContractManifest};

// Invoke contract function (read-only)
let result = client.invoke_function(
    &contract_hash,
    "balanceOf".to_string(),
    vec![ContractParameter::h160(&account_hash)],
    None,
).await?;

// Parse result
let balance = result.stack[0].as_int().unwrap_or(0);
println!("Balance: {}", balance);

// Invoke with transaction (state change)
let tx = client.create_invocation_transaction(
    contract_hash,
    "transfer",
    vec![
        ContractParameter::h160(&from),
        ContractParameter::h160(&to),
        ContractParameter::integer(amount),
    ],
    vec![Signer::called_by_entry(&from)],
).await?;
```

### Contract Deployment

```rust
// Load contract files
let nef = NefFile::from_file("contract.nef")?;
let manifest = ContractManifest::from_file("contract.manifest.json")?;

// Create deployment transaction
let tx = client.create_contract_deployment_transaction(
    nef,
    manifest,
    vec![Signer::called_by_entry(&deployer)],
).await?;

// Sign and send
let mut signed_tx = tx.sign().await?;
let result = signed_tx.send_tx().await?;
println!("Contract deployed: {}", result.hash);
```

### NEP-17 Token Operations

```rust
use neo3::neo_contract::Nep17Token;

// Initialize NEP-17 token
let neo = Nep17Token::new(&client, NEO_TOKEN_HASH);
let gas = Nep17Token::new(&client, GAS_TOKEN_HASH);

// Query token info
let symbol = neo.symbol().await?;
let decimals = neo.decimals().await?;
let total_supply = neo.total_supply().await?;

// Check balance
let balance = neo.balance_of(&account_hash).await?;

// Transfer tokens
let tx = neo.transfer(
    &from_account,
    &to_address,
    amount,
    Some("Payment for services"),
).await?;
```

## Network APIs

### RPC Methods

```rust
// Network information
async fn get_version(&self) -> Result<Version, Error>;
async fn get_peers(&self) -> Result<Peers, Error>;
async fn get_connection_count(&self) -> Result<u32, Error>;

// Mempool
async fn get_raw_mem_pool(&self) -> Result<Vec<H256>, Error>;
async fn get_mem_pool(&self) -> Result<MemPoolDetails, Error>;

// State service
async fn get_state_root(&self, index: u32) -> Result<StateRoot, Error>;
async fn get_state_height(&self) -> Result<StateHeight, Error>;
async fn get_proof(&self, root: H256, contract: H160, key: &str) -> Result<String, Error>;

// Application logs
async fn get_application_log(&self, tx_hash: H256) -> Result<ApplicationLog, Error>;
```

### Subscription APIs

```rust
use neo3::neo_clients::SubscriptionStream;

// Subscribe to new blocks
let mut stream = client.subscribe_blocks().await?;
while let Some(block) = stream.next().await {
    println!("New block: {}", block.height);
}

// Subscribe to transactions
let mut stream = client.subscribe_transactions().await?;
while let Some(tx) = stream.next().await {
    println!("New transaction: {}", tx.hash);
}

// Subscribe to notifications
let mut stream = client.subscribe_notifications(contract_hash).await?;
while let Some(notification) = stream.next().await {
    println!("Notification: {:?}", notification);
}
```

## Cryptography APIs

### Key Operations

```rust
use neo3::neo_crypto::{KeyPair, PublicKey, PrivateKey};

// Generate new key pair
let keypair = KeyPair::generate()?;
let private_key = keypair.private_key();
let public_key = keypair.public_key();

// Import from hex
let private_key = PrivateKey::from_hex("0x...")?;
let keypair = KeyPair::from_private_key(&private_key)?;

// Sign and verify
let message = b"Hello, Neo!";
let signature = keypair.sign(message)?;
let is_valid = keypair.verify(message, &signature)?;
```

### Hashing

```rust
use neo3::neo_crypto::{sha256, hash160, hash256};

// SHA256
let hash = sha256(data);

// RIPEMD160(SHA256(data))
let hash = hash160(data);

// SHA256(SHA256(data))
let hash = hash256(data);
```

### NEP-2 Encryption

```rust
use neo3::neo_crypto::Nep2;

// Encrypt private key
let encrypted = Nep2::encrypt(&private_key, "password")?;
println!("Encrypted: {}", encrypted);

// Decrypt private key
let decrypted = Nep2::decrypt(&encrypted, "password")?;
```

## Error Handling

### Error Types

```rust
use neo3::neo_error::{Neo3Error, Neo3Result};

// Comprehensive error handling
match client.get_block_count().await {
    Ok(count) => println!("Block count: {}", count),
    Err(Neo3Error::Network(e)) => eprintln!("Network error: {}", e),
    Err(Neo3Error::Crypto(e)) => eprintln!("Crypto error: {}", e),
    Err(Neo3Error::Transaction(e)) => eprintln!("Transaction error: {}", e),
    Err(e) => eprintln!("Other error: {}", e),
}

// Using context
let result = operation()
    .with_context(|| "Failed to perform critical operation")?;

// Custom error creation
fn validate_amount(amount: u64) -> Neo3Result<()> {
    ensure!(amount > 0, "Amount must be positive");
    ensure!(amount <= MAX_AMOUNT, "Amount exceeds maximum");
    Ok(())
}
```

## Rate Limiting

### Configuration

```rust
use neo3::neo_clients::{RateLimiter, RateLimiterBuilder, RateLimiterPresets};

// Use preset
let limiter = RateLimiterPresets::standard(); // 100 req/s, 20 concurrent

// Custom configuration
let limiter = RateLimiterBuilder::new()
    .max_requests(50)
    .window(Duration::from_secs(1))
    .max_concurrent(10)
    .build();

// Apply to client
let client = RpcClient::new(provider)
    .with_rate_limiter(limiter);
```

### Usage

```rust
// Acquire permit before request
let permit = limiter.acquire().await?;
let result = client.get_block_count().await?;
drop(permit); // Release permit

// Try without waiting
match limiter.try_acquire().await {
    Ok(permit) => {
        // Perform operation
    },
    Err(_) => {
        println!("Rate limit exceeded, try again later");
    }
}

// Check available capacity
let tokens = limiter.available_tokens().await;
println!("Available requests: {}", tokens);
```

## WebSocket APIs

```rust
use neo3::neo_clients::Ws;

// Connect via WebSocket
let ws = Ws::connect("wss://mainnet1.neo.org/ws").await?;
let client = RpcClient::new(ws);

// Bidirectional communication
let (tx, rx) = client.split();

// Send requests
tx.send(request).await?;

// Receive responses
while let Some(response) = rx.next().await {
    process_response(response)?;
}
```

## Batch Operations

```rust
// Batch RPC requests
let requests = vec![
    client.get_block_count(),
    client.get_best_block_hash(),
    client.get_version(),
];

let results = futures::future::join_all(requests).await;

// Batch transaction creation
let transactions = accounts
    .iter()
    .map(|account| create_transaction(account))
    .collect::<Vec<_>>();

let results = futures::future::try_join_all(transactions).await?;
```

---

## API Versioning

Current API Version: **0.4.4**

### Breaking Changes Policy
- Major version (1.0.0): Breaking API changes
- Minor version (0.5.0): New features, backward compatible
- Patch version (0.4.5): Bug fixes only

### Deprecation Policy
1. Mark as deprecated with warning
2. Maintain for 2 minor versions
3. Remove in next major version

---

**Last Updated**: August 19, 2025  
**API Status**: Stable  
**Next Review**: v0.5.0