# NeoRust Examples

This directory contains comprehensive, production-ready examples demonstrating how to use the NeoRust SDK for various Neo N3 blockchain operations.

## 🎯 **Quick Start**

Choose an example category based on what you want to accomplish:

- **New to Neo N3?** → Start with `neo_nodes` for basic connectivity
- **Building a wallet?** → Check `neo_wallets` for account management  
- **Working with tokens?** → Explore `neo_nep17_tokens` for token operations
- **Smart contracts?** → See `neo_smart_contracts` for contract interaction
- **Deploying contracts?** → Look at `neo_contracts` for deployment guide

## 📚 **Example Categories**

### **Core Neo N3 Examples** (✅ Production Ready)

| Category | Description | Key Features |
|----------|-------------|--------------|
| **[neo_nodes](neo_nodes/)** | Node connectivity and blockchain queries | Network health, block exploration, multi-endpoint testing |
| **[neo_wallets](neo_wallets/)** | Wallet and account management | Account creation, encryption, backup/recovery, multi-sig |
| **[neo_transactions](neo_transactions/)** | Transaction creation and broadcasting | GAS/NEO transfers, multi-call transactions, fee estimation |
| **[neo_smart_contracts](neo_smart_contracts/)** | Smart contract interaction | Read-only calls, state changes, best practices |
| **[neo_contracts](neo_contracts/)** | Contract deployment guide | NEF files, manifests, deployment workflow |
| **[neo_nep17_tokens](neo_nep17_tokens/)** | NEP-17 token operations | Token info, balance queries, transfer scripts |

### **Advanced Examples** 

| Category | Description | Status |
|----------|-------------|---------|
| **[neo_famous_contracts](neo_famous_contracts/)** | Well-known Neo contracts | ✅ Production Ready |
| **[neo_nns](neo_nns/)** | Neo Name Service | 🔄 Needs updating |
| **[neo_x](neo_x/)** | EVM compatibility layer | 🔄 Needs updating |

### **General Examples** (📝 Legacy)

| Category | Description | Status |
|----------|-------------|---------|
| **[wallets](wallets/)** | General wallet examples | 📝 Legacy |
| **[transactions](transactions/)** | General transaction examples | 📝 Legacy |
| **[providers](providers/)** | Provider examples | 📝 Legacy |
| **[contracts](contracts/)** | Contract examples | 📝 Legacy |

## 🚀 **Running Examples**

### **Method 1: Run Specific Example**
```bash
# Navigate to example category
cd examples/neo_nodes

# Run specific example
cargo run --example connect_to_node
```

### **Method 2: Run from Workspace Root**
```bash
# From project root
cargo run --example connect_to_node -p neo_nodes_examples
```

### **Method 3: Test Compilation**
```bash
# Test all examples compile
cd examples/neo_nodes && cargo check
```

## 🛠 **Example Features**

### **Production-Ready Code**
- ✅ Proper error handling and edge cases
- ✅ Comprehensive documentation and comments
- ✅ Real-world usage patterns
- ✅ Security best practices
- ✅ Network connectivity resilience

### **Educational Content**
- 📚 Step-by-step explanations
- 💡 Best practices and security tips
- 🔧 Common pitfalls and solutions
- 📊 Performance considerations
- 🎯 Real-world use cases

### **API Compatibility**
- ✅ Uses latest NeoRust production APIs
- ✅ Follows current Neo N3 protocols
- ✅ Compatible with TestNet and MainNet
- ✅ Proper type safety and validation

## 📋 **Prerequisites**

### **Development Environment**
- **Rust**: 1.70+ (2021 edition)
- **Platform**: macOS, Linux, Windows
- **Network**: Internet connection for blockchain interaction

### **Neo N3 Knowledge**
- Basic understanding of blockchain concepts
- Familiarity with Neo N3 architecture
- Knowledge of smart contract principles (for contract examples)

### **Optional Tools**
- **Neo CLI**: For advanced blockchain operations
- **Neo-Express**: For local development blockchain
- **TestNet Wallet**: For testing with real network

## 🌐 **Network Configuration**

Examples are configured for **Neo N3 TestNet** by default:
- **TestNet RPC**: `https://testnet1.neo.org:443/`
- **Explorer**: [TestNet Neotube](https://testnet.neotube.io/)
- **Faucet**: Get test GAS/NEO from community faucets

To use **MainNet**, update RPC endpoints in examples:
- **MainNet RPC**: `https://mainnet1.neo.org:443/`
- **Explorer**: [MainNet Neotube](https://neotube.io/)

Environment toggles:
- Set `NEO_RPC_URL` to point examples at a specific node (otherwise they use TestNet defaults or skip live calls when possible).
- Some examples include mock/offline paths; they will print a hint when a live RPC URL is required.
- Feature flags: enable `ws` for WebSocket transport and `ipc` for IPC transport when running examples that use those clients.

## 🔧 **Troubleshooting**

### **Common Issues**

| Issue | Solution |
|-------|----------|
| **Compilation errors** | Run `cargo update` and ensure Rust 1.70+ |
| **Network timeouts** | Check internet connection and try different RPC endpoint |
| **Missing dependencies** | Run `cargo clean && cargo build` |
| **Type errors** | Ensure using latest NeoRust version |

### **Debug Mode**
Enable detailed logging for troubleshooting:
```bash
RUST_LOG=debug cargo run --example connect_to_node
```

### **Example-Specific Issues**

| Example | Common Issues | Solutions |
|---------|---------------|-----------|
| **neo_nodes** | Network connectivity | Try multiple endpoints |
| **neo_wallets** | Key management | Use proper secure storage |
| **neo_contracts** | Deployment costs | Ensure sufficient GAS balance |
| **neo_nep17_tokens** | Token queries | Verify contract addresses |

## 📖 **Learning Path**

### **Beginner Path**
1. **[connect_to_node](neo_nodes/examples/connect_to_node.rs)** - Basic connectivity
2. **[wallet_management](neo_wallets/examples/wallet_management.rs)** - Account creation
3. **[nep17_token_operations](neo_nep17_tokens/examples/nep17_token_operations.rs)** - Token basics

### **Intermediate Path**  
1. **[create_and_send_transaction](neo_transactions/examples/create_and_send_transaction.rs)** - Transaction creation
2. **[interact_with_contract](neo_smart_contracts/examples/interact_with_contract.rs)** - Contract interaction
3. **[deploy_neo_contract](neo_contracts/examples/deploy_neo_contract.rs)** - Contract deployment

### **Advanced Path**
1. **[famous_contracts](neo_famous_contracts/)** - Production contract interaction
2. **[neo_nns](neo_nns/)** - Name service integration
3. **[neo_x](neo_x/)** - EVM compatibility

## 💡 **Best Practices**

### **Development Workflow**
1. **Start with TestNet** - Always test on TestNet first
2. **Use proper error handling** - Implement comprehensive error management
3. **Validate inputs** - Check all user inputs and contract parameters
4. **Monitor transactions** - Track transaction status and confirmations
5. **Security first** - Never log private keys or sensitive data

### **Production Deployment**
1. **Audit thoroughly** - Review all code before MainNet deployment
2. **Use hardware wallets** - For signing valuable transactions
3. **Implement monitoring** - Track contract health and performance
4. **Plan for upgrades** - Design contracts with upgrade mechanisms
5. **Have recovery plans** - Prepare for emergency scenarios

## 🤝 **Contributing**

### **Adding New Examples**
1. Create example in appropriate category directory
2. Follow the established code structure and documentation style
3. Include comprehensive comments and error handling
4. Add example to category's `Cargo.toml`
5. Update this README with new example information
6. Test thoroughly on TestNet

### **Improving Existing Examples**
1. Ensure compatibility with latest NeoRust APIs
2. Add more comprehensive error handling
3. Improve documentation and comments
4. Add performance optimizations
5. Include additional security considerations

### **Documentation Standards**
- Start with clear overview and purpose
- Include step-by-step explanations
- Provide best practices and security tips
- Add troubleshooting information
- Include links to relevant resources

## 🔗 **Additional Resources**

### **Neo N3 Documentation**
- [Neo Developer Guide](https://docs.neo.org/)
- [Neo N3 RPC API](https://docs.neo.org/docs/en-us/reference/rpc/latest-version/api.html)
- [Smart Contract Development](https://docs.neo.org/docs/en-us/develop/write/basics.html)

### **NeoRust SDK**
- [SDK Documentation](https://docs.rs/neo3)
- [GitHub Repository](https://github.com/R3E-Network/NeoRust)
- [Release Notes](https://github.com/R3E-Network/NeoRust/releases)

### **Community Resources**  
- [Neo Discord](https://discord.gg/neo)
- [Neo Reddit](https://reddit.com/r/NEO)
- [Neo Developer Community](https://neo.org/dev)

---

**Happy coding with NeoRust! 🦀⚡**

> 💡 **Pro Tip**: Start with the `neo_nodes` examples to understand basic connectivity, then progress through the categories based on your specific use case. 
