# Getting Started with NeoRust SDK

This guide will help you get started with the NeoRust SDK for interacting with the Neo N3 blockchain.

## Prerequisites

- Rust 1.70.0 or later
- Cargo package manager
- Basic knowledge of the Neo blockchain

## Installation

Add the NeoRust SDK to your Cargo.toml:

```toml
[dependencies]
neo3 = { git = "https://github.com/R3E-Network/NeoRust" }
```

Or if you prefer to use a specific version:

```toml
[dependencies]
neo3 = "0.1.0"
```

## Basic Usage

Here's a simple example of connecting to a Neo N3 node and getting the current block height:

```rust,edition2021
use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo N3 TestNet node
    let provider = Provider::new_http("https://testnet1.neo.coz.io:443");
    
    // Get the current block height
    let block_count = provider.get_block_count().await?;
    println!("Current block height: {}", block_count);
    
    Ok(())
}
```

## Next Steps

- Learn about [Wallet Management](../tutorials/wallet-management.md)
- Explore [Smart Contract Interaction](../tutorials/smart-contracts.md)
- See [Examples](../examples/README.md) for more code samples

<!-- toc -->
