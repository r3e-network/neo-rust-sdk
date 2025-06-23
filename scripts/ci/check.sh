#!/bin/bash

# Simple CI check runner that can run individual or all checks
# Usage: ./check.sh [check-name]
# Examples:
#   ./check.sh          # Run all checks
#   ./check.sh format   # Run format check only
#   ./check.sh clippy   # Run clippy check only

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR/../.." || exit 1

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_info() {
    echo -e "${YELLOW}ℹ️  $1${NC}"
}

# Function to run a specific check
run_single_check() {
    local check_type="$1"
    
    case "$check_type" in
        format|fmt)
            print_info "Running format check..."
            cargo fmt --all -- --check
            ;;
        clippy|lint)
            print_info "Running clippy..."
            cargo clippy --workspace --all-targets --all-features --exclude neo-gui -- -D warnings
            ;;
        test|tests)
            print_info "Running tests (excluding neo-gui)..."
            # Backup and modify Cargo.toml
            cp Cargo.toml Cargo.toml.backup
            sed -i.bak 's/"neo-gui",//g; s/, "neo-gui"//g; s/"neo-gui"//g' Cargo.toml
            rm -rf neo-gui
            
            # Run tests
            NEORUST_SKIP_NETWORK_TESTS=1 cargo test --package neo3 --all-features
            cd neo-cli && NEORUST_SKIP_NETWORK_TESTS=1 cargo test --all-features && cd ..
            
            # Restore
            mv Cargo.toml.backup Cargo.toml
            git checkout -- neo-gui 2>/dev/null || true
            ;;
        bench|benchmark)
            print_info "Running benchmarks..."
            # Backup and modify Cargo.toml
            cp Cargo.toml Cargo.toml.backup
            sed -i.bak 's/"neo-gui",//g; s/, "neo-gui"//g; s/"neo-gui"//g' Cargo.toml
            rm -rf neo-gui
            
            # Run benchmarks
            cargo bench --package neo3
            
            # Restore
            mv Cargo.toml.backup Cargo.toml
            git checkout -- neo-gui 2>/dev/null || true
            ;;
        doc|docs)
            print_info "Building documentation..."
            # Backup and modify Cargo.toml
            cp Cargo.toml Cargo.toml.backup
            sed -i.bak 's/"neo-gui",//g; s/, "neo-gui"//g; s/"neo-gui"//g' Cargo.toml
            rm -rf neo-gui
            
            # Build docs
            cargo doc --package neo3 --no-deps --document-private-items
            cd neo-cli && cargo doc --no-deps --document-private-items && cd ..
            
            # Restore
            mv Cargo.toml.backup Cargo.toml
            git checkout -- neo-gui 2>/dev/null || true
            ;;
        audit|security)
            print_info "Running security audit..."
            if ! command -v cargo-audit &> /dev/null; then
                print_info "Installing cargo-audit..."
                cargo install cargo-audit
            fi
            cargo audit
            ;;
        release)
            print_info "Running release check..."
            # Backup and modify Cargo.toml
            cp Cargo.toml Cargo.toml.backup
            sed -i.bak 's/"neo-gui",//g; s/, "neo-gui"//g; s/"neo-gui"//g' Cargo.toml
            rm -rf neo-gui
            
            # Check release
            cargo publish --dry-run --allow-dirty
            
            # Restore
            mv Cargo.toml.backup Cargo.toml
            git checkout -- neo-gui 2>/dev/null || true
            ;;
        gui)
            print_info "Running neo-gui tests..."
            if [ -d "neo-gui" ]; then
                cd neo-gui
                [ ! -d "node_modules" ] && npm ci
                npm run test:coverage
                npm run build
                cd ..
            else
                print_error "neo-gui directory not found"
                return 1
            fi
            ;;
        *)
            print_error "Unknown check type: $check_type"
            echo "Available checks: format, clippy, test, bench, doc, audit, release, gui"
            return 1
            ;;
    esac
}

# Main logic
if [ $# -eq 0 ]; then
    # Run all checks
    print_info "Running all CI checks..."
    
    FAILED_CHECKS=()
    
    for check in format clippy test bench doc audit release gui; do
        echo ""
        echo "================================================================"
        echo "Running: $check"
        echo "================================================================"
        
        if run_single_check "$check"; then
            print_success "$check check passed"
        else
            print_error "$check check failed"
            FAILED_CHECKS+=("$check")
        fi
    done
    
    echo ""
    echo "================================================================"
    echo "Summary"
    echo "================================================================"
    
    if [ ${#FAILED_CHECKS[@]} -eq 0 ]; then
        print_success "All checks passed!"
        exit 0
    else
        print_error "Failed checks: ${FAILED_CHECKS[*]}"
        exit 1
    fi
else
    # Run specific check
    run_single_check "$1"
fi