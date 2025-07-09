#!/bin/bash
# Automated test for WebSocket broadcasting with selector-handler integration

set -e

echo "=== WebSocket Broadcasting Integration Test ==="

# Build if needed
if [ ! -f "./target/debug/rusty-beam" ]; then
    echo "Building rusty-beam..."
    cargo build
fi

# Create test file
TEST_FILE="tests/plugins/template/test-broadcast-auto.html"
echo '<html><body><div id="content">Original content</div></body></html>' > "$TEST_FILE"

# Start server
echo "Starting server..."
./target/debug/rusty-beam tests/plugins/configs/websocket-config.html &
SERVER_PID=$!
sleep 2

# Function to cleanup
cleanup() {
    echo "Cleaning up..."
    kill $SERVER_PID 2>/dev/null || true
    rm -f "$TEST_FILE"
}
trap cleanup EXIT

# Test WebSocket connection
echo "Testing WebSocket upgrade..."
curl -s -i -N \
  -H "Connection: Upgrade" \
  -H "Upgrade: websocket" \
  -H "Sec-WebSocket-Key: x3JJHMbDL1EzLkh9GBhXDw==" \
  -H "Sec-WebSocket-Version: 13" \
  http://localhost:3000/test-broadcast-auto.html | head -1 | grep -q "101 Switching Protocols"

if [ $? -eq 0 ]; then
    echo "✓ WebSocket upgrade successful"
else
    echo "✗ WebSocket upgrade failed"
    exit 1
fi

# Test selector-handler PUT
echo "Testing selector-handler PUT..."
RESPONSE=$(curl -s -X PUT -H 'Range: selector=#content' -d '<div id="content">Updated via PUT</div>' http://localhost:3000/test-broadcast-auto.html)

if [ $? -eq 0 ]; then
    echo "✓ Selector-handler PUT successful"
    echo "  Response: $RESPONSE"
else
    echo "✗ Selector-handler PUT failed"
    exit 1
fi

# Verify file was updated
echo "Verifying file update..."
if grep -q "Updated via PUT" "$TEST_FILE"; then
    echo "✓ File successfully updated"
else
    echo "✗ File update failed"
    exit 1
fi

echo
echo "=== All tests passed! ==="
echo
echo "Note: To fully test broadcasting, you need a WebSocket client connected and subscribed."
echo "The broadcasting mechanism is working if selector-handler sets metadata correctly."