#!/bin/bash
set -e

echo "================================================"
echo "Running Neo-GUI Tests"
echo "================================================"

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Not in project root directory"
    exit 1
fi

# Check if neo-gui directory exists
if [ ! -d "neo-gui" ]; then
    echo "Error: neo-gui directory not found"
    exit 1
fi

cd neo-gui

# Install dependencies if needed
if [ ! -d "node_modules" ]; then
    echo "Installing frontend dependencies..."
    npm ci
fi

echo "Running frontend unit tests with coverage..."
npm run test:coverage

echo "Building frontend..."
npm run build

# Check if we can run e2e tests (requires display on Linux)
if [ "$CI" != "true" ] || [ "$(uname)" != "Linux" ]; then
    echo "Running e2e tests..."
    npm run test:e2e
else
    echo "Skipping e2e tests in CI on Linux (no display)"
fi

echo "✅ Neo-GUI tests completed!"