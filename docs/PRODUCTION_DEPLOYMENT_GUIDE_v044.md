# Production Deployment Guide - NeoRust v0.4.4

## Overview

This guide covers best practices for deploying NeoRust applications in production environments, including configuration, monitoring, security, and optimization strategies.

## Table of Contents

1. [Environment Setup](#environment-setup)
2. [Configuration Management](#configuration-management)
3. [Client Configuration](#client-configuration)
4. [Connection Management](#connection-management)
5. [Security Best Practices](#security-best-practices)
6. [Monitoring and Observability](#monitoring-and-observability)
7. [Performance Optimization](#performance-optimization)
8. [High Availability](#high-availability)
9. [Deployment Strategies](#deployment-strategies)
10. [Troubleshooting](#troubleshooting)

## Environment Setup

### System Requirements

```yaml
# Minimum Requirements
CPU: 2 cores
RAM: 4 GB
Storage: 20 GB SSD
Network: 100 Mbps

# Recommended Production
CPU: 4+ cores
RAM: 8+ GB
Storage: 50+ GB SSD
Network: 1 Gbps
OS: Ubuntu 22.04 LTS / Debian 11
```

### Rust Installation

```bash
# Install Rust (stable)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable
rustup update

# Verify installation
rustc --version
cargo --version

# Install additional tools
cargo install cargo-audit
cargo install cargo-outdated
cargo install cargo-deny
```

### Project Setup

```toml
# Cargo.toml - Production configuration
[package]
name = "neo-app"
version = "1.0.0"
edition = "2021"

[dependencies]
neo3 = { version = "0.4.4", features = ["futures", "impl-serde"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
env_logger = "0.11"
log = "0.4"
config = "0.14"
prometheus = "0.13"
opentelemetry = "0.24"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
panic = "abort"

[profile.release-with-debug]
inherits = "release"
strip = false
debug = true
```

## Configuration Management

### Environment Variables

```bash
# .env.production
# Node Configuration
NEO_NODE_URL=https://mainnet.neo.org
NEO_NODE_BACKUP_URL=https://mainnet2.neo.org
NEO_NETWORK=MainNet

# Rate Limiting
RATE_LIMIT_PRESET=Standard
RATE_LIMIT_CUSTOM_RPS=10

# Connection Pool
CONNECTION_POOL_SIZE=20
CONNECTION_TIMEOUT_SECS=10
IDLE_TIMEOUT_SECS=300

# Security
WALLET_PATH=/secure/path/wallet.json
WALLET_PASSWORD_FILE=/secure/path/.wallet_pass
API_KEY_FILE=/secure/path/.api_key

# Monitoring
METRICS_ENABLED=true
METRICS_PORT=9090
LOG_LEVEL=info
LOG_FORMAT=json

# Performance
CACHE_ENABLED=true
CACHE_TTL_SECS=300
MAX_RETRIES=3
RETRY_DELAY_MS=1000
```

### Configuration File

```rust
// src/config.rs
use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub neo: NeoConfig,
    pub rate_limit: RateLimitConfig,
    pub connection: ConnectionConfig,
    pub security: SecurityConfig,
    pub monitoring: MonitoringConfig,
    pub performance: PerformanceConfig,
}

#[derive(Debug, Deserialize)]
pub struct NeoConfig {
    pub node_url: String,
    pub backup_url: Option<String>,
    pub network: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Config::builder()
            .add_source(File::with_name("config/default"))
            .add_source(File::with_name("config/production").required(false))
            .add_source(Environment::with_prefix("NEO"))
            .build()?
            .try_deserialize()
    }
}
```

## Client Configuration

### Production Client Setup

```rust
use neo3::prelude::*;
use neo3::neo_clients::{
    ProductionNeoClient, RateLimitPreset, ConnectionPoolConfig,
    CircuitBreaker, RetryPolicy
};
use std::time::Duration;

pub async fn create_production_client(config: &AppConfig) -> Result<ProductionNeoClient, ClientError> {
    // Connection pool configuration
    let pool_config = ConnectionPoolConfig {
        min_idle: 5,
        max_size: config.connection.pool_size,
        idle_timeout: Duration::from_secs(config.connection.idle_timeout),
        connection_timeout: Duration::from_secs(config.connection.timeout),
    };
    
    // Retry policy
    let retry_policy = RetryPolicy {
        max_retries: config.performance.max_retries,
        initial_delay: Duration::from_millis(config.performance.retry_delay_ms),
        max_delay: Duration::from_secs(30),
        exponential_base: 2,
    };
    
    // Circuit breaker
    let circuit_breaker = CircuitBreaker::new()
        .failure_threshold(5)
        .success_threshold(2)
        .timeout(Duration::from_secs(60));
    
    // Build client with all production features
    let client = ProductionNeoClient::builder()
        .endpoint(&config.neo.node_url)
        .backup_endpoint(config.neo.backup_url.as_deref())
        .rate_limit(RateLimitPreset::from_str(&config.rate_limit.preset)?)
        .connection_pool(pool_config)
        .retry_policy(retry_policy)
        .circuit_breaker(circuit_breaker)
        .build()
        .await?;
    
    Ok(client)
}
```

### Multi-Node Configuration

```rust
use neo3::neo_clients::MultiNodeClient;

pub async fn create_multi_node_client(nodes: Vec<String>) -> Result<MultiNodeClient, ClientError> {
    let client = MultiNodeClient::builder()
        .add_nodes(nodes)
        .health_check_interval(Duration::from_secs(30))
        .failover_threshold(3)
        .load_balancing_strategy(LoadBalancingStrategy::RoundRobin)
        .build()
        .await?;
    
    Ok(client)
}
```

## Connection Management

### Connection Pool Optimization

```rust
use neo3::neo_clients::ConnectionManager;

pub struct OptimizedConnectionManager {
    primary_pool: ConnectionPool,
    backup_pool: Option<ConnectionPool>,
    metrics: ConnectionMetrics,
}

impl OptimizedConnectionManager {
    pub async fn new(config: &ConnectionConfig) -> Result<Self, Error> {
        // Primary connection pool
        let primary_pool = ConnectionPool::builder()
            .min_idle(config.min_idle)
            .max_size(config.max_size)
            .idle_timeout(Duration::from_secs(config.idle_timeout))
            .max_lifetime(Duration::from_secs(3600))
            .connection_timeout(Duration::from_secs(config.timeout))
            .validation_interval(Duration::from_secs(30))
            .build()
            .await?;
        
        // Backup pool for failover
        let backup_pool = if let Some(backup_url) = &config.backup_url {
            Some(ConnectionPool::builder()
                .endpoint(backup_url)
                .max_size(config.max_size / 2)
                .build()
                .await?)
        } else {
            None
        };
        
        Ok(Self {
            primary_pool,
            backup_pool,
            metrics: ConnectionMetrics::new(),
        })
    }
    
    pub async fn get_connection(&self) -> Result<Connection, Error> {
        // Try primary pool first
        match self.primary_pool.get_timeout(Duration::from_secs(5)).await {
            Ok(conn) => {
                self.metrics.record_primary_success();
                Ok(conn)
            }
            Err(e) => {
                self.metrics.record_primary_failure();
                
                // Failover to backup
                if let Some(backup) = &self.backup_pool {
                    self.metrics.record_failover();
                    backup.get().await
                } else {
                    Err(e)
                }
            }
        }
    }
}
```

### Health Checks

```rust
use neo3::neo_clients::HealthChecker;

pub async fn setup_health_checks(client: &ProductionNeoClient) -> Result<(), Error> {
    let health_checker = HealthChecker::new(client.clone());
    
    // Start periodic health checks
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        
        loop {
            interval.tick().await;
            
            match health_checker.check_health().await {
                Ok(health) => {
                    if !health.is_healthy {
                        log::warn!("Node unhealthy: {:?}", health);
                        // Trigger alerts
                    }
                }
                Err(e) => {
                    log::error!("Health check failed: {}", e);
                    // Trigger critical alert
                }
            }
        }
    });
    
    Ok(())
}
```

## Security Best Practices

### Wallet Security

```rust
use neo3::neo_wallets::{SecureWallet, EncryptionConfig};

pub struct SecureWalletManager {
    wallet: SecureWallet,
    encryption: EncryptionConfig,
}

impl SecureWalletManager {
    pub fn new(wallet_path: &str, password_file: &str) -> Result<Self, Error> {
        // Read password from secure file
        let password = std::fs::read_to_string(password_file)?;
        
        // Configure encryption
        let encryption = EncryptionConfig {
            algorithm: EncryptionAlgorithm::AES256GCM,
            iterations: 100_000,
            key_derivation: KeyDerivation::Argon2id,
        };
        
        // Load wallet with encryption
        let wallet = SecureWallet::from_file_encrypted(
            wallet_path,
            password.trim(),
            encryption.clone(),
        )?;
        
        Ok(Self { wallet, encryption })
    }
    
    pub fn sign_transaction(&self, tx: &Transaction) -> Result<SignedTransaction, Error> {
        // Sign with secure wallet
        self.wallet.sign_transaction(tx)
    }
}
```

### API Key Management

```rust
use std::collections::HashMap;
use hmac::{Hmac, Mac};
use sha2::Sha256;

pub struct ApiKeyManager {
    keys: HashMap<String, ApiKey>,
    rate_limits: HashMap<String, RateLimit>,
}

impl ApiKeyManager {
    pub fn validate_request(&self, api_key: &str, signature: &str, body: &[u8]) -> bool {
        if let Some(key) = self.keys.get(api_key) {
            // Verify HMAC signature
            let mut mac = Hmac::<Sha256>::new_from_slice(key.secret.as_bytes()).unwrap();
            mac.update(body);
            
            // Constant-time comparison
            mac.verify_slice(signature.as_bytes()).is_ok()
        } else {
            false
        }
    }
    
    pub fn check_rate_limit(&mut self, api_key: &str) -> bool {
        if let Some(limit) = self.rate_limits.get_mut(api_key) {
            limit.check_and_update()
        } else {
            false
        }
    }
}
```

### Secure Communication

```rust
use rustls::ClientConfig;
use std::sync::Arc;

pub fn create_secure_client() -> Result<reqwest::Client, Error> {
    // Configure TLS
    let tls_config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(load_ca_certificates()?)
        .with_no_client_auth();
    
    // Build HTTPS client
    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .min_tls_version(reqwest::tls::Version::TLS_1_2)
        .https_only(true)
        .timeout(Duration::from_secs(30))
        .pool_idle_timeout(Duration::from_secs(300))
        .pool_max_idle_per_host(10)
        .build()?;
    
    Ok(client)
}
```

## Monitoring and Observability

### Prometheus Metrics

```rust
use prometheus::{Counter, Histogram, Registry, Gauge};
use neo3::neo_monitoring::MetricsExporter;

pub struct ApplicationMetrics {
    pub requests_total: Counter,
    pub request_duration: Histogram,
    pub active_connections: Gauge,
    pub gas_consumed: Counter,
    pub errors_total: Counter,
}

impl ApplicationMetrics {
    pub fn new(registry: &Registry) -> Result<Self, Error> {
        Ok(Self {
            requests_total: Counter::new("neo_requests_total", "Total requests")?,
            request_duration: Histogram::new("neo_request_duration_seconds", "Request duration")?,
            active_connections: Gauge::new("neo_active_connections", "Active connections")?,
            gas_consumed: Counter::new("neo_gas_consumed_total", "Total GAS consumed")?,
            errors_total: Counter::new("neo_errors_total", "Total errors")?,
        })
    }
    
    pub fn register(&self, registry: &Registry) -> Result<(), Error> {
        registry.register(Box::new(self.requests_total.clone()))?;
        registry.register(Box::new(self.request_duration.clone()))?;
        registry.register(Box::new(self.active_connections.clone()))?;
        registry.register(Box::new(self.gas_consumed.clone()))?;
        registry.register(Box::new(self.errors_total.clone()))?;
        Ok(())
    }
}

// Start metrics server
pub async fn start_metrics_server(port: u16) -> Result<(), Error> {
    let registry = Registry::new();
    let metrics = ApplicationMetrics::new(&registry)?;
    metrics.register(&registry)?;
    
    let exporter = MetricsExporter::new(registry);
    exporter.serve(port).await?;
    
    Ok(())
}
```

### Structured Logging

```rust
use serde_json::json;
use log::{info, error, warn};

pub fn setup_logging() {
    env_logger::Builder::from_default_env()
        .format(|buf, record| {
            use std::io::Write;
            
            let log_entry = json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "level": record.level().to_string(),
                "target": record.target(),
                "message": record.args().to_string(),
                "file": record.file().unwrap_or(""),
                "line": record.line().unwrap_or(0),
            });
            
            writeln!(buf, "{}", log_entry)
        })
        .init();
}

// Usage
pub fn log_transaction(tx_hash: &str, gas_used: i64, success: bool) {
    if success {
        info!(
            "Transaction successful";
            "tx_hash" => tx_hash,
            "gas_used" => gas_used,
        );
    } else {
        error!(
            "Transaction failed";
            "tx_hash" => tx_hash,
            "gas_used" => gas_used,
        );
    }
}
```

### Distributed Tracing

```rust
use opentelemetry::{global, sdk::trace as sdktrace, trace::Tracer};
use opentelemetry_jaeger::JaegerExporter;

pub fn setup_tracing() -> Result<(), Error> {
    global::set_text_map_propagator(opentelemetry_jaeger::Propagator::new());
    
    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_service_name("neo-app")
        .with_max_packet_size(9216)
        .install_batch(opentelemetry::runtime::Tokio)?;
    
    global::set_tracer_provider(tracer.provider().unwrap());
    
    Ok(())
}

// Usage
pub async fn traced_operation<T>(
    name: &str,
    operation: impl Future<Output = Result<T, Error>>,
) -> Result<T, Error> {
    let tracer = global::tracer("neo-app");
    let span = tracer.start(name);
    let cx = opentelemetry::Context::current_with_span(span);
    
    operation.with_context(cx).await
}
```

## Performance Optimization

### Caching Strategy

```rust
use lru::LruCache;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct CacheManager {
    block_cache: Arc<RwLock<LruCache<u32, Block>>>,
    tx_cache: Arc<RwLock<LruCache<H256, Transaction>>>,
    gas_cache: Arc<RwLock<LruCache<Vec<u8>, i64>>>,
}

impl CacheManager {
    pub fn new(capacity: usize) -> Self {
        Self {
            block_cache: Arc::new(RwLock::new(LruCache::new(capacity))),
            tx_cache: Arc::new(RwLock::new(LruCache::new(capacity))),
            gas_cache: Arc::new(RwLock::new(LruCache::new(capacity / 2))),
        }
    }
    
    pub async fn get_block(&self, height: u32, client: &ProductionNeoClient) -> Result<Block, Error> {
        // Check cache first
        {
            let cache = self.block_cache.read().await;
            if let Some(block) = cache.get(&height) {
                return Ok(block.clone());
            }
        }
        
        // Fetch from network
        let block = client.get_block(BlockId::Height(height)).await?;
        
        // Update cache
        {
            let mut cache = self.block_cache.write().await;
            cache.put(height, block.clone());
        }
        
        Ok(block)
    }
}
```

### Batch Processing

```rust
use futures::stream::{self, StreamExt};

pub async fn batch_process_transactions(
    client: &ProductionNeoClient,
    transactions: Vec<Transaction>,
    batch_size: usize,
) -> Vec<Result<H256, Error>> {
    stream::iter(transactions)
        .chunks(batch_size)
        .map(|batch| async move {
            let futures: Vec<_> = batch
                .into_iter()
                .map(|tx| client.send_transaction(tx))
                .collect();
            
            futures::future::join_all(futures).await
        })
        .buffer_unordered(4) // Process 4 batches concurrently
        .flat_map(stream::iter)
        .collect()
        .await
}
```

## High Availability

### Failover Strategy

```rust
pub struct FailoverClient {
    primary: ProductionNeoClient,
    secondary: ProductionNeoClient,
    current: Arc<RwLock<ClientSelection>>,
}

enum ClientSelection {
    Primary,
    Secondary,
}

impl FailoverClient {
    pub async fn execute<T, F, Fut>(&self, operation: F) -> Result<T, Error>
    where
        F: Fn(ProductionNeoClient) -> Fut,
        Fut: Future<Output = Result<T, Error>>,
    {
        let current = self.current.read().await.clone();
        
        let result = match current {
            ClientSelection::Primary => operation(self.primary.clone()).await,
            ClientSelection::Secondary => operation(self.secondary.clone()).await,
        };
        
        match result {
            Ok(value) => Ok(value),
            Err(e) => {
                // Switch to backup
                let mut selection = self.current.write().await;
                *selection = match *selection {
                    ClientSelection::Primary => ClientSelection::Secondary,
                    ClientSelection::Secondary => ClientSelection::Primary,
                };
                
                // Retry with backup
                match *selection {
                    ClientSelection::Primary => operation(self.primary.clone()).await,
                    ClientSelection::Secondary => operation(self.secondary.clone()).await,
                }
            }
        }
    }
}
```

## Deployment Strategies

### Docker Deployment

```dockerfile
# Dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/neo-app /usr/local/bin/

ENV RUST_LOG=info
ENV NEO_NODE_URL=https://mainnet.neo.org

EXPOSE 8080 9090

CMD ["neo-app"]
```

### Kubernetes Deployment

```yaml
# deployment.yaml
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
        - containerPort: 9090
        env:
        - name: NEO_NODE_URL
          valueFrom:
            configMapKeyRef:
              name: neo-config
              key: node_url
        - name: RATE_LIMIT_PRESET
          value: "Standard"
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

## Troubleshooting

### Common Issues

#### Connection Timeouts
```rust
// Increase timeout and add retry
let client = ProductionNeoClient::builder()
    .endpoint(url)
    .connection_timeout(Duration::from_secs(30))
    .retry_policy(RetryPolicy::exponential(3))
    .build()
    .await?;
```

#### High Memory Usage
```rust
// Limit cache size and connection pool
let cache = LruCache::new(100); // Limit to 100 items
let pool_config = ConnectionPoolConfig {
    max_size: 10, // Reduce pool size
    idle_timeout: Duration::from_secs(60), // Shorter idle timeout
    ..Default::default()
};
```

#### Rate Limiting Issues
```rust
// Switch to more conservative preset
let client = ProductionNeoClient::new(
    url,
    RateLimitPreset::Conservative, // 5 req/s instead of 10
).await?;
```

### Debug Mode

```rust
#[cfg(debug_assertions)]
pub fn enable_debug_mode() {
    std::env::set_var("RUST_LOG", "debug");
    std::env::set_var("RUST_BACKTRACE", "1");
    
    // Enable detailed logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();
}
```

## Summary

Key production deployment considerations:

1. **Configuration**: Use environment variables and config files
2. **Security**: Encrypt wallets, use TLS, validate API keys
3. **Monitoring**: Implement metrics, logging, and tracing
4. **Performance**: Cache data, batch operations, optimize connections
5. **Reliability**: Use circuit breakers, retries, and failover
6. **Deployment**: Containerize with Docker/Kubernetes
7. **Maintenance**: Regular updates and security audits

Following these practices ensures your NeoRust application runs reliably and efficiently in production.