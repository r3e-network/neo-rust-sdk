#!/bin/bash
# Don't use set -e here because we want to continue even if checks fail

echo "================================================================"
echo "Running All CI Checks Locally"
echo "================================================================"
echo ""

# Get the script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Not in project root directory"
    echo "Please run this script from the NeoRust project root"
    exit 1
fi

# Make all scripts executable
chmod +x "$SCRIPT_DIR"/*.sh

# Track failures
FAILED_CHECKS=()

# Function to run a check and track failures
run_check() {
    local check_name="$1"
    local script_path="$2"
    
    echo ""
    echo "----------------------------------------------------------------"
    echo "Running: $check_name"
    echo "----------------------------------------------------------------"
    
    # Run the script and capture the exit code
    set +e  # Disable exit on error temporarily
    "$script_path"
    local exit_code=$?
    set -e  # Re-enable exit on error
    
    if [ $exit_code -eq 0 ]; then
        echo "✅ $check_name passed"
    else
        echo "❌ $check_name failed (exit code: $exit_code)"
        FAILED_CHECKS+=("$check_name")
    fi
    
    return 0  # Always return success to continue running checks
}

# Run all checks
run_check "Format Check" "$SCRIPT_DIR/01-format-check.sh"
run_check "Clippy Linting" "$SCRIPT_DIR/02-clippy-check.sh"
run_check "Rust Tests" "$SCRIPT_DIR/03-rust-tests.sh"
run_check "Benchmarks" "$SCRIPT_DIR/04-benchmarks.sh"
run_check "Documentation" "$SCRIPT_DIR/05-documentation.sh"
run_check "Security Audit" "$SCRIPT_DIR/06-security-audit.sh"
run_check "Release Check" "$SCRIPT_DIR/07-release-check.sh"

# Optional: Run neo-gui tests if available
if [ -d "neo-gui" ]; then
    run_check "Neo-GUI Tests" "$SCRIPT_DIR/08-neo-gui-tests.sh"
fi

# Summary
echo ""
echo "================================================================"
echo "CI Check Summary"
echo "================================================================"

if [ ${#FAILED_CHECKS[@]} -eq 0 ]; then
    echo "✅ All CI checks passed!"
    echo ""
    echo "It's safe to commit and push your changes."
    exit 0
else
    echo "❌ Some CI checks failed:"
    for check in "${FAILED_CHECKS[@]}"; do
        echo "  - $check"
    done
    echo ""
    echo "Please fix the issues before pushing."
    exit 1
fi