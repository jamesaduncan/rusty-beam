# Directory Structure

This document describes the organization of the Rusty-beam project directory.

## 📁 Project Layout

```
rusty-beam/
├── 📁 .github/              # GitHub-specific files
│   └── workflows/           # GitHub Actions workflows
├── 📁 build/                # Build and packaging
│   ├── 📁 debian/           # Debian packaging scripts
│   ├── 📁 docker/           # Docker files
│   ├── 📁 homebrew/         # Homebrew formula
│   └── 📁 scripts/          # Build scripts
├── 📁 config/               # Configuration files
│   ├── 📁 examples/         # Example configurations
│   └── config.html          # Default server configuration
├── 📁 docs/                 # All documentation
│   ├── 📁 api/              # API documentation
│   ├── 📁 auth/             # Authentication & authorization docs
│   ├── 📁 development/      # Development documentation
│   ├── 📁 examples/         # Usage examples
│   ├── 📁 guides/           # User guides
│   └── 📁 plugins/          # Plugin documentation
├── 📁 examples/             # Example sites and content
│   ├── 📁 localhost/        # localhost example host
│   ├── 📁 example-com/      # example.com example host
│   └── 📁 files/            # Default file root
├── 📁 plugins/              # Plugin source and binaries
│   ├── 📁 basic-auth/       # Basic auth plugin source
│   ├── 📁 google-oauth2/    # Google OAuth2 plugin source
│   └── 📁 lib/              # Compiled plugin libraries
├── 📁 src/                  # Rust source code
│   ├── 📁 plugins/          # Plugin management code
│   └── *.rs                 # Core server source files
├── 📁 target/               # Cargo build artifacts (gitignored)
├── 📁 tests/                # All testing files
│   ├── 📁 integration/      # Integration tests (.hurl files, scripts)
│   ├── 📁 unit/             # Unit tests
│   └── 📁 fixtures/         # Test fixtures and data
├── 📄 Cargo.toml           # Main Cargo manifest
├── 📄 Cargo.lock           # Cargo lock file
├── 📄 CLAUDE.md            # Claude Code development instructions
├── 📄 LICENSE              # MIT license file
└── 📄 README.md            # Main project README
```

## 📋 Directory Purpose

### Build & Packaging (`build/`)
- **`debian/`** - Debian package maintainer scripts
- **`docker/`** - Docker build files and configurations
- **`homebrew/`** - Homebrew formula for macOS installation
- **`scripts/`** - Build automation scripts

### Configuration (`config/`)
- **`config.html`** - Main server configuration file
- **`examples/`** - Example configuration files for different scenarios

### Documentation (`docs/`)
- **`api/`** - API reference documentation
- **`auth/`** - Authentication and authorization guides
- **`development/`** - Developer documentation
- **`examples/`** - Usage examples and tutorials
- **`guides/`** - User guides and how-to documentation
- **`plugins/`** - Plugin development documentation

### Examples (`examples/`)
- **`localhost/`** - Example localhost host with authentication
- **`example-com/`** - Example external host configuration  
- **`files/`** - Default file serving root

### Plugins (`plugins/`)
- **`basic-auth/`** - HTTP Basic Authentication plugin source
- **`google-oauth2/`** - Google OAuth2 authentication plugin source
- **`lib/`** - Compiled plugin libraries (.so, .dylib, .dll)

### Source Code (`src/`)
- **`main.rs`** - Main server entry point
- **`config.rs`** - Configuration loading and parsing
- **`auth.rs`** - Authorization engine
- **`handlers.rs`** - HTTP request handlers
- **`utils.rs`** - Utility functions
- **`plugins/`** - Plugin management and FFI interface

### Testing (`tests/`)
- **`integration/`** - Integration tests using Hurl and shell scripts
- **`unit/`** - Unit tests for individual components
- **`fixtures/`** - Test data and fixtures

## 🔧 Key Files

| File | Purpose |
|------|---------|
| `Cargo.toml` | Rust project configuration and dependencies |
| `config/config.html` | Main server configuration |
| `src/main.rs` | Server entry point |
| `build/scripts/build-packages.sh` | Main build script |
| `docs/README.md` | Documentation index |
| `CLAUDE.md` | Development instructions for Claude Code |

## 🚀 Quick Start

```bash
# Build the project
cargo build --release

# Build plugins
./build/scripts/build-plugins.sh

# Run the server
./target/release/rusty-beam

# Build distribution packages
./build/scripts/build-packages.sh
```

## 📖 Documentation Navigation

- **Getting Started**: [docs/README.md](docs/README.md)
- **API Reference**: [docs/api/](docs/api/)
- **User Guides**: [docs/guides/](docs/guides/)
- **Examples**: [docs/examples/](docs/examples/)
- **Plugin Development**: [docs/plugins/](docs/plugins/)

## 🔄 Migration Notes

This structure was reorganized to reduce root directory clutter and improve project maintainability. Key changes:

- Build scripts moved from root to `build/scripts/`
- Configuration moved from root to `config/`
- Example sites moved to `examples/`
- All documentation consolidated in `docs/`
- Testing files moved to `tests/`
- Development docs moved to `docs/development/`

All references and paths have been updated accordingly.