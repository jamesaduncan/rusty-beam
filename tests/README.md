# Rusty-beam Test Suite

This directory contains the test suite for rusty-beam, including both unit tests and integration tests.

## Test Structure

```
tests/
├── README.md                    # This file
├── MIGRATION_NOTES.md          # Notes on test consolidation
├── integration/                 # Integration tests using Hurl
│   ├── tests.hurl              # Main HTTP API tests (81 tests)
│   ├── tests_auth.hurl         # Authentication tests
│   ├── test-*.hurl             # Other specific test files
│   ├── run-tests.sh            # Main test runner
│   ├── setup-tests.sh          # Test environment setup script
│   └── teardown-tests.sh       # Test cleanup script
├── config/                      # Test configurations
│   ├── test-config.html        # Main test server configuration
│   └── test-auth-config.html   # Authentication test configuration
├── files/                       # Test file assets
│   ├── localhost/              # Files for localhost host
│   └── example-com/            # Files for example.com host
├── scripts/                     # Additional test scripts
│   ├── test-debug-auth.sh      # Debug authentication tests
│   └── test-root-auth.sh       # Root authentication tests
├── simple_test.rs              # Basic smoke tests
└── run_integration_tests.sh    # Alternative test runner
```

Note: Unit tests follow Rust conventions and remain in their source files using `#[cfg(test)]` modules.

## Running Tests

### Quick Start

```bash
# 1. Build plugins
./build-plugins.sh

# 2. Run unit tests
cargo test

# 3. Run full integration test suite
./run_hurl_tests.sh
```

### Unit Tests

Unit tests verify core functionality, including:
- Integration test environment setup
- Microdata extraction functionality
- Plugin builds and configuration

```bash
# Run all unit tests
cargo test

# Run specific test modules
cargo test unit::integration_sanity_tests
cargo test unit::microdata_extract
```

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

**Recommended: Use the test runner script**
```bash
./run_hurl_tests.sh
```

This script handles:
- Building plugins
- Building server
- Setting up test environment
- Starting server
- Running all tests
- Cleaning up afterwards

**Manual execution (if needed)**
```bash
# Start server
./target/release/rusty-beam tests/config/test-config.html &
SERVER_PID=$!

# Run tests
hurl --test tests/integration/tests.hurl \
     --variable host=localhost \
     --variable port=3000 \
     --variable test_host=localhost

# Stop server
kill $SERVER_PID
```

#### Test Output

The integration test runner provides clean, structured output:

- ✅ **Clean Mode (default)**: Shows only test progress and results
- 🔍 **Verbose Mode (`--verbose`)**: Shows detailed HTTP request/response data  
- 📊 **HTML Reports**: Generates detailed test reports in `test-report/`
- 📋 **Server Logs**: Automatically managed, shown only when errors occur

#### Integration Test Coverage

**`tests.hurl`** (81 tests) covers:
- Basic HTTP operations (GET, HEAD, PUT, POST, DELETE)
- CSS selector-based HTML manipulation
- File uploads and content creation
- Host-based routing (localhost vs example.com)
- Content-Type handling (HTML, CSS, JS, JSON)
- Error handling (404s, empty selectors)
- URL-encoded selectors
- Complex HTML structures (tables, nested elements)

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

For CI environments:

```bash
# 1. Install dependencies (if needed)
curl -sL https://github.com/Orange-OpenSource/hurl/releases/latest/download/hurl-installer.sh | sh

# 2. Build and test
./build-plugins.sh
cargo test
./run_hurl_tests.sh

# 3. Code quality checks
cargo clippy
cargo fmt -- --check
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