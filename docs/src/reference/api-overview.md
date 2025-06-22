# API Overview

This reference provides an overview of the NeoRust SDK API, including the main modules and their functionality.

## Core Modules

The NeoRust SDK is organized into several core modules, each responsible for a specific aspect of Neo blockchain interaction:

### neo_wallets

The `neo_wallets` module provides functionality for creating, loading, and managing Neo wallets and accounts.

```rust,no_run
use neo3::prelude::*;

// Create a new wallet
let wallet = Wallet::new("password")?;

// Create a new account
let account = wallet.create_account()?;

// Get account address
let address = account.address();
```

Key components:
- `Wallet`: Manages multiple accounts and provides wallet-level operations
- `Account`: Represents a Neo account with a key pair
- `Address`: Represents a Neo address

### neo_clients

The `neo_clients` module provides clients for interacting with Neo nodes via RPC.

```rust,no_run
use neo3::prelude::*;

// Create a provider connected to a Neo node
let provider = Provider::new_http("https://testnet1.neo.coz.io:443");

// Get the current block count
let block_count = provider.get_block_count().await?;
```

Key components:
- `Provider`: Main client for interacting with Neo nodes
- `RpcClient`: Low-level RPC client
- `WebSocketProvider`: Provider with WebSocket support for subscriptions

### neo_types

The `neo_types` module provides fundamental Neo blockchain types.

```rust,no_run
use neo3::prelude::*;

// Create a script hash from a string
let script_hash = "0xd2a4cff31913016155e38e474a2c06d08be276cf".parse::<ScriptHash>()?;

// Create an address from a string
let address = "NZNos2WqTbu5oCgyfss9kUJgBXJqhuYAaj".parse::<Address>()?;

// Create a transaction hash from a string
let tx_hash = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".parse::<TxHash>()?;
```

Key components:
- `Address`: Neo address
- `ScriptHash`: Contract script hash
- `TxHash`: Transaction hash
- `ContractParameter`: Parameter for contract invocation

### neo_crypto

The `neo_crypto` module provides cryptographic functionality.

```rust,no_run
use neo3::prelude::*;

// Generate a new key pair
let key_pair = KeyPair::new()?;

// Sign a message
let message = b"Hello, Neo!";
let signature = key_pair.sign_message(message)?;

// Verify a signature
let is_valid = key_pair.verify_signature(message, &signature)?;
```

Key components:
- `KeyPair`: Represents a public/private key pair
- `PublicKey`: Represents a public key
- `PrivateKey`: Represents a private key
- `Signature`: Represents a cryptographic signature

### neo_builder

The `neo_builder` module provides builders for creating transactions and scripts.

```rust,no_run
use neo3::prelude::*;

// Create a transaction
let transaction = TransactionBuilder::new()
    .version(0)
    .nonce(rand::random::<u32>())
    .valid_until_block(block_count + 100)
    .script(script)
    .sign(account)?
    .build();

// Create a script
let script = ScriptBuilder::new()
    .contract_call(
        script_hash,
        "transfer",
        &[
            ContractParameter::hash160(from_address.script_hash()),
            ContractParameter::hash160(to_address.script_hash()),
            ContractParameter::integer(amount),
            ContractParameter::any(None),
        ],
    )
    .to_array();
```

Key components:
- `TransactionBuilder`: Builder for creating transactions
- `ScriptBuilder`: Builder for creating VM scripts

### neo_contract

The `neo_contract` module provides interfaces for interacting with Neo smart contracts.

```rust,no_run
use neo3::prelude::*;

// Create a NEP-17 token instance
let token = Nep17Contract::new(token_hash, provider.clone());

// Get token information
let symbol = token.symbol().await?;
let decimals = token.decimals().await?;
let total_supply = token.total_supply().await?;

// Get token balance
let balance = token.balance_of(address).await?;
```

Key components:
- `Nep17Contract`: Interface for NEP-17 tokens
- `NeoToken`: Interface for the NEO token
- `GasToken`: Interface for the GAS token
- `NameService`: Interface for the Neo Name Service

### neo_x

The `neo_x` module provides support for Neo X, an EVM-compatible chain maintained by Neo.

```rust,no_run
use neo3::prelude::*;
use neo3::neo_x::evm::*;

// Create a Neo X provider
let provider = NeoXProvider::new_http("https://rpc.neoX.io");

// Create a transaction
let transaction = NeoXTransaction::new()
    .to("0x1234567890123456789012345678901234567890")
    .value(1_000_000_000_000_000_000u128) // 1 ETH in wei
    .gas_price(20_000_000_000u64) // 20 Gwei
    .gas_limit(21_000u64)
    .build();
```

Key components:
- `NeoXProvider`: Provider for interacting with Neo X nodes
- `NeoXTransaction`: Transaction for Neo X
- `NeoXBridgeContract`: Interface for the Neo X bridge

### neo_sgx

The `neo_sgx` module provides support for Intel SGX (Software Guard Extensions) for secure operations.

```rust,no_run
use neo3::prelude::*;

// Initialize the SGX enclave
let enclave_manager = SgxEnclaveManager::new("path/to/enclave.so")?;

// Create a wallet with a password
let wallet = enclave_manager.create_wallet("password")?;

// Sign a transaction securely within the enclave
let signed_tx = wallet.sign_transaction(&transaction)?;
```

Key components:
- `SgxEnclaveManager`: Manager for SGX enclaves
- `SgxWallet`: Secure wallet implementation
- `SgxRpcClient`: Secure RPC client

## Prelude

The `prelude` module re-exports commonly used types and functions for convenience:

```rust,no_run
use neo3::prelude::*;
```

This imports all the essential types and functions you need for most operations with the NeoRust SDK.

## Feature Flags

The NeoRust SDK supports various feature flags to enable specific functionality:

- `ledger`: Support for Ledger hardware wallets
- `aws`: AWS integration
- `sgx`: Intel SGX support

Enable these features in your Cargo.toml:

```toml
[dependencies]
neo3 = { git = "https://github.com/R3E-Network/NeoRust", features = ["ledger", "aws", "sgx"] }
```

## Error Handling

The NeoRust SDK uses Rust's `Result` type for error handling. Most functions return a `Result<T, Error>` where `Error` is a custom error type that can represent various error conditions.

```rust,no_run
use neo3::prelude::*;

fn example() -> Result<(), Box<dyn std::error::Error>> {
    // Create a provider
    let provider = Provider::new_http("https://testnet1.neo.coz.io:443");
    
    // Get the current block count
    match provider.get_block_count().await {
        Ok(block_count) => println!("Current block count: {}", block_count),
        Err(e) => println!("Error: {}", e),
    }
    
    Ok(())
}
```

For more detailed information on specific modules and types, see the corresponding reference pages.

<!-- toc -->
