#!/bin/bash
set -e

echo "================================================"
echo "Running Security Audit"
echo "================================================"

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Not in project root directory"
    exit 1
fi

# Check if cargo-audit is installed
if ! command -v cargo-audit &> /dev/null; then
    echo "Installing cargo-audit..."
    cargo install cargo-audit
fi

echo "Running Rust security audit..."
cargo audit

# Check if we have neo-gui directory for frontend audit
if [ -d "neo-gui" ]; then
    echo ""
    echo "Running frontend security audit..."
    cd neo-gui
    
    # Check if node_modules exists
    if [ ! -d "node_modules" ]; then
        echo "Installing frontend dependencies..."
        npm ci
    fi
    
    npm audit --audit-level=high
    cd ..
else
    echo "Skipping frontend audit (neo-gui not found)"
fi

echo "✅ Security audit completed!"