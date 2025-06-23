#!/bin/bash
set -e

echo "================================================"
echo "Running Release Check"
echo "================================================"

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Not in project root directory"
    exit 1
fi

# Check if we have enough commits
COMMIT_COUNT=$(git rev-list --count HEAD)
if [ "$COMMIT_COUNT" -gt 1 ]; then
    if ! git diff --quiet HEAD~1 HEAD -- Cargo.toml; then
        echo "Cargo.toml changed, checking version bump"
        git diff HEAD~1 HEAD -- Cargo.toml
    fi
else
    echo "Not enough commits to check for changes"
fi

# Backup original Cargo.toml
cp Cargo.toml Cargo.toml.release-backup

# Function to restore workspace
restore_workspace() {
    echo "Restoring workspace..."
    if [ -f "Cargo.toml.release-backup" ]; then
        mv Cargo.toml.release-backup Cargo.toml
    fi
    git checkout -- neo-gui 2>/dev/null || true
}

# Set trap to restore on exit
trap restore_workspace EXIT

echo "Modifying workspace to exclude neo-gui..."
sed -i.bak 's/"neo-gui",//g; s/, "neo-gui"//g; s/"neo-gui"//g' Cargo.toml
rm -rf neo-gui

echo "Running cargo publish dry run..."
cargo publish --dry-run --all-features --allow-dirty

echo "✅ Release check completed!"