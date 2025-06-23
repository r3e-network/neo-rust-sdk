#!/bin/bash

echo "================================================"
echo "Running Rust Format Check"
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

echo "Checking Rust formatting..."
if cargo fmt --all -- --check; then
    echo "✅ Rust formatting check passed!"
    exit 0
else
    echo "❌ Rust formatting check failed!"
    echo "Run 'cargo fmt --all' to fix formatting issues"
    exit 1
fi