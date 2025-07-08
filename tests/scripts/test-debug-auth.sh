#!/bin/bash

# Start server in background and capture stderr
cargo run --release -- config/config.html 2>&1 | tee server.log &
SERVER_PID=$!

# Wait for server to start
sleep 3

echo "=== Testing PUT to / with selector ==="
curl -X PUT -u admin:admin123 -H "Range: selector=h1" -d "Modified Title" http://localhost:3000/ -w "\nStatus: %{http_code}\n"

echo
echo "=== Testing PUT to /index.html with selector ==="
curl -X PUT -u admin:admin123 -H "Range: selector=h1" -d "Modified Title" http://localhost:3000/index.html -w "\nStatus: %{http_code}\n"

# Kill server
kill $SERVER_PID 2>/dev/null

# Show relevant debug output
echo
echo "=== Authorization debug output for / request ==="
grep -A5 -B5 "path: /" server.log | grep -E "(Authorization|path:|Method:|resource:|Resource:|Permission:|decision)" | tail -20