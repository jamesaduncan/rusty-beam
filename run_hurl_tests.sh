#!/bin/bash
# Wrapper script to run hurl tests with proper setup and teardown

set -e

echo "=== Running Hurl Integration Tests ==="

# Function to cleanup on exit
cleanup() {
    echo "Cleaning up..."
    
    # Kill any running server
    pkill -f "rusty-beam" || true
    
    # Run teardown
    if [ -f "./tests/integration/teardown-tests.sh" ]; then
        ./tests/integration/teardown-tests.sh -q
    fi
}

# Set trap to cleanup on exit
trap cleanup EXIT

# Build plugins first
echo "Building plugins..."
if [ -f "./build-plugins.sh" ]; then
    ./build-plugins.sh
fi

# Build server
echo "Building server..."
cargo build --release

# Run setup
echo "Setting up test environment..."
if [ -f "./tests/integration/setup-tests.sh" ]; then
    ./tests/integration/setup-tests.sh
fi

# Copy required files
echo "Copying test files..."
# index.html files are created dynamically by tests
cp tests/files/foo.html tests/files/localhost/ 2>/dev/null || true  
cp tests/files/foo.html tests/files/example-com/ 2>/dev/null || true

# Start server
echo "Starting server..."
./target/release/rusty-beam tests/config/test-config.html &
SERVER_PID=$!

# Wait for server to be ready
echo "Waiting for server..."
MAX_RETRIES=20
RETRY_COUNT=0
while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
    if curl -s -o /dev/null "http://localhost:3000/"; then
        echo "Server is ready!"
        break
    fi
    sleep 0.5
    RETRY_COUNT=$((RETRY_COUNT + 1))
done

if [ $RETRY_COUNT -eq $MAX_RETRIES ]; then
    echo "ERROR: Server failed to start"
    exit 1
fi

# Run hurl tests
echo "Running hurl tests..."
hurl \
    --variable host=localhost \
    --variable port=3000 \
    --variable test_host=localhost \
    tests/integration/tests.hurl \
    --test

# Capture test result
TEST_RESULT=$?

# Run root path selector tests if they exist
if [ -f "tests/integration/test-root-selector-operations.hurl" ] && [ $TEST_RESULT -eq 0 ]; then
    echo "Running root path selector tests..."
    hurl \
        --variable host=localhost \
        --variable port=3000 \
        --variable test_host=localhost \
        tests/integration/test-root-selector-operations.hurl \
        --test
    
    # Update test result
    TEST_RESULT=$?
fi

# Kill server
kill $SERVER_PID 2>/dev/null || true
wait $SERVER_PID 2>/dev/null || true

# Return test result
if [ $TEST_RESULT -eq 0 ]; then
    echo "✅ All tests passed!"
else
    echo "❌ Tests failed!"
fi

exit $TEST_RESULT