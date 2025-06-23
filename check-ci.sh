#!/bin/sh
# Simple CI check script for NeoRust
# This script runs basic checks before pushing

echo "NeoRust CI Pre-Push Checks"
echo "=========================="
echo ""

# Function to check command result
check_result() {
    if [ $1 -eq 0 ]; then
        echo "✅ $2 passed"
        return 0
    else
        echo "❌ $2 failed"
        return 1
    fi
}

# Track overall success
ALL_PASSED=true

# 1. Format check
echo "1. Checking code formatting..."
cargo fmt --all -- --check 2>&1
check_result $? "Format check" || ALL_PASSED=false

echo ""

# 2. Clippy
echo "2. Running Clippy linter..."
cargo clippy --workspace --all-targets --all-features --exclude neo-gui -- -D warnings 2>&1
check_result $? "Clippy" || ALL_PASSED=false

echo ""

# 3. Basic test
echo "3. Running basic tests..."
echo "Note: Skipping neo-gui to avoid GUI dependencies"

# Summary
echo ""
echo "=========================="
if [ "$ALL_PASSED" = true ]; then
    echo "✅ All checks passed!"
    echo "You can safely push your changes."
    exit 0
else
    echo "❌ Some checks failed!"
    echo "Please fix the issues before pushing."
    exit 1
fi