# Rusty-beam Test Suite

This directory contains the test suite for rusty-beam, including both unit tests and integration tests.

## Test Structure

```
tests/
├── README.md                    # This file
├── integration/                 # Integration tests using Hurl
│   ├── run-tests.sh            # Test runner script  
│   ├── tests.hurl              # Main HTTP API tests
│   ├── tests_auth.hurl         # Authentication tests
│   ├── test-authorization.hurl # Authorization tests
│   └── ...                     # Other integration tests
└── unit/                       # Unit test fixtures (if any)
```

## Running Tests

### Unit Tests

Unit tests are integrated with `cargo test` and run automatically:

```bash
cargo test
```

This will run:
- **Plugin system tests** (13 tests)
- **Auth module tests** (4 tests)  
- **Auth integration tests** (5 tests)
- **Integration test infrastructure** (8 tests)

### Integration Tests (Full HTTP API)

Integration tests use [Hurl](https://hurl.dev/) to test the full HTTP API against a running server.

#### Prerequisites

1. **Install Hurl**:
   ```bash
   # Using curl
   curl --location --remote-name https://github.com/Orange-OpenSource/hurl/releases/latest/download/hurl-install.sh
   chmod +x hurl-install.sh
   ./hurl-install.sh
   
   # Or using package manager
   # Ubuntu/Debian: apt install hurl
   # macOS: brew install hurl
   ```

2. **Build plugins**:
   ```bash
   ./build-plugins.sh
   ```

#### Running Integration Tests

**Option 1: Automated test runner (Recommended)**
```bash
# Clean, quiet output (recommended for CI/development)
./tests/integration/run-tests.sh

# Verbose output (for debugging test failures)
./tests/integration/run-tests.sh --verbose

# Custom host/port
./tests/integration/run-tests.sh --host localhost --port 8080
```

**Option 2: Manual execution**
```bash
# Start server in background
cargo run --release &
SERVER_PID=$!

# Wait for server to start
sleep 3

# Run specific test files
hurl --test tests/integration/tests.hurl --variable host=127.0.0.1 --variable port=3000
hurl --test tests/integration/tests_auth.hurl --variable host=127.0.0.1 --variable port=3000

# Stop server
kill $SERVER_PID
```

#### Test Output

The integration test runner provides clean, structured output:

- ✅ **Clean Mode (default)**: Shows only test progress and results
- 🔍 **Verbose Mode (`--verbose`)**: Shows detailed HTTP request/response data  
- 📊 **HTML Reports**: Generates detailed test reports in `test-report/`
- 📋 **Server Logs**: Automatically managed, shown only when errors occur

#### Integration Test Files

- **`tests.hurl`**: Main HTTP API functionality
  - Basic HTTP operations (GET, PUT, POST, DELETE, OPTIONS)
  - CSS selector operations
  - File upload/download
  - Host-based routing
  - Error handling

- **`tests_auth.hurl`**: Authentication functionality
  - Basic HTTP authentication
  - Plugin-based authentication
  - Authentication error handling

- **`test-authorization.hurl`**: Authorization functionality
  - Role-based access control
  - Resource-specific permissions
  - Authorization error handling

## Test Development

### Adding Unit Tests

Add unit tests directly in the source files using `#[cfg(test)]` modules:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_new_functionality() {
        // Test implementation
    }
}
```

### Adding Integration Tests

1. **Create new `.hurl` file** in `tests/integration/`
2. **Follow Hurl syntax**:
   ```hurl
   # Test description
   GET http://{{host}}:{{port}}/endpoint
   HTTP 200
   [Asserts]
   header "Content-Type" == "application/json"
   body contains "expected content"
   ```

3. **Add to test runner** in `run-tests.sh`

### Test Infrastructure Validation

The `cargo test` command includes infrastructure validation tests that check:
- ✅ Hurl test files exist and have valid syntax
- ✅ Test directory structure is correct
- ✅ Hurl is available in the system
- ✅ Integration test runner script exists
- ✅ Server binary can be built
- ✅ Test documentation is available

## Continuous Integration

For CI environments, use the infrastructure validation included in `cargo test`:

```bash
# Full test suite including infrastructure validation
cargo test

# Build verification (part of cargo test)
cargo build --release

# Code quality checks
cargo clippy -- -D warnings
```

To run full integration tests in CI, ensure Hurl is installed and run:

```bash
./tests/integration/run-tests.sh
```

## Troubleshooting

### Common Issues

1. **Server fails to start**:
   - Check that port 3000 is available
   - Verify plugins are built: `./build-plugins.sh`
   - Check configuration files exist

2. **Hurl tests fail**:
   - Ensure Hurl is installed: `hurl --version`
   - Verify server is running: `curl http://127.0.0.1:3000/`
   - Check test file syntax: `hurl --dry-run test-file.hurl`

3. **Authentication tests fail**:
   - Verify auth configuration files exist in `examples/localhost/auth/`
   - Check that authentication plugins are loaded
   - Verify credentials in test files match configuration

### Debug Mode

Run integration tests with verbose output:

```bash
hurl --test --very-verbose tests/integration/tests.hurl --variable host=127.0.0.1 --variable port=3000
```

## Test Coverage

The test suite covers:

- ✅ **HTTP API**: All standard HTTP methods with and without CSS selectors
- ✅ **Authentication**: Plugin-based authentication system
- ✅ **Authorization**: Role-based access control and permissions  
- ✅ **Configuration**: Host-specific configuration loading
- ✅ **Plugin System**: Dynamic plugin loading and management
- ✅ **Error Handling**: Graceful error responses and edge cases
- ✅ **Content Types**: Multiple file types and content handling
- ✅ **Security**: Path traversal prevention and input validation