#!/bin/bash
# Test setup and cleanup script for Rusty Beam

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
TEST_FILES_DIR="$PROJECT_ROOT/tests/files"
TEST_CONFIG="$PROJECT_ROOT/tests/config/test-config.html"

echo "Setting up test environment..."

# Create test directories
mkdir -p "$TEST_FILES_DIR/localhost"
mkdir -p "$TEST_FILES_DIR/example-com"

# Ensure test files exist with correct content
# Always recreate index.html to ensure it has the expected content
cat > "$TEST_FILES_DIR/localhost/index.html" << 'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <title>Title</title>
    <script type="module" src="https://jamesaduncan.github.io/dom-aware-primitives/index.mjs"></script>
</head>
<body>
    <h1>Example Page</h1>
    <ul>
        <li>One</li>
        <li>Two</li>
        <li>Three</li>
    </ul>
</body>
</html>
EOF

if [ ! -f "$TEST_FILES_DIR/localhost/foo.html" ]; then
    cp "$PROJECT_ROOT/examples/files/foo.html" "$TEST_FILES_DIR/localhost/"
fi

# Clean up any test artifacts from previous runs
echo "Cleaning up previous test artifacts..."
find "$TEST_FILES_DIR" -name "test-*.txt" -delete 2>/dev/null || true
find "$TEST_FILES_DIR" -name "test.html" -delete 2>/dev/null || true
find "$TEST_FILES_DIR" -name "test.css" -delete 2>/dev/null || true
find "$TEST_FILES_DIR" -name "test.js" -delete 2>/dev/null || true
find "$TEST_FILES_DIR" -name "test.json" -delete 2>/dev/null || true
find "$TEST_FILES_DIR" -name "complex.html" -delete 2>/dev/null || true
find "$TEST_FILES_DIR" -name "table-test.html" -delete 2>/dev/null || true
find "$TEST_FILES_DIR" -name "post-created.txt" -delete 2>/dev/null || true
find "$TEST_FILES_DIR" -name "put-status-test.txt" -delete 2>/dev/null || true

echo "Test environment setup complete"
echo "Test files directory: $TEST_FILES_DIR"
echo "Test config file: $TEST_CONFIG"