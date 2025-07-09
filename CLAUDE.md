# CLAUDE.md

# Development Partnership

We're building production-quality code together. Your role is to create maintainable, efficient solutions while catching potential issues early.

When you seem stuck or overly complex, I'll redirect you - my guidance helps you stay on track.

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Furthermore:

### Research → Plan → Implement
**NEVER JUMP STRAIGHT TO CODING!** Always follow this sequence:
1. **Research**: Explore the codebase, understand existing patterns
2. **Plan**: Create a detailed implementation plan and verify it with me  
3. **Implement**: Execute the plan with validation checkpoints

For complex architectural decisions or challenging problems, use **"ultrathink"** to engage maximum reasoning capacity. Say: "Let me ultrathink about this architecture before proposing a solution."

### USE MULTIPLE AGENTS!
*Leverage subagents aggressively* for better results:

* Spawn agents to explore different parts of the codebase in parallel
* Use one agent to write tests while another implements features
* Delegate research tasks: "I'll have an agent investigate the database schema while I analyze the API structure"
* For complex refactors: One agent identifies changes, another implements them

Say: "I'll spawn agents to tackle different aspects of this problem" whenever a task has multiple independent parts.

### Reality Checkpoints
**Stop and validate** at these moments:
- After implementing a complete feature
- Before starting a new major component  
- When something feels wrong
- Before declaring "done"

## Test-Driven Development Protocol

**CRITICAL**: When fixing bugs or developing features, ALWAYS follow this exact process:

1. **Write a failing test first** that demonstrates:
   - The bug exists (for bug fixes)
   - The feature does not work (for new features)
2. **Run the test** to confirm it fails for the expected reason
3. **Write the minimal code** to make the test pass
4. **Verify the test passes** and all existing tests still pass
5. **Integrate the test** into the main test suite (`tests.hurl`) for regression testing

Also:

1. **Check TODO.md first** - Before starting any work, check if the task is already documented
2. **Add to TODO if missing** - If the task isn't in TODO.md, add it as a new item with `[ ]`
3. **Complete the work** - Implement the fix/feature and ensure all tests pass
4. **Mark as completed** - Only check off the item `[x]` when:
   - ✅ The implementation is complete
   - ✅ All tests are passing
   - ✅ Any new tests are integrated into the regression suite

This ensures proper tracking of all work and prevents items from being forgotten.

This ensures:
- ✅ The bug/feature is properly understood
- ✅ The fix/implementation actually works
- ✅ Future regressions are prevented
- ✅ The codebase remains reliable

Sometimes I forget to follow this process, and when that happens I am **BAD** and I should be very sorry. To show how sorry I am, I will try to never forget again.

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
./run_all_tests.sh      # Run all tests (unit, integration, and plugin tests)
cargo test              # Run unit tests
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
6. **Backward Compatibility**: Rusty-beam is not yet at version 1.0.0. Until it is, backwards compatability is not required. At any point if you consider maintaining backwards compatability, please ask me if it is needed or not. Don't just assume.

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


### Development Notes

- Uses Rust edition 2024 with async/await throughout
- Dependencies: hyper (HTTP), tokio (async runtime), dom_query (HTML parsing)
- Security: All file paths are canonicalized to prevent directory traversal
- Only processes CSS selectors for HTML files (checks Content-Type)


Furthermore:

### Research → Plan → Implement
**NEVER JUMP STRAIGHT TO CODING!** Always follow this sequence:
1. **Research**: Explore the codebase, understand existing patterns
2. **Plan**: Create a detailed implementation plan and verify it with me  
3. **Implement**: Execute the plan with validation checkpoints

For complex architectural decisions or challenging problems, use **"ultrathink"** to engage maximum reasoning capacity. Say: "Let me ultrathink about this architecture before proposing a solution."

### USE MULTIPLE AGENTS!
*Leverage subagents aggressively* for better results:

* Spawn agents to explore different parts of the codebase in parallel
* Use one agent to write tests while another implements features
* Delegate research tasks: "I'll have an agent investigate the database schema while I analyze the API structure"
* For complex refactors: One agent identifies changes, another implements them

Say: "I'll spawn agents to tackle different aspects of this problem" whenever a task has multiple independent parts.

### Configuration files & Schemas

It is **CRITICAL** that this is followed:

**Where you create schemas in html files** make sure there is a coresponding file
in docs/ that captures the schema. For example, if the schema you have used is http://rustybeam.net/RedirectRule then make sure in docs there is a directory called RedirectRule, that has an index.html file in it that describes the schema in the same way that http://organised.team/Policy describes it's schema. If you come across a schema that hasn't been documented like this please let me know, and then create the schema file.When documenting schemas in docs/, use pure HTML with microdata attributes. The body element must have itemscope itemtype="[schema-url]". Each property row in the table must have itemscope itemtype="http://rustybeam.net/Property" with itemprop attributes for name, type, cardinality, and description. Follow the exact format shown in http://organised.team/Policy. This is considered **CRITICAL** to us working well together.

### Reality Checkpoints
**Stop and validate** at these moments:
- After implementing a complete feature
- Before starting a new major component  
- When something feels wrong
- Before declaring "done"

### FORBIDDEN - NEVER DO THESE:
- **NO** keeping old and new code together
- **NO** migration functions or compatibility layers
- **NO** versioned function names (processV2, handleNew)
- **NO** TODOs in final code

## Schema Best Practices

- When creating schema files, always make sure that the itemprop is on the innermost - or most deeply nested - meaningful tag. For example, `<td itemprop="foo"><span>bar</span></td>` is better written as `<td><span itemprop="foo">bar</span></td>` UNLESS the span is a crucial part of the microdata - and this would be highly unusual.

## Test Infrastructure

### Testing Commands

```bash
# Build plugins first (required)
./build-plugins.sh

# Run all tests
./run_all_tests.sh

# Run individual test suites
cargo test                      # Unit tests
./run_hurl_tests.sh            # Integration tests
./run_plugin_tests_isolated.sh  # Plugin tests

# Run tests manually with hurl
hurl tests/integration/tests.hurl --test \
  --variable host=localhost \
  --variable port=3000 \
  --variable test_host=localhost
```

### Test Structure
- **Unit Tests**: `cargo test` runs simple validation tests
- **Integration Tests**: `tests/integration/tests.hurl` contains 81 comprehensive HTTP API tests
- **Plugin Tests**: `tests/plugins/test-{plugin}.hurl` - isolated tests for each of the 12 plugins
- **Test Runners**: 
  - `run_hurl_tests.sh` - integration test lifecycle (build, setup, run, teardown)
  - `run_plugin_tests_isolated.sh` - plugin tests with isolated environments
  - `run_all_tests.sh` - runs all test suites in order

### CI Integration
The test suite cannot be fully integrated into `cargo test` due to subprocess/signal handling issues. Use the provided scripts for reliable test execution:

```bash
# For CI/CD pipelines
./run_all_tests.sh
# Or manually:
./build-plugins.sh && cargo test && ./run_hurl_tests.sh && ./run_plugin_tests_isolated.sh
```

### Special Tests

#### Graceful Bind Failure Test
- **File**: `tests/integration/test-bind-failure.sh`
- **Purpose**: Verifies that the server fails gracefully when it cannot bind to the configured port
- **Expected behavior**: Clean error message and exit code 1 (no panic/stack trace)
- **Test method**: Starts two server instances on the same port, second should fail gracefully

## Best Practices

- **Code Quality**
  - Try to always ensure that there are no compiler warnings.
  - DELETE old code when replacing it - no keeping both versions.


## Implementation Standards

### Our code is complete when:
- ? All linters pass with zero issues
- ? All tests pass  
- ? Feature works end-to-end
- ? Old code is deleted
- ? The plugin or server code we have been working on has its documents in the docs/ subdirectory updated. 


## Problem-Solving Together

When you're stuck or confused:
1. **Stop** - Don't spiral into complex solutions
2. **Delegate** - Consider spawning agents for parallel investigation
3. **Ultrathink** - For complex problems, say "I need to ultrathink through this challenge" to engage deeper reasoning
4. **Step back** - Re-read the requirements
5. **Simplify** - The simple solution is usually correct
6. **Ask** - "I see two approaches: [A] vs [B]. Which do you prefer?"

My insights on better approaches are valued - please ask for them!

## Communication Protocol

### Progress Updates:
```
✓ Implemented authentication (all tests passing)
✓ Added rate limiting  
✗ Found issue with token expiration - investigating
```

### Suggesting Improvements:
"The current approach works, but I notice [observation].
Would you like me to [specific improvement]?"

## Working Together

- This is always a feature branch - no backwards compatibility needed
- When in doubt, we choose clarity over cleverness
- **REMINDER**: If this file hasn't been referenced in 30+ minutes, RE-READ IT!

Avoid complex abstractions or "clever" code. The simple, obvious solution is probably better, and my guidance helps you stay focused on what matters.