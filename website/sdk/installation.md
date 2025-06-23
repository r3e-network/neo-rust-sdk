# Installation

Get started with NeoRust SDK v0.4.2 by installing it in your Rust project.

## Prerequisites

- Rust 1.70 or later
- Cargo package manager

## Adding to Your Project

Add NeoRust to your `Cargo.toml`:

```toml
[dependencies]
neo3 = "0.4.2"
```

For specific features, use:

```toml
[dependencies]
neo3 = { version = "0.4.2", features = ["futures", "ledger"] }
```

## Available Features

- `futures` - Async/await support (recommended)
- `ledger` - Hardware wallet support
- `websocket` - WebSocket client support

## Verification

Verify your installation:

```rust
use neo3::prelude::*;

fn main() {
    println!("NeoRust SDK v0.4.2 is ready!");
}
```

## Next Steps

- [Quick Start Guide](./quick-start.md)
- [Examples](./examples.md) 