#!/bin/bash
# Isolated test runner that ensures clean state between test runs
# This prevents test pollution from affecting results

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# Run setup before tests
print_status "Running test setup..."
"$SCRIPT_DIR/setup-tests.sh"

# Run the tests
print_status "Running tests..."
"$SCRIPT_DIR/run-tests.sh" "$@"
TEST_RESULT=$?

# Teardown is now called automatically by run-tests.sh via trap
# But we'll also ensure cleanup here for safety
if [ -f "$SCRIPT_DIR/teardown-tests.sh" ]; then
    "$SCRIPT_DIR/teardown-tests.sh" 2>/dev/null || true
fi

exit $TEST_RESULT