#!/bin/bash
# Test WebSocket plugin with a running server

set -e

echo "Starting test server..."
./target/debug/rusty-beam tests/plugins/configs/websocket-config.html &
SERVER_PID=$!

# Wait for server to start
sleep 2

echo "Running WebSocket tests..."
cargo test --test websocket-tests -- --nocapture || TEST_RESULT=$?

echo "Stopping server..."
kill $SERVER_PID

if [ "${TEST_RESULT}" != "" ]; then
    echo "WebSocket tests failed"
    exit 1
fi

echo "WebSocket tests passed!"