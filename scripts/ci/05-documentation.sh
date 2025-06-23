#!/bin/bash
set -e

echo "================================================"
echo "Running Documentation Generation"
echo "================================================"

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Not in project root directory"
    exit 1
fi

# Backup original Cargo.toml
cp Cargo.toml Cargo.toml.doc-backup

# Function to restore workspace
restore_workspace() {
    echo "Restoring workspace..."
    if [ -f "Cargo.toml.doc-backup" ]; then
        mv Cargo.toml.doc-backup Cargo.toml
    fi
    git checkout -- neo-gui 2>/dev/null || true
}

# Set trap to restore on exit
trap restore_workspace EXIT

echo "Modifying workspace to exclude neo-gui..."
sed -i.bak 's/"neo-gui",//g; s/, "neo-gui"//g; s/"neo-gui"//g' Cargo.toml
rm -rf neo-gui

echo "Building documentation for neo3 package..."
cargo doc --manifest-path Cargo.toml --package neo3 --no-deps --document-private-items

echo "Building documentation for neo-cli package..."
cd neo-cli && cargo doc --no-deps --document-private-items
cd ..

echo "✅ Documentation generation completed!"