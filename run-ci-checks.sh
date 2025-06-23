#!/bin/bash

# Simple CI check runner for NeoRust
# This script runs all CI checks locally before pushing

echo "NeoRust CI Check Runner"
echo "======================"
echo ""

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Not in NeoRust project root"
    exit 1
fi

# Track results
PASSED=()
FAILED=()

# Function to run a check
run_check() {
    local name="$1"
    shift
    local cmd="$@"
    
    echo "Running: $name"
    echo "Command: $cmd"
    echo "---"
    
    if eval "$cmd"; then
        echo "✅ PASSED: $name"
        PASSED+=("$name")
    else
        echo "❌ FAILED: $name"
        FAILED+=("$name")
    fi
    echo ""
}

# 1. Format check
run_check "Format Check" "cargo fmt --all -- --check"

# 2. Clippy
run_check "Clippy" "cargo clippy --workspace --all-targets --all-features --exclude neo-gui -- -D warnings"

# 3. Tests (excluding neo-gui)
echo "Preparing for tests (excluding neo-gui)..."
cp Cargo.toml Cargo.toml.backup
sed -i.bak 's/"neo-gui",//g; s/, "neo-gui"//g; s/"neo-gui"//g' Cargo.toml
rm -rf neo-gui

run_check "Rust Tests" "NEORUST_SKIP_NETWORK_TESTS=1 cargo test --package neo3 --all-features"

# Restore workspace
mv Cargo.toml.backup Cargo.toml
git checkout -- neo-gui 2>/dev/null || true

# Summary
echo ""
echo "=============================="
echo "CI Check Summary"
echo "=============================="
echo "Passed: ${#PASSED[@]} checks"
for check in "${PASSED[@]}"; do
    echo "  ✅ $check"
done

if [ ${#FAILED[@]} -gt 0 ]; then
    echo ""
    echo "Failed: ${#FAILED[@]} checks"
    for check in "${FAILED[@]}"; do
        echo "  ❌ $check"
    done
    echo ""
    echo "Please fix the failing checks before pushing!"
    exit 1
else
    echo ""
    echo "All checks passed! Safe to push."
    exit 0
fi