#!/bin/bash
# Isolated plugin test runner - each plugin gets its own environment

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== Running Isolated Plugin Tests ==="
echo

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Test results
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
FAILED_FILES=()

# Function to setup test environment for a plugin
setup_plugin_test() {
    local plugin_name=$1
    local host_dir="tests/plugins/hosts/$plugin_name"
    
    # Clean up any existing host directory
    rm -rf "$host_dir"
    
    # Copy template directory
    cp -r tests/plugins/template "$host_dir"
    
    echo "Setup complete for $plugin_name"
}

# Function to teardown test environment
teardown_plugin_test() {
    local plugin_name=$1
    local host_dir="tests/plugins/hosts/$plugin_name"
    
    # Clean up host directory
    rm -rf "$host_dir"
    
    echo "Teardown complete for $plugin_name"
}

# Function to run a single plugin test
run_plugin_test() {
    local plugin_name=$1
    # Try simple version first, fall back to regular
    local test_file="tests/plugins/test-${plugin_name}-simple.hurl"
    if [ ! -f "$test_file" ]; then
        test_file="tests/plugins/test-${plugin_name}.hurl"
    fi
    local config_file="tests/plugins/configs/${plugin_name}-config.html"
    
    # Check if test and config exist
    if [ ! -f "$test_file" ]; then
        echo "Skipping $plugin_name - no test file"
        return
    fi
    
    if [ ! -f "$config_file" ]; then
        echo "Skipping $plugin_name - no config file"
        return
    fi
    
    echo -n "Testing $plugin_name... "
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    # Setup test environment
    setup_plugin_test "$plugin_name" >/dev/null 2>&1
    
    # Debug: check if files were copied
    if [ ! -z "$PLUGIN_TEST_DEBUG" ]; then
        echo "Host directory contents:"
        ls -la "tests/plugins/hosts/$plugin_name/" || echo "Directory not found"
    fi
    
    # Start server (with verbose mode if debugging)
    if [ ! -z "$PLUGIN_TEST_DEBUG" ]; then
        ./target/release/rusty-beam -v "$config_file" >/tmp/rusty-beam-$plugin_name.log 2>&1 &
    else
        ./target/release/rusty-beam "$config_file" >/tmp/rusty-beam-$plugin_name.log 2>&1 &
    fi
    SERVER_PID=$!
    
    # Wait for server to start
    local waited=0
    while [ $waited -lt 30 ]; do
        if curl -s -o /dev/null http://localhost:3000/ 2>/dev/null; then
            sleep 0.5  # Extra wait to ensure server is ready
            break
        fi
        sleep 0.1
        waited=$((waited + 1))
    done
    
    # Debug: test if server is responding
    if [ ! -z "$PLUGIN_TEST_DEBUG" ]; then
        echo "Testing server response:"
        curl -v http://localhost:3000/foo.html 2>&1 | grep -E "< HTTP|404|200" || echo "No response"
    fi
    
    # Run the test
    if hurl "$test_file" --test \
        --variable host=localhost \
        --variable port=3000 \
        --variable test_host=localhost \
        >/tmp/hurl-$plugin_name.log 2>&1; then
        
        # For WebSocket, also run Rust tests
        if [ "$plugin_name" = "websocket" ] && [ -f "tests/plugins/run-websocket-rust-tests.sh" ]; then
            echo -e "${GREEN}PASSED${NC} (Hurl)"
            echo -n "  Running Rust tests... "
            if tests/plugins/run-websocket-rust-tests.sh >/tmp/websocket-rust.log 2>&1; then
                echo -e "${GREEN}PASSED${NC}"
                PASSED_TESTS=$((PASSED_TESTS + 1))
            else
                echo -e "${RED}FAILED${NC}"
                FAILED_TESTS=$((FAILED_TESTS + 1))
                FAILED_FILES+=("$plugin_name-rust")
                
                # Show error if requested
                if [ ! -z "$PLUGIN_TEST_DEBUG" ]; then
                    echo "Rust test error output:"
                    tail -20 /tmp/websocket-rust.log
                fi
            fi
        else
            echo -e "${GREEN}PASSED${NC}"
            PASSED_TESTS=$((PASSED_TESTS + 1))
        fi
    else
        echo -e "${RED}FAILED${NC}"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        FAILED_FILES+=("$plugin_name")
        
        # Show error if requested
        if [ ! -z "$PLUGIN_TEST_DEBUG" ]; then
            echo "Error output:"
            tail -20 /tmp/hurl-$plugin_name.log
        fi
    fi
    
    # Stop server
    kill $SERVER_PID 2>/dev/null || true
    wait $SERVER_PID 2>/dev/null || true
    
    # Teardown
    teardown_plugin_test "$plugin_name" >/dev/null 2>&1
}

# Build plugins and server first
echo "Building plugins..."
./build-plugins.sh >/dev/null 2>&1
echo "Building server..."
cargo build --release >/dev/null 2>&1
echo

# Create hosts directory
mkdir -p tests/plugins/hosts

# List of plugins to test
PLUGINS=(
    "file-handler"
    "selector-handler"
    "health-check"
    "cors"
    "compression"
    "security-headers"
    "rate-limit"
    "redirect"
    "error-handler"
    "access-log"
    "basic-auth"
    "authorization"
    "websocket"
)

# Run tests for each plugin
for plugin in "${PLUGINS[@]}"; do
    run_plugin_test "$plugin"
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