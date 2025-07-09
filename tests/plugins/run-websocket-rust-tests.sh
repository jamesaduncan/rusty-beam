#!/bin/bash
# Run Rust WebSocket tests after Hurl tests
# This is called from run_plugin_tests_isolated.sh for the websocket plugin

set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Running WebSocket Rust tests...${NC}"

# The server should already be running from the Hurl tests
# We just need to run the Rust tests

# Set a shorter timeout for tests since server is already running
export WEBSOCKET_TEST_TIMEOUT=5

# Run the Rust tests
if cargo test --test websocket-broadcast-tests -- --test-threads=1 --nocapture 2>/tmp/websocket-rust-tests.log; then
    echo -e "${GREEN}WebSocket Rust tests passed${NC}"
    exit 0
else
    echo -e "${RED}WebSocket Rust tests failed${NC}"
    # Show relevant errors
    grep -A5 -B5 "FAILED\|panicked\|error" /tmp/websocket-rust-tests.log | head -30
    exit 1
fi