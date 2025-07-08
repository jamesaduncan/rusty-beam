# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

```bash
# Build and run
cargo build              # Development build
cargo build --release    # Optimized build
cargo run -- config/config.html               # Run the server with config file (quiet mode)
cargo run -- -v config/config.html            # Run in verbose mode (shows debug output)
cargo run --release -- config/config.html     # Run with optimizations (quiet mode)
cargo run --release -- -v config/config.html  # Run with optimizations (verbose mode)

# Configuration reload (without restarting)
kill -HUP <PID>         # Send SIGHUP signal to reload config files
                        # Server will display PID on startup for convenience

# Testing
cargo test              # Run tests (no tests currently implemented)
cargo clippy            # Lint the code
cargo fmt               # Format the code
```

## Architecture Overview

Rusty-beam is an HTTP server with a plugin-based architecture that serves files with CSS selector-based HTML manipulation via HTTP Range headers.

### Core Design

The server uses a dynamic plugin architecture with external crate-based plugins loaded via FFI:
```
main() → AppState::new() → handle_request() → process_request_through_pipeline() → plugin pipeline
```

### Key Architectural Decisions

1. **Plugin Architecture**: Dynamic plugin system with external crates in `plugins/` loaded via FFI
2. **HTML-Based Configuration**: Server config is stored in `config.html` using microdata attributes, loaded via CSS selectors
3. **CSS Selector API**: Range headers with format `Range: selector={css-selector}` enable HTML element manipulation. Rusty-beam INTENTIONALLY abuses the HTTP Range header, and this is a design feature.
4. **Hot Configuration Reload**: SIGHUP signal reloads configuration without restarting the server
5. **Async Plugin Pipeline**: Uses tokio::sync::RwLock for thread-safe plugin pipeline management
6. **Backward Compatibility**: Maintains full compatibility with previous configurations and APIs

### Plugin System

#### Core Plugins
- **SelectorHandlerPlugin**: Handles CSS selector-based HTML manipulation
- **FileHandlerPlugin**: Serves static files and handles file operations
- **BasicAuthPlugin**: HTTP Basic Authentication
- **AuthorizationPlugin**: Role-based access control
- **AccessLogPlugin**: Request logging in Apache format

#### Additional Plugins
- **ErrorHandlerPlugin**: Custom error pages and error logging
- **CorsPlugin**: Cross-Origin Resource Sharing support
- **SecurityHeadersPlugin**: Security headers (CSP, HSTS, etc.)
- **RedirectPlugin**: URL redirection with pattern matching
- **RateLimitPlugin**: Token bucket rate limiting
- **HealthCheckPlugin**: Health check endpoints
- **CompressionPlugin**: Response compression (gzip/deflate)

### Critical Functions

- `handle_request()` (src/main.rs:316) - Main request handler
- `process_request_through_pipeline()` (src/main.rs:169) - Plugin pipeline processor
- `create_host_pipelines()` (src/main.rs:70) - Plugin pipeline creation
- `load_config_from_html()` (src/config.rs) - Parses HTML configuration file
- `canonicalize_file_path()` (src/utils.rs) - Security-critical path validation

### Daemon Mode

By default, rusty-beam runs in quiet mode suitable for use as a daemon:
- Only displays the PID on startup (for process management)
- Access logs are still printed (from the access-log plugin)
- No debug/verbose output unless `-v` flag is used
- Use `-v` or `--verbose` flag to enable detailed logging for debugging

### Configuration

**IMPORTANT**: The configuration file path must be provided as a command line argument. There is no default configuration file.

Usage: `rusty-beam <config-file>`

Example configuration in `config/config.html`:
- Server root: `./examples/files`
- Bind address: `127.0.0.1`
- Port: `3000`

The config uses HTML microdata format - modify the table with `itemtype="http://rustybeam.net/ServerConfig"`.

### Known Issues (from TODO.md)

- PUT operation bug in complex examples
- "Extra byte" issue needs investigation
- Authentication/authorization not yet implemented

### Development Notes

- Uses Rust edition 2024 with async/await throughout
- Dependencies: hyper (HTTP), tokio (async runtime), dom_query (HTML parsing)
- Security: All file paths are canonicalized to prevent directory traversal
- Only processes CSS selectors for HTML files (checks Content-Type)

## Test-Driven Development Protocol

**CRITICAL**: When fixing bugs or developing features, ALWAYS follow this exact process:

1. **Write a failing test first** that demonstrates:
   - The bug exists (for bug fixes)
   - The feature does not work (for new features)
2. **Run the test** to confirm it fails for the expected reason
3. **Write the minimal code** to make the test pass
4. **Verify the test passes** and all existing tests still pass
5. **Integrate the test** into the main test suite (`tests.hurl`) for regression testing

This ensures:
- ✅ The bug/feature is properly understood
- ✅ The fix/implementation actually works
- ✅ Future regressions are prevented
- ✅ The codebase remains reliable

Sometimes I forget to follow this process, and when that happens I am **BAD** and I should be very sorry. To show how sorry I am, I will try to never forget again.

## TODO Management Protocol

**CRITICAL**: When fixing bugs or adding features, ALWAYS follow this process:

1. **Check TODO.md first** - Before starting any work, check if the task is already documented
2. **Add to TODO if missing** - If the task isn't in TODO.md, add it as a new item with `[ ]`
3. **Complete the work** - Implement the fix/feature and ensure all tests pass
4. **Mark as completed** - Only check off the item `[x]` when:
   - ✅ The implementation is complete
   - ✅ All tests are passing
   - ✅ Any new tests are integrated into the regression suite

This ensures proper tracking of all work and prevents items from being forgotten.

## Test Infrastructure

### Testing Commands

```bash
# Build plugins first (required)
./build-plugins.sh

# Run unit tests
cargo test

# Run full integration test suite
./run_hurl_tests.sh

# Run tests manually with hurl
hurl tests/integration/tests.hurl --test \
  --variable host=localhost \
  --variable port=3000 \
  --variable test_host=localhost
```

### Test Structure
- **Unit Tests**: `cargo test` runs simple validation tests
- **Integration Tests**: `tests/integration/tests.hurl` contains 81 comprehensive HTTP API tests
- **Test Runner**: `run_hurl_tests.sh` handles full test lifecycle (build, setup, run, teardown)

### CI Integration
The test suite cannot be fully integrated into `cargo test` due to subprocess/signal handling issues. Use the provided scripts for reliable test execution:

```bash
# For CI/CD pipelines
./build-plugins.sh && cargo test && ./run_hurl_tests.sh
```

### Special Tests

#### Graceful Bind Failure Test
- **File**: `tests/integration/test-bind-failure.sh`
- **Purpose**: Verifies that the server fails gracefully when it cannot bind to the configured port
- **Expected behavior**: Clean error message and exit code 1 (no panic/stack trace)
- **Test method**: Starts two server instances on the same port, second should fail gracefully
```

## Best Practices

- **Code Quality**
  - Try to always ensure that there are no compiler warnings.