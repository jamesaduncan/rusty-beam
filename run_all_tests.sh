#!/bin/bash
# Run all tests for rusty-beam

set -e

echo "=== Running All Tests ==="
echo

echo "1. Building plugins..."
./build-plugins.sh

echo
echo "2. Running unit tests..."
cargo test

echo
echo "3. Running integration tests..."
./run_hurl_tests.sh

echo
echo "4. Running plugin tests..."
./run_plugin_tests_isolated.sh

echo
echo "=== All Tests Complete ==="