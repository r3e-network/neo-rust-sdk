#!/bin/bash
set -e

# Prepare Release Script for NeoRust
# Usage: ./scripts/prepare-release.sh <version>

if [ $# -eq 0 ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 0.5.0"
    exit 1
fi

VERSION="$1"

echo "🚀 Preparing release $VERSION..."

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d ".git" ]; then
    echo "❌ Error: This script must be run from the root of the NeoRust repository"
    exit 1
fi

# Check if git working directory is clean
if [ -n "$(git status --porcelain)" ]; then
    echo "❌ Error: Git working directory is not clean. Please commit or stash changes."
    git status --short
    exit 1
fi

# Check if we're on the main branch
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [ "$CURRENT_BRANCH" != "main" ] && [ "$CURRENT_BRANCH" != "master" ]; then
    echo "⚠️  Warning: Not on main/master branch (currently on $CURRENT_BRANCH)"
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Update version numbers
echo "📝 Updating version numbers..."
./scripts/update-version.sh "$VERSION"

# Run quality checks
echo "🔍 Running quality checks..."

# Check code formatting
echo "📐 Checking code formatting..."
if ! cargo fmt --all -- --check; then
    echo "❌ Code formatting issues found. Run 'cargo fmt --all' to fix."
    exit 1
fi

# Run clippy
echo "📎 Running clippy..."
if ! cargo clippy --workspace --all-features -- -D warnings; then
    echo "❌ Clippy warnings found. Please fix before release."
    exit 1
fi

# Run tests
echo "🧪 Running tests..."
if ! cargo test --workspace; then
    echo "❌ Tests failed. Please fix before release."
    exit 1
fi

# Check for security vulnerabilities
echo "🔒 Checking for security vulnerabilities..."
if command -v cargo-audit &> /dev/null; then
    if ! cargo audit; then
        echo "❌ Security vulnerabilities found. Please fix before release."
        exit 1
    fi
else
    echo "⚠️  cargo-audit not found. Install with: cargo install cargo-audit"
fi

# Build documentation
echo "📚 Building documentation..."
if ! cargo doc --workspace --no-deps; then
    echo "❌ Documentation build failed."
    exit 1
fi

# Test examples compilation
echo "📋 Testing examples compilation..."
for example_dir in examples/*/; do
    if [ -f "$example_dir/Cargo.toml" ]; then
        echo "  Testing $example_dir..."
        if ! (cd "$example_dir" && cargo check); then
            echo "❌ Example in $example_dir failed to compile."
            exit 1
        fi
    fi
done

# Check CHANGELOG.md
echo "📰 Checking CHANGELOG.md..."
if ! grep -q "## \[$VERSION\]" CHANGELOG.md; then
    echo "❌ Version $VERSION not found in CHANGELOG.md"
    echo "Please add release notes for version $VERSION to CHANGELOG.md"
    exit 1
fi

# Build release binaries to test
echo "🔨 Building release binaries..."
if ! cargo build --release; then
    echo "❌ Release build failed."
    exit 1
fi

# Test CLI binary
echo "🖥️  Testing CLI binary..."
if ! ./target/release/neo-cli --help > /dev/null; then
    echo "❌ CLI binary test failed."
    exit 1
fi

# Create release commit
echo "📝 Creating release commit..."
git add -A
git commit -m "Release version $VERSION

- Updated version numbers across all packages
- Verified all quality checks pass
- Updated documentation and changelog"

echo "✅ Release preparation completed successfully!"
echo ""
echo "📋 Summary:"
echo "  Version: $VERSION"
echo "  Tests: ✅ Passed"
echo "  Linting: ✅ Passed"
echo "  Security: ✅ Checked"
echo "  Documentation: ✅ Built"
echo "  Examples: ✅ Compiled"
echo "  Changelog: ✅ Updated"
echo ""
echo "🚀 Next steps:"
echo "1. Review the changes: git log -1 --stat"
echo "2. Push the release commit: git push origin $CURRENT_BRANCH"
echo "3. Create release branch: git checkout -b v$VERSION && git push origin v$VERSION"
echo "4. GitHub Actions will automatically:"
echo "   - Run quality checks"
echo "   - Create GitHub release"
echo "   - Publish to crates.io"
echo "5. Monitor the release: https://github.com/R3E-Network/NeoRust/actions"
echo ""
echo "🎉 Ready for automated release!"