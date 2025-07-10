# Contributing to Rusty Beam

Thank you for your interest in contributing to Rusty Beam! This guide will help you get started with contributing to the project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [How to Contribute](#how-to-contribute)
- [Development Workflow](#development-workflow)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Documentation](#documentation)
- [Pull Request Process](#pull-request-process)
- [Plugin Development](#plugin-development)
- [Community](#community)

## Code of Conduct

This project adheres to a Code of Conduct that all contributors are expected to follow. Please be respectful, inclusive, and considerate in all interactions.

### Our Standards

- Be welcoming and inclusive
- Be respectful of differing viewpoints
- Accept constructive criticism gracefully
- Focus on what's best for the community
- Show empathy towards others

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Git
- Basic knowledge of HTTP and web servers
- Familiarity with async Rust is helpful

### First Steps

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/rusty-beam.git
   cd rusty-beam
   ```
3. Add the upstream repository:
   ```bash
   git remote add upstream https://github.com/jamesaduncan/rusty-beam.git
   ```
4. Create a new branch for your work:
   ```bash
   git checkout -b feature/your-feature-name
   ```

## Development Setup

### Building the Project

```bash
# Build in debug mode
cargo build

# Build in release mode
cargo build --release

# Build all plugins
./build-plugins.sh
```

### Running Tests

```bash
# Run all tests
./run_all_tests.sh

# Run unit tests only
cargo test

# Run integration tests
./run_hurl_tests.sh

# Run plugin tests
./run_plugin_tests_isolated.sh

# Run with verbose output
cargo test -- --nocapture
```

### Development Server

```bash
# Run in development mode with verbose output
cargo run -- -v docs/config/config.html

# Run with specific log levels
RUST_LOG=debug cargo run -- docs/config/config.html

# Run with cargo-watch for auto-reload
cargo install cargo-watch
cargo watch -x "run -- -v docs/config/config.html"
```

## How to Contribute

### Types of Contributions

1. **Bug Fixes**: Fix existing issues or report new ones
2. **Features**: Implement new functionality
3. **Plugins**: Create new plugins or improve existing ones
4. **Documentation**: Improve or add documentation
5. **Tests**: Add test coverage
6. **Performance**: Optimize code for better performance
7. **Refactoring**: Improve code quality and maintainability

### Finding Issues

- Check the [issue tracker](https://github.com/jamesaduncan/rusty-beam/issues)
- Look for issues labeled `good first issue` or `help wanted`
- Check TODO.md for planned features
- Ask in discussions if you're unsure where to start

## Development Workflow

### 1. Research Phase

Before coding, understand the existing codebase:

```bash
# Search for related code
grep -r "feature_name" src/
rg "pattern" --type rust

# Understand the architecture
cat CLAUDE.md  # Development guidelines
tree src/      # Code structure
```

### 2. Plan Phase

- Write a brief plan in the issue or pull request
- Discuss significant changes before implementing
- Consider edge cases and error handling

### 3. Implementation Phase

Follow Test-Driven Development (TDD):

1. Write a failing test first
2. Run the test to confirm it fails
3. Write minimal code to make it pass
4. Refactor if needed
5. Ensure all tests still pass

Example:
```rust
#[test]
fn test_new_feature() {
    // Arrange
    let input = "test";
    
    // Act
    let result = new_feature(input);
    
    // Assert
    assert_eq!(result, "expected");
}
```

## Coding Standards

### Rust Style Guide

- Follow standard Rust conventions
- Use `cargo fmt` before committing
- Run `cargo clippy` and fix warnings
- Use meaningful variable and function names

```bash
# Format code
cargo fmt

# Check lints
cargo clippy -- -D warnings

# Check for common mistakes
cargo check
```

### Code Organization

```rust
// Good: Clear module organization
mod config;
mod handlers;
mod plugins;

use crate::config::Config;

// Good: Clear function signatures
pub async fn handle_request(
    req: Request<Body>,
    state: Arc<AppState>,
) -> Result<Response<Body>, Error> {
    // Implementation
}

// Good: Error handling
match operation() {
    Ok(result) => process(result),
    Err(e) => {
        error!("Operation failed: {}", e);
        return Err(e.into());
    }
}
```

### Comments and Documentation

```rust
/// Processes HTTP requests through the plugin pipeline.
///
/// # Arguments
/// * `req` - The incoming HTTP request
/// * `state` - Shared application state
///
/// # Returns
/// * `Ok(Response)` - Processed response
/// * `Err(Error)` - Processing error
///
/// # Example
/// ```
/// let response = process_request(request, state).await?;
/// ```
pub async fn process_request(
    req: Request<Body>,
    state: Arc<AppState>,
) -> Result<Response<Body>, Error> {
    // Implementation with inline comments for complex logic
}
```

## Testing

### Test Requirements

1. All new features must have tests
2. Bug fixes must include regression tests
3. Maintain or improve code coverage
4. Tests must be deterministic and fast

### Writing Tests

#### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonicalize_path() {
        let result = canonicalize_file_path(
            "/var/www",
            "../../etc/passwd"
        );
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_async_operation() {
        let result = async_function().await;
        assert_eq!(result, expected);
    }
}
```

#### Integration Tests

Create `.hurl` files in `tests/integration/`:

```hurl
# Test file upload
PUT http://localhost:3000/test.txt
Content-Type: text/plain
```
Test content
```
HTTP 201
[Asserts]
body == "File uploaded successfully"
```

### Running Tests

```bash
# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --show-output

# Run integration tests
./run_hurl_tests.sh

# Run benchmarks
cargo bench
```

## Documentation

### Code Documentation

- Add doc comments to all public APIs
- Include examples in doc comments
- Update relevant documentation files

### Documentation Files

When adding features, update:

1. **README.md** - If it affects basic usage
2. **API documentation** - For new endpoints or changes
3. **Plugin documentation** - For plugin changes
4. **Configuration reference** - For new config options
5. **Guides** - For significant features

### Documentation Style

Follow the existing HTML documentation style:

```html
<!DOCTYPE html>
<html>
<head>
    <title>Feature Name - Rusty Beam</title>
    <!-- Standard styles -->
</head>
<body>
    <nav>
        <a href="/">Home</a> → 
        <a href="/category/">Category</a> → 
        Feature
    </nav>
    
    <h1>Feature Name</h1>
    
    <!-- Content with examples -->
</body>
</html>
```

## Pull Request Process

### Before Submitting

1. **Test your changes**:
   ```bash
   ./run_all_tests.sh
   cargo clippy -- -D warnings
   cargo fmt -- --check
   ```

2. **Update documentation** if needed

3. **Rebase on latest main**:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

### PR Guidelines

1. **Title**: Clear and descriptive
   - Good: "Add WebSocket support for real-time updates"
   - Bad: "Fix stuff"

2. **Description**: Include:
   - What changes were made
   - Why they were made
   - How to test them
   - Related issues

3. **Commits**:
   - Use meaningful commit messages
   - Keep commits focused and atomic
   - Reference issues: "Fix #123: Add validation"

### PR Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Manual testing completed

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] Tests added/updated
```

## Plugin Development

### Creating a New Plugin

1. **Use the plugin template**:
   ```bash
   cp -r plugins/template plugins/my-plugin
   cd plugins/my-plugin
   ```

2. **Update Cargo.toml**:
   ```toml
   [package]
   name = "my-plugin"
   version = "0.1.0"
   
   [dependencies]
   rusty-beam-plugin-api = { path = "../rusty-beam-plugin-api" }
   ```

3. **Implement the plugin trait**:
   ```rust
   #[derive(Debug)]
   pub struct MyPlugin {
       config: Config,
   }
   
   #[async_trait]
   impl Plugin for MyPlugin {
       fn name(&self) -> &str {
           "my-plugin"
       }
       
       async fn handle_request(
           &self,
           req: Request<Body>,
           next: Next,
       ) -> Result<Response<Body>, Error> {
           // Implementation
       }
   }
   ```

4. **Add tests** in `src/lib.rs`

5. **Create documentation** in `docs/plugins/my-plugin/index.html`

### Plugin Best Practices

- Keep plugins focused on a single responsibility
- Handle errors gracefully
- Provide configuration options
- Document all configuration parameters
- Include usage examples

## Community

### Getting Help

- Open an issue for bugs or features
- Start a discussion for questions
- Check existing issues and discussions first

### Code Review

- Be constructive and helpful
- Suggest improvements, don't demand them
- Explain the reasoning behind suggestions
- Be open to feedback on your own code

### Recognition

Contributors are recognized in:
- The git history
- Release notes for significant contributions
- The project README for major contributors

## Quick Commands Reference

```bash
# Development
cargo build                     # Build debug
cargo build --release          # Build release
cargo run -- -v config.html    # Run with verbose
cargo test                     # Run unit tests
cargo clippy                   # Run linter
cargo fmt                      # Format code

# Testing
./run_all_tests.sh            # Run all tests
./run_hurl_tests.sh           # Integration tests
./build-plugins.sh            # Build plugins

# Git workflow
git checkout -b feature/name   # New feature branch
git add -A                    # Stage changes
git commit -m "Message"       # Commit
git push origin feature/name  # Push to fork
```

## Thank You!

Thank you for contributing to Rusty Beam! Your efforts help make this project better for everyone.

If you have questions or need help, don't hesitate to ask in the issues or discussions.