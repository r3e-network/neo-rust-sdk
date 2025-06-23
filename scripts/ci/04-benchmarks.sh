#!/bin/bash
set -e

echo "================================================"
echo "Running Rust Benchmarks"
echo "================================================"

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Not in project root directory"
    exit 1
fi

# Backup original Cargo.toml
cp Cargo.toml Cargo.toml.bench-backup

# Function to restore workspace
restore_workspace() {
    echo "Restoring workspace..."
    if [ -f "Cargo.toml.bench-backup" ]; then
        mv Cargo.toml.bench-backup Cargo.toml
    fi
    git checkout -- neo-gui 2>/dev/null || true
}

# Set trap to restore on exit
trap restore_workspace EXIT

echo "Modifying workspace to exclude neo-gui..."
sed -i.bak 's/"neo-gui",//g; s/, "neo-gui"//g; s/"neo-gui"//g' Cargo.toml
rm -rf neo-gui

echo "Running benchmarks for neo3 package..."
cargo bench --manifest-path Cargo.toml --package neo3 --verbose

echo "✅ Benchmarks completed!"