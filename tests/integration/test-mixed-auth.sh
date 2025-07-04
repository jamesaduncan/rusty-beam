#!/bin/bash

# Test runner for mixed encryption authentication
# This script tests the mixed plaintext/bcrypt password functionality

set -e

# Default configuration
HOST="127.0.0.1"
PORT="3000"
SERVER_PID=""

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
    
    # Restore original config
    if [ -f "config.html.backup" ]; then
        print_status "Restoring original configuration"
        mv config.html.backup config.html
    fi
}

# Set up cleanup on exit
trap cleanup EXIT

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
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo "Options:"
            echo "  --host HOST    Server host (default: 127.0.0.1)"
            echo "  --port PORT    Server port (default: 3000)"
            echo "  --help         Show this help message"
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Check if required tools are available
if ! command -v hurl &> /dev/null; then
    print_error "Hurl is not installed. Please install it first."
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    print_error "Cargo is not installed. Please install Rust first."
    exit 1
fi

print_status "Setting up mixed encryption authentication test..."

# Backup original config
cp config.html config.html.backup

# Update config to use mixed auth file
sed -i 's|<td itemref="localhost-plugin-basic-auth" itemprop="authFile" contenteditable="plaintext-only">./localhost/auth/users.html</td>|<td itemref="localhost-plugin-basic-auth" itemprop="authFile" contenteditable="plaintext-only">./localhost/auth/users_mixed.html</td>|' config.html

print_status "Building rusty-beam server..."
cargo build --release

print_status "Starting server with mixed authentication on $HOST:$PORT..."
cargo run --release -- config/config.html &
SERVER_PID=$!

# Wait for server to start
print_status "Waiting for server to be ready..."
for i in {1..30}; do
    if curl -s -f -u admin:admin123 "http://$HOST:$PORT/index.html" > /dev/null 2>&1; then
        print_status "Server is ready!"
        break
    fi
    if [ $i -eq 30 ]; then
        print_error "Server failed to start within 30 seconds"
        exit 1
    fi
    sleep 1
done

print_status "Running mixed encryption authentication tests..."
echo "=================================="

# Run the mixed encryption tests
if hurl tests_auth_mixed.hurl --variable host=$HOST --variable port=$PORT --test --report-html test-report-mixed; then
    print_status "All mixed encryption tests passed!"
    echo "=================================="
    print_status "Test report generated in test-report-mixed/ directory"
else
    print_error "Some mixed encryption tests failed!"
    echo "=================================="
    print_warning "Check the test report in test-report-mixed/ directory for details"
    exit 1
fi