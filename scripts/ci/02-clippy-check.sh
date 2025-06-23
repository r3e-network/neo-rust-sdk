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

echo "Running Clippy..."
if cargo clippy --workspace --all-targets --all-features --exclude neo-gui -- -D warnings; then
    echo "✅ Clippy linting passed!"
    exit 0
else
    echo "❌ Clippy linting failed!"
    echo "Fix the warnings above before pushing"
    exit 1
fi