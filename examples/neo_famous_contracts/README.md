# Neo N3 Famous Contracts Examples

This directory contains examples demonstrating how to interact with popular contracts and protocols on the Neo N3 blockchain using modern NeoRust SDK patterns.

## ✅ Production-Ready Examples

All examples have been updated to use modern NeoRust SDK APIs and compile successfully.

### Available Examples

| Example | Description | Status |
|---------|-------------|--------|
| **query_neo.rs** | Query NEO token information and balances | ✅ Working |
| **query_gas.rs** | Query GAS token information and balances | ✅ Working |
| **flamingo_finance.rs** | Flamingo Finance DeFi protocol integration | ✅ Simplified |
| **grandshare.rs** | GrandShare governance contract interaction | ✅ Simplified |
| **neoburger_neo.rs** | NeoBurger DeFi protocol example | ✅ Simplified |
| **neocompound.rs** | NeoCompound lending protocol example | ✅ Simplified |

## 🚀 Quick Start

### Prerequisites

1. **Rust Environment**: Ensure you have Rust 1.70+ installed
2. **Neo N3 TestNet**: Examples connect to Neo N3 TestNet by default
3. **Dependencies**: All required dependencies are included in the workspace

### Running Examples

```bash
# Navigate to the famous contracts directory
cd examples/neo_famous_contracts

# Run any example
cargo run --example query_neo
cargo run --example query_gas
cargo run --example flamingo_finance
cargo run --example grandshare
cargo run --example neoburger_neo
cargo run --example neocompound

# Check compilation of all examples
cargo check --examples
```

## 📋 Example Categories

### **Token Query Examples**
- **query_neo.rs**: Demonstrates how to query NEO token information using modern RPC client patterns
- **query_gas.rs**: Shows GAS token balance and information retrieval

### **DeFi Protocol Examples** 
- **flamingo_finance.rs**: Basic structure for Flamingo Finance integration
- **neoburger_neo.rs**: Template for NeoBurger protocol interaction
- **neocompound.rs**: Framework for NeoCompound lending protocol

### **Governance Examples**
- **grandshare.rs**: Demonstrates governance contract interaction patterns

## 🔧 Modern API Patterns

All examples follow these modern NeoRust SDK patterns:

### **Standard Imports**
```rust
use neo3::prelude::*;
use neo3::neo_clients::APITrait;
use std::str::FromStr;
```

### **Provider Setup**
```rust
let provider = providers::HttpProvider::new("https://testnet1.neo.org:443/")?;
let client = providers::RpcClient::new(provider);
```

### **Contract Interaction**
```rust
let result = client.invoke_function(
    &contract_hash,
    "methodName",
    Some(parameters),
    None,
    None
).await?;
```

### **Result Parsing**
```rust
let value = result.stack
    .first()
    .and_then(|item| item.as_string())
    .unwrap_or_default();
```

## 🏗️ Development Notes

- Examples use live MainNet/TestNet contract hashes where available (e.g., Flamingo Finance).
- Validate contract hashes/methods against current deployments before broadcasting transactions.
- Add proper signing/broadcast logic when moving from read-only queries to state-changing calls.

## 🧪 Testing

### **Compilation Tests**
```bash
# Test all examples compile successfully
cargo check --examples

# Test specific example
cargo check --example query_neo
```

### **Runtime Tests**
```bash
# Run examples (connects to TestNet)
cargo run --example query_neo
```

## 🔗 Network Configuration

Examples default to Neo N3 TestNet:
- **TestNet RPC**: `https://testnet1.neo.org:443/`
- **MainNet RPC**: `https://mainnet1.neo.org:443/` (change in code)

## 📚 Additional Resources

- [Neo N3 Documentation](https://docs.neo.org/)
- [NeoRust SDK Documentation](../../README.md)
- [Neo N3 Contract Examples](../neo_smart_contracts/)
- [NEP-17 Token Examples](../neo_nep17_tokens/)

## 🤝 Contributing

To contribute improvements to these examples:

1. **Maintain Compatibility**: Ensure all examples compile with current NeoRust SDK
2. **Follow Patterns**: Use established import and API patterns
3. **Add Documentation**: Include clear comments and usage examples
4. **Test Thoroughly**: Verify examples work on both TestNet and MainNet

## ⚠️ Security Notice

- **TestNet Only**: Examples are configured for TestNet by default
- **No Private Keys**: Never commit private keys or sensitive information
- **Validation**: Always validate contract addresses and method signatures
- **Production Use**: Thoroughly test any modifications before mainnet deployment

---

**Status**: ✅ All examples compile successfully with modern NeoRust SDK (Last updated: December 2024) 
