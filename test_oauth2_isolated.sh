#!/bin/bash
# Test OAuth2 plugin in isolation

set -e

# Start server with OAuth2 config
echo "Starting server with OAuth2 plugin..."
./target/release/rusty-beam tests/plugins/configs/google-oauth2-config.html &
SERVER_PID=$!

# Wait for server to start
sleep 2

# Run tests
echo "Running OAuth2 tests..."
hurl tests/plugins/test-google-oauth2.hurl --test \
    --variable host=localhost \
    --variable port=3456 \
    --variable test_host=localhost

# Cleanup
kill $SERVER_PID 2>/dev/null || true

echo "Test complete"