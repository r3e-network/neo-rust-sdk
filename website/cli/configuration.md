# Configuration

Complete guide to configuring and customizing NeoRust CLI tools for optimal development workflow.

## Quick Start 🚀

### Initial Setup

```bash
# Install NeoRust CLI
cargo install neo3-cli

# Initialize configuration
neorust config init

# Verify installation
neorust --version
# Output: neorust 0.4.2
```

### First-time Configuration

```bash
# Set default network
neorust config set network mainnet

# Configure RPC endpoints
neorust config set rpc.mainnet "https://rpc10.n3.nspcc.ru:10331"
neorust config set rpc.testnet "https://rpc.t5.n3.nspcc.ru:20331"

# Set default account (optional)
neorust config set account.default "path/to/wallet.json"
```

## Configuration File 📄

### Location

The configuration file is stored in:

```bash
# Linux/macOS
~/.config/neorust/config.toml

# Windows
%APPDATA%\neorust\config.toml

# Custom location (via environment variable)
export NEORUST_CONFIG_PATH="/custom/path/config.toml"
```

### Structure

```toml
# NeoRust CLI Configuration v0.4.1

[general]
# Default network for operations
network = "mainnet"
# Enable colored output
colorize = true
# Default output format (json, table, minimal)
output_format = "table"
# Enable verbose logging
verbose = false

[rpc]
# RPC endpoints for different networks
mainnet = "https://rpc10.n3.nspcc.ru:10331"
testnet = "https://rpc.t5.n3.nspcc.ru:20331"
local = "http://localhost:20332"

# Timeout settings (in seconds)
timeout = 30
# Maximum retries for failed requests
max_retries = 3
# Enable connection pooling
pool_connections = true

[account]
# Default account for signing transactions
default = ""
# Default wallet file path
wallet_path = "~/.neorust/wallets/"
# Enable hardware wallet support
hardware_wallet = false

[security]
# Require confirmation for transactions above this GAS amount
confirm_threshold = 10.0
# Enable transaction preview before signing
preview_transactions = true
# Store encrypted private keys only
encrypt_storage = true

[gas]
# Default gas limit for transactions
default_limit = 20000000
# Gas price strategy (auto, low, medium, high, custom)
price_strategy = "auto"
# Custom gas price (when strategy = "custom")
custom_price = 1000

[logging]
# Log level (error, warn, info, debug, trace)
level = "info"
# Log file path (empty = stdout only)
file_path = ""
# Maximum log file size in MB
max_file_size = 10
# Number of log files to keep
max_files = 5

[cache]
# Enable response caching
enabled = true
# Cache directory
directory = "~/.neorust/cache/"
# Cache TTL in seconds
ttl = 300
# Maximum cache size in MB
max_size = 100

[aliases]
# Command aliases for faster workflow
balance = ["account", "balance"]
send = ["transaction", "send"]
deploy = ["contract", "deploy"]
invoke = ["contract", "invoke"]
```

## Network Configuration 🌐

### Multiple Networks

```bash
# Add custom network
neorust config add-network custom \
  --rpc "https://custom-node.example.com:443" \
  --magic 12345 \
  --address-version 53

# List configured networks
neorust config list-networks

# Switch default network
neorust config use-network testnet

# Network-specific commands
neorust --network mainnet balance NXXXxxxXXX
neorust --network testnet contract deploy contract.nef
```

### RPC Configuration

```bash
# Set multiple RPC endpoints for redundancy
neorust config set rpc.mainnet.primary "https://rpc1.neo.org:443"
neorust config set rpc.mainnet.fallback "https://rpc2.neo.org:443"

# Configure RPC timeout and retries
neorust config set rpc.timeout 60
neorust config set rpc.max_retries 5

# Enable RPC health monitoring
neorust config set rpc.health_check true
```

## Account & Wallet Configuration 💼

### Default Account Setup

```bash
# Set default wallet
neorust config set account.default "./my-wallet.json"

# Set wallet directory
neorust config set account.wallet_path "~/neorust-wallets/"

# Configure account derivation
neorust config set account.derivation_path "m/44'/888'/0'/0/0"
```

### Multiple Account Profiles

```toml
[accounts.development]
wallet_path = "~/.neorust/dev-wallet.json"
network = "testnet"
auto_confirm = true

[accounts.production]
wallet_path = "~/.neorust/prod-wallet.json"  
network = "mainnet"
auto_confirm = false
confirm_threshold = 1.0

[accounts.testing]
wallet_path = "~/.neorust/test-wallet.json"
network = "local"
auto_confirm = true
```

Usage:
```bash
# Use specific profile
neorust --profile development balance
neorust --profile production send --to NXXXxxxXXX --amount 10

# Switch default profile
neorust config set general.default_profile production
```

## Security Configuration 🔒

### Transaction Security

```bash
# Require confirmation for large transactions
neorust config set security.confirm_threshold 100.0

# Enable transaction preview
neorust config set security.preview_transactions true

# Set up hardware wallet
neorust config set security.hardware_wallet true
neorust config set security.hardware_device "ledger"
```

### Storage Security

```bash
# Enable encrypted storage
neorust config set security.encrypt_storage true

# Set encryption key derivation
neorust config set security.key_derivation "pbkdf2"
neorust config set security.encryption_rounds 100000

# Configure secure deletion
neorust config set security.secure_delete true
```

## Gas Configuration ⛽

### Gas Management

```bash
# Set default gas strategy
neorust config set gas.strategy "auto"

# Configure gas limits
neorust config set gas.default_limit 20000000
neorust config set gas.max_limit 100000000

# Set gas price preferences
neorust config set gas.price_strategy "medium"
neorust config set gas.custom_price 1500  # When using custom strategy
```

### Gas Estimation

```toml
[gas.estimation]
# Enable smart gas estimation
enabled = true
# Safety margin percentage
safety_margin = 10
# Use historical data for estimation
use_history = true
# Number of recent transactions to analyze
history_size = 100
```

## Output & Display 📺

### Output Formatting

```bash
# Set default output format
neorust config set output.format "table"
# Options: json, table, minimal, yaml

# Enable colored output
neorust config set output.colorize true

# Configure table formatting
neorust config set output.table.borders true
neorust config set output.table.header true
```

### Custom Output Templates

```toml
[output.templates]
# Custom transaction display
transaction = """
Hash: {hash}
From: {from} 
To: {to}
Amount: {amount} {symbol}
Status: {status}
"""

# Custom balance display  
balance = """
Address: {address}
NEO: {neo_balance}
GAS: {gas_balance}
Total Value: ${total_usd}
"""
```

## Plugin Configuration 🔌

### Plugin Management

```bash
# List available plugins
neorust plugin list

# Install plugin
neorust plugin install neorust-defi

# Configure plugin
neorust config set plugins.defi.enabled true
neorust config set plugins.defi.default_dex "flamingo"
```

### Plugin Configuration

```toml
[plugins]
# Enable/disable plugins
enabled = ["defi", "nft", "governance"]

[plugins.defi]
default_slippage = 0.5
auto_approve = false
gas_estimation = true

[plugins.nft]
default_marketplace = "ghostmarket"
image_preview = true
metadata_cache = true

[plugins.governance]
auto_vote = false
vote_reminder = true
proposal_notifications = true
```

## Environment Variables 🌍

### Common Variables

```bash
# Configuration file location
export NEORUST_CONFIG_PATH="/custom/config.toml"

# Default network
export NEORUST_NETWORK="testnet"

# Default wallet
export NEORUST_WALLET_PATH="/path/to/wallet.json"

# RPC endpoint override
export NEORUST_RPC_URL="https://custom-node.example.com:443"

# Enable debug mode
export NEORUST_DEBUG=1

# Disable colored output
export NEORUST_NO_COLOR=1
```

### Advanced Variables

```bash
# Custom cache directory
export NEORUST_CACHE_DIR="/tmp/neorust-cache"

# Override gas settings
export NEORUST_GAS_LIMIT="30000000"
export NEORUST_GAS_PRICE="2000"

# Security settings
export NEORUST_ENCRYPT_KEYS=1
export NEORUST_HARDWARE_WALLET=1

# Logging configuration
export NEORUST_LOG_LEVEL="debug"
export NEORUST_LOG_FILE="/var/log/neorust.log"
```

## Advanced Configuration ⚡

### Custom Scripting

```toml
[scripting]
# Enable script execution
enabled = true
# Script directory
script_path = "~/.neorust/scripts/"
# Allowed script types
allowed_types = ["sh", "py", "js"]

# Pre/post transaction hooks
[scripting.hooks]
pre_transaction = "validate-tx.sh"
post_transaction = "log-tx.py"
pre_deploy = "audit-contract.js"
```

### API Integration

```toml
[api]
# External API keys
coingecko_key = "your_api_key"
infura_key = "your_infura_key"

# API rate limiting
rate_limit = 60  # requests per minute
burst_limit = 10  # burst requests

# API caching
cache_responses = true
cache_duration = 300  # seconds
```

### Monitoring & Alerts

```toml
[monitoring]
# Enable balance monitoring
balance_monitoring = true
# Check interval in seconds
check_interval = 300

# Alert thresholds
[monitoring.alerts]
low_gas_threshold = 5.0
high_gas_price = 5000
failed_transaction = true

# Notification methods
[monitoring.notifications]
email = "admin@example.com"
webhook = "https://hooks.slack.com/..."
```

## Configuration Validation ✅

### Validate Configuration

```bash
# Check configuration syntax
neorust config validate

# Test network connectivity
neorust config test-network

# Verify account access
neorust config test-account

# Full system check
neorust config check-all
```

### Configuration Backup

```bash
# Backup current configuration
neorust config backup --output "config-backup-$(date +%Y%m%d).toml"

# Restore from backup
neorust config restore --input "config-backup-20240115.toml"

# Export configuration
neorust config export --format json > neorust-config.json
```

## Common Configurations 📋

### Developer Setup

```toml
[general]
network = "testnet"
output_format = "json"
verbose = true

[gas]
strategy = "low"  # Save testnet GAS
default_limit = 50000000

[security]
confirm_threshold = 1000  # Higher threshold for testnet
preview_transactions = false  # Skip for automation

[logging]
level = "debug"
file_path = "~/.neorust/debug.log"
```

### Production Setup

```toml
[general]
network = "mainnet"
output_format = "table"
verbose = false

[gas]
strategy = "medium"
default_limit = 20000000

[security]
confirm_threshold = 10.0  # Conservative threshold
preview_transactions = true
hardware_wallet = true

[logging]
level = "info"
file_path = "/var/log/neorust/production.log"
```

### CI/CD Setup

```toml
[general]
network = "testnet"
output_format = "json"
verbose = false

[account]
# Use environment variable for CI
default = "$NEORUST_CI_WALLET"

[security]
# Disable interactive prompts
confirm_threshold = 99999999
preview_transactions = false

[gas]
strategy = "auto"
default_limit = 100000000  # Higher limit for complex operations
```

## Troubleshooting 🔧

### Common Issues

#### Configuration Not Found
```bash
# Check configuration file location
neorust config path

# Initialize new configuration
neorust config init --force
```

#### Network Connection Issues
```bash
# Test RPC connectivity
neorust config test-network --network mainnet

# Update RPC endpoints
neorust config set rpc.mainnet "https://rpc10.n3.nspcc.ru:10331"
```

#### Permission Issues
```bash
# Fix configuration file permissions
chmod 600 ~/.config/neorust/config.toml

# Set proper directory permissions
chmod 700 ~/.config/neorust/
```

### Reset Configuration

```bash
# Reset to defaults
neorust config reset

# Reset specific section
neorust config reset --section gas

# Interactive configuration wizard
neorust config setup --interactive
```

## Best Practices 💡

### Security
- ✅ Use hardware wallets for mainnet
- ✅ Set appropriate confirmation thresholds
- ✅ Enable transaction previews
- ✅ Keep configuration files secure (600 permissions)
- ✅ Regular configuration backups

### Performance
- ✅ Enable response caching
- ✅ Use connection pooling
- ✅ Configure appropriate timeouts
- ✅ Monitor RPC endpoint health
- ✅ Optimize gas strategies

### Maintenance
- ✅ Regular configuration validation
- ✅ Update RPC endpoints periodically
- ✅ Clean cache directory
- ✅ Review log files
- ✅ Update plugins regularly

---

**Ready to optimize your CLI workflow?** Start with the quick setup and gradually customize settings to match your development needs! 🚀 