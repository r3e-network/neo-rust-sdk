# NeoRust Automated Release Process

This document describes the automated release process for NeoRust using GitHub Actions and branch-based triggers.

## 🚀 Quick Start for New Releases

### 1. Add Crates.io API Key to GitHub Secrets

**⚠️ SECURITY**: Never commit API keys to the repository!

1. Go to your GitHub repository → **Settings** → **Secrets and variables** → **Actions**
2. Click **"New repository secret"**
3. Name: `CRATES_IO_TOKEN`
4. Value: Your crates.io API token
5. Click **"Add secret"**

### 2. Create a New Release

```bash
# Option 1: Use the automated script (Recommended)
./scripts/prepare-release.sh 0.5.2

# Option 2: Manual process
# 1. Update version numbers manually
# 2. Create release branch: git checkout -b v0.5.2
# 3. Push branch: git push origin v0.5.2
```

### 3. Monitor the Automated Release

The GitHub Actions workflow will automatically:
- ✅ Validate version format
- ✅ Run comprehensive quality checks
- ✅ Create GitHub release
- ✅ Publish to crates.io

## 🔄 Automated Workflow Details

## 🎯 Release Strategy

### Versioning Scheme

NeoRust follows [Semantic Versioning (SemVer)](https://semver.org/) with the format `MAJOR.MINOR.PATCH`:

- **MAJOR**: Breaking changes that require user migration
- **MINOR**: New features that are backwards compatible
- **PATCH**: Bug fixes and security updates

### Pre-Release Versions

Pre-release versions use suffixes to indicate stability:
- `alpha` - Early development, significant changes expected
- `beta` - Feature complete, testing and stabilization phase
- `rc` - Release candidate, final testing before stable

Examples: `0.5.2-alpha.1`, `0.5.2-beta.2`, `0.5.2-rc.1`

### Release Channels

1. **Stable** - Production-ready releases (no suffix)
2. **Beta** - Preview releases for testing new features
3. **Alpha** - Early access to development features

## 📋 Release Checklist

### Pre-Release Phase

#### 1. Version Planning
- [ ] Review feature completeness and breaking changes
- [ ] Determine appropriate version number (MAJOR.MINOR.PATCH)
- [ ] Create release milestone in GitHub
- [ ] Update project roadmap and documentation

#### 2. Code Quality Gates
- [ ] All CI tests passing on main branch
- [ ] Code coverage meets minimum threshold (>80%)
- [ ] No high or critical security vulnerabilities
- [ ] Clippy lints passing with no warnings
- [ ] Code formatting consistent (`cargo fmt --check`)

#### 3. Documentation Updates
- [ ] Update CHANGELOG.md with all changes
- [ ] Update version numbers in all Cargo.toml files
- [ ] Update README.md if needed
- [ ] Regenerate API documentation
- [ ] Review and update user guides

#### 4. Testing Phase
- [ ] Run full test suite on all supported platforms
- [ ] Test examples and tutorials
- [ ] Performance benchmarks regression testing
- [ ] Integration testing with real Neo N3 networks
- [ ] GUI application testing on all platforms

### Release Phase

#### 1. Version Tagging
```bash
# Create and push version tag
git tag -a v0.5.2 -m "Release version 0.5.2"
git push origin v0.5.2
```

#### 2. Automated Release (GitHub Actions)
The release workflow automatically:
- [ ] Validates version format and changelog
- [ ] Runs comprehensive test suite
- [ ] Builds binaries for all platforms
- [ ] Publishes crates to crates.io in dependency order
- [ ] Creates GitHub release with artifacts
- [ ] Updates documentation website

#### 3. Manual Verification
- [ ] Verify crates.io publication
- [ ] Test installation from crates.io
- [ ] Verify GitHub release artifacts
- [ ] Check documentation website updates

### Post-Release Phase

#### 1. Communication
- [ ] Announce release on GitHub Discussions
- [ ] Update community channels (Discord, Reddit)
- [ ] Create blog post for major releases
- [ ] Update project website

#### 2. Monitoring
- [ ] Monitor for bug reports and issues
- [ ] Track download statistics
- [ ] Monitor security alerts
- [ ] Collect user feedback

## 🤖 Automation Workflows

### Continuous Integration (`ci.yml`)
Runs on every push and PR:
- Multi-platform testing (Linux, Windows, macOS)
- Multiple Rust versions (stable, beta)
- Linting and formatting checks
- Security auditing
- Documentation building
- Example compilation

### Security Scanning (`security.yml`)
Runs weekly and on security-related changes:
- Dependency vulnerability scanning
- License compliance checking
- Secret scanning
- Supply chain security analysis
- SBOM generation

### Release Automation (`release.yml`)
Triggered by version tags:
- Version validation
- Comprehensive testing
- Multi-platform binary builds
- Crate publishing to crates.io
- GitHub release creation
- Documentation updates

### Documentation (`docs.yml`)
Builds and deploys documentation:
- Rust API documentation
- mdBook user guides
- GUI TypeScript documentation
- Website deployment to GitHub Pages

## 📦 Multi-Crate Publishing

### Publishing Order
Crates are published in dependency order to avoid failures:

1. `neo3-types` - Core types and primitives
2. `neo3-crypto` - Cryptographic operations
3. `neo3-rpc` - RPC client implementation
4. `neo3-builder` - Transaction builders
5. `neo3-wallets` - Wallet management
6. `neo3-contracts` - Smart contract tools
7. `neo3-macros` - Procedural macros
8. `neo3` - Main SDK crate

### Version Synchronization
All crates use the same version number for consistency:
- Simplifies dependency management
- Ensures compatibility between crates
- Provides clear release tracking

## 🔍 Quality Gates

### Automated Checks
- **Compilation**: All crates must compile successfully
- **Tests**: All tests must pass (unit, integration, doc tests)
- **Linting**: Clippy must pass with no warnings
- **Formatting**: Code must be properly formatted
- **Security**: No known vulnerabilities in dependencies
- **Documentation**: All public APIs must be documented

### Manual Reviews
- **Breaking Changes**: Review impact and migration path
- **Security Changes**: Security team review required
- **Performance**: Benchmark regression analysis
- **User Experience**: Documentation and API usability review

## 🚨 Hotfix Process

For critical security issues or major bugs:

1. **Assessment**: Determine severity and impact
2. **Branch Creation**: Create hotfix branch from latest release tag
3. **Fix Development**: Implement minimal fix with tests
4. **Expedited Review**: Fast-track code review process
5. **Release**: Follow abbreviated release process
6. **Communication**: Immediate security advisory if needed

### Hotfix Version Numbers
- Increment PATCH version: `0.5.2` → `0.5.3`
- For security fixes, consider pre-release: `0.5.2-security.1`

## 📊 Release Metrics

### Success Criteria
- **Build Success**: 100% CI pass rate
- **Test Coverage**: >80% line coverage maintained
- **Security**: Zero high/critical vulnerabilities
- **Performance**: No significant regressions
- **Documentation**: Complete API coverage

### Monitoring
- Download statistics from crates.io
- GitHub release download counts
- Issue reports and bug tracking
- Community feedback and adoption metrics

## 🔧 Tools and Scripts

### Version Management
```bash
# Update all Cargo.toml versions
./scripts/update-version.sh 0.5.0

# Generate changelog entries
./scripts/generate-changelog.sh

# Validate release readiness
./scripts/check-release.sh
```

### Testing Scripts
```bash
# Run full test suite
./scripts/test-all.sh

# Test examples
./scripts/test-examples.sh

# Cross-platform testing
./scripts/test-platforms.sh
```

### Release Scripts
```bash
# Prepare release
./scripts/prepare-release.sh 0.5.2

# Build release artifacts
./scripts/build-release.sh

# Publish release
./scripts/publish-release.sh
```

## 🔗 Related Documentation

- [Contributing Guidelines](../CONTRIBUTING.md)
- [Security Policy](../SECURITY.md)
- [API Guidelines](../API_GUIDELINES.md)
- [Project Structure](../PROJECT_STRUCTURE.md)

## 📞 Support

For release-related questions:
- **GitHub Discussions**: Technical questions and feedback
- **GitHub Issues**: Bug reports and feature requests
- **Security**: security@r3e.network for security issues

---

This release process ensures high-quality, reliable releases while maintaining development velocity and community trust.
