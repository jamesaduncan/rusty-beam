#!/bin/bash
# Script to run all plugin tests

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== Running Plugin Tests ==="
echo

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test results
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
FAILED_FILES=()

# Function to run a single test file
run_test() {
    local test_file=$1
    local test_name=$(basename "$test_file" .hurl)
    local config_file="tests/config/test-all-plugins-config.html"
    
    # Use auth config for auth-related tests
    if [[ "$test_name" == "test-basic-auth" ]] || [[ "$test_name" == "test-authorization" ]]; then
        config_file="tests/config/test-auth-config.html"
    fi
    
    # Use basic config for core plugin tests
    if [[ "$test_name" == "test-selector-handler" ]] || [[ "$test_name" == "test-file-handler" ]]; then
        config_file="tests/config/test-config.html"
    fi
    
    echo -n "Running $test_name... "
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    # Setup test environment
    ./tests/integration/setup-tests.sh >/dev/null 2>&1
    
    # Clean up any test files from previous runs
    rm -f tests/files/localhost/*.txt tests/files/localhost/*.json tests/files/localhost/*.css tests/files/localhost/*.js tests/files/localhost/*.bin tests/files/localhost/*.xml tests/files/localhost/*.svg tests/files/localhost/*.md 2>/dev/null || true
    rm -f tests/files/localhost/selector-test.html tests/files/localhost/auth-test.html tests/files/localhost/error-test.html 2>/dev/null || true
    rm -f tests/files/localhost/file-handler-test.* tests/files/localhost/test.* tests/files/localhost/new-post-file.txt 2>/dev/null || true
    rm -rf tests/files/localhost/test-dir 2>/dev/null || true
    
    # Start server in background
    ./target/release/rusty-beam "$config_file" >/dev/null 2>&1 &
    SERVER_PID=$!
    
    # Wait for server to start
    for i in {1..30}; do
        if curl -s http://localhost:3000/health >/dev/null 2>&1; then
            break
        fi
        sleep 0.1
    done
    
    # Run the test
    if hurl "$test_file" --test \
        --variable host=localhost \
        --variable port=3000 \
        --variable test_host=localhost \
        >/dev/null 2>&1; then
        echo -e "${GREEN}PASSED${NC}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        echo -e "${RED}FAILED${NC}"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        FAILED_FILES+=("$test_name")
    fi
    
    # Stop server
    kill $SERVER_PID 2>/dev/null || true
    wait $SERVER_PID 2>/dev/null || true
}

# Build plugins and server first
echo "Building plugins..."
./build-plugins.sh >/dev/null 2>&1
echo "Building server..."
cargo build --release >/dev/null 2>&1
echo

# Run each plugin test
for test_file in tests/plugins/test-*.hurl; do
    if [ -f "$test_file" ]; then
        run_test "$test_file"
    fi
done

echo
echo "=== Test Summary ==="
echo "Total tests: $TOTAL_TESTS"
echo -e "Passed: ${GREEN}$PASSED_TESTS${NC}"
echo -e "Failed: ${RED}$FAILED_TESTS${NC}"

if [ ${#FAILED_FILES[@]} -gt 0 ]; then
    echo
    echo -e "${RED}Failed tests:${NC}"
    for failed in "${FAILED_FILES[@]}"; do
        echo "  - $failed"
    done
    exit 1
else
    echo
    echo -e "${GREEN}All plugin tests passed!${NC}"
    exit 0
fi