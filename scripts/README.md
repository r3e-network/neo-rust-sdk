# NeoRust Build Scripts v0.4.2

This directory contains build and test scripts for the NeoRust SDK.

## Available Scripts

### Build Scripts

#### Unix/Linux/macOS: `build.sh`
```bash
# Build with default features (futures, ledger)
./scripts/build.sh

# Build with specific features
./scripts/build.sh --features futures,ledger

# Build with minimal features
./scripts/build.sh --features futures

# Show help
./scripts/build.sh --help
```

#### Windows: `build.bat`
```cmd
REM Build with default features (futures, ledger)
.\scripts\build.bat

REM Build with specific features
.\scripts\build.bat --features futures,ledger

REM Build with minimal features
.\scripts\build.bat --features futures

REM Show help
.\scripts\build.bat --help
```

### Test Scripts

#### Unix/Linux/macOS: `test.sh`
```bash
# Run tests with default features (futures, ledger)
./scripts/test.sh

# Run tests with specific features
./scripts/test.sh --features futures,ledger

# Run tests with minimal features
./scripts/test.sh --features futures

# Show help
./scripts/test.sh --help
```

#### Windows: `test.bat`
```cmd
REM Run tests with default features (futures, ledger)
.\scripts\test.bat

REM Run tests with specific features
.\scripts\test.bat --features futures,ledger

REM Run tests with minimal features
.\scripts\test.bat --features futures

REM Show help
.\scripts\test.bat --help
```

## Available Features

### Core Features
- **futures**: Enables async/futures support for asynchronous blockchain operations
- **ledger**: Enables Ledger hardware wallet support for enhanced security

### Removed Features (v0.4.2)
- **aws**: ⚠️ **DISABLED** in v0.4.2 due to security vulnerabilities in rusoto dependencies
  - Will be re-enabled in v0.5.0 with modern AWS SDK
  - Use v0.3.0 if AWS KMS integration is required

## Security Notes

### v0.4.2 Security Improvements
- **Zero vulnerabilities**: All security issues have been resolved
- **AWS feature disabled**: Temporarily removed to eliminate vulnerable dependencies
- **Updated dependencies**: All dependencies updated to secure versions

### Verification Commands
```bash
# Check for security vulnerabilities
cargo audit

# Verify dependency tree
cargo tree | grep -E "(rusoto|ring|rustls)"

# Run full test suite
./scripts/test.sh

# Build release version
./scripts/build.sh
```

## Examples

### Development Workflow
```bash
# 1. Clean build
cargo clean

# 2. Run tests
./scripts/test.sh --features futures,ledger

# 3. Build release
./scripts/build.sh --features futures,ledger

# 4. Security audit
cargo audit
```

### CI/CD Integration
```yaml
# GitHub Actions example
- name: Test NeoRust
  run: ./scripts/test.sh --features futures,ledger

- name: Build NeoRust
  run: ./scripts/build.sh --features futures,ledger

- name: Security Audit
  run: cargo audit
```

## Troubleshooting

### Common Issues

#### Missing Features Error
```
error: Package `neo3` does not have feature `aws`
```
**Solution**: Remove `aws` from your feature list in v0.4.2

#### Build Failures
```bash
# Clean and rebuild
cargo clean
./scripts/build.sh --features futures,ledger
```

#### Test Failures
```bash
# Run tests with verbose output
cargo test --lib --features futures,ledger
```

### Getting Help
- **Documentation**: https://docs.rs/neo3
- **Issues**: https://github.com/R3E-Network/NeoRust/issues
- **Security**: security@r3e.network

## Migration from v0.3.0

### Update Feature Lists
```toml
# Before (v0.3.0)
neo3 = { version = "0.3.0", features = ["futures", "ledger", "aws"] }

# After (v0.4.2)
neo3 = { version = "0.4.2", features = ["futures", "ledger"] }
```

### Update Build Scripts
```bash
# Before
./scripts/build.sh --features futures,ledger,aws

# After
./scripts/build.sh --features futures,ledger
```

---

**NeoRust v0.4.2 - Secure, Tested, Production-Ready** 🔒✅ 