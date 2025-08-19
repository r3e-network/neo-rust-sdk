# NeoRust SDK v0.4.4 Migration Guide

## Overview

This guide provides step-by-step instructions for migrating existing projects from NeoRust SDK v0.4.3 or earlier to v0.4.4, which includes significant enhancements in gas estimation, rate limiting, testing infrastructure, and production readiness features.

## Table of Contents
1. [Breaking Changes](#breaking-changes)
2. [New Features](#new-features)
3. [Migration Steps](#migration-steps)
4. [Code Examples](#code-examples)
5. [Testing Migration](#testing-migration)
6. [Production Deployment](#production-deployment)

## Breaking Changes

### Minimal Breaking Changes
Version 0.4.4 maintains backward compatibility with v0.4.3. The only changes required are:

1. **Compilation Warnings**: Previously suppressed warnings are now errors
   - Remove any code that was previously generating warnings
   - Fix all clippy lints

2. **Import Path Updates**: Update version references in documentation
   - Change `neo3 = "0.4.3"` to `neo3 = "0.4.4"` in Cargo.toml

## New Features

### 1. Real-Time Gas Estimation

The new gas estimation system provides accurate, real-time gas calculations using the blockchain's `invokescript` RPC method.

#### Before (v0.4.3)
```rust
// Manual gas estimation
let system_fee = 1000000; // Fixed estimate
let network_fee = 500000; // Fixed estimate
```

#### After (v0.4.4)
```rust
use neo3::neo_builder::transaction::gas_estimator::GasEstimator;

// Real-time gas estimation
let gas = GasEstimator::estimate_gas_realtime(
    &client,
    &script,
    signers.clone(),
).await?;

// With safety margin (15%)
let safe_gas = GasEstimator::estimate_gas_with_margin(
    &client,
    &script,
    signers.clone(),
    15,
).await?;
```

### 2. Rate Limiting

Protect your application from API throttling and network overwhelming with built-in rate limiting.

#### Implementation
```rust
use neo3::neo_clients::{RateLimiter, RateLimiterPresets};

// Use preset configuration
let limiter = RateLimiterPresets::standard(); // 100 req/s, 20 concurrent

// Apply to client
let client = RpcClient::new(provider)
    .with_rate_limiter(limiter);

// Or use custom configuration
use neo3::neo_clients::RateLimiterBuilder;

let limiter = RateLimiterBuilder::new()
    .max_requests(50)
    .window(Duration::from_secs(1))
    .max_concurrent(10)
    .build();
```

### 3. Production Client

Enhanced client with enterprise features for production environments.

```rust
use neo3::neo_clients::{ProductionRpcClient, ProductionClientConfig};

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

## Migration Steps

### Step 1: Update Dependencies

```toml
# Cargo.toml
[dependencies]
neo3 = "0.4.4"

# Optional: Add property testing
[dev-dependencies]
proptest = "1.5"
```

### Step 2: Update Client Initialization

#### Basic Migration (Minimal Changes)
```rust
// No changes required - existing code continues to work
let provider = HttpProvider::new("https://mainnet1.neo.org:443")?;
let client = RpcClient::new(provider);
```

#### Enhanced Migration (Recommended for Production)
```rust
use neo3::neo_clients::{
    ProductionRpcClient, 
    ProductionClientConfig,
    RateLimiterPresets,
};

// Create production-ready client
let config = ProductionClientConfig::default()
    .with_rate_limiter(RateLimiterPresets::standard())
    .with_caching(true)
    .with_metrics(true);

let client = ProductionRpcClient::new(endpoint, config);
```

### Step 3: Update Transaction Building

#### Old Pattern (v0.4.3)
```rust
let mut builder = TransactionBuilder::with_client(&client);
builder
    .set_script(Some(script))
    .set_signers(vec![signer])
    .set_system_fee(1000000)?  // Manual estimate
    .set_network_fee(500000)?; // Manual estimate
```

#### New Pattern (v0.4.4)
```rust
use neo3::neo_builder::transaction::gas_estimator::GasEstimator;

let mut builder = TransactionBuilder::with_client(&client);

// Estimate gas automatically
let gas_estimate = GasEstimator::estimate_gas_with_margin(
    &client,
    &script,
    vec![signer.clone()],
    15, // 15% safety margin
).await?;

builder
    .set_script(Some(script))
    .set_signers(vec![signer])
    .set_system_fee(gas_estimate)?
    .set_network_fee(500000)?; // Or estimate this too
```

### Step 4: Implement Rate Limiting

```rust
// For existing code, wrap client calls with rate limiting
let permit = limiter.acquire().await?;
let result = client.get_block_count().await?;
drop(permit); // Release permit

// Or use try_acquire for non-blocking
match limiter.try_acquire().await {
    Ok(permit) => {
        let result = client.get_block_count().await?;
        // Process result
    },
    Err(_) => {
        // Handle rate limit exceeded
        println!("Rate limit exceeded, retry later");
    }
}
```

## Code Examples

### Complete Migration Example

```rust
use neo3::prelude::*;
use neo3::neo_clients::{
    ProductionRpcClient, ProductionClientConfig,
    RateLimiterPresets,
};
use neo3::neo_builder::transaction::gas_estimator::GasEstimator;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize production client
    let config = ProductionClientConfig::default()
        .with_rate_limiter(RateLimiterPresets::standard());
    
    let client = ProductionRpcClient::new(
        "https://mainnet1.neo.org:443",
        config
    );

    // 2. Build transaction with automatic gas estimation
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

    // 3. Estimate gas with safety margin
    let gas = GasEstimator::estimate_gas_with_margin(
        &client,
        &script,
        vec![Signer::called_by_entry(&account)?],
        20, // 20% safety margin
    ).await?;

    // 4. Build and send transaction
    let mut builder = TransactionBuilder::with_client(&client);
    builder
        .set_script(Some(script))
        .set_signers(vec![Signer::called_by_entry(&account)?])
        .set_system_fee(gas)?
        .set_network_fee(500000)?;

    let mut tx = builder.sign().await?;
    let result = tx.send_tx().await?;
    
    println!("Transaction sent: {}", result.hash);
    Ok(())
}
```

### Batch Operations with Rate Limiting

```rust
use futures::future::try_join_all;

async fn batch_transfers(
    client: &ProductionRpcClient,
    transfers: Vec<Transfer>,
) -> Result<Vec<H256>, Error> {
    // Estimate gas for all transfers
    let scripts: Vec<_> = transfers.iter()
        .map(|t| (build_transfer_script(t), vec![t.signer.clone()]))
        .collect();
    
    let gas_estimates = GasEstimator::batch_estimate_gas(
        client,
        scripts
    ).await?;

    // Build transactions with estimated gas
    let transactions = transfers.into_iter()
        .zip(gas_estimates)
        .map(|(transfer, gas)| {
            build_transaction(client, transfer, gas)
        })
        .collect::<Vec<_>>();

    // Send all transactions (rate limiting handled by client)
    let results = try_join_all(transactions).await?;
    Ok(results)
}
```

## Testing Migration

### Adding Property-Based Tests

```rust
#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn test_gas_estimation_bounds(
            script_size in 1..10000usize,
            margin in 0..100u8,
        ) {
            // Property: Gas with margin should always be >= base gas
            let base_gas = calculate_base_gas(script_size);
            let gas_with_margin = add_margin(base_gas, margin);
            assert!(gas_with_margin >= base_gas);
        }
    }
}
```

### Running New Test Suite

```bash
# Run all tests including property tests
cargo test --all-features

# Run with coverage reporting
cargo llvm-cov --html

# Run benchmarks
cargo bench

# Check for security vulnerabilities
cargo audit
```

## Production Deployment

### Pre-Deployment Checklist

1. **Configuration**
   ```rust
   // Ensure production configuration
   let config = ProductionClientConfig {
       pool_config: PoolConfig {
           max_connections: 50, // Increase for production
           min_idle: 10,
           max_idle_time: Duration::from_secs(600),
       },
       cache_config: CacheConfig {
           max_entries: 50000, // Larger cache for production
           default_ttl: Duration::from_secs(60),
       },
       circuit_breaker_config: CircuitBreakerConfig {
           failure_threshold: 10,
           timeout: Duration::from_secs(120),
       },
       enable_logging: true,
       enable_metrics: true,
   };
   ```

2. **Environment Variables**
   ```bash
   export NEO_ENDPOINT="https://mainnet1.neo.org:443"
   export NEO_RATE_LIMIT="conservative"  # or "standard", "aggressive"
   export NEO_ENABLE_METRICS="true"
   export NEO_LOG_LEVEL="info"
   ```

3. **Monitoring Setup**
   ```rust
   // Enable metrics collection
   use neo3::monitoring::MetricsCollector;
   
   let metrics = MetricsCollector::new();
   client.set_metrics_collector(metrics);
   
   // Export metrics to Prometheus
   let metrics_endpoint = metrics.prometheus_endpoint();
   ```

4. **Health Checks**
   ```rust
   // Implement health check endpoint
   async fn health_check(client: &ProductionRpcClient) -> HealthStatus {
       match client.get_block_count().await {
           Ok(_) => HealthStatus::Healthy,
           Err(e) => HealthStatus::Unhealthy(e.to_string()),
       }
   }
   ```

### Rollback Plan

If issues occur after deployment:

1. **Quick Rollback**
   ```toml
   # Revert to previous version in Cargo.toml
   neo3 = "0.4.3"
   ```

2. **Disable New Features**
   ```rust
   // Disable rate limiting
   let client = RpcClient::new(provider); // Without rate limiter
   
   // Use fixed gas estimates
   builder.set_system_fee(1000000)?;
   ```

## Troubleshooting

### Common Issues

1. **Rate Limiting Too Aggressive**
   ```rust
   // Adjust rate limits
   let limiter = RateLimiterBuilder::new()
       .max_requests(200) // Increase limit
       .window(Duration::from_secs(1))
       .build();
   ```

2. **Gas Estimation Failures**
   ```rust
   // Fallback to manual estimation
   let gas = match GasEstimator::estimate_gas_realtime(&client, &script, signers).await {
       Ok(g) => g,
       Err(_) => 1000000, // Fallback value
   };
   ```

3. **Connection Pool Exhaustion**
   ```rust
   // Increase pool size
   let config = ProductionClientConfig {
       pool_config: PoolConfig {
           max_connections: 100, // Increase max
           // ...
       },
       // ...
   };
   ```

## Support

For migration assistance:
- GitHub Issues: https://github.com/r3e/NeoRust/issues
- Documentation: https://docs.neorust.org
- Discord: https://discord.gg/neo

---

**Migration Guide Version**: 1.0  
**SDK Version**: 0.4.4  
**Last Updated**: August 19, 2025