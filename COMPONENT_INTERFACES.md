# NeoRust SDK - Component Interface Design

## 1. Component Overview

This document defines the interfaces between major components of the NeoRust SDK, ensuring loose coupling and high cohesion.

## 2. Core Component Interfaces

### 2.1 Transaction Builder Interface

```rust
/// Transaction building interface with fluent API
pub trait TransactionBuilderInterface: Send + Sync {
    /// Set the transaction script
    fn set_script(&mut self, script: Vec<u8>) -> &mut Self;
    
    /// Add a signer to the transaction
    fn add_signer(&mut self, signer: Signer) -> &mut Self;
    
    /// Set transaction validity period
    fn valid_until_block(&mut self, block: u32) -> Result<&mut Self, Error>;
    
    /// Set system fee
    fn set_system_fee(&mut self, fee: i64) -> Result<&mut Self, Error>;
    
    /// Set network fee
    fn set_network_fee(&mut self, fee: i64) -> Result<&mut Self, Error>;
    
    /// Build the transaction
    async fn build(&mut self) -> Result<Transaction, Error>;
    
    /// Sign the transaction
    async fn sign(&mut self) -> Result<Transaction, Error>;
    
    /// Estimate gas consumption
    async fn estimate_gas(&self) -> Result<GasEstimate, Error>;
}

/// Gas estimation interface
pub trait GasEstimatorInterface {
    /// Estimate gas for a script
    async fn estimate(&self, script: &[u8], signers: Vec<Signer>) -> Result<i64, Error>;
    
    /// Estimate with safety margin
    async fn estimate_with_margin(&self, script: &[u8], signers: Vec<Signer>, margin: u8) -> Result<i64, Error>;
    
    /// Batch estimation
    async fn batch_estimate(&self, scripts: Vec<(&[u8], Vec<Signer>)>) -> Result<Vec<i64>, Error>;
}
```

### 2.2 Network Client Interface

```rust
/// Core network client interface
#[async_trait]
pub trait NetworkClientInterface: Send + Sync {
    /// Execute RPC call
    async fn call(&self, method: &str, params: Vec<Value>) -> Result<Value, Error>;
    
    /// Get network status
    async fn status(&self) -> Result<NetworkStatus, Error>;
    
    /// Check if connected
    fn is_connected(&self) -> bool;
    
    /// Reconnect to network
    async fn reconnect(&mut self) -> Result<(), Error>;
}

/// Connection pool interface
pub trait ConnectionPoolInterface {
    /// Get a connection from the pool
    async fn get_connection(&self) -> Result<Connection, Error>;
    
    /// Return a connection to the pool
    fn return_connection(&self, conn: Connection);
    
    /// Get pool statistics
    fn stats(&self) -> PoolStats;
    
    /// Clear all connections
    async fn clear(&mut self);
}

/// Rate limiter interface
pub trait RateLimiterInterface {
    /// Acquire permission to make a request
    async fn acquire(&self) -> Result<Permit, Error>;
    
    /// Try to acquire without waiting
    async fn try_acquire(&self) -> Result<Permit, Error>;
    
    /// Get remaining capacity
    async fn remaining_capacity(&self) -> usize;
    
    /// Reset the limiter
    async fn reset(&mut self);
}
```

### 2.3 Wallet Interface

```rust
/// Wallet management interface
pub trait WalletInterface: Send + Sync {
    /// Create a new account
    fn create_account(&mut self) -> Result<Account, Error>;
    
    /// Import account from WIF
    fn import_account(&mut self, wif: &str) -> Result<Account, Error>;
    
    /// Get account by address
    fn get_account(&self, address: &str) -> Option<&Account>;
    
    /// List all accounts
    fn list_accounts(&self) -> Vec<&Account>;
    
    /// Set default account
    fn set_default_account(&mut self, index: usize) -> Result<(), Error>;
    
    /// Get default account
    fn get_default_account(&self) -> Option<&Account>;
    
    /// Encrypt all accounts
    fn encrypt(&mut self, password: &str) -> Result<(), Error>;
    
    /// Decrypt all accounts
    fn decrypt(&mut self, password: &str) -> Result<(), Error>;
    
    /// Save wallet to file
    fn save(&self, path: &Path) -> Result<(), Error>;
    
    /// Load wallet from file
    fn load(path: &Path) -> Result<Self, Error> where Self: Sized;
}

/// Account interface
pub trait AccountInterface {
    /// Get account address
    fn get_address(&self) -> String;
    
    /// Get script hash
    fn get_script_hash(&self) -> ScriptHash;
    
    /// Get public key
    fn get_public_key(&self) -> PublicKey;
    
    /// Sign a message
    fn sign(&self, message: &[u8]) -> Result<Signature, Error>;
    
    /// Verify a signature
    fn verify(&self, message: &[u8], signature: &Signature) -> Result<bool, Error>;
    
    /// Export as WIF
    fn export(&self) -> Result<String, Error>;
    
    /// Check if multi-signature
    fn is_multi_sig(&self) -> bool;
}
```

### 2.4 Smart Contract Interface

```rust
/// Smart contract interaction interface
#[async_trait]
pub trait ContractInterface: Send + Sync {
    /// Get contract hash
    fn hash(&self) -> ScriptHash;
    
    /// Invoke contract method (read-only)
    async fn invoke(&self, method: &str, params: Vec<ContractParameter>) -> Result<InvocationResult, Error>;
    
    /// Call contract method (state change)
    async fn call(&self, method: &str, params: Vec<ContractParameter>, signers: Vec<Signer>) -> Result<Transaction, Error>;
    
    /// Get contract state
    async fn get_state(&self) -> Result<ContractState, Error>;
    
    /// Get contract manifest
    async fn get_manifest(&self) -> Result<ContractManifest, Error>;
}

/// NEP-17 token interface
#[async_trait]
pub trait Nep17Interface: ContractInterface {
    /// Get token symbol
    async fn symbol(&self) -> Result<String, Error>;
    
    /// Get token decimals
    async fn decimals(&self) -> Result<u8, Error>;
    
    /// Get total supply
    async fn total_supply(&self) -> Result<BigInt, Error>;
    
    /// Get balance of account
    async fn balance_of(&self, account: &ScriptHash) -> Result<BigInt, Error>;
    
    /// Transfer tokens
    async fn transfer(&self, from: &Account, to: &ScriptHash, amount: BigInt, data: Option<String>) -> Result<Transaction, Error>;
}

/// NEP-11 NFT interface
#[async_trait]
pub trait Nep11Interface: ContractInterface {
    /// Get token properties
    async fn properties(&self, token_id: &[u8]) -> Result<Map<String, Value>, Error>;
    
    /// Get owner of token
    async fn owner_of(&self, token_id: &[u8]) -> Result<ScriptHash, Error>;
    
    /// Get tokens of owner
    async fn tokens_of(&self, owner: &ScriptHash) -> Result<Vec<Vec<u8>>, Error>;
    
    /// Transfer NFT
    async fn transfer(&self, from: &Account, to: &ScriptHash, token_id: &[u8]) -> Result<Transaction, Error>;
}
```

### 2.5 Storage Interface

```rust
/// Cache interface for various storage backends
pub trait CacheInterface: Send + Sync {
    /// Get value by key
    async fn get(&self, key: &str) -> Option<Vec<u8>>;
    
    /// Set value with TTL
    async fn set(&self, key: &str, value: Vec<u8>, ttl: Duration) -> Result<(), Error>;
    
    /// Delete key
    async fn delete(&self, key: &str) -> Result<(), Error>;
    
    /// Clear all entries
    async fn clear(&mut self) -> Result<(), Error>;
    
    /// Get cache statistics
    fn stats(&self) -> CacheStats;
}

/// Persistent storage interface
#[async_trait]
pub trait StorageInterface: Send + Sync {
    /// Store data
    async fn put(&self, key: &[u8], value: &[u8]) -> Result<(), Error>;
    
    /// Retrieve data
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Error>;
    
    /// Delete data
    async fn delete(&self, key: &[u8]) -> Result<(), Error>;
    
    /// Iterate over keys with prefix
    async fn iterate_prefix(&self, prefix: &[u8]) -> Result<Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)>>, Error>;
    
    /// Batch operations
    async fn batch(&self, ops: Vec<StorageOp>) -> Result<(), Error>;
}
```

### 2.6 Monitoring Interface

```rust
/// Metrics collection interface
pub trait MetricsInterface: Send + Sync {
    /// Increment counter
    fn inc_counter(&self, name: &str, labels: &[(&str, &str)]);
    
    /// Record histogram value
    fn record_histogram(&self, name: &str, value: f64, labels: &[(&str, &str)]);
    
    /// Set gauge value
    fn set_gauge(&self, name: &str, value: f64, labels: &[(&str, &str)]);
    
    /// Export metrics
    fn export(&self) -> String;
}

/// Health check interface
#[async_trait]
pub trait HealthCheckInterface: Send + Sync {
    /// Check if service is healthy
    async fn is_healthy(&self) -> bool;
    
    /// Get detailed health status
    async fn health_status(&self) -> HealthStatus;
    
    /// Register health check
    fn register_check(&mut self, name: String, check: Box<dyn HealthCheck>);
}

/// Tracing interface
pub trait TracingInterface {
    /// Create a new span
    fn span(&self, name: &str) -> Span;
    
    /// Log an event
    fn event(&self, level: Level, message: &str);
    
    /// Add context to current span
    fn add_context(&self, key: &str, value: &dyn Debug);
}
```

## 3. Component Interaction Patterns

### 3.1 Dependency Injection Pattern

```rust
/// Service container for dependency injection
pub struct ServiceContainer {
    network_client: Arc<dyn NetworkClientInterface>,
    wallet: Arc<dyn WalletInterface>,
    cache: Arc<dyn CacheInterface>,
    metrics: Arc<dyn MetricsInterface>,
}

impl ServiceContainer {
    /// Create new container with dependencies
    pub fn new(
        network_client: Arc<dyn NetworkClientInterface>,
        wallet: Arc<dyn WalletInterface>,
        cache: Arc<dyn CacheInterface>,
        metrics: Arc<dyn MetricsInterface>,
    ) -> Self {
        Self {
            network_client,
            wallet,
            cache,
            metrics,
        }
    }
    
    /// Get network client
    pub fn network(&self) -> Arc<dyn NetworkClientInterface> {
        self.network_client.clone()
    }
    
    /// Get wallet
    pub fn wallet(&self) -> Arc<dyn WalletInterface> {
        self.wallet.clone()
    }
}
```

### 3.2 Observer Pattern for Events

```rust
/// Event emitter interface
pub trait EventEmitter {
    /// Subscribe to events
    fn subscribe(&mut self, event: EventType, handler: Box<dyn EventHandler>);
    
    /// Emit an event
    fn emit(&self, event: Event);
}

/// Event handler interface
pub trait EventHandler: Send + Sync {
    /// Handle an event
    fn handle(&self, event: &Event);
}

/// Event types
#[derive(Clone, Debug)]
pub enum EventType {
    BlockAdded,
    TransactionConfirmed,
    ContractDeployed,
    TokenTransferred,
    WalletUpdated,
}

/// Event data
#[derive(Clone, Debug)]
pub struct Event {
    pub event_type: EventType,
    pub data: Value,
    pub timestamp: DateTime<Utc>,
}
```

### 3.3 Strategy Pattern for Providers

```rust
/// Provider strategy interface
pub trait ProviderStrategy: Send + Sync {
    /// Select provider based on criteria
    fn select(&self, providers: &[Provider]) -> Option<&Provider>;
    
    /// Update provider statistics
    fn update_stats(&mut self, provider: &Provider, success: bool, latency: Duration);
}

/// Round-robin strategy
pub struct RoundRobinStrategy {
    current: AtomicUsize,
}

impl ProviderStrategy for RoundRobinStrategy {
    fn select(&self, providers: &[Provider]) -> Option<&Provider> {
        if providers.is_empty() {
            return None;
        }
        let index = self.current.fetch_add(1, Ordering::Relaxed) % providers.len();
        Some(&providers[index])
    }
    
    fn update_stats(&mut self, _provider: &Provider, _success: bool, _latency: Duration) {
        // No statistics needed for round-robin
    }
}

/// Latency-based strategy
pub struct LatencyStrategy {
    stats: HashMap<String, ProviderStats>,
}

impl ProviderStrategy for LatencyStrategy {
    fn select(&self, providers: &[Provider]) -> Option<&Provider> {
        providers
            .iter()
            .min_by_key(|p| {
                self.stats
                    .get(&p.id)
                    .map(|s| s.avg_latency)
                    .unwrap_or(Duration::from_secs(0))
            })
    }
    
    fn update_stats(&mut self, provider: &Provider, success: bool, latency: Duration) {
        let stats = self.stats.entry(provider.id.clone()).or_default();
        stats.update(success, latency);
    }
}
```

## 4. Plugin System Interface

### 4.1 Plugin Definition

```rust
/// Plugin interface for extending SDK functionality
pub trait Plugin: Send + Sync {
    /// Plugin name
    fn name(&self) -> &str;
    
    /// Plugin version
    fn version(&self) -> &str;
    
    /// Initialize plugin
    async fn init(&mut self, context: &PluginContext) -> Result<(), Error>;
    
    /// Start plugin
    async fn start(&mut self) -> Result<(), Error>;
    
    /// Stop plugin
    async fn stop(&mut self) -> Result<(), Error>;
    
    /// Handle SDK events
    async fn on_event(&mut self, event: &Event) -> Result<(), Error>;
}

/// Plugin context for accessing SDK services
pub struct PluginContext {
    pub network: Arc<dyn NetworkClientInterface>,
    pub wallet: Arc<dyn WalletInterface>,
    pub cache: Arc<dyn CacheInterface>,
    pub metrics: Arc<dyn MetricsInterface>,
}

/// Plugin manager
pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
    context: PluginContext,
}

impl PluginManager {
    /// Register a plugin
    pub fn register(&mut self, plugin: Box<dyn Plugin>) -> Result<(), Error> {
        self.plugins.push(plugin);
        Ok(())
    }
    
    /// Initialize all plugins
    pub async fn init_all(&mut self) -> Result<(), Error> {
        for plugin in &mut self.plugins {
            plugin.init(&self.context).await?;
        }
        Ok(())
    }
    
    /// Start all plugins
    pub async fn start_all(&mut self) -> Result<(), Error> {
        for plugin in &mut self.plugins {
            plugin.start().await?;
        }
        Ok(())
    }
}
```

## 5. Middleware Interface

### 5.1 Request/Response Middleware

```rust
/// Middleware trait for processing requests and responses
#[async_trait]
pub trait Middleware: Send + Sync {
    /// Process request before sending
    async fn process_request(&self, request: &mut Request) -> Result<(), Error>;
    
    /// Process response after receiving
    async fn process_response(&self, response: &mut Response) -> Result<(), Error>;
}

/// Logging middleware
pub struct LoggingMiddleware {
    logger: Logger,
}

#[async_trait]
impl Middleware for LoggingMiddleware {
    async fn process_request(&self, request: &mut Request) -> Result<(), Error> {
        self.logger.info(&format!("Request: {:?}", request));
        Ok(())
    }
    
    async fn process_response(&self, response: &mut Response) -> Result<(), Error> {
        self.logger.info(&format!("Response: {:?}", response));
        Ok(())
    }
}

/// Retry middleware
pub struct RetryMiddleware {
    max_retries: u32,
    backoff: Duration,
}

#[async_trait]
impl Middleware for RetryMiddleware {
    async fn process_request(&self, request: &mut Request) -> Result<(), Error> {
        request.metadata.insert("retry_count", "0");
        Ok(())
    }
    
    async fn process_response(&self, response: &mut Response) -> Result<(), Error> {
        if response.is_error() && response.is_retryable() {
            let retry_count = request.metadata
                .get("retry_count")
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0);
                
            if retry_count < self.max_retries {
                sleep(self.backoff * 2_u32.pow(retry_count)).await;
                request.metadata.insert("retry_count", &(retry_count + 1).to_string());
                return Err(Error::Retry);
            }
        }
        Ok(())
    }
}
```

## 6. Testing Interfaces

### 6.1 Mock Interfaces

```rust
/// Mock network client for testing
pub struct MockNetworkClient {
    responses: HashMap<String, Value>,
    call_count: AtomicUsize,
}

#[async_trait]
impl NetworkClientInterface for MockNetworkClient {
    async fn call(&self, method: &str, _params: Vec<Value>) -> Result<Value, Error> {
        self.call_count.fetch_add(1, Ordering::Relaxed);
        self.responses
            .get(method)
            .cloned()
            .ok_or_else(|| Error::NotFound(method.to_string()))
    }
    
    async fn status(&self) -> Result<NetworkStatus, Error> {
        Ok(NetworkStatus::Connected)
    }
    
    fn is_connected(&self) -> bool {
        true
    }
    
    async fn reconnect(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

/// Test fixture interface
pub trait TestFixture {
    /// Setup test environment
    async fn setup(&mut self) -> Result<(), Error>;
    
    /// Teardown test environment
    async fn teardown(&mut self) -> Result<(), Error>;
    
    /// Get test client
    fn client(&self) -> &dyn NetworkClientInterface;
    
    /// Get test wallet
    fn wallet(&self) -> &dyn WalletInterface;
}
```

## 7. Interface Versioning

### 7.1 Version Management

```rust
/// Versioned interface trait
pub trait VersionedInterface {
    /// Get interface version
    fn version(&self) -> Version;
    
    /// Check if version is compatible
    fn is_compatible(&self, required: &Version) -> bool;
}

/// Version struct
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    /// Check if compatible with another version
    pub fn is_compatible_with(&self, other: &Version) -> bool {
        self.major == other.major && self.minor >= other.minor
    }
}
```

## 8. Interface Documentation

### 8.1 Documentation Standards

All interfaces must include:
- Purpose description
- Method documentation with parameters and return values
- Example usage
- Error conditions
- Thread safety guarantees
- Version information

### 8.2 Example Documentation

```rust
/// Network client interface for blockchain communication
/// 
/// This interface provides methods for interacting with Neo blockchain nodes
/// through various transport protocols (HTTP, WebSocket, IPC).
/// 
/// # Thread Safety
/// 
/// All implementations must be thread-safe and support concurrent access.
/// 
/// # Example
/// 
/// ```rust
/// let client: Arc<dyn NetworkClientInterface> = Arc::new(HttpClient::new(url)?);
/// let result = client.call("getblockcount", vec![]).await?;
/// ```
/// 
/// # Version
/// 
/// Interface version: 1.0.0
#[async_trait]
pub trait NetworkClientInterface: Send + Sync {
    // ... methods ...
}
```

---

**Document Version**: 1.0  
**Last Updated**: August 19, 2025  
**Interface Stability**: Stable