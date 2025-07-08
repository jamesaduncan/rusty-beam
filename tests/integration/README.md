# Rusty Beam Integration Tests

This directory contains integration tests for the Rusty Beam server.

## Test Structure

- `tests.hurl` - Main test file containing all HTTP tests
- `setup-tests.sh` - Sets up the test environment before running tests
- `teardown-tests.sh` - Cleans up test artifacts after tests complete
- `run-tests.sh` - Main test runner that orchestrates the test execution
- `run-isolated-tests.sh` - Runs tests with guaranteed setup/teardown

## Running Tests

### Basic Test Run
```bash
./tests/integration/run-tests.sh
```

### Verbose Output
```bash
./tests/integration/run-tests.sh --verbose
```

### Isolated Test Run (Recommended)
```bash
./tests/integration/run-isolated-tests.sh
```

## Test Cleanup

The test suite includes automatic cleanup functionality:

1. **Setup Phase** (`setup-tests.sh`):
   - Creates necessary test directories
   - Copies required test files
   - Removes any artifacts from previous test runs

2. **Teardown Phase** (`teardown-tests.sh`):
   - Removes all files created during tests
   - Cleans up test reports
   - Removes server logs (unless there were errors)

### Manual Cleanup
If needed, you can manually run the cleanup:
```bash
./tests/integration/teardown-tests.sh
```

## Known Issues

### Test Interdependencies
Some tests create files that affect later tests in the same run. For example:
- Test at line 121 creates `test.html`
- Test at line 663 expects to create `test.html` but it already exists

This happens because the tests run in sequence and share the same file system.

### Host-Specific Root Directories
The current plugin architecture uses a single root directory per plugin instance.
Tests that expect different hosts to have separate root directories may fail.

To properly support host-specific directories, plugins would need to:
1. Get the host configuration from the context
2. Use the host's root directory instead of the plugin's configured root

## Test Files Created

During test execution, the following files may be created:
- `test.html`
- `test-*.txt`
- `test.css`
- `test.js`
- `test.json`
- `complex.html`
- `table-test.html`
- `post-created.txt`
- `put-status-test.txt`
- `README.md`

All these files are automatically cleaned up by the teardown script.

## Future Improvements

1. **Test Isolation**: Each test could run in its own temporary directory
2. **Host-Aware Plugins**: Update plugin architecture to use host-specific configurations
3. **Test Parallelization**: Run independent tests in parallel for faster execution
4. **Better Error Reporting**: Capture and report specific test failures with context