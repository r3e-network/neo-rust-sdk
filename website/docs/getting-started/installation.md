# Installation

Get started with NeoRust SDK by installing it in your development environment.

## System Requirements

- **Rust**: Version 1.70 or later
- **Cargo**: Rust's package manager (included with Rust)
- **Operating System**: Windows, macOS, or Linux

## Install Rust

If you don't have Rust installed, get it from [rustup.rs](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Create a New Project

```bash
cargo new my-neo-app
cd my-neo-app
```

## Add NeoRust Dependency

Add NeoRust to your `Cargo.toml`:

```toml
[dependencies]
neo3 = "0.4.2"
tokio = { version = "1.0", features = ["full"] }
```

## Feature Flags

NeoRust provides several optional features:

```toml
[dependencies]
neo3 = { version = "0.4.2", features = ["futures", "ledger", "websocket"] }
```

### Available Features

- **`futures`** - Async/await support (recommended)
- **`ledger`** - Hardware wallet support
- **`websocket`** - WebSocket client support
- **`serde`** - Serialization support

## Verify Installation

Create a simple test to verify everything works:

```rust
// src/main.rs
use neo3::prelude::*;

fn main() {
    println!("NeoRust SDK v0.4.1 is ready!");
    
    // Create a simple account
    let account = Account::create().expect("Failed to create account");
    println!("Generated address: {}", account.get_address());
}
```

Run it:

```bash
cargo run
```

You should see output like:
```
NeoRust SDK v0.4.1 is ready!
Generated address: NXXXXxxxXXXxxxXXXxxxXXXxxxXXXxxx
```

## Troubleshooting

### Build Errors

If you encounter build errors, make sure you have the latest stable Rust:

```bash
rustup update stable
```

### Platform-Specific Issues

#### macOS
You may need to install additional tools:
```bash
xcode-select --install
```

#### Windows
Ensure you have the Microsoft C++ Build Tools installed.

#### Linux
Install build essentials:
```bash
# Ubuntu/Debian
sudo apt update && sudo apt install build-essential

# CentOS/RHEL
sudo yum groupinstall "Development Tools"
```

## Next Steps

- [Quick Start Guide](./quick-start.md) - Your first Neo application
- [Examples](/examples) - Practical code examples 