#!/bin/bash

# Test runner for rusty-beam server
# This script starts the server and runs the Hurl tests

set -e

# Default configuration
HOST="localhost"
PORT="3000"
SERVER_PID=""
VERBOSE=false

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

cleanup() {
    if [ ! -z "$SERVER_PID" ]; then
        print_status "Stopping server (PID: $SERVER_PID)"
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
    fi
    
    # Show server logs if there were errors
    if [ -f "tests/integration/server.error.log" ] && [ -s "tests/integration/server.error.log" ]; then
        print_warning "Server errors detected. Log contents:"
        echo "--- Server Error Log ---"
        cat tests/integration/server.error.log
        echo "--- End Server Error Log ---"
    fi
    
    # Clean up log files unless there were errors
    if [ ! -s "tests/integration/server.error.log" ]; then
        rm -f tests/integration/server.log tests/integration/server.error.log
    else
        print_status "Server logs saved as tests/integration/server.log and tests/integration/server.error.log"
    fi
    
    # Run teardown script to clean up test artifacts
    print_status "Running test teardown..."
    ./tests/integration/teardown-tests.sh
}

# Set up cleanup on exit
trap cleanup EXIT

# Check if Hurl is installed
if ! command -v hurl &> /dev/null; then
    print_error "Hurl is not installed. Please install it first:"
    echo "  - On macOS: brew install hurl"
    echo "  - On Ubuntu/Debian: sudo apt install hurl"
    echo "  - On Arch Linux: sudo pacman -S hurl"
    echo "  - Or visit: https://hurl.dev/docs/installation.html"
    exit 1
fi

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    print_error "Cargo is not installed. Please install Rust first."
    exit 1
fi

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --host)
            HOST="$2"
            shift 2
            ;;
        --port)
            PORT="$2"
            shift 2
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo "Options:"
            echo "  --host HOST    Server host (default: 127.0.0.1)"
            echo "  --port PORT    Server port (default: 3000)"
            echo "  --verbose      Show verbose test output"
            echo "  --help         Show this help message"
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Change to project root directory
cd "$(dirname "$0")/../.."

# Set up test environment
print_status "Setting up test environment..."
./tests/integration/setup-tests.sh

print_status "Building rusty-beam server..."
cargo build --release

print_status "Starting server on $HOST:$PORT with test configuration..."
# Redirect server output to log files to keep test output clean
cargo run --release -- tests/config/test-config.html > tests/integration/server.log 2> tests/integration/server.error.log &
SERVER_PID=$!

# Wait for server to start
print_status "Waiting for server to be ready..."
for i in {1..30}; do
    # Check if server process is still running
    if ! kill -0 $SERVER_PID 2>/dev/null; then
        print_error "Server process died unexpectedly"
        if [ -f "tests/integration/server.error.log" ]; then
            print_error "Server error log:"
            cat tests/integration/server.error.log
        fi
        exit 1
    fi
    
    # Check if server is responding (accept any HTTP response, including auth errors)
    HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "http://127.0.0.1:$PORT/" -H "Host: $HOST" 2>/dev/null)
    # Debug: show what we got (only on last few attempts)
    if [ $i -gt 25 ]; then
        echo -n " [HTTP:$HTTP_CODE]"
    fi
    if [ ! -z "$HTTP_CODE" ] && [ "$HTTP_CODE" -ge 200 ] && [ "$HTTP_CODE" -lt 500 ]; then
        print_status "Server is ready! (HTTP $HTTP_CODE)"
        break
    fi
    
    if [ $i -eq 30 ]; then
        print_error "Server failed to start within 30 seconds"
        print_status "Server process is running but not responding"
        if [ -f "tests/integration/server.log" ]; then
            print_status "Server output (last 20 lines):"
            tail -20 tests/integration/server.log
        fi
        exit 1
    fi
    
    # Show progress dots
    echo -n "."
    sleep 1
done
echo  # New line after progress dots

print_status "Running Hurl tests..."
echo "=================================="

# Determine verbosity flags
if [ "$VERBOSE" = true ]; then
    HURL_VERBOSITY="--very-verbose"
else
    HURL_VERBOSITY=""
fi

# Change to integration test directory for running tests
cd tests/integration

# Run the main functionality tests
print_status "Running main functionality tests..."
if hurl tests.hurl --variable host=$HOST --variable port=$PORT --variable test_host=localhost --test --report-html test-report $HURL_VERBOSITY; then
    print_status "✓ Main functionality tests passed!"
else
    print_error "✗ Main functionality tests failed!"
    if [ "$VERBOSE" != true ]; then
        echo "=================================="
        print_status "Re-running failed tests with verbose output..."
        hurl tests.hurl --variable host=$HOST --variable port=$PORT --variable test_host=localhost --test --very-verbose
        echo "=================================="
    fi
    print_warning "Check the test report in test-report/ directory for details"
    exit 1
fi

echo "=================================="
print_status "🎉 All integration tests passed!"
echo
print_status "Test Summary:"
print_status "  ✓ Main functionality tests (79 tests)"
echo
print_status "Reports and logs:"
print_status "  📊 HTML test report: test-report/index.html"
if [ -f "server.log" ]; then
    print_status "  📋 Server log: server.log"
fi
if [ "$VERBOSE" != true ]; then
    echo
    print_status "💡 Tip: Use --verbose flag to see detailed test output"
fi