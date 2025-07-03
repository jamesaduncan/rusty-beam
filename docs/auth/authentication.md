# Authentication System

Rusty-beam uses a plugin-based authentication system that supports multiple authentication methods through dynamically loaded plugins. The authentication system is flexible, extensible, and integrates seamlessly with the authorization framework.

## Overview

Authentication in Rusty-beam follows a two-phase process:

1. **Authentication**: Verify user identity through plugins
2. **Authorization**: Check permissions based on user roles and resources

```
┌─────────────┐    ┌──────────────┐    ┌─────────────────┐
│   Request   │───►│ Auth Plugin  │───►│ Authorization   │
│             │    │ (Identity)   │    │ Engine (Access) │
└─────────────┘    └──────────────┘    └─────────────────┘
```

## Plugin Architecture

### Authentication Flow

1. **Plugin Discovery**: Server loads authentication plugins from configuration
2. **Request Processing**: Each plugin checks if authentication is required for the resource
3. **Identity Verification**: Plugins validate credentials and return user information
4. **User Context**: Authenticated user information is passed to authorization system

### Plugin Interface

Authentication plugins must implement:

```rust
pub trait AuthPlugin {
    async fn authenticate(&self, req: &Request<Body>) -> AuthResult;
    fn name(&self) -> &'static str;
    fn requires_authentication(&self, path: &str) -> bool;
}

pub enum AuthResult {
    Authorized(UserInfo),
    Unauthorized,
    Error(String),
}

pub struct UserInfo {
    pub username: String,
    pub roles: Vec<String>,
}
```

## Configuration

### Host-based Authentication

Authentication is configured per host in `config.html`:

```html
<tbody itemprop="host" itemscope itemtype="http://rustybeam.net/HostConfig">
    <tr>
        <td>Host Name</td>
        <td itemprop="hostName">localhost</td>
    </tr>
    <tr>
        <td>Host Root</td>
        <td itemprop="hostRoot">./localhost</td>
    </tr>
    <tr id="localhost-plugin-basic-auth" itemprop="plugin">
        <td>Plugin</td>
        <td itemprop="plugin-path">plugins/lib/libbasic_auth.so</td>
    </tr>
    <tr>
        <td>Authentication File</td>
        <td itemref="localhost-plugin-basic-auth" itemprop="authFile">./localhost/auth/users.html</td>
    </tr>
    <tr>
        <td>Authorization File</td>
        <td itemprop="authorizationFile">./localhost/auth/users.html</td>
    </tr>
</tbody>
```

### Plugin Configuration

Each plugin can have custom configuration parameters:

```html
<!-- Basic Auth Plugin Configuration -->
<tr itemprop="plugin" itemscope itemtype="http://rustybeam.net/PluginConfig">
    <td>Plugin Path</td>
    <td itemprop="plugin-path">plugins/lib/libbasic_auth.so</td>
</tr>
<tr>
    <td>Auth File</td>
    <td itemref="localhost-plugin-basic-auth" itemprop="authFile">./localhost/auth/users.html</td>
</tr>

<!-- Google OAuth2 Plugin Configuration -->
<tr itemprop="plugin" itemscope itemtype="http://rustybeam.net/PluginConfig">
    <td>Plugin Path</td>
    <td itemprop="plugin-path">plugins/lib/libgoogle_oauth2.so</td>
</tr>
<tr>
    <td>Client ID</td>
    <td itemref="google-oauth2-plugin" itemprop="clientId">your-client-id.googleusercontent.com</td>
</tr>
<tr>
    <td>Client Secret</td>
    <td itemref="google-oauth2-plugin" itemprop="clientSecret">your-client-secret</td>
</tr>
```

## User Data Format

### HTML-based User Storage

Users are defined using HTML microdata format:

```html
<table id="users">
    <thead>
        <tr>
            <td>Username</td>
            <td>Password</td>
            <td>Roles</td>
            <td>Meta</td>
        </tr>
    </thead>
    <tbody>
        <tr itemscope itemtype="http://rustybeam.net/User">
            <td itemprop="username">admin</td>
            <td itemprop="password">admin123</td>
            <td>
                <ul>
                    <li itemprop="role">administrators</li>
                    <li itemprop="role">user</li>
                </ul>
            </td>
            <td>
                <ul>
                    <li itemprop="encryption">plaintext</li>
                </ul>
            </td>
        </tr>
        <tr itemscope itemtype="http://rustybeam.net/User">
            <td itemprop="username">johndoe</td>
            <td itemprop="password">$2b$12$hash...</td>
            <td>
                <ul>
                    <li itemprop="role">user</li>
                    <li itemprop="role">editor</li>
                </ul>
            </td>
            <td>
                <ul>
                    <li itemprop="encryption">bcrypt</li>
                </ul>
            </td>
        </tr>
    </tbody>
</table>
```

### User Schema

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `username` | string | Yes | Unique user identifier |
| `password` | string | Yes | Password (format depends on encryption) |
| `role` | string[] | No | User roles for authorization |
| `encryption` | string | No | Password encryption method |

### Supported Encryption Methods

| Method | Description | Example |
|--------|-------------|---------|
| `plaintext` | No encryption (development only) | `password123` |
| `bcrypt` | bcrypt hashing (recommended) | `$2b$12$hash...` |

### Role System

Roles are used by the authorization system to grant permissions:

```html
<!-- Admin user with multiple roles -->
<li itemprop="role">administrators</li>
<li itemprop="role">user</li>
<li itemprop="role">content-editor</li>

<!-- Standard user -->
<li itemprop="role">user</li>

<!-- Editor role -->
<li itemprop="role">editor</li>
<li itemprop="role">content-creator</li>
```

## Built-in Authentication Plugins

### 1. Basic Authentication Plugin

**File**: `plugins/lib/libbasic_auth.so`

HTTP Basic Authentication using username/password credentials.

#### Features
- HTTP Basic Auth support
- Multiple password encryption methods
- Role-based user management
- HTML-based user storage

#### Usage

```bash
# Authenticate with Basic Auth
curl -u username:password http://localhost:3000/protected.html

# With explicit Basic Auth header
curl -H "Authorization: Basic dXNlcm5hbWU6cGFzc3dvcmQ=" http://localhost:3000/protected.html
```

#### Configuration

```html
<tr itemprop="plugin" itemscope itemtype="http://rustybeam.net/PluginConfig">
    <td itemprop="plugin-path">plugins/lib/libbasic_auth.so</td>
</tr>
<tr>
    <td itemprop="authFile">./localhost/auth/users.html</td>
</tr>
```

### 2. Google OAuth2 Plugin

**File**: `plugins/lib/libgoogle_oauth2.so`

Google OAuth2 authentication for Google Workspace users.

#### Features
- Google OAuth2 flow
- Domain-based access control
- Automatic user provisioning
- Token validation

#### Configuration

```html
<tr itemprop="plugin" itemscope itemtype="http://rustybeam.net/PluginConfig">
    <td itemprop="plugin-path">plugins/lib/libgoogle_oauth2.so</td>
</tr>
<tr>
    <td itemprop="clientId">your-app.googleusercontent.com</td>
</tr>
<tr>
    <td itemprop="clientSecret">your-client-secret</td>
</tr>
<tr>
    <td itemprop="allowedDomains">example.com,trusted.org</td>
</tr>
```

#### Usage

```bash
# OAuth2 flow requires browser interaction
# Plugin redirects to Google for authentication
curl http://localhost:3000/protected.html
# -> Redirects to Google OAuth2
# -> User authorizes
# -> Redirects back with token
# -> Plugin validates and creates session
```

## Authentication Process

### 1. Request Evaluation

For each incoming request:

```rust
// Pseudo-code
for plugin in host_plugins {
    if plugin.requires_authentication(path) {
        match plugin.authenticate(request).await {
            AuthResult::Authorized(user) => {
                // Proceed to authorization check
                return check_authorization(user, resource, method);
            },
            AuthResult::Unauthorized => {
                // Return 401 with WWW-Authenticate header
                return unauthorized_response();
            },
            AuthResult::Error(msg) => {
                // Return 500 with error
                return error_response(msg);
            }
        }
    }
}
// No authentication required
return anonymous_access();
```

### 2. Plugin Priority

When multiple plugins are configured:

1. **First Match Wins**: The first plugin that requires authentication for a path handles it
2. **Plugin Order**: Defined by configuration order in `config.html`
3. **Host Isolation**: Each host has independent plugin configuration

### 3. Anonymous Access

When no plugin requires authentication:

```rust
// Anonymous user context
UserInfo {
    username: "anonymous".to_string(),
    roles: vec!["anonymous".to_string()],
}
```

## Error Handling

### Common Authentication Errors

| Scenario | Status | Response |
|----------|--------|----------|
| No credentials provided | 401 | `WWW-Authenticate` header |
| Invalid credentials | 401 | Authentication failed |
| Plugin error | 500 | Internal server error |
| User not found | 401 | Authentication failed |
| Account locked | 423 | Account locked |

### Error Responses

#### 401 Unauthorized

```http
HTTP/1.1 401 Unauthorized
WWW-Authenticate: Basic realm="Rusty Beam"
Content-Type: text/plain
Content-Length: 23

Authentication required
```

#### 500 Internal Server Error

```http
HTTP/1.1 500 Internal Server Error
Content-Type: text/plain
Content-Length: 21

Authentication error
```

## Security Considerations

### Password Security

1. **Never use plaintext in production**:
   ```html
   <!-- Development only -->
   <li itemprop="encryption">plaintext</li>
   
   <!-- Production -->
   <li itemprop="encryption">bcrypt</li>
   ```

2. **Use strong bcrypt cost factors**:
   ```bash
   # Generate bcrypt hash with cost 12
   python3 -c "import bcrypt; print(bcrypt.hashpw(b'password', bcrypt.gensalt(12)).decode())"
   ```

### Session Management

- **Stateless Authentication**: Each request is authenticated independently
- **No Server-side Sessions**: Reduces complexity and improves scalability
- **Token-based Auth**: OAuth2 tokens are validated per request

### Transport Security

- **HTTPS Required**: Always use HTTPS in production
- **Secure Headers**: Authentication plugins should set secure headers
- **CSRF Protection**: Consider CSRF tokens for state-changing operations

## Configuration Examples

### Multi-Plugin Setup

```html
<!-- Primary: Basic Auth for local users -->
<tr itemprop="plugin" itemscope itemtype="http://rustybeam.net/PluginConfig">
    <td itemprop="plugin-path">plugins/lib/libbasic_auth.so</td>
</tr>
<tr>
    <td itemprop="authFile">./localhost/auth/local-users.html</td>
</tr>

<!-- Secondary: OAuth2 for external users -->
<tr itemprop="plugin" itemscope itemtype="http://rustybeam.net/PluginConfig">
    <td itemprop="plugin-path">plugins/lib/libgoogle_oauth2.so</td>
</tr>
<tr>
    <td itemprop="clientId">app.googleusercontent.com</td>
</tr>
<tr>
    <td itemprop="clientSecret">secret</td>
</tr>
```

### Development vs Production

**Development Configuration**:
```html
<tr itemscope itemtype="http://rustybeam.net/User">
    <td itemprop="username">dev</td>
    <td itemprop="password">dev123</td>
    <td>
        <ul>
            <li itemprop="role">developers</li>
        </ul>
    </td>
    <td>
        <ul>
            <li itemprop="encryption">plaintext</li>
        </ul>
    </td>
</tr>
```

**Production Configuration**:
```html
<tr itemscope itemtype="http://rustybeam.net/User">
    <td itemprop="username">admin</td>
    <td itemprop="password">$2b$12$very.long.secure.hash</td>
    <td>
        <ul>
            <li itemprop="role">administrators</li>
        </ul>
    </td>
    <td>
        <ul>
            <li itemprop="encryption">bcrypt</li>
        </ul>
    </td>
</tr>
```

## Troubleshooting

### Common Issues

1. **Plugin Not Loading**:
   ```bash
   # Check plugin file exists and is executable
   ls -la plugins/lib/libbasic_auth.so
   
   # Check server logs for loading errors
   cargo run 2>&1 | grep -i plugin
   ```

2. **Authentication Always Fails**:
   ```bash
   # Verify user file path in configuration
   # Check user file format and encoding
   # Test with known good credentials
   ```

3. **Authorization File Not Found**:
   ```html
   <!-- Ensure authFile path is correct -->
   <td itemprop="authFile">./localhost/auth/users.html</td>
   ```

### Debug Mode

Enable debug logging to troubleshoot authentication issues:

```bash
# Run with debug output
RUST_LOG=debug cargo run
```

### Testing Authentication

```bash
# Test basic auth
curl -v -u admin:admin123 http://localhost:3000/protected.html

# Test without auth (should get 401)
curl -v http://localhost:3000/protected.html

# Test with wrong credentials
curl -v -u admin:wrongpass http://localhost:3000/protected.html
```

## See Also

- [Authorization System](authorization.md) - Access control after authentication
- [User Management](user-management.md) - Managing users and roles
- [Plugin Development](../plugins/writing-plugins.md) - Creating custom auth plugins
- [Security Best Practices](../guides/security.md) - Production security guidelines