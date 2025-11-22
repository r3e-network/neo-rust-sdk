# Neo X EVM Compatibility Examples

This directory contains examples demonstrating Neo X, Neo's EVM-compatible sidechain that enables seamless integration between Neo N3 and Ethereum ecosystems.

## ✅ Production-Ready Examples

All examples have been updated to use modern NeoRust SDK APIs and compile successfully.

### Available Examples

| Example | Description | Status |
|---------|-------------|--------|
| **neo_x_bridge.rs** | Cross-chain bridge operations between Neo N3 and Neo X | ✅ Working |
| **neo_x_evm.rs** | EVM compatibility layer concepts and development tools | ✅ Working |

## 🚀 Quick Start

### Prerequisites

1. **Rust Environment**: Ensure you have Rust 1.70+ installed
2. **Neo N3 Access**: Examples connect to Neo N3 for context
3. **Dependencies**: All required dependencies are included in the workspace

### Running Examples

```bash
# Navigate to the Neo X directory
cd examples/neo_x

# Run Neo X Bridge example
cargo run --example neo_x_bridge

# Run Neo X EVM example
cargo run --example neo_x_evm

# Check compilation of all examples
cargo check --examples
```

## 🌉 Neo X Overview

### **What is Neo X?**

Neo X is Neo's EVM-compatible sidechain that brings the best of both worlds:

- **Full EVM Compatibility**: Run Ethereum smart contracts natively
- **Neo N3 Integration**: Seamless asset bridging and cross-chain operations
- **High Performance**: Fast transaction processing with low fees
- **Developer Friendly**: Use familiar Ethereum tools and libraries

### **Key Features**

- ⚡ **EVM Compatibility**: Full Ethereum Virtual Machine support
- 🌉 **Cross-Chain Bridge**: Seamless asset transfers between Neo N3 and Neo X
- 🔧 **Development Tools**: Hardhat, Truffle, Remix, MetaMask support
- 💰 **Low Fees**: Cost-effective transactions using GAS token
- 🔐 **Security**: Inherits security from both Neo N3 and Ethereum standards

## 🔧 Modern API Patterns

The Neo X examples follow these modern NeoRust SDK patterns:

### **Standard Imports**
```rust
use neo3::prelude::*; // H160/H256/U256 are re-exported here
use neo3::neo_clients::APITrait;
use std::str::FromStr;
```

### **Provider Setup**
```rust
let provider = providers::HttpProvider::new("https://testnet1.neo.org:443/")?;
let client = providers::RpcClient::new(provider);
```

### **Network Information**
- **Neo X MainNet RPC**: `https://rpc.neo-x.org`
- **Neo X TestNet RPC**: `https://rpc.x.testnet.neo.org`
- **Chain ID (MainNet)**: `47763`
- **Chain ID (TestNet)**: `12227332`

## 🌉 Bridge Operations

### **Neo N3 → Neo X (Deposit)**

1. **Connect to Neo N3**: Use Neo N3 wallet (NeoLine, O3, etc.)
2. **Check Bridge Fees**: Verify current bridging costs
3. **Create Deposit**: Send assets to bridge contract on Neo N3
4. **Wait for Confirmation**: Assets appear on Neo X after confirmation
5. **Use on Neo X**: Interact with EVM dApps using bridged assets

### **Neo X → Neo N3 (Withdraw)**

1. **Connect to Neo X**: Use MetaMask or EVM-compatible wallet
2. **Initiate Withdrawal**: Send transaction on Neo X
3. **Wait for Processing**: Cross-chain validation occurs
4. **Receive on Neo N3**: Assets released on Neo N3 network

### **Supported Assets**

| Asset | Neo N3 | Neo X | Description |
|-------|--------|-------|-------------|
| **GAS** | Native | Bridged | Transaction fees on both chains |
| **NEO** | Native | bNEO | Governance token, wrapped on Neo X |
| **NEP-17** | Native | Bridged | Selected tokens bridge automatically |
| **NFTs** | NEP-11 | ERC-721 | Cross-chain NFT support |

## ⚡ EVM Development

### **Smart Contract Development**

```solidity
// Standard Ethereum contracts work on Neo X
pragma solidity ^0.8.19;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract MyToken is ERC20 {
    constructor() ERC20("MyToken", "MTK") {
        _mint(msg.sender, 1000000 * 10**decimals());
    }
}
```

### **Development Workflow**

1. **Write Contracts**: Use Solidity, Vyper, or other EVM languages
2. **Test Locally**: Use Hardhat/Truffle for local development
3. **Deploy to TestNet**: Test on Neo X TestNet first
4. **Audit & Review**: Ensure security best practices
5. **Deploy to MainNet**: Launch on Neo X MainNet

### **Compatible Tools**

| Category | Tools | Description |
|----------|-------|-------------|
| **Frameworks** | Hardhat, Truffle, Foundry | Smart contract development |
| **IDEs** | Remix, VS Code, IntelliJ | Code editing and debugging |
| **Wallets** | MetaMask, WalletConnect | Transaction signing |
| **Libraries** | Web3.js, Ethers.js, Viem | Frontend integration |

## 🧪 Testing

### **Compilation Tests**
```bash
# Test all examples compile successfully
cargo check --examples

# Test specific example
cargo check --example neo_x_bridge
```

### **Runtime Tests**
```bash
# Run bridge concepts demonstration
cargo run --example neo_x_bridge

# Run EVM compatibility demonstration  
cargo run --example neo_x_evm
```

## 🔗 Network Configuration

### **Neo X Networks**

| Network | RPC Endpoint | Chain ID | Explorer |
|---------|--------------|----------|----------|
| **MainNet** | https://rpc.neo-x.org | 47763 | https://explorer.neo-x.org |
| **TestNet** | https://rpc.x.testnet.neo.org | 12227332 | https://testnet.explorer.neo-x.org |

### **Adding to MetaMask**

```javascript
// Neo X MainNet
{
  chainId: '0xBA93',
  chainName: 'Neo X',
  rpcUrls: ['https://rpc.neo-x.org'],
  nativeCurrency: {
    name: 'GAS',
    symbol: 'GAS',
    decimals: 18
  },
  blockExplorerUrls: ['https://explorer.neo-x.org']
}
```

## 💡 Best Practices

### **Development Guidelines**
- 🧪 **Test First**: Always test on TestNet before MainNet
- 🔐 **Security**: Follow Ethereum security best practices
- 💰 **Gas Optimization**: Optimize contracts for lower gas costs
- 🌉 **Bridge Awareness**: Understand cross-chain asset behavior
- 📊 **Monitoring**: Set up proper monitoring and alerts

### **Cross-Chain Considerations**
- **Finality**: Wait for sufficient confirmations
- **Fees**: Factor in bridge fees for user experience
- **Liquidity**: Ensure adequate bridge liquidity
- **Timing**: Bridge operations may take several minutes
- **Failure Handling**: Implement proper error recovery

## 🛡️ Security Considerations

### **Bridge Security**
- **Contract Verification**: Always verify bridge contract addresses
- **Amount Limits**: Be aware of bridge caps and limits
- **Validator Sets**: Understand bridge validator mechanisms
- **Emergency Procedures**: Know emergency pause mechanisms

### **EVM Security**
- **Smart Contract Audits**: Audit contracts before deployment
- **Access Controls**: Implement proper permission systems
- **Upgrade Patterns**: Design secure upgrade mechanisms
- **Oracle Security**: Use trusted oracle networks

## 📚 Additional Resources

### **Neo X Documentation**
- [Neo X Official Website](https://neo-x.org/)
- [Neo X Developer Docs](https://docs.neo-x.org/)
- [Bridge User Guide](https://bridge.neo-x.org/)

### **Development Resources**
- [Hardhat Documentation](https://hardhat.org/docs)
- [OpenZeppelin Contracts](https://docs.openzeppelin.com/contracts)
- [Solidity Documentation](https://docs.soliditylang.org/)

### **Community & Support**
- [Neo Developer Discord](https://discord.gg/neo)
- [Neo X GitHub](https://github.com/neo-project/neo-x)
- [Neo Forum](https://forum.neo.org/)

## 🤝 Contributing

To improve Neo X examples:

1. **Modern Patterns**: Use current NeoRust SDK APIs
2. **EVM Standards**: Follow Ethereum development standards
3. **Cross-Chain Focus**: Emphasize bridge and interoperability features
4. **Security First**: Include security considerations and best practices
5. **Documentation**: Provide clear examples and explanations

## ⚠️ Important Notes

- **Network Effects**: Neo X benefits from both Neo and Ethereum ecosystems
- **Bridge Dependencies**: Cross-chain operations depend on bridge availability
- **Gas Tokens**: GAS is used for transaction fees on Neo X
- **EVM Compatibility**: Full compatibility with Ethereum tools and contracts
- **Active Development**: Neo X is actively developed with regular updates

---

**Status**: ✅ All examples compile successfully with modern NeoRust SDK (Last updated: December 2024) 
