#!/bin/bash
set -e

echo "================================================"
echo "Running Rust Tests (without neo-gui)"
echo "================================================"

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Not in project root directory"
    exit 1
fi

# Backup original Cargo.toml
cp Cargo.toml Cargo.toml.test-backup

# Function to restore workspace
restore_workspace() {
    echo "Restoring workspace..."
    if [ -f "Cargo.toml.test-backup" ]; then
        mv Cargo.toml.test-backup Cargo.toml
    fi
    git checkout -- neo-gui 2>/dev/null || true
}

# Set trap to restore on exit
trap restore_workspace EXIT

echo "Modifying workspace to exclude neo-gui..."
sed -i.bak 's/"neo-gui",//g; s/, "neo-gui"//g; s/"neo-gui"//g' Cargo.toml
rm -rf neo-gui

echo "Testing neo3 package..."
NEORUST_SKIP_NETWORK_TESTS=1 cargo test --manifest-path Cargo.toml --package neo3 --verbose --all-features

echo "Testing neo-cli package..."
cd neo-cli
NEORUST_SKIP_NETWORK_TESTS=1 cargo test --verbose --all-features
cd ..

echo "✅ Rust tests passed!"