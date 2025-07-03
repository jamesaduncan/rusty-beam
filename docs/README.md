# Rusty-beam Documentation

Welcome to the Rusty-beam documentation! Rusty-beam is an experimental HTTP server that serves files with CSS selector-based HTML manipulation via HTTP Range headers.

## 📚 Documentation Overview

### Core Concepts
- **[Getting Started](guides/getting-started.md)** - Quick start guide and basic usage
- **[Range Selector Syntax](api/range-selectors.md)** - How to use `Range: selector=` headers
- **[Resource Selectors](api/resource-selectors.md)** - The `#(selector=foo)` syntax for URLs
- **[Configuration](guides/configuration.md)** - Server and host configuration

### API Reference
- **[HTTP API](api/http-api.md)** - Complete HTTP API reference
- **[CSS Selectors](api/css-selectors.md)** - Supported CSS selector syntax
- **[Examples](examples/)** - Working examples and use cases

### Authentication & Authorization
- **[Authentication](auth/authentication.md)** - Plugin-based user authentication system
- **[Authorization](auth/authorization.md)** - Plugin-based access control and permissions
- **[User Management](auth/user-management.md)** - Managing users and roles

### Plugin System
- **[Plugin Architecture](plugins/architecture.md)** - Authentication and authorization plugin architecture
- **[Authentication Plugins](plugins/authentication-plugin-development.md)** - Developing authentication plugins
- **[Authorization Plugins](plugins/authorization-plugin-development.md)** - Developing authorization plugins
- **[Basic Auth Plugin](plugins/basic-auth.md)** - HTTP Basic Authentication
- **[Google OAuth2 Plugin](plugins/google-oauth2.md)** - Google OAuth2 integration

### Deployment & Operations
- **[Installation](guides/installation.md)** - Installation methods
- **[Distribution Packages](guides/distribution.md)** - Download and install packages
- **[Deployment](guides/deployment.md)** - Production deployment
- **[Security](guides/security.md)** - Security considerations

## 🚀 Quick Example

```bash
# Start the server
rusty-beam

# Get an HTML element using CSS selector
curl -H "Range: selector=div.content" http://localhost:3000/page.html

# Update an element
curl -X PUT -H "Range: selector=#main" -d "<p>New content</p>" http://localhost:3000/page.html

# Access with authentication
curl -u admin:admin123 -H "Range: selector=.admin-panel" http://localhost:3000/admin.html
```

## 🎯 Key Features

- **CSS Selector API**: Manipulate HTML using familiar CSS selectors
- **HTTP Range Abuse**: Creative use of Range headers for element selection
- **Plugin Authentication**: Extensible authentication system
- **Dynamic Authorization**: Fine-grained access control
- **Multi-host Support**: Virtual host configuration
- **Real-time Manipulation**: Live HTML modification via HTTP

## 🧩 Architecture

Rusty-beam follows a plugin-based architecture:

```
┌─────────────────┐    ┌──────────────┐    ┌─────────────┐
│   HTTP Client   │◄──►│ Rusty-beam   │◄──►│   Plugins   │
└─────────────────┘    │   Server     │    │ (Auth, etc) │
                       └──────────────┘    └─────────────┘
                              │
                       ┌──────▼──────┐
                       │ HTML Files  │
                       │ CSS Parsing │
                       └─────────────┘
```

## 📋 Table of Contents

### Getting Started
1. [Installation & Setup](guides/getting-started.md)
2. [Basic Usage](guides/basic-usage.md)
3. [Configuration](guides/configuration.md)

### API Documentation
1. [HTTP API Reference](api/http-api.md)
2. [Range Selector Syntax](api/range-selectors.md)
3. [Resource Selector Syntax](api/resource-selectors.md)
4. [CSS Selector Support](api/css-selectors.md)

### Authentication & Security
1. [Authentication Overview](auth/authentication.md)
2. [Authorization System](auth/authorization.md)
3. [User & Role Management](auth/user-management.md)
4. [Security Best Practices](guides/security.md)

### Plugin Development
1. [Plugin Architecture](plugins/architecture.md)
2. [Authentication Plugins](plugins/authentication-plugins.md)
3. [Writing Custom Plugins](plugins/writing-plugins.md)
4. [Plugin Examples](plugins/examples.md)

### Examples & Tutorials
1. [Basic Examples](examples/basic-examples.md)
2. [Advanced Use Cases](examples/advanced-examples.md)
3. [Integration Examples](examples/integration-examples.md)

## 🤝 Contributing

See the main [README.md](../README.md) for contribution guidelines.

## 📄 License

Rusty-beam is licensed under the MIT License. See [LICENSE](../LICENSE) for details.