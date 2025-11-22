# Rate Limiting Guide - NeoRust (historical v0.4.4)
> Authored for v0.4.4; APIs remain available, but the current SDK release is v0.5.2. See the main README/crate docs for any updates.

## Overview

NeoRust introduced a sophisticated rate limiting system based on the token bucket algorithm, designed to protect Neo N3 nodes from being overwhelmed while maintaining optimal throughput for your applications.

## Why Rate Limiting?

### Benefits
- **Node Protection**: Prevents overwhelming public or private Neo nodes
- **Fair Resource Usage**: Ensures equitable access to shared infrastructure
- **Improved Reliability**: Reduces connection drops and timeout errors
- **Cost Optimization**: Helps manage API costs for paid node services
- **Compliance**: Meets rate limit requirements of node providers

### When to Use
- Connecting to public Neo nodes
- Using shared infrastructure
- High-throughput applications
- Batch processing operations
- Production deployments

## Architecture

### Token Bucket Algorithm

The rate limiter uses a token bucket algorithm with the following characteristics:

```
┌─────────────────┐
│   Token Bucket  │
│  ┌───────────┐  │
│  │  Tokens   │  │ <- Refilled at constant rate
│  │  ○○○○○○   │  │
│  │  ○○○○○○   │  │
│  └───────────┘  │
│                 │
│  Capacity: 20   │
│  Rate: 10/sec   │
└─────────────────┘
        ↓
   [API Request]
```

- **Tokens**: Represent permission to make requests
- **Bucket Capacity**: Maximum burst size
- **Refill Rate**: Tokens added per second
- **Request Cost**: Usually 1 token per request

## Configuration

### Using Presets

```rust
use neo3::neo_clients::{ProductionNeoClient, RateLimitPreset};

// Conservative - 5 requests/second
let client = ProductionNeoClient::new(
    "https://mainnet.neo.org",
    RateLimitPreset::Conservative,
).await?;

// Standard - 10 requests/second (recommended)
let client = ProductionNeoClient::new(
    "https://mainnet.neo.org",
    RateLimitPreset::Standard,
).await?;

// Performance - 20 requests/second
let client = ProductionNeoClient::new(
    "https://mainnet.neo.org",
    RateLimitPreset::Performance,
).await?;

// Aggressive - 50 requests/second
let client = ProductionNeoClient::new(
    "https://mainnet.neo.org",
    RateLimitPreset::Aggressive,
).await?;
```

### Preset Specifications

| Preset | RPS | Burst | Refill | Use Case |
|--------|-----|-------|--------|----------|
| Conservative | 5 | 10 | 200ms | Public nodes, shared resources |
| Standard | 10 | 20 | 100ms | Default, most applications |
| Performance | 20 | 40 | 50ms | Private nodes, high-throughput |
| Aggressive | 50 | 100 | 20ms | Local nodes, testing |

### Custom Configuration

```rust
use neo3::neo_clients::{RateLimiter, RateLimitConfig};
use std::time::Duration;

// Create custom configuration
let config = RateLimitConfig {
    requests_per_second: 15,
    burst_size: 30,
    refill_interval: Duration::from_millis(67), // 1000ms / 15
};

// Create rate limiter
let rate_limiter = RateLimiter::with_config(config);

// Apply to client
let client = ProductionNeoClient::with_rate_limiter(
    "https://mainnet.neo.org",
    rate_limiter,
).await?;
```

## Usage Patterns

### Basic Usage

```rust
// All requests are automatically rate limited
let block_count = client.get_block_count().await?;
let best_hash = client.get_best_block_hash().await?;
let version = client.get_version().await?;

// No manual rate limiting needed!
```

### Batch Operations

```rust
use futures::future::join_all;

// Fetch multiple blocks with automatic rate limiting
async fn fetch_blocks(client: &ProductionNeoClient, start: u32, count: u32) -> Vec<Block> {
    let mut futures = vec![];
    
    for height in start..start + count {
        let client = client.clone();
        futures.push(async move {
            client.get_block(BlockId::Height(height)).await
        });
    }
    
    // All requests respect rate limits
    join_all(futures)
        .await
        .into_iter()
        .filter_map(Result::ok)
        .collect()
}
```

### Parallel Processing

```rust
use tokio::task::JoinSet;

async fn process_addresses(client: &ProductionNeoClient, addresses: Vec<Address>) {
    let mut set = JoinSet::new();
    
    for address in addresses {
        let client = client.clone();
        set.spawn(async move {
            // Each request waits for available tokens
            let balance = client.get_nep17_balances(address).await?;
            Ok::<_, ClientError>((address, balance))
        });
    }
    
    // Collect results as they complete
    while let Some(result) = set.join_next().await {
        match result {
            Ok(Ok((address, balance))) => {
                println!("{}: {:?}", address, balance);
            }
            _ => {}
        }
    }
}
```

## Advanced Features

### Adaptive Rate Limiting

```rust
use neo3::neo_clients::AdaptiveRateLimiter;

// Rate limiter that adjusts based on response times
let adaptive_limiter = AdaptiveRateLimiter::new()
    .target_latency(Duration::from_millis(200))
    .min_rate(5)
    .max_rate(50)
    .adjustment_factor(0.1);

let client = ProductionNeoClient::with_adaptive_limiter(
    "https://mainnet.neo.org",
    adaptive_limiter,
).await?;
```

### Multi-Node Rate Limiting

```rust
use neo3::neo_clients::MultiNodeClient;

// Different rate limits for different nodes
let nodes = vec![
    ("https://node1.neo.org", RateLimitPreset::Conservative),
    ("https://node2.neo.org", RateLimitPreset::Standard),
    ("https://node3.neo.org", RateLimitPreset::Performance),
];

let client = MultiNodeClient::new(nodes).await?;

// Automatically distributes load across nodes
let result = client.get_block_count().await?;
```

### Priority Queue

```rust
use neo3::neo_clients::{PriorityRateLimiter, Priority};

// Rate limiter with priority queues
let priority_limiter = PriorityRateLimiter::new()
    .high_priority_rate(20)
    .normal_priority_rate(10)
    .low_priority_rate(5);

// High priority request
let critical_data = client
    .with_priority(Priority::High)
    .get_account_state(address)
    .await?;

// Low priority background task
let historical_block = client
    .with_priority(Priority::Low)
    .get_block(BlockId::Height(old_height))
    .await?;
```

## Monitoring and Metrics

### Rate Limiter Statistics

```rust
// Get rate limiter stats
let stats = client.get_rate_limiter_stats();

println!("Total requests: {}", stats.total_requests);
println!("Accepted requests: {}", stats.accepted_requests);
println!("Rejected requests: {}", stats.rejected_requests);
println!("Average wait time: {:?}", stats.avg_wait_time);
println!("Current tokens: {}", stats.available_tokens);
```

### Prometheus Metrics

```rust
use neo3::neo_monitoring::RateLimiterMetrics;

// Export rate limiter metrics
let metrics = RateLimiterMetrics::new(&client);

// Metrics available:
// - neo_rate_limiter_requests_total
// - neo_rate_limiter_requests_accepted
// - neo_rate_limiter_requests_rejected
// - neo_rate_limiter_wait_time_seconds
// - neo_rate_limiter_tokens_available
```

## Best Practices

### 1. Choose Appropriate Presets

```rust
// Development
#[cfg(debug_assertions)]
let preset = RateLimitPreset::Aggressive;

// Production
#[cfg(not(debug_assertions))]
let preset = RateLimitPreset::Standard;

let client = ProductionNeoClient::new(url, preset).await?;
```

### 2. Handle Rate Limit Errors

```rust
use neo3::prelude::ClientError;
use tokio::time::{sleep, Duration};

async fn with_retry<T, F, Fut>(f: F) -> Result<T, ClientError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, ClientError>>,
{
    let mut retries = 0;
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(ClientError::RateLimited) if retries < 3 => {
                retries += 1;
                sleep(Duration::from_secs(retries)).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

### 3. Batch Similar Operations

```rust
// Instead of individual requests
for address in addresses {
    let balance = client.get_nep17_balances(address).await?;
}

// Batch them together
let balances = client.batch_get_nep17_balances(addresses).await?;
```

### 4. Use Connection Pooling

```rust
// Combine rate limiting with connection pooling
let client = ProductionNeoClient::builder()
    .endpoint("https://mainnet.neo.org")
    .rate_limit(RateLimitPreset::Standard)
    .connection_pool_size(10)
    .build()
    .await?;
```

## Performance Considerations

### Token Bucket vs Other Algorithms

| Algorithm | Pros | Cons | Use Case |
|-----------|------|------|----------|
| Token Bucket | Allows bursts, simple | Memory per limiter | General use |
| Leaky Bucket | Smooth output | No bursts | Steady flow |
| Fixed Window | Very simple | Boundary issues | Basic limiting |
| Sliding Window | Accurate | Complex, more memory | Precision needed |

### Memory Usage

Each rate limiter instance uses approximately:
- Base: 128 bytes
- Per token: 8 bytes
- Total: ~1KB for typical configuration

### CPU Overhead

- Token check: O(1) - constant time
- Token refill: O(1) - constant time
- Wait time calculation: O(1) - constant time
- Overall impact: <1% CPU overhead

## Troubleshooting

### Common Issues

#### 1. Requests Still Failing

```rust
// Check if rate limit is too high for node
let stats = client.get_rate_limiter_stats();
if stats.rejected_requests > 0 {
    // Switch to more conservative preset
    let client = ProductionNeoClient::new(
        url,
        RateLimitPreset::Conservative,
    ).await?;
}
```

#### 2. Poor Performance

```rust
// Increase rate limit if node can handle it
let client = ProductionNeoClient::new(
    url,
    RateLimitPreset::Performance,
).await?;

// Or use custom configuration
let config = RateLimitConfig {
    requests_per_second: 30,
    burst_size: 60,
    refill_interval: Duration::from_millis(33),
};
```

#### 3. Uneven Request Distribution

```rust
// Use smooth refill for more even distribution
let config = RateLimitConfig {
    requests_per_second: 10,
    burst_size: 10, // Same as rate for smooth flow
    refill_interval: Duration::from_millis(100),
};
```

### Debug Logging

```rust
// Enable debug logging for rate limiter
env_logger::Builder::from_env(Env::default())
    .filter_module("neo3::neo_clients::rate_limiter", log::LevelFilter::Debug)
    .init();

// Logs will show:
// [DEBUG] Rate limiter: Waiting 45ms for token
// [DEBUG] Rate limiter: Token acquired, 9 remaining
```

## Examples

### Example 1: Data Migration

```rust
async fn migrate_data(old_node: &str, new_node: &str) -> Result<(), Box<dyn Error>> {
    // Conservative rate limit for old node
    let old_client = ProductionNeoClient::new(
        old_node,
        RateLimitPreset::Conservative,
    ).await?;
    
    // Higher rate limit for new node
    let new_client = ProductionNeoClient::new(
        new_node,
        RateLimitPreset::Performance,
    ).await?;
    
    // Fetch data from old node (rate limited to 5 req/s)
    let height = old_client.get_block_count().await?;
    
    for i in 0..height {
        let block = old_client.get_block(BlockId::Height(i)).await?;
        // Process block...
    }
    
    Ok(())
}
```

### Example 2: High-Frequency Trading

```rust
async fn trading_bot(client: ProductionNeoClient) -> Result<(), Box<dyn Error>> {
    // Use aggressive rate limiting for local node
    let client = ProductionNeoClient::new(
        "http://localhost:10332",
        RateLimitPreset::Aggressive,
    ).await?;
    
    loop {
        // Check price feeds (up to 50 req/s)
        let price = get_token_price(&client).await?;
        
        if should_trade(price) {
            execute_trade(&client).await?;
        }
        
        // Rate limiter ensures we don't overwhelm the node
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
```

### Example 3: Monitoring Service

```rust
async fn monitor_network(nodes: Vec<String>) -> Result<(), Box<dyn Error>> {
    let mut clients = vec![];
    
    // Create clients with different rate limits
    for node in nodes {
        let preset = if node.contains("public") {
            RateLimitPreset::Conservative
        } else {
            RateLimitPreset::Standard
        };
        
        let client = ProductionNeoClient::new(&node, preset).await?;
        clients.push(client);
    }
    
    // Monitor all nodes
    loop {
        for client in &clients {
            // Each client respects its own rate limit
            if let Ok(height) = client.get_block_count().await {
                log::info!("Node {} at height {}", client.endpoint(), height);
            }
        }
        
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
```

## Summary

The rate limiting system in NeoRust v0.4.4 provides:

- **Automatic Protection**: All requests are automatically rate limited
- **Flexible Configuration**: Presets for common scenarios or custom configs
- **High Performance**: Minimal overhead with O(1) operations
- **Production Ready**: Battle-tested token bucket algorithm
- **Easy Integration**: Works transparently with existing code

For optimal results:
1. Start with `Standard` preset
2. Monitor performance metrics
3. Adjust based on node capabilities
4. Use connection pooling for high throughput
5. Implement proper error handling

The rate limiter ensures your Neo N3 applications are good citizens of the blockchain network while maintaining optimal performance.
