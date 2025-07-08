#!/bin/bash
# Helper script to ensure plugins are built before running tests

set -e

# Build plugins first
echo "Building plugins..."
if [ -f "./build-plugins.sh" ]; then
    ./build-plugins.sh
else
    echo "Warning: build-plugins.sh not found, assuming plugins are already built"
fi

# Run cargo test
echo "Running integration tests..."
cargo test --test integration_tests -- --nocapture