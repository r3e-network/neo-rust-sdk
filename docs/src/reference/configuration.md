# Configuration

This reference provides information about configuring the NeoRust SDK, including environment variables, network settings, and other configuration options.

## Network Configuration

The NeoRust SDK supports connecting to different Neo networks, including MainNet, TestNet, and private networks.

### Predefined Networks

The SDK includes predefined configurations for common Neo networks:

```rust,no_run
use neo3::prelude::*;

// Connect to Neo N3 MainNet
let mainnet_provider = Provider::new_http("https://mainnet1.neo.coz.io:443");

// Connect to Neo N3 TestNet
let testnet_provider = Provider::new_http("https://testnet1.neo.coz.io:443");

// Connect to a local Neo Express instance
let local_provider = Provider::new_http("http://localhost:10332");
```

### Custom Networks

You can also connect to custom Neo networks by providing the RPC URL:

```rust,no_run
use neo3::prelude::*;

// Connect to a custom Neo N3 node
let custom_provider = Provider::new_http("https://my-custom-neo-node.example.com:10332");
```

### WebSocket Connections

For applications that need real-time updates, you can use WebSocket connections:

```rust,no_run
use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo N3 TestNet node with WebSocket support
    let ws_provider = Provider::new_ws("wss://testnet1.neo.coz.io:4443/ws").await?;
    
    // Subscribe to new blocks
    let mut blocks = ws_provider.subscribe_blocks().await?;
    
    println!("Listening for new blocks...");
    
    // Process new blocks as they arrive
    while let Some(block) = blocks.next().await {
        println!("New block: {} (hash: {})", block.index, block.hash);
    }
    
    Ok(())
}
```

## SDK Configuration

The NeoRust SDK can be configured using the `Config` struct:

```rust,no_run
use neo3::prelude::*;

// Create a custom configuration
let config = Config::new()
    .network(Network::TestNet)
    .timeout(std::time::Duration::from_secs(30))
    .max_retry(3)
    .build();

// Create a provider with the custom configuration
let provider = Provider::with_config("https://testnet1.neo.coz.io:443", config);
```

### Configuration Options

The following options can be configured:

| Option | Description | Default |
|--------|-------------|---------|
| `network` | The Neo network to connect to | `Network::MainNet` |
| `timeout` | Request timeout | 30 seconds |
| `max_retry` | Maximum number of retry attempts | 3 |
| `retry_delay` | Delay between retry attempts | 1 second |
| `user_agent` | User agent string for HTTP requests | `"NeoRust/{version}"` |

## Environment Variables

The NeoRust SDK respects the following environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `NEO_RPC_URL` | Default RPC URL for Neo N3 | None |
| `NEO_WS_URL` | Default WebSocket URL for Neo N3 | None |
| `NEO_NETWORK` | Default network (`mainnet`, `testnet`) | `mainnet` |
| `NEO_PRIVATE_KEY` | Default private key for signing transactions | None |
| `NEO_GAS_PRICE` | Default gas price for transactions | Network default |
| `NEO_LOG_LEVEL` | Logging level (`error`, `warn`, `info`, `debug`, `trace`) | `info` |

You can set these environment variables in your shell or use a `.env` file with the `dotenv` crate:

```rust,no_run
use dotenv::dotenv;
use neo3::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenv().ok();
    
    // Create a provider using the NEO_RPC_URL environment variable
    let provider = Provider::from_env()?;
    
    Ok(())
}
```

## Logging Configuration

The NeoRust SDK uses the `tracing` crate for logging. You can configure the logging level and output:

```rust,no_run
use neo3::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

fn main() {
    // Initialize the logger with custom configuration
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env()
            .add_directive("neo=debug".parse().unwrap())
            .add_directive("warn".parse().unwrap()))
        .init();
    
    // Now logs will be output according to the configuration
}
```

You can also use the `NEO_LOG_LEVEL` environment variable to control the logging level:

```bash
# Set the log level to debug
export NEO_LOG_LEVEL=debug

# Run your application
cargo run
```

## Gas Configuration

You can configure gas settings for transactions:

```rust,no_run
use neo3::prelude::*;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo N3 TestNet node
    let provider = Provider::new_http("https://testnet1.neo.coz.io:443");
    
    // Load your wallet
    let wallet_path = Path::new("my-wallet.json");
    let password = "my-secure-password";
    let wallet = Wallet::load(wallet_path, password)?;
    
    // Get the account that will send the transaction
    let account = wallet.default_account()?;
    
    // Create a transaction with custom gas settings
    let transaction = TransactionBuilder::new()
        .version(0)
        .nonce(rand::random::<u32>())
        .valid_until_block(provider.get_block_count().await? + 100)
        .script(
            ScriptBuilder::new()
                .contract_call(
                    "d2a4cff31913016155e38e474a2c06d08be276cf".parse::<ScriptHash>()?,
                    "transfer",
                    &[
                        ContractParameter::hash160(account.address().script_hash()),
                        ContractParameter::hash160("NZNos2WqTbu5oCgyfss9kUJgBXJqhuYAaj".parse::<Address>()?),
                        ContractParameter::integer(1_00000000), // 1 GAS
                        ContractParameter::any(None),
                    ],
                )
                .to_array()
        )
        .system_fee(1_00000000) // 1 GAS system fee
        .network_fee(0_50000000) // 0.5 GAS network fee
        .sign(account)?
        .build();
    
    // Send the transaction
    let txid = provider.send_raw_transaction(&transaction).await?;
    println!("Transaction sent with ID: {}", txid);
    
    Ok(())
}
```

## SGX Configuration

If you're using the SGX features, you can configure the SGX environment:

```rust,no_run
use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure SGX
    let sgx_config = SgxConfig::new()
        .enclave_path("path/to/enclave.so")
        .simulation_mode(false)
        .build();
    
    // Initialize the SGX enclave with the configuration
    let enclave_manager = SgxEnclaveManager::with_config(sgx_config)?;
    
    // Use the enclave manager
    let wallet = enclave_manager.create_wallet("my-secure-password")?;
    
    Ok(())
}
```

You can also use environment variables for SGX configuration:

| Variable | Description | Default |
|----------|-------------|---------|
| `SGX_MODE` | SGX mode (`HW` or `SIM`) | `HW` |
| `SGX_ENCLAVE_PATH` | Path to the enclave shared object | None |
| `SGX_AESM_ADDR` | Address of the AESM service | `127.0.0.1:2222` |

## Best Practices

1. **Environment-Specific Configuration**: Use different configurations for development, testing, and production environments.
2. **Secure Credential Management**: Never hardcode private keys or passwords in your code.
3. **Timeout Configuration**: Set appropriate timeouts based on your network conditions.
4. **Logging Configuration**: Configure logging appropriately for your environment.
5. **Gas Estimation**: Use gas estimation functions instead of hardcoding gas values.
6. **Error Handling**: Implement proper error handling for configuration errors.

<!-- toc -->
