# Testing Documentation for Rusty Beam Authentication System

This document describes the comprehensive test suite for the Rusty Beam authentication plugin system, covering both unit tests and integration tests.

## Test Coverage

### Unit Tests

#### Plugin Manager Tests (`src/plugins/mod.rs`)
- ✅ **Plugin Manager Creation**: Tests creating new plugin managers
- ✅ **Server-Wide Plugin Registration**: Tests adding plugins that apply to all hosts
- ✅ **Host-Specific Plugin Registration**: Tests adding plugins for specific hosts
- ✅ **Authentication Requirements**: Tests whether authentication is required for specific paths
- ✅ **Authentication Flow**: Tests the complete authentication process
- ✅ **Error Handling**: Tests plugin error scenarios
- ✅ **Host Precedence**: Tests that host-specific plugins override server-wide plugins
- ✅ **Anonymous Access**: Tests fallback to anonymous access when no authentication required

#### Basic Auth Plugin Tests (`src/plugins/basic_auth.rs`)
- ✅ **Plugin Creation**: Tests creating plugins with valid/invalid auth files
- ✅ **Password Hashing**: Tests both plaintext and bcrypt password hashing
- ✅ **Password Verification**: Tests password verification for both encryption types
- ✅ **HTTP Header Parsing**: Tests parsing of Basic Auth headers
- ✅ **Authentication Scenarios**:
  - No authentication header provided
  - Invalid credentials
  - Valid credentials (plaintext passwords)
  - Valid credentials (bcrypt passwords)
  - Mixed encryption types in same file
  - Non-existent users
- ✅ **Plugin Interface**: Tests plugin name and authentication requirements

### Integration Tests

#### Basic Authentication Tests (`tests_auth.hurl`)
- ✅ **Unauthenticated Access**: Tests that protected resources require authentication
- ✅ **Invalid Credentials**: Tests various forms of invalid authentication
- ✅ **Valid Credentials**: Tests successful authentication with plaintext passwords
- ✅ **CRUD Operations**: Tests authenticated file operations (GET, PUT, DELETE)
- ✅ **CSS Selector Operations**: Tests authenticated CSS selector functionality
- ✅ **Host-Based Authentication**: Tests host-specific authentication
- ✅ **Error Handling**: Tests malformed headers and edge cases
- ✅ **Performance**: Tests multiple rapid requests

#### Mixed Encryption Tests (`tests_auth_mixed.hurl`)
- ✅ **Bcrypt Authentication**: Tests authentication with bcrypt-hashed passwords
- ✅ **Plaintext Authentication**: Tests authentication with plaintext passwords
- ✅ **Mixed File Support**: Tests both encryption types in the same auth file
- ✅ **Role Verification**: Tests that different users have appropriate access
- ✅ **Cross-User Operations**: Tests that both user types can perform CRUD operations
- ✅ **Stress Testing**: Tests rapid alternating requests between encryption types
- ✅ **Security Validation**: Tests that bcrypt hashes don't work as plaintext

## Running Tests

### Unit Tests
```bash
# Run all unit tests
cargo test

# Run plugin manager tests only
cargo test plugins::tests --lib

# Run basic auth plugin tests only
cargo test plugins::basic_auth::tests --lib
```

### Integration Tests
```bash
# Run all tests (main functionality + authentication)
./run-tests.sh

# Run mixed encryption tests specifically
./test-mixed-auth.sh
```

## Test Configuration

### Test Users

#### Default Authentication File (`localhost/auth/users.html`)
- **admin:admin123** (plaintext) - admin, user roles
- **johndoe:doe123** (plaintext) - user, editor roles

#### Mixed Encryption File (`localhost/auth/users_mixed.html`)
- **admin:admin123** (bcrypt) - admin, user roles  
- **johndoe:doe123** (plaintext) - user, editor roles
- **testuser:test123** (plaintext) - user role

### Test Scenarios Covered

1. **Plugin Architecture**
   - Plugin registration and management
   - Host-specific vs server-wide plugins
   - Plugin precedence and fallback behavior

2. **Authentication Methods**
   - HTTP Basic Authentication
   - Plaintext password storage
   - Bcrypt password encryption
   - Mixed encryption types in same file

3. **Security Scenarios**
   - No credentials provided
   - Invalid username/password combinations
   - Malformed authentication headers
   - Case sensitivity tests
   - Long password tests

4. **Functionality Integration**
   - File operations (GET, PUT, DELETE)
   - CSS selector operations
   - Host-based routing with authentication
   - Multiple concurrent requests

5. **Error Handling**
   - Invalid authentication files
   - Network errors
   - Malformed requests
   - Plugin loading failures

## Regression Prevention

The test suite is designed to prevent regressions in:

1. **Plugin System Architecture**
   - Plugin loading and registration
   - Authentication flow logic
   - Host-specific configuration

2. **Password Security**
   - Encryption type handling
   - Password verification logic
   - Mixed encryption support

3. **HTTP Protocol Compliance**
   - Proper status codes (401, 200, 500)
   - Correct WWW-Authenticate headers
   - Basic Auth header parsing

4. **Integration Points**
   - Configuration file parsing
   - Request routing with authentication
   - File system operations with auth

## Test Results

All tests are designed to:
- ✅ Pass consistently across multiple runs
- ✅ Provide clear failure messages
- ✅ Cover edge cases and error conditions
- ✅ Validate both positive and negative scenarios
- ✅ Ensure backward compatibility

## Continuous Integration

The test suite can be integrated into CI/CD pipelines with:
```bash
# Quick validation (unit tests only)
cargo test

# Full validation (unit + integration tests)
./run-tests.sh && ./test-mixed-auth.sh
```

This comprehensive test coverage ensures the authentication system is robust, secure, and resistant to regressions during future development.