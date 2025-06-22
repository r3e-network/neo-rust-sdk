# NeoRust SDK Production Deployment Guide

## Overview

This guide provides comprehensive instructions for deploying NeoRust SDK applications in production environments. It covers security considerations, performance optimization, monitoring, and best practices.

## Table of Contents

1. [Security Considerations](#security-considerations)
2. [Performance Optimization](#performance-optimization)
3. [Monitoring & Logging](#monitoring--logging)
4. [Deployment Strategies](#deployment-strategies)
5. [Error Handling](#error-handling)
6. [Testing in Production](#testing-in-production)

## Security Considerations

### Wallet Security

#### Private Key Management
```rust,no_run
use neo3::prelude::*;
use neo3::neo_wallets::{Wallet, WalletTrait};

// ✅ GOOD: Use encrypted wallets in production
let mut wallet = Wallet::new();
wallet.encrypt_accounts("strong_password_from_env");

// ❌ BAD: Never store unencrypted private keys
// let account = Account::from_wif("unencrypted_wif");
```

#### Environment Variables
```bash
# Use strong passwords from environment
export NEO_WALLET_PASSWORD="$(openssl rand -base64 32)"
export NEO_RPC_ENDPOINT="https://mainnet.neo.org:443"
export NEO_NETWORK_MAGIC="860833102"  # MainNet
```

#### Backup Strategy
```rust,no_run
use neo3::neo_wallets::WalletBackup;
use std::path::PathBuf;

async fn secure_backup_strategy(wallet: &Wallet) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create encrypted backup
    let backup_path = PathBuf::from("/secure/backup/location/wallet.json");
    WalletBackup::backup(wallet, backup_path.clone())?;
    
    // 2. Verify backup integrity
    let recovered = WalletBackup::recover(backup_path.clone())?;
    assert_eq!(wallet.accounts().len(), recovered.accounts().len());
    
    // 3. Store in multiple secure locations
    // - Encrypted cloud storage
    // - Hardware security modules
    // - Offline storage devices
    
    Ok(())
}
```

### Network Security

#### TLS Configuration
```rust,no_run
use neo3::neo_clients::{HttpProvider, RpcClient};

// ✅ Always use HTTPS in production
let provider = HttpProvider::new("https://mainnet.neo.org:443")?;
let client = RpcClient::new(provider);

// Configure timeouts and retry policies
let client = client
    .with_timeout(std::time::Duration::from_secs(30))
    .with_retry_policy(3, std::time::Duration::from_millis(1000));
```

#### Rate Limiting
```rust,no_run
use std::time::{Duration, Instant};
use tokio::time::sleep;

struct RateLimiter {
    last_request: Instant,
    min_interval: Duration,
}

impl RateLimiter {
    pub fn new(requests_per_second: u32) -> Self {
        Self {
            last_request: Instant::now(),
            min_interval: Duration::from_millis(1000 / requests_per_second as u64),
        }
    }
    
    pub async fn wait(&mut self) {
        let elapsed = self.last_request.elapsed();
        if elapsed < self.min_interval {
            sleep(self.min_interval - elapsed).await;
        }
        self.last_request = Instant::now();
    }
}
```

## Performance Optimization

### Connection Pooling
```rust,no_run
use std::sync::Arc;
use tokio::sync::Semaphore;

pub struct ConnectionPool {
    clients: Vec<RpcClient<HttpProvider>>,
    semaphore: Arc<Semaphore>,
}

impl ConnectionPool {
    pub fn new(endpoints: Vec<&str>, max_concurrent: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let clients = endpoints
            .into_iter()
            .map(|endpoint| {
                let provider = HttpProvider::new(endpoint)?;
                Ok(RpcClient::new(provider))
            })
            .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
        
        Ok(Self {
            clients,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        })
    }
    
    pub async fn execute<F, T>(&self, operation: F) -> Result<T, Box<dyn std::error::Error>>
    where
        F: Fn(&RpcClient<HttpProvider>) -> Result<T, Box<dyn std::error::Error>>,
    {
        let _permit = self.semaphore.acquire().await?;
        let client = &self.clients[rand::random::<usize>() % self.clients.len()];
        operation(client)
    }
}
```

### Caching Strategy
```rust,no_run
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

pub struct ResponseCache<T> {
    cache: RwLock<HashMap<String, (T, Instant)>>,
    ttl: Duration,
}

impl<T: Clone> ResponseCache<T> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            ttl,
        }
    }
    
    pub fn get(&self, key: &str) -> Option<T> {
        let cache = self.cache.read().ok()?;
        let (value, timestamp) = cache.get(key)?;
        
        if timestamp.elapsed() < self.ttl {
            Some(value.clone())
        } else {
            None
        }
    }
    
    pub fn set(&self, key: String, value: T) {
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(key, (value, Instant::now()));
        }
    }
}
```

## Monitoring & Logging

### Structured Logging
```rust,no_run
use tracing::{info, warn, error, instrument};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Initialize logging
fn init_logging() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().json())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}

#[instrument(skip(client))]
async fn monitored_rpc_call(
    client: &RpcClient<HttpProvider>,
    method: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let start = Instant::now();
    
    match client.call(method, vec![]).await {
        Ok(result) => {
            info!(
                method = method,
                duration_ms = start.elapsed().as_millis(),
                "RPC call successful"
            );
            Ok(result)
        }
        Err(e) => {
            error!(
                method = method,
                duration_ms = start.elapsed().as_millis(),
                error = %e,
                "RPC call failed"
            );
            Err(e)
        }
    }
}
```

### Health Checks
```rust,no_run
use serde_json::json;

pub struct HealthChecker {
    client: RpcClient<HttpProvider>,
}

impl HealthChecker {
    pub async fn check_health(&self) -> serde_json::Value {
        let mut status = json!({
            "status": "healthy",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "checks": {}
        });
        
        // Check RPC connectivity
        match self.client.get_block_count().await {
            Ok(block_count) => {
                status["checks"]["rpc"] = json!({
                    "status": "healthy",
                    "block_count": block_count
                });
            }
            Err(e) => {
                status["status"] = json!("unhealthy");
                status["checks"]["rpc"] = json!({
                    "status": "unhealthy",
                    "error": e.to_string()
                });
            }
        }
        
        status
    }
}
```

## Deployment Strategies

### Docker Configuration
```dockerfile
FROM rust:1.75-slim as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/neo-app /usr/local/bin/

# Security: Run as non-root user
RUN useradd -r -s /bin/false neoapp
USER neoapp

EXPOSE 8080
CMD ["neo-app"]
```

### Kubernetes Deployment
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: neo-app
spec:
  replicas: 3
  selector:
    matchLabels:
      app: neo-app
  template:
    metadata:
      labels:
        app: neo-app
    spec:
      containers:
      - name: neo-app
        image: neo-app:latest
        ports:
        - containerPort: 8080
        env:
        - name: NEO_WALLET_PASSWORD
          valueFrom:
            secretKeyRef:
              name: neo-secrets
              key: wallet-password
        - name: NEO_RPC_ENDPOINT
          value: "https://mainnet.neo.org:443"
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
```

## Error Handling

### Production Error Handling
```rust,no_run
use neo3::neo_error::{Neo3Error, Neo3Result};
use tracing::error;

pub async fn robust_transaction_send(
    client: &RpcClient<HttpProvider>,
    transaction: &Transaction,
) -> Neo3Result<String> {
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY: Duration = Duration::from_secs(1);
    
    for attempt in 1..=MAX_RETRIES {
        match client.send_raw_transaction(transaction).await {
            Ok(tx_hash) => {
                info!(
                    tx_hash = %tx_hash,
                    attempt = attempt,
                    "Transaction sent successfully"
                );
                return Ok(tx_hash);
            }
            Err(e) if attempt < MAX_RETRIES => {
                warn!(
                    error = %e,
                    attempt = attempt,
                    max_retries = MAX_RETRIES,
                    "Transaction send failed, retrying"
                );
                tokio::time::sleep(RETRY_DELAY * attempt).await;
            }
            Err(e) => {
                error!(
                    error = %e,
                    attempts = MAX_RETRIES,
                    "Transaction send failed after all retries"
                );
                return Err(Neo3Error::Network(e.into()));
            }
        }
    }
    
    unreachable!()
}
```

## Testing in Production

### Canary Deployments
```rust,no_run
pub struct CanaryDeployment {
    primary_client: RpcClient<HttpProvider>,
    canary_client: RpcClient<HttpProvider>,
    canary_percentage: u8,
}

impl CanaryDeployment {
    pub async fn execute_request<T>(&self, request: impl Fn(&RpcClient<HttpProvider>) -> T) -> T {
        if rand::random::<u8>() < self.canary_percentage {
            request(&self.canary_client)
        } else {
            request(&self.primary_client)
        }
    }
}
```

### Circuit Breaker Pattern
```rust,no_run
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};

pub struct CircuitBreaker {
    failure_count: AtomicU32,
    failure_threshold: u32,
    is_open: AtomicBool,
    last_failure_time: std::sync::Mutex<Option<Instant>>,
    timeout: Duration,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, timeout: Duration) -> Self {
        Self {
            failure_count: AtomicU32::new(0),
            failure_threshold,
            is_open: AtomicBool::new(false),
            last_failure_time: std::sync::Mutex::new(None),
            timeout,
        }
    }
    
    pub async fn call<F, T, E>(&self, operation: F) -> Result<T, E>
    where
        F: std::future::Future<Output = Result<T, E>>,
    {
        if self.is_open.load(Ordering::Relaxed) {
            if let Ok(last_failure) = self.last_failure_time.lock() {
                if let Some(time) = *last_failure {
                    if time.elapsed() > self.timeout {
                        self.is_open.store(false, Ordering::Relaxed);
                        self.failure_count.store(0, Ordering::Relaxed);
                    } else {
                        return Err(/* circuit breaker open error */);
                    }
                }
            }
        }
        
        match operation.await {
            Ok(result) => {
                self.failure_count.store(0, Ordering::Relaxed);
                Ok(result)
            }
            Err(e) => {
                let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
                if failures >= self.failure_threshold {
                    self.is_open.store(true, Ordering::Relaxed);
                    if let Ok(mut last_failure) = self.last_failure_time.lock() {
                        *last_failure = Some(Instant::now());
                    }
                }
                Err(e)
            }
        }
    }
}
```

## Conclusion

This production guide provides the foundation for deploying NeoRust SDK applications securely and efficiently. Always test thoroughly in staging environments that mirror production, implement comprehensive monitoring, and follow security best practices.

For additional support, consult the [API documentation](https://docs.rs/neo3) and [community resources](https://github.com/R3E-Network/NeoRust).
