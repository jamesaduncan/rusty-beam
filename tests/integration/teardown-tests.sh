#!/bin/bash
# Test teardown script for Rusty Beam
# Cleans up all test artifacts created during test execution

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
TEST_FILES_DIR="$PROJECT_ROOT/tests/files"

# Allow quiet mode
QUIET=false
if [ "$1" = "-q" ] || [ "$1" = "--quiet" ]; then
    QUIET=true
fi

if [ "$QUIET" = false ]; then
    echo "Cleaning up test artifacts..."
fi

# List of test files that may be created during tests
TEST_ARTIFACTS=(
    "test-*.txt"
    "test.html"
    "test.css"
    "test.js"
    "test.json"
    "complex.html"
    "table-test.html"
    "post-created.txt"
    "put-status-test.txt"
    "README.md"
    "index.html"
    "body-tag-test.html"
    "post-list-test.html"
)

# Remove test artifacts from all test directories
for artifact in "${TEST_ARTIFACTS[@]}"; do
    find "$TEST_FILES_DIR" -name "$artifact" -delete 2>/dev/null || true
done

# Clean up any empty directories created during tests
find "$TEST_FILES_DIR" -type d -empty -delete 2>/dev/null || true

# Clean up test report directory if it exists
if [ -d "$SCRIPT_DIR/test-report" ]; then
    rm -rf "$SCRIPT_DIR/test-report"
fi

# Clean up server logs unless there were errors
if [ -f "$SCRIPT_DIR/server.error.log" ] && [ ! -s "$SCRIPT_DIR/server.error.log" ]; then
    rm -f "$SCRIPT_DIR/server.log" "$SCRIPT_DIR/server.error.log"
fi

if [ "$QUIET" = false ]; then
    echo "Test teardown complete"
fi