#!/bin/bash
# Manual test for WebSocket broadcasting functionality

set -e

echo "=== WebSocket Broadcasting Test ==="
echo

# Start the server
echo "1. Starting server..."
./target/debug/rusty-beam tests/plugins/configs/websocket-config.html &
SERVER_PID=$!
sleep 2

# Create a test file
echo "2. Creating test file..."
echo '<html><body><div id="content">Original content</div></body></html>' > tests/plugins/template/test-broadcast.html

echo
echo "3. To test WebSocket broadcasting:"
echo "   a) Open a WebSocket client (e.g., wscat or a browser console)"
echo "   b) Connect to: ws://localhost:3000/test-broadcast.html"
echo "   c) Send: {\"action\": \"subscribe\", \"selector\": \"#content\", \"url\": \"/test-broadcast.html\"}"
echo "   d) In another terminal, run:"
echo "      curl -X PUT -H 'Range: selector=#content' -d '<div id=\"content\">Updated content</div>' http://localhost:3000/test-broadcast.html"
echo "   e) The WebSocket client should receive a StreamItem with the update"
echo
echo "Server PID: $SERVER_PID"
echo "To stop: kill $SERVER_PID"
echo
echo "Press Ctrl+C to stop the server..."

# Wait for user to stop
trap "kill $SERVER_PID; rm -f tests/plugins/template/test-broadcast.html" EXIT
wait