# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

```bash
# Build and run
cargo build              # Development build
cargo build --release    # Optimized build
cargo run               # Run the server (defaults to http://127.0.0.1:3000)
cargo run --release     # Run with optimizations

# Testing
cargo test              # Run tests (no tests currently implemented)
cargo clippy            # Lint the code
cargo fmt               # Format the code
```

## Architecture Overview

Rusty-beam is an experimental HTTP server that serves files with CSS selector-based HTML manipulation via HTTP Range headers.

### Core Design

The entire server logic resides in `src/main.rs` (550 lines) and follows this request flow:
```
main() → Server::bind() → handle_request() → method-specific handlers
```

### Key Architectural Decisions

1. **HTML-Based Configuration**: Server config is stored in `config.html` using microdata attributes, loaded via CSS selectors
2. **CSS Selector API**: Range headers with format `Range: selector={css-selector}` enable HTML element manipulation
3. **Single File Architecture**: All logic in main.rs - straightforward to navigate but consider splitting if growing
4. **Global Config**: Uses `LazyLock<ServerConfig>` for configuration management

### Critical Functions

- `handle_request()` (src/main.rs:103) - Main request router
- `handle_get_with_selector()` (src/main.rs:277) - CSS selector GET operations
- `handle_put_with_selector()` (src/main.rs:306) - CSS selector PUT operations
- `load_config_from_html()` (src/main.rs:57) - Parses HTML configuration file
- `canonicalize_file_path()` (src/main.rs:458) - Security-critical path validation

### Configuration

Default configuration in `config.html`:
- Server root: `./files`
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

### Testing Commands

```bash
# Run main test suite
./run-tests.sh

# Run specific test file
hurl test-name.hurl --test

# Run tests with verbose output
hurl test-name.hurl --test --verbose

# Test graceful bind failure (server startup edge case)
./test-bind-failure.sh
```

### Special Tests

#### Graceful Bind Failure Test
- **File**: `test-bind-failure.sh`
- **Purpose**: Verifies that the server fails gracefully when it cannot bind to the configured port
- **Expected behavior**: Clean error message and exit code 1 (no panic/stack trace)
- **Test method**: Starts two server instances on the same port, second should fail gracefully