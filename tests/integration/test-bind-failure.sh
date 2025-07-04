#!/bin/bash

# Test script to demonstrate graceful bind failure
# This shows the current behavior (panic) vs expected behavior (graceful exit)

set -e

echo "Testing graceful bind failure behavior..."
echo "========================================"

# Build the server
cargo build --release

echo "Step 1: Starting first server instance..."
cargo run --release -- config/config.html > server1.log 2>&1 &
SERVER1_PID=$!

# Wait for first server to start
sleep 3

echo "Step 2: Starting second server instance (should fail gracefully)..."
echo "Current behavior: This will panic and show stack trace"
echo "Expected behavior: Should show clean error message and exit"
echo ""

# Try to start second server - this should fail
set +e  # Don't exit on error
cargo run --release -- config/config.html > server2.log 2>&1
EXIT_CODE=$?
set -e

echo "Exit code: $EXIT_CODE"
echo ""
echo "Server 2 output:"
echo "=================="
cat server2.log
echo ""

# Cleanup
echo "Cleaning up..."
kill $SERVER1_PID 2>/dev/null || true
wait $SERVER1_PID 2>/dev/null || true
rm -f server1.log server2.log

if [ $EXIT_CODE -eq 0 ]; then
    echo "❌ UNEXPECTED: Second server started successfully (should have failed)"
    exit 1
elif [ $EXIT_CODE -eq 1 ]; then
    # Check if output contains panic or stack trace
    if grep -q "panicked at\|stack backtrace" server2.log; then
        echo "✅ Second server failed as expected"
        echo "❌ BUT: The failure was not graceful (panic/stack trace shown)"
        echo "🔧 Need to implement graceful error handling"
        exit 1
    else
        echo "✅ Second server failed as expected"
        echo "✅ Failure was graceful (no panic/stack trace)"
        echo "🎉 Graceful error handling is working correctly!"
        exit 0
    fi
else
    echo "❌ Unexpected exit code: $EXIT_CODE"
    exit 1
fi