# CLI Tools Overview

Welcome to **NeoRust CLI Tools** - your command-line interface for Neo N3 blockchain development and operations. Built with Rust for maximum performance and reliability.

## What is NeoRust CLI? ⌨️

NeoRust CLI is a comprehensive command-line toolkit that brings the full power of Neo N3 blockchain to your terminal. Whether you're developing smart contracts, managing wallets, or automating blockchain operations, our CLI tools provide everything you need for efficient Neo development.

### ✨ **Key Highlights**

- **🚀 Performance**: Built with Rust for lightning-fast operations
- **🔧 Comprehensive**: Complete toolkit for Neo N3 development
- **⚙️ Configurable**: Highly customizable to fit your workflow
- **🔒 Secure**: Enterprise-grade security features
- **📊 Detailed**: Rich output and comprehensive logging
- **🌐 Cross-platform**: Works on Windows, macOS, and Linux

## Quick Start 🚀

### Installation

```bash
# Install via Cargo (Rust package manager)
cargo install neo3-cli

# Verify installation
neorust --version
# Output: neorust 0.4.2

# Initialize configuration
neorust config init
```

### Your First Commands

```bash
# Check Neo N3 network status
neorust network status

# Get current block height
neorust blockchain height

# Create a new wallet
neorust wallet create --name my-wallet

# Check account balance
neorust account balance NXXXxxxXXX...

# Send tokens
neorust send --to NRecipientAddress --amount 10 --token GAS
```

## Core Features 🔧

### 🏦 **Wallet Management**
- **Create & Import**: Generate new wallets or import existing ones
- **Multi-format Support**: NEP-6, WIF, private key, and mnemonic
- **Hardware Wallet**: Ledger device integration
- **Security**: Encrypted storage with multiple authentication methods

```bash
# Create new wallet
neorust wallet create --name production-wallet

# Import from mnemonic
neorust wallet import --mnemonic "your twelve word mnemonic phrase here..."

# List all wallets
neorust wallet list
```

### 💰 **Account Operations**
- **Balance Checking**: View NEO, GAS, and custom token balances
- **Transaction History**: Complete transaction records with filtering
- **Multi-signature**: Support for multi-sig accounts
- **Watch-only**: Monitor addresses without private keys

```bash
# Check all balances
neorust account balance --all

# View transaction history
neorust account history --limit 10 --format table

# Create multi-signature account
neorust account multisig create --threshold 2 --keys key1,key2,key3
```

### 💸 **Token Operations**
- **Send Transactions**: Transfer NEO, GAS, and NEP-17 tokens
- **Batch Transfers**: Send to multiple recipients in one transaction
- **Token Information**: Query token metadata and supply
- **Custom Tokens**: Work with any NEP-17 compliant token

```bash
# Send GAS tokens
neorust send --to NRecipient --amount 100 --token GAS

# Batch transfer
neorust send batch --file recipients.csv --token NEO

# Get token information
neorust token info --contract 0x1234...
```

### 📋 **Smart Contract Interaction**
- **Deploy Contracts**: Deploy .nef and .nvm contract files
- **Invoke Methods**: Call contract methods with parameters
- **Contract Testing**: Test contracts on TestNet before MainNet
- **Event Monitoring**: Watch for contract events in real-time

```bash
# Deploy contract
neorust contract deploy --nef contract.nef --manifest contract.manifest.json

# Invoke contract method
neorust contract invoke 0x1234... methodName param1 param2

# Monitor contract events
neorust contract events 0x1234... --event Transfer
```

### 🌐 **Network Operations**
- **RPC Calls**: Direct JSON-RPC calls to Neo nodes
- **Network Stats**: Real-time blockchain statistics
- **Node Management**: Connect to different networks and nodes
- **Health Monitoring**: Check node status and connectivity

```bash
# Get network information
neorust network info

# Switch to TestNet
neorust config set network testnet

# Custom RPC call
neorust rpc call getblockcount

# Monitor network health
neorust network monitor --interval 30
```

### 🔐 **Security Features**
- **Encrypted Storage**: All private keys encrypted at rest
- **Hardware Wallet**: Ledger device support for signing
- **Transaction Preview**: Review transactions before signing
- **Secure Configuration**: Encrypted configuration files

## Command Categories 📚

### Core Commands
| Command | Description |
|---------|-------------|
| `neorust wallet` | Wallet management operations |
| `neorust account` | Account and balance operations |
| `neorust send` | Send tokens and transfers |
| `neorust contract` | Smart contract interactions |
| `neorust network` | Network and blockchain queries |

### Utility Commands
| Command | Description |
|---------|-------------|
| `neorust config` | Configuration management |
| `neorust keys` | Cryptographic key operations |
| `neorust convert` | Data format conversions |
| `neorust monitor` | Real-time monitoring |
| `neorust backup` | Backup and restore operations |

## Configuration 🔧

### Quick Configuration
```bash
# Set default network
neorust config set network mainnet

# Set default RPC endpoint
neorust config set rpc.mainnet "https://rpc10.n3.nspcc.ru:10331"

# Configure output format
neorust config set output.format table
```

### Advanced Configuration
Create `~/.config/neorust/config.toml`:

```toml
[general]
network = "mainnet"
output_format = "table"
colorize = true

[rpc]
mainnet = "https://rpc10.n3.nspcc.ru:10331"
testnet = "https://rpc.t5.n3.nspcc.ru:20331"
timeout = 30

[security]
confirm_threshold = 10.0
preview_transactions = true
hardware_wallet = false

[gas]
strategy = "auto"
default_limit = 20000000
```

Learn more in our [Configuration Guide](./configuration).

## Output Formats 📊

### Table Format (Default)
```
┌─────────────────────────┬────────────┬──────────────┐
│ Address                 │ Token      │ Balance      │
├─────────────────────────┼────────────┼──────────────┤
│ NXXXxxxXXX...          │ NEO        │ 100          │
│ NXXXxxxXXX...          │ GAS        │ 1,234.56789  │
└─────────────────────────┴────────────┴──────────────┘
```

### JSON Format
```json
{
  "address": "NXXXxxxXXX...",
  "balances": [
    {"token": "NEO", "balance": "100", "decimals": 0},
    {"token": "GAS", "balance": "1234.56789", "decimals": 8}
  ]
}
```

### Minimal Format
```
NEO: 100
GAS: 1,234.56789
```

## Integration Examples 🔗

### Bash Scripting
```bash
#!/bin/bash
# Automated balance checker

ADDRESSES=("NAddr1..." "NAddr2..." "NAddr3...")

for addr in "${ADDRESSES[@]}"; do
    echo "Checking balance for $addr"
    neorust account balance $addr --format json | jq '.balances[0].balance'
done
```

### CI/CD Pipeline
```yaml
# GitHub Actions example
- name: Deploy Contract
  run: |
    neorust config set network testnet
    neorust wallet import --wif ${{ secrets.DEPLOY_KEY }}
    neorust contract deploy --nef contract.nef --manifest contract.manifest.json
```

### Monitoring Script
```bash
#!/bin/bash
# Monitor contract events
neorust contract events 0x1234... --event Transfer --format json \
  | while read event; do
    echo "New transfer: $(echo $event | jq '.amount')"
    # Process event...
  done
```

## Advanced Features ⚡

### Plugin System
```bash
# List available plugins
neorust plugin list

# Install DeFi plugin
neorust plugin install neorust-defi

# Use plugin commands
neorust defi swap --from GAS --to fWBTC --amount 100
```

### Batch Operations
```bash
# Create batch transaction file
cat > batch_transfers.json << EOF
{
  "transfers": [
    {"to": "NAddr1...", "amount": "10", "token": "GAS"},
    {"to": "NAddr2...", "amount": "20", "token": "GAS"},
    {"to": "NAddr3...", "amount": "30", "token": "GAS"}
  ]
}
EOF

# Execute batch transfer
neorust send batch --file batch_transfers.json
```

### Real-time Monitoring
```bash
# Monitor blockchain in real-time
neorust monitor blockchain --interval 15

# Watch specific address
neorust monitor address NXXXxxxXXX... --notifications

# Monitor contract events
neorust monitor contract 0x1234... --event Transfer
```

## Best Practices 📋

### Security
- ✅ **Use hardware wallets** for MainNet operations
- ✅ **Enable transaction preview** before signing
- ✅ **Set confirmation thresholds** for large amounts
- ✅ **Keep configuration files secure** (600 permissions)
- ✅ **Use environment variables** for sensitive data

### Performance
- ✅ **Enable response caching** for repeated queries
- ✅ **Use batch operations** for multiple transactions
- ✅ **Configure connection pooling** for better throughput
- ✅ **Set appropriate timeouts** for network calls
- ✅ **Use local nodes** when possible for faster responses

### Automation
- ✅ **Use JSON output** for scripting
- ✅ **Set up proper error handling** in scripts
- ✅ **Log operations** for audit trails
- ✅ **Use configuration profiles** for different environments
- ✅ **Implement retry logic** for network operations

## Common Use Cases 💼

### Development Workflow
```bash
# 1. Set up development environment
neorust config set network testnet
neorust config set gas.strategy low

# 2. Create development wallet
neorust wallet create --name dev-wallet

# 3. Get testnet tokens (external faucet)
# 4. Deploy and test contract
neorust contract deploy --nef contract.nef --manifest contract.manifest.json

# 5. Test contract methods
neorust contract invoke $CONTRACT_HASH testMethod param1
```

### Production Deployment
```bash
# 1. Switch to MainNet
neorust config set network mainnet
neorust config set security.hardware_wallet true

# 2. Load production wallet
neorust wallet import --hardware ledger

# 3. Deploy with confirmation
neorust contract deploy --nef contract.nef --confirm

# 4. Verify deployment
neorust contract info $CONTRACT_HASH
```

### Portfolio Management
```bash
# Create portfolio monitoring script
neorust account balance --all --format json > portfolio.json

# Set up alerts for balance changes
neorust monitor address $MY_ADDRESS --threshold 1000 --notification email
```

## Troubleshooting 🔧

### Common Issues

#### Installation Problems
```bash
# Update Rust toolchain
rustup update

# Clear cargo cache
cargo cache --autoclean

# Reinstall CLI
cargo uninstall neo3-cli
cargo install neo3-cli --force
```

#### Network Connection Issues
```bash
# Test network connectivity
neorust network test

# Try different RPC endpoint
neorust config set rpc.mainnet "https://rpc1.neo.org:443"

# Check firewall settings
curl -X POST https://rpc10.n3.nspcc.ru:10331
```

#### Configuration Problems
```bash
# Reset configuration
neorust config reset

# Validate configuration
neorust config validate

# Show current configuration
neorust config show
```

## Getting Help 🆘

### Documentation
- **[Commands Reference](./commands)**: Complete command documentation
- **[Configuration Guide](./configuration)**: Detailed configuration options
- **[Examples](../examples)**: Real-world usage examples

### Community Support
- **GitHub Issues**: [Report bugs and request features](https://github.com/R3E-Network/NeoRust/issues)
- **Discord Chat**: [Join our community](https://discord.gg/neo-rust)
- **Forum**: [Ask questions and share knowledge](https://forum.neorust.org)

### Professional Support
- **Enterprise Support**: Available for commercial users
- **Custom Development**: Tailored solutions for specific needs
- **Training Services**: Team training and workshops

---

**Ready to master the command line?** Start with the [Commands Reference](./commands) and explore the full power of NeoRust CLI! ⚡🦀 