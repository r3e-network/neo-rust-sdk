#!/bin/bash

echo "================================================"
echo "Running Clippy Linting"
echo "================================================"

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Not in project root directory"
    exit 1
fi

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo not found in PATH"
    exit 1
fi

# Backup original Cargo.toml
cp Cargo.toml Cargo.toml.clippy-backup

# Function to restore workspace
restore_workspace() {
    echo "Restoring workspace..."
    if [ -f "Cargo.toml.clippy-backup" ]; then
        mv Cargo.toml.clippy-backup Cargo.toml
    fi
    git checkout -- neo-gui 2>/dev/null || true
}

# Set trap to restore on exit
trap restore_workspace EXIT

echo "Modifying workspace to exclude neo-gui..."
sed -i.bak 's/"neo-gui",//g; s/, "neo-gui"//g; s/"neo-gui"//g' Cargo.toml
rm -rf neo-gui

echo "Running Clippy..."
if cargo clippy --workspace --all-targets --all-features -- -D warnings; then
    echo "✅ Clippy linting passed!"
    exit 0
else
    echo "❌ Clippy linting failed!"
    echo "Fix the warnings above before pushing"
    exit 1
fi