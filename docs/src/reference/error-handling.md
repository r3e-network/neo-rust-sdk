# Error Handling

This reference provides information about error handling in the NeoRust SDK, including error types, error propagation, and best practices for handling errors.

## Error Types

The NeoRust SDK uses a comprehensive error handling system based on Rust's `Result` type. The main error types in the SDK include:

### Neo3Error

The `Neo3Error` is the primary error type used throughout the SDK. It encompasses various error categories:

```rust,no_run
use neo3::neo_error::Neo3Error;

pub enum Neo3Error {
    // Cryptographic operation errors
    Crypto(CryptoError),
    
    // Wallet operation errors
    Wallet(WalletError),
    
    // Network/RPC communication errors
    Network(NetworkError),
    
    // Transaction building/validation errors
    Transaction(TransactionError),
    
    // Smart contract interaction errors
    Contract(ContractError),
    
    // Serialization/deserialization errors
    Serialization(SerializationError),
    
    // Configuration errors
    Config(String),
    
    // Generic errors with context
    Generic { message: String },
    
    // Unsupported operation error
    UnsupportedOperation(String),
}
```

### RpcError

The `RpcError` represents errors that occur during RPC communication with Neo nodes:

```rust,no_run
pub enum RpcError {
    // HTTP errors
    HttpError(reqwest::Error),
    
    // JSON-RPC errors
    JsonRpcError {
        code: i64,
        message: String,
        data: Option<serde_json::Value>,
    },
    
    // WebSocket errors
    WebSocketError(String),
    
    // Timeout errors
    TimeoutError,
    
    // Other errors
    Other(String),
}
```

### WalletError

The `WalletError` represents errors related to wallet operations:

```rust,no_run
pub enum WalletError {
    // Password errors
    InvalidPassword,
    
    // Account errors
    AccountNotFound,
    InvalidAccount,
    
    // Key errors
    InvalidPrivateKey,
    InvalidPublicKey,
    
    // File errors
    FileError(std::io::Error),
    
    // Other errors
    Other(String),
}
```

### CryptoError

The `CryptoError` represents errors related to cryptographic operations:

```rust,no_run
pub enum CryptoError {
    // Signature errors
    SignatureError,
    VerificationError,
    
    // Key errors
    InvalidKey,
    
    // Hash errors
    HashError,
    
    // Other errors
    Other(String),
}
```

### TransactionError

The `TransactionError` represents errors related to transaction operations:

```rust,no_run
pub enum TransactionError {
    // Validation errors
    InvalidTransaction,
    InvalidSignature,
    
    // Fee errors
    InsufficientFunds,
    
    // Network errors
    NetworkError,
    
    // Other errors
    Other(String),
}
```

### ContractError

The `ContractError` represents errors related to smart contract operations:

```rust,no_run
pub enum ContractError {
    // Invocation errors
    InvocationError,
    
    // Parameter errors
    InvalidParameter,
    
    // Execution errors
    ExecutionError,
    
    // Other errors
    Other(String),
}
```

## Error Propagation

The NeoRust SDK uses Rust's `?` operator for error propagation. This allows for concise error handling code:

```rust,no_run
use neo3::prelude::*;
use std::path::Path;

fn load_wallet_and_get_balance(
    wallet_path: &Path,
    password: &str,
    provider: &Provider,
    token_hash: ScriptHash,
) -> Result<u64, NeoError> {
    // Load the wallet
    let wallet = Wallet::load(wallet_path, password)?;
    
    // Get the default account
    let account = wallet.default_account()?;
    
    // Create a NEP-17 token instance
    let token = Nep17Contract::new(token_hash, provider.clone());
    
    // Get the token balance
    let balance = token.balance_of(account.address())?;
    
    Ok(balance)
}
```

In this example, if any of the operations fail, the error is propagated up the call stack.

### Adding Context to Errors

Sometimes it's useful to add context to errors to make them more informative:

```rust,no_run
use neo3::prelude::*;
use std::path::Path;

fn load_wallet_and_get_balance(
    wallet_path: &Path,
    password: &str,
    provider: &Provider,
    token_hash: ScriptHash,
) -> Result<u64, NeoError> {
    // Load the wallet with context
    let wallet = Wallet::load(wallet_path, password)
        .map_err(|e| NeoError::IllegalState(format!("Failed to load wallet: {}", e)))?;
    
    // Get the default account with context
    let account = wallet.default_account()
        .map_err(|e| NeoError::IllegalState(format!("Failed to get default account: {}", e)))?;
    
    // Create a NEP-17 token instance
    let token = Nep17Contract::new(token_hash, provider.clone());
    
    // Get the token balance with context
    let balance = token.balance_of(account.address())
        .map_err(|e| NeoError::IllegalState(format!("Failed to get token balance: {}", e)))?;
    
    Ok(balance)
}
```

### Using Option to Result Conversion

The NeoRust SDK provides utility functions for converting `Option` to `Result`:

```rust,no_run
use neo3::prelude::*;

fn get_value_from_option<T>(option: Option<T>, error_message: &str) -> Result<T, NeoError> {
    option.ok_or_else(|| NeoError::IllegalState(error_message.to_string()))
}

fn example() -> Result<(), NeoError> {
    let optional_value: Option<u64> = Some(42);
    
    // Convert Option to Result with a custom error message
    let value = get_value_from_option(optional_value, "Value is None")?;
    
    // Or use the ok_or_else method directly
    let value = optional_value.ok_or_else(|| NeoError::IllegalState("Value is None".to_string()))?;
    
    Ok(())
}
```

## Converting Between Error Types

The NeoRust SDK provides comprehensive `From` implementations for converting between different error types:

```rust,no_run
// From implementations for domain-specific errors
impl From<BuilderError> for NeoError {
    fn from(err: BuilderError) -> Self {
        // Conversion logic that maps BuilderError variants to appropriate NeoError variants
    }
}

impl From<CryptoError> for NeoError {
    fn from(err: CryptoError) -> Self {
        // Conversion logic that maps CryptoError variants to appropriate NeoError variants
    }
}

impl From<WalletError> for NeoError {
    fn from(err: WalletError) -> Self {
        NeoError::WalletError(err)
    }
}

// From implementations for standard library errors
impl From<std::io::Error> for NeoError {
    fn from(err: std::io::Error) -> Self {
        NeoError::IoError(err)
    }
}

impl From<serde_json::Error> for NeoError {
    fn from(err: serde_json::Error) -> Self {
        NeoError::SerializationError(err.to_string())
    }
}

impl From<hex::FromHexError> for NeoError {
    fn from(err: hex::FromHexError) -> Self {
        NeoError::InvalidEncoding(format!("Hex error: {}", err))
    }
}

impl From<std::num::ParseIntError> for NeoError {
    fn from(err: std::num::ParseIntError) -> Self {
        NeoError::IllegalArgument(format!("Integer parsing error: {}", err))
    }
}
```

This allows for easy conversion between error types using the `?` operator. When you use the `?` operator on a function that returns a domain-specific error type, it will be automatically converted to `NeoError` if you're in a function that returns `Result<T, NeoError>`.

## Handling RPC Errors

When working with RPC calls, you may need to handle specific error codes:

```rust,no_run
use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo N3 TestNet node
    let provider = Provider::new_http("https://testnet1.neo.coz.io:443");
    
    // Try to get a transaction
    let tx_hash = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".parse::<TxHash>()?;
    
    match provider.get_transaction(&tx_hash).await {
        Ok(tx) => {
            println!("Transaction found: {:?}", tx);
        },
        Err(NeoError::RpcError(RpcError::JsonRpcError { code, message, .. })) if code == -100 => {
            println!("Transaction not found: {}", message);
        },
        Err(e) => {
            println!("Error: {}", e);
            return Err(e.into());
        }
    }
    
    Ok(())
}
```

## Handling Wallet Errors

When working with wallets, you may need to handle specific wallet errors:

```rust,no_run
use neo3::prelude::*;
use std::path::Path;

fn open_wallet(wallet_path: &Path, password: &str) -> Result<Wallet, NeoError> {
    match Wallet::load(wallet_path, password) {
        Ok(wallet) => {
            println!("Wallet loaded successfully");
            Ok(wallet)
        },
        Err(NeoError::WalletError(WalletError::InvalidPassword)) => {
            println!("Invalid password");
            Err(NeoError::WalletError(WalletError::InvalidPassword))
        },
        Err(NeoError::WalletError(WalletError::FileError(e))) => {
            println!("File error: {}", e);
            Err(NeoError::WalletError(WalletError::FileError(e)))
        },
        Err(e) => {
            println!("Error: {}", e);
            Err(e)
        }
    }
}
```

## Handling Transaction Errors

When sending transactions, you may need to handle specific transaction errors:

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
    
    // Create a transaction
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
        .sign(account)?
        .build();
    
    // Send the transaction
    match provider.send_raw_transaction(&transaction).await {
        Ok(txid) => {
            println!("Transaction sent with ID: {}", txid);
        },
        Err(NeoError::TransactionError(TransactionError::InsufficientFunds)) => {
            println!("Insufficient funds to send the transaction");
        },
        Err(NeoError::RpcError(RpcError::JsonRpcError { code, message, .. })) => {
            println!("RPC error: {} (code: {})", message, code);
        },
        Err(e) => {
            println!("Error: {}", e);
            return Err(e.into());
        }
    }
    
    Ok(())
}
```

## Custom Error Types

You can create custom error types for your application that wrap the NeoRust SDK errors:

```rust,no_run
use neo3::prelude::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Neo SDK error: {0}")]
    NeoError(#[from] NeoError),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Database error: {0}")]
    DbError(String),
    
    #[error("User error: {0}")]
    UserError(String),
}

fn app_function() -> Result<(), AppError> {
    // Use the NeoRust SDK
    let wallet = Wallet::new("password").map_err(AppError::NeoError)?;
    
    // Or with the ? operator
    let wallet = Wallet::new("password")?;
    
    Ok(())
}
```

## Error Logging

The NeoRust SDK uses the `tracing` crate for logging errors. You can configure the logging level to see more detailed error information:

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
    
    // Now errors will be logged with more detail
}
```

## Neo X Error Handling

If you're using the Neo X features, there are additional error types for Neo X-specific operations:

```rust,no_run
pub enum NeoXError {
    // EVM errors
    EvmError(String),
    
    // Bridge errors
    BridgeError(String),
    
    // Other errors
    Other(String),
}
```

Handling Neo X errors:

```rust,no_run
use neo3::prelude::*;
use neo3::neo_x::evm::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a Neo X provider
    let provider = NeoXProvider::new_http("https://rpc.neoX.io");
    
    // Create a transaction
    let transaction = NeoXTransaction::new()
        .to("0x1234567890123456789012345678901234567890")
        .value(1_000_000_000_000_000_000u128) // 1 ETH in wei
        .gas_price(20_000_000_000u64) // 20 Gwei
        .gas_limit(21_000u64)
        .build();
    
    // Send the transaction
    match provider.send_transaction(&transaction).await {
        Ok(txid) => {
            println!("Transaction sent with ID: {}", txid);
        },
        Err(NeoError::NeoXError(NeoXError::EvmError(message))) => {
            println!("EVM error: {}", message);
        },
        Err(e) => {
            println!("Error: {}", e);
            return Err(e.into());
        }
    }
    
    Ok(())
}
```

## Best Practices

1. **Use the `?` Operator**: Use the `?` operator for concise error propagation.
2. **Match on Specific Errors**: Match on specific error types when you need to handle them differently.
3. **Custom Error Types**: Create custom error types for your application that wrap the NeoRust SDK errors.
4. **Error Logging**: Configure logging to see more detailed error information.
5. **Error Context**: Add context to errors to make them more informative.
6. **Error Recovery**: Implement recovery strategies for recoverable errors.
7. **Error Testing**: Write tests for error conditions to ensure they're handled correctly.
8. **Avoid `unwrap()` and `expect()`**: In production code, avoid using `unwrap()` or `expect()` as they will panic on errors. Instead, use proper error handling with `Result` and the `?` operator.
9. **Convert Domain-Specific Errors**: Use `.into()` to convert domain-specific errors to `NeoError` when needed.
10. **Provide Descriptive Error Messages**: When creating errors, provide descriptive messages that help identify the cause of the error.

<!-- toc -->
