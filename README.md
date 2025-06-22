# NeoRust

[![Rust CI](https://github.com/r3e-network/NeoRust/actions/workflows/rust.yml/badge.svg)](https://github.com/r3e-network/NeoRust/actions/workflows/rust.yml)
[![Build & Test](https://github.com/r3e-network/NeoRust/actions/workflows/neorust-build-test.yml/badge.svg)](https://github.com/r3e-network/NeoRust/actions/workflows/neorust-build-test.yml)
[![Security](https://github.com/r3e-network/NeoRust/actions/workflows/security.yml/badge.svg)](https://github.com/r3e-network/NeoRust/actions/workflows/security.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Crates.io](https://img.shields.io/crates/v/neo3.svg)](https://crates.io/crates/neo3)
[![Documentation](https://docs.rs/neo3/badge.svg)](https://docs.rs/neo3)
[![MSRV](https://img.shields.io/badge/MSRV-1.70.0-blue)](https://blog.rust-lang.org/2023/06/01/Rust-1.70.0.html)

A comprehensive Rust SDK for the Neo N3 blockchain platform, providing a complete toolkit for interacting with Neo N3 networks.

## 📊 Project Status

- **Version**: 0.4.1 (Production Ready)
- **Rust Version**: 1.70.0+
- **Platform Support**: Windows, macOS, Linux
- **Security**: All dependencies audited, 0 known vulnerabilities
- **Coverage**: Core functionality tested with integration tests

## Features

- 🔐 **Cryptography** - Complete cryptographic functions including key generation, signing, and verification
- 💼 **Wallet Management** - Create, import, and manage Neo wallets with hardware wallet support
- 🔗 **RPC Client** - Full-featured RPC client for Neo N3 node interaction
- 📦 **Smart Contracts** - Deploy, invoke, and interact with Neo N3 smart contracts
- 🪙 **Token Support** - Native NEP-17 token operations and custom token support
- 🌐 **Network Support** - Mainnet, Testnet, and custom network configurations
- 🖥️ **CLI Tools** - Command-line interface for common blockchain operations
- 🖼️ **GUI Application** - Desktop GUI application built with Tauri and React

## Quick Start

Add NeoRust to your `Cargo.toml`:

```toml
[dependencies]
neo3 = "0.4.1"
```

## Basic Usage

```rust
use neo3::prelude::*;

// Create a new wallet
let wallet = Wallet::new().unwrap();

// Connect to Neo testnet
let client = RpcClient::new("https://testnet1.neo.coz.io:443").unwrap();

// Get account balance
let balance = client.get_balance(&wallet.address()).await?;
println!("Balance: {} NEO", balance.neo);
```

## Components

### Core SDK (`neo3`)
The main Rust SDK providing all blockchain functionality.

### CLI Tool (`neo-cli`)
Command-line interface for blockchain operations:
```bash
cargo run --bin neo-cli -- wallet create
```

### GUI Application (`neo-gui`)
Desktop application with modern React UI. **Note:** Requires GTK libraries on Linux.

## Building

### Core SDK and CLI
```bash
cargo build --workspace --exclude neo-gui
```

### GUI Application (requires additional dependencies)

**Linux (Ubuntu/Debian):**
```bash
sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.0-dev libayatana-appindicator3-dev librsvg2-dev
cd neo-gui && npm install && cargo build
```

**macOS and Windows:**
```bash
cd neo-gui && npm install && cargo build
```

## Documentation

- [Getting Started Guide](docs/guides/getting-started.md)
- [API Documentation](https://docs.rs/neo3)
- [Examples](examples/)
- [CLI Documentation](neo-cli/README.md)
- [GUI Documentation](neo-gui/README.md)

## Examples

Explore our comprehensive examples:

- **Basic Operations**: Wallet creation, token transfers, balance queries
- **Smart Contracts**: Deploy and interact with Neo N3 contracts
- **Advanced Features**: Multi-sig wallets, hardware wallet integration
- **DeFi Integration**: Interact with popular Neo DeFi protocols
- **Neo X**: Cross-chain bridge operations

See the [examples directory](examples/) for full code samples.

## License

Licensed under MIT license ([LICENSE](LICENSE) or http://opensource.org/licenses/MIT)

## Testing

```bash
# Run all tests
cargo test --workspace

# Run specific component tests
cargo test -p neo3
cargo test -p neo-cli

# Run integration tests
cargo test --test integration_tests
```

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

Please ensure:
- All tests pass (`cargo test --workspace`)
- Code is formatted (`cargo fmt`)
- No clippy warnings (`cargo clippy -- -D warnings`)
- Documentation is updated

## Security

For security issues, please email security@r3e.network instead of using the issue tracker.

## Acknowledgments

- Neo Foundation for the Neo N3 blockchain
- Rust community for excellent tooling
- All contributors who have helped shape this project