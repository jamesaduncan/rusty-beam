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