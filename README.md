# Rusty Beam

A high-performance HTTP server with unique CSS selector-based HTML manipulation capabilities, plugin architecture, and built-in authentication/authorization.

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![Documentation](https://img.shields.io/badge/docs-available-green.svg)](docs/)

## Overview

Rusty Beam is an experimental HTTP server that extends standard REST operations with CSS selector support, allowing you to manipulate HTML documents through simple HTTP requests. Built on high-performance async Rust with `hyper` and `tokio`, it provides a unique approach to dynamic content management.

## ✨ Key Features

### Core Capabilities
- **🎯 CSS Selector API**: Manipulate HTML using standard CSS selectors via HTTP Range headers
- **🔌 Plugin Architecture**: Extensible system with authentication, compression, CORS, and more
- **⚡ High Performance**: Async Rust with zero-copy operations and efficient concurrency
- **🔒 Built-in Security**: Authentication, authorization, rate limiting, and security headers
- **📝 HTML Configuration**: Human-readable configuration using HTML with microdata
- **🔄 Hot Reload**: Update configuration without restarting via SIGHUP

### Standard File Operations
- **GET**: Serve files with automatic index.html serving
- **PUT**: Upload or replace files (201 for new, 200 for updates)
- **POST**: Append to files or create in directories
- **DELETE**: Remove files or empty directories

### HTML Manipulation via CSS Selectors
- **GET + Selector**: Extract specific elements (returns 206 Partial Content)
- **PUT + Selector**: Replace elements (returns 206 with updated element)
- **POST + Selector**: Append to elements (returns 206 with updated element)
- **DELETE + Selector**: Remove elements (returns 204 No Content)

## 🚀 Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/jamesaduncan/rusty-beam.git
cd rusty-beam

# Build the server and plugins
cargo build --release
./build-plugins.sh

# Run with example configuration
./target/release/rusty-beam config/config.html
```

### Docker Quick Start

```bash
# Using Docker
docker pull rustybeam/rusty-beam:latest
docker run -p 3000:3000 -v ./config:/etc/rusty-beam rustybeam/rusty-beam

# Using Docker Compose
docker-compose up -d
```

See the [Installation Guide](docs/guides/installation.html) for detailed instructions.

## 🛠️ Usage Examples

### Basic Configuration

Create a simple configuration file `config.html`:

```html
<!DOCTYPE html>
<html>
<body>
    <table itemscope itemtype="http://rustybeam.net/ServerConfig">
        <tr><td itemprop="bind">127.0.0.1</td></tr>
        <tr><td itemprop="port">3000</td></tr>
        <tr><td itemprop="root">./public</td></tr>
    </table>
    
    <ul>
        <li itemscope itemtype="http://rustybeam.net/HostConfig">
            <span itemprop="host">*</span>
            <ul>
                <li itemprop="plugin" itemscope itemtype="http://rustybeam.net/Plugin">
                    <span itemprop="library">file://./plugins/lib/selector-handler.so</span>
                </li>
                <li itemprop="plugin" itemscope itemtype="http://rustybeam.net/Plugin">
                    <span itemprop="library">file://./plugins/lib/file-handler.so</span>
                </li>
            </ul>
        </li>
    </ul>
</body>
</html>
```

### Basic File Operations

```bash
# Upload an HTML file
curl -X PUT -d '<html><body><h1 id="title">Hello</h1><p class="content">World</p></body></html>' \
  http://localhost:3000/test.html
# Response: 201 Created

# Retrieve the file
curl http://localhost:3000/test.html
# Response: 200 OK (full document)
```

### CSS Selector Operations

```bash
# Extract specific element
curl -H "Range: selector=#title" http://localhost:3000/test.html
# Response: 206 Partial Content
# Body: <h1 id="title">Hello</h1>

# Replace element
curl -X PUT -H "Range: selector=#title" \
  -d '<h1 id="title">Updated Title</h1>' \
  http://localhost:3000/test.html
# Response: 206 Partial Content
# Body: <h1 id="title">Updated Title</h1>

# Append to element
curl -X POST -H "Range: selector=body" \
  -d '<footer>Copyright 2024</footer>' \
  http://localhost:3000/test.html
# Response: 206 Partial Content
# Body: (body element with appended footer)

# Remove element
curl -X DELETE -H "Range: selector=.old-content" \
  http://localhost:3000/test.html
# Response: 204 No Content
```

## 🔌 Plugin System

Rusty Beam features a powerful plugin architecture. Available plugins include:

### Core Plugins
- **selector-handler** - CSS selector operations for HTML manipulation
- **file-handler** - Static file serving with directory index support
- **basic-auth** - HTTP Basic Authentication with bcrypt support
- **authorization** - Role-based access control (RBAC)
- **access-log** - Request logging in Apache/Combined format

### Additional Plugins
- **compression** - Gzip/deflate compression for responses
- **cors** - Cross-Origin Resource Sharing configuration
- **security-headers** - Security headers (HSTS, CSP, etc.)
- **rate-limit** - Token bucket rate limiting
- **redirect** - URL redirection with pattern matching
- **cache-control** - HTTP caching headers
- **error-handler** - Custom error pages
- **websocket** - WebSocket support for real-time updates
- **health-check** - Health monitoring endpoints

See the [Plugin Documentation](docs/plugins/) for configuration details.

## 📚 Documentation

### Guides
- [Installation Guide](docs/guides/installation.html) - Detailed installation instructions
- [Getting Started](docs/guides/getting-started.html) - Step-by-step tutorial
- [Deployment Guide](docs/guides/deployment.html) - Production deployment strategies
- [Security Guide](docs/guides/security.html) - Security best practices
- [Performance Guide](docs/guides/performance.html) - Optimization techniques
- [Troubleshooting](docs/guides/troubleshooting.html) - Common issues and solutions

### Reference
- [Configuration Reference](docs/reference/configuration.html) - All configuration options
- [HTTP API Reference](docs/reference/http-api.html) - Complete API documentation
- [Plugin Documentation](docs/plugins/) - Individual plugin guides
- [Schema Documentation](docs/schema/) - Configuration schemas

### Examples
- [Tutorials](docs/tutorials/) - Practical examples and use cases
- [Example Configurations](examples/) - Sample configuration files

## 🧪 Development

### Building from Source

```bash
# Clone and build
git clone https://github.com/jamesaduncan/rusty-beam.git
cd rusty-beam
cargo build --release
./build-plugins.sh

# Run tests
./run_all_tests.sh

# Run with verbose logging
cargo run -- -v config/config.html
```

### Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details on:
- Code standards
- Development workflow
- Testing requirements
- Pull request process

### Project Structure

```
rusty-beam/
├── src/              # Core server code
├── plugins/          # Plugin implementations
├── tests/            # Test suites
├── docs/             # Documentation
├── examples/         # Example configurations
└── config/           # Default configuration
```

## 🚀 Performance

Rusty Beam is designed for high performance:

- **Concurrent Connections**: 10,000+ simultaneous connections
- **Throughput**: 50,000+ requests/second (small files)
- **Latency**: < 1ms for cached static files
- **Memory Usage**: ~50MB base + content cache

See the [Performance Guide](docs/guides/performance.html) for optimization tips.

## 🔒 Security

Security features include:

- Path traversal protection
- Configurable authentication and authorization
- Rate limiting and DDoS protection
- Security headers (HSTS, CSP, X-Frame-Options, etc.)
- Input validation for CSS selectors

See the [Security Guide](docs/guides/security.html) for best practices.

## 🌟 Use Cases

Rusty Beam is ideal for:

- **Dynamic Websites**: Update content without regenerating pages
- **Content Management**: Direct HTML manipulation via API
- **API Gateways**: Proxy with authentication and rate limiting
- **Static Site Enhancement**: Add dynamic features to static sites
- **Development Tools**: Rapid prototyping with live updates
- **Microservices**: Lightweight service with plugin architecture

## 🤝 Community

- **Issues**: [GitHub Issues](https://github.com/jamesaduncan/rusty-beam/issues)
- **Discussions**: [GitHub Discussions](https://github.com/jamesaduncan/rusty-beam/discussions)
- **Contributing**: See [CONTRIBUTING.md](CONTRIBUTING.md)

## 📜 License

This project is licensed under the Apache License, Version 2.0. See the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

Built with:
- [Hyper](https://hyper.rs/) - Fast HTTP implementation
- [Tokio](https://tokio.rs/) - Async runtime
- [dom_query](https://github.com/niklak/dom_query) - HTML parsing and manipulation

---

**Note**: This is experimental software. While it's designed for production use, please test thoroughly in your environment before deploying to production.
