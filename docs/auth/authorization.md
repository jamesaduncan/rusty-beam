# Plugin-Based Authorization System

Rusty-beam's authorization system provides fine-grained access control through a plugin-based architecture. After successful authentication, authorization plugins determine whether users can access specific resources, HTML elements, and HTTP methods.

## Overview

Authorization in Rusty-beam operates through a plugin system that supports:

- **Multiple authorization sources** (files, databases, APIs, etc.)
- **Host-specific authorization** plugins
- **Fine-grained permissions** for files, directories, and HTML elements
- **CSS selector-level access control**
- **Role-based and user-specific permissions**

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ Authenticated   │───►│ Authorization   │───►│ Resource Access │
│ User + Roles    │    │ Plugin System   │    │ Granted/Denied  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Plugin Architecture

### Authorization Plugin Interface

Authorization plugins implement the `AuthzPlugin` trait:

```rust
#[async_trait]
pub trait AuthzPlugin: Send + Sync + Debug {
    async fn authorize(&self, request: &AuthzRequest) -> AuthzResult;
    fn name(&self) -> &'static str;
    fn handles_resource(&self, resource: &str) -> bool;
}
```

### Authorization Request

```rust
pub struct AuthzRequest {
    pub user: UserInfo,
    pub resource: String,
    pub method: String,
}

pub struct UserInfo {
    pub username: String,
    pub roles: Vec<String>,
}
```

### Authorization Results

```rust
pub enum AuthzResult {
    Authorized,     // Access granted
    Denied,         // Access denied
    Error(String),  // Plugin error
}
```

## Plugin Configuration

### Host-Specific Authorization

Authorization plugins are configured per host in `config.html`:

```html
<tbody itemprop="host" itemscope itemtype="http://rustybeam.net/HostConfig">
    <tr>
        <td>Host Name</td>
        <td itemprop="hostName">localhost</td>
    </tr>
    <tr>
        <td>Host Root</td>
        <td itemprop="hostRoot">./examples/localhost</td>
    </tr>
    <!-- Authentication plugin -->
    <tr itemprop="plugin" itemscope itemtype="http://rustybeam.net/PluginConfig">
        <td>Authentication Plugin</td>
        <td itemprop="plugin-path">plugins/lib/libbasic_auth.so</td>
    </tr>
    <tr>
        <td>Auth File</td>
        <td itemprop="authFile">./examples/localhost/auth/users.html</td>
    </tr>
    <!-- Authorization automatically loads file-authz plugin if auth config exists -->
</tbody>
```

### Automatic Plugin Loading

When a host has authentication configured, Rusty-beam automatically loads the appropriate authorization plugin:

- **File-based authorization**: `file-authz` plugin for HTML-based rules
- **Future plugins**: Database, LDAP, API-based authorization

## Built-in Authorization Plugins

### 1. File-Based Authorization Plugin (`file-authz`)

The default authorization plugin that reads rules from HTML files.

**Features:**
- HTML microdata configuration
- Wildcard patterns and role-based access
- CSS selector permissions
- Dynamic user variables
- Rule specificity and precedence

**Configuration:**
- Automatically loaded for hosts with `authFile` configuration
- Uses the same HTML file as authentication (users.html)
- No additional configuration required

## Authorization Rules Format

### HTML Microdata Schema

Authorization rules are defined using HTML microdata:

```html
<table id="authorization">
    <thead>
        <tr>
            <td>Username</td>
            <td>Resource</td>
            <td>Method</td>
            <td>Permission</td>
        </tr>
    </thead>
    <tbody>
        <tr itemscope itemtype="http://rustybeam.net/Authorization">
            <td itemprop="username">admin</td>
            <td itemprop="resource">/admin/*</td>
            <td>
                <ul>
                    <li itemprop="method">GET</li>
                    <li itemprop="method">PUT</li>
                    <li itemprop="method">POST</li>
                    <li itemprop="method">DELETE</li>
                </ul>
            </td>
            <td itemprop="permission">allow</td>
        </tr>
    </tbody>
</table>
```

### Rule Schema

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `username` | string | Yes | User or role name, or `*` for all, or `:username` for dynamic |
| `resource` | string | Yes | Resource path pattern with optional selector |
| `method` | string[] | Yes | HTTP methods (GET, PUT, POST, DELETE, or `*`) |
| `permission` | string | Yes | `allow` or `deny` |

## Resource Patterns

### Basic Path Patterns

```html
<!-- Exact path match -->
<td itemprop="resource">/admin/users.html</td>

<!-- Directory wildcard -->
<td itemprop="resource">/admin/*</td>

<!-- Recursive wildcard -->
<td itemprop="resource">/content/**</td>

<!-- File extension pattern -->
<td itemprop="resource">*.html</td>

<!-- Root level files -->
<td itemprop="resource">/*</td>
```

### CSS Selector Resources

Resources can include CSS selectors for element-level permissions:

```html
<!-- Allow access to specific HTML elements -->
<td itemprop="resource">/page.html#(selector=.content)</td>

<!-- Wildcard selector permissions -->
<td itemprop="resource">/*#(selector=.public)</td>

<!-- Admin-only elements -->
<td itemprop="resource">/admin.html#(selector=.admin-panel)</td>

<!-- User-specific content -->
<td itemprop="resource">/users/:username/profile.html#(selector=.personal-info)</td>
```

### Dynamic Variables

Resources support dynamic variables that are replaced at runtime:

```html
<!-- :username is replaced with the authenticated user's username -->
<td itemprop="resource">/users/:username/*</td>

<!-- Multiple variables possible -->
<td itemprop="resource">/tenants/:tenant/users/:username/data/*</td>
```

## User and Role Matching

### Direct User Matching

```html
<!-- Specific user -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">johndoe</td>
    <td itemprop="resource">/users/johndoe/*</td>
    <td><ul><li itemprop="method">GET</li></ul></td>
    <td itemprop="permission">allow</td>
</tr>
```

### Role-based Matching

```html
<!-- Role-based access -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">administrators</td>
    <td itemprop="resource">/admin/*</td>
    <td><ul><li itemprop="method">*</li></ul></td>
    <td itemprop="permission">allow</td>
</tr>
```

### Wildcard Matching

```html
<!-- All users -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">*</td>
    <td itemprop="resource">/*</td>
    <td><ul><li itemprop="method">GET</li></ul></td>
    <td itemprop="permission">allow</td>
</tr>
```

### Dynamic User Variables

```html
<!-- Current user variable -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">:username</td>
    <td itemprop="resource">/users/:username/*</td>
    <td><ul><li itemprop="method">*</li></ul></td>
    <td itemprop="permission">allow</td>
</tr>
```

## HTTP Method Permissions

### Single Method

```html
<td>
    <ul>
        <li itemprop="method">GET</li>
    </ul>
</td>
```

### Multiple Methods

```html
<td>
    <ul>
        <li itemprop="method">GET</li>
        <li itemprop="method">PUT</li>
        <li itemprop="method">POST</li>
    </ul>
</td>
```

### All Methods

```html
<td>
    <ul>
        <li itemprop="method">*</li>
    </ul>
</td>
```

## Rule Evaluation Engine

### Rule Precedence

Rules are evaluated in order of specificity (most specific first):

1. **User specificity**: Exact username > Role name > Wildcard (`*`)
2. **Path specificity**: Exact path > Path segments > Wildcards
3. **Method specificity**: Exact method > Wildcard method
4. **Selector specificity**: Exact selector > No selector

### Evaluation Algorithm

The plugin system evaluates authorization through the following process:

1. **Plugin Selection**: Host-specific authorization plugins are checked first
2. **Resource Matching**: Plugins indicate if they handle the resource
3. **Rule Application**: The first plugin that handles the resource makes the decision
4. **Default Behavior**: If no plugin handles the resource, access is denied

```rust
// Pseudo-code for plugin evaluation
async fn authorize_request(user: &UserInfo, resource: &str, method: &str, host: &str) -> AuthzResult {
    // Check host-specific authorization plugins first
    for plugin in host_authz_plugins.get(host) {
        if plugin.handles_resource(resource) {
            let result = plugin.authorize(&AuthzRequest {
                user: user.clone(),
                resource: resource.to_string(),
                method: method.to_string(),
            }).await;
            return result;
        }
    }
    
    // Check server-wide authorization plugins
    for plugin in server_wide_authz_plugins {
        if plugin.handles_resource(resource) {
            return plugin.authorize(&request).await;
        }
    }
    
    // Default deny if no plugin handles this resource
    AuthzResult::Denied
}
```

## Common Authorization Patterns

### 1. Hierarchical Permissions

```html
<!-- Base rule: Deny all admin access -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">*</td>
    <td itemprop="resource">/admin/*</td>
    <td><ul><li itemprop="method">*</li></ul></td>
    <td itemprop="permission">deny</td>
</tr>

<!-- Override: Allow admins -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">administrators</td>
    <td itemprop="resource">/admin/*</td>
    <td><ul><li itemprop="method">*</li></ul></td>
    <td itemprop="permission">allow</td>
</tr>
```

### 2. Read-Only Access

```html
<!-- Public read access -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">*</td>
    <td itemprop="resource">/*</td>
    <td><ul><li itemprop="method">GET</li></ul></td>
    <td itemprop="permission">allow</td>
</tr>

<!-- Deny write operations for non-editors -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">*</td>
    <td itemprop="resource">/*</td>
    <td>
        <ul>
            <li itemprop="method">PUT</li>
            <li itemprop="method">POST</li>
            <li itemprop="method">DELETE</li>
        </ul>
    </td>
    <td itemprop="permission">deny</td>
</tr>

<!-- Allow editors to write -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">editors</td>
    <td itemprop="resource">/content/*</td>
    <td>
        <ul>
            <li itemprop="method">PUT</li>
            <li itemprop="method">POST</li>
        </ul>
    </td>
    <td itemprop="permission">allow</td>
</tr>
```

### 3. Element-Level Permissions

```html
<!-- Public content is viewable by all -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">*</td>
    <td itemprop="resource">/*#(selector=.public)</td>
    <td><ul><li itemprop="method">GET</li></ul></td>
    <td itemprop="permission">allow</td>
</tr>

<!-- Admin panels only for administrators -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">administrators</td>
    <td itemprop="resource">/*#(selector=.admin-panel)</td>
    <td><ul><li itemprop="method">*</li></ul></td>
    <td itemprop="permission">allow</td>
</tr>
```

## Creating Custom Authorization Plugins

### Plugin Development

Authorization plugins are dynamic libraries that implement the FFI interface:

```c
// Required FFI functions for authorization plugins
extern "C" {
    // Create plugin instance
    void* authz_plugin_create(
        const char** config_keys,
        const char** config_values,
        size_t config_count
    );
    
    // Destroy plugin instance
    void authz_plugin_destroy(void* plugin);
    
    // Authorize a request
    CAuthzResult authz_plugin_authorize(
        void* plugin,
        const CAuthzRequest* request
    );
    
    // Get plugin name
    const char* authz_plugin_name();
    
    // Check if plugin handles resource
    int authz_plugin_handles_resource(
        void* plugin,
        const char* resource
    );
}
```

### Database Authorization Plugin Example

```rust
// Example: PostgreSQL authorization plugin
pub struct PostgresAuthzPlugin {
    pool: PgPool,
}

impl PostgresAuthzPlugin {
    pub fn new(config: &HashMap<String, String>) -> Result<Self, String> {
        let database_url = config.get("database_url")
            .ok_or("database_url required")?;
        
        let pool = PgPool::connect(database_url)
            .await
            .map_err(|e| format!("Database connection failed: {}", e))?;
        
        Ok(PostgresAuthzPlugin { pool })
    }
}

#[async_trait]
impl AuthzPlugin for PostgresAuthzPlugin {
    async fn authorize(&self, request: &AuthzRequest) -> AuthzResult {
        let result = sqlx::query!(
            "SELECT permission FROM authorization_rules 
             WHERE (username = $1 OR username = ANY($2) OR username = '*')
               AND resource_pattern ~ $3
               AND method = ANY($4)
             ORDER BY specificity DESC
             LIMIT 1",
            request.user.username,
            &request.user.roles,
            request.resource,
            &[request.method.clone()]
        )
        .fetch_optional(&self.pool)
        .await;
        
        match result {
            Ok(Some(row)) => {
                if row.permission == "allow" {
                    AuthzResult::Authorized
                } else {
                    AuthzResult::Denied
                }
            }
            Ok(None) => AuthzResult::Denied,
            Err(e) => AuthzResult::Error(format!("Database error: {}", e)),
        }
    }
    
    fn name(&self) -> &'static str {
        "postgres-authz"
    }
    
    fn handles_resource(&self, _resource: &str) -> bool {
        true // Handle all resources
    }
}
```

### Plugin Configuration

Add your custom plugin to the host configuration:

```html
<tbody itemprop="host" itemscope itemtype="http://rustybeam.net/HostConfig">
    <tr>
        <td>Host Name</td>
        <td itemprop="hostName">api.example.com</td>
    </tr>
    <tr>
        <td>Authorization Plugin</td>
        <td itemprop="authz-plugin-path">plugins/lib/libpostgres_authz.so</td>
    </tr>
    <tr>
        <td>Database URL</td>
        <td itemprop="database_url">postgresql://user:pass@localhost/authz_db</td>
    </tr>
</tbody>
```

## Available Authorization Plugins

### 1. File-Based Authorization (`file-authz`)

**Source**: Built-in plugin  
**Description**: HTML microdata-based authorization rules  
**Configuration**: Uses `authFile` path from authentication configuration  
**Best for**: Simple to medium complexity authorization scenarios

### 2. Future Plugin Ideas

**Database Authorization**:
- PostgreSQL, MySQL, SQLite integration
- Complex rule engines with SQL queries
- High-performance authorization for large applications

**LDAP Authorization**:
- Active Directory integration
- Group-based permissions
- Enterprise environments

**API Authorization**:
- External authorization services
- OAuth scopes and permissions
- Microservice architectures

**JWT Authorization**:
- Token-based permissions
- Stateless authorization
- Modern web applications

## Testing Authorization

### Manual Testing

```bash
# Test as admin user
curl -u admin:admin123 -X GET http://localhost:3000/admin/users.html
# Expected: 200 OK

# Test as regular user
curl -u johndoe:password -X GET http://localhost:3000/admin/users.html  
# Expected: 403 Forbidden

# Test selector-specific access
curl -u editor:password -H "Range: selector=.content" http://localhost:3000/article.html
# Expected: 206 Partial Content

# Test method restrictions
curl -u viewer:password -X PUT http://localhost:3000/content/article.html
# Expected: 403 Forbidden
```

### Automated Testing

```bash
#!/bin/bash
# test-authorization.sh

BASE_URL="http://localhost:3000"

# Test admin access
echo "Testing admin access..."
status=$(curl -s -o /dev/null -w "%{http_code}" -u admin:admin123 "$BASE_URL/admin/")
if [ "$status" = "200" ]; then
    echo "✓ Admin access allowed"
else
    echo "✗ Admin access denied (got $status)"
fi

# Test user access to admin area
echo "Testing user access to admin area..."
status=$(curl -s -o /dev/null -w "%{http_code}" -u user:password "$BASE_URL/admin/")
if [ "$status" = "403" ]; then
    echo "✓ User correctly denied admin access"
else
    echo "✗ User incorrectly allowed admin access (got $status)"
fi
```

## Error Responses

### 403 Forbidden

When authorization fails:

```http
HTTP/1.1 403 Forbidden
Content-Type: text/plain
Content-Length: 13

Access denied
```

### 401 Unauthorized

When authentication is required but not provided:

```http
HTTP/1.1 401 Unauthorized
WWW-Authenticate: Basic realm="Rusty Beam"
Content-Type: text/plain
Content-Length: 23

Authentication required
```

### 500 Internal Server Error

When authorization plugin encounters an error:

```http
HTTP/1.1 500 Internal Server Error
Content-Type: text/plain
Content-Length: 19

Authorization error
```

## Performance Considerations

### Plugin Performance

1. **Host-Specific Plugins**: Only relevant plugins are consulted
2. **Resource Matching**: Plugins can quickly determine if they handle a resource
3. **Rule Caching**: File-based plugin caches parsed rules
4. **Async Operations**: All plugin operations are asynchronous

### Large Rule Sets

For applications with many authorization rules:

- Use database-backed authorization plugins
- Implement rule indexing and caching
- Consider rule compilation for better performance
- Use role-based permissions instead of user-specific rules

## Debugging Authorization

### Enable Debug Logging

```bash
RUST_LOG=debug cargo run
```

### Common Issues

1. **Plugin Not Loading**:
   - Check plugin library path
   - Verify FFI function exports
   - Check configuration syntax

2. **Rules Not Matching**:
   - Verify user roles are correct
   - Check resource path patterns
   - Test rule specificity order

3. **Plugin Errors**:
   - Check plugin logs and error messages
   - Verify plugin configuration
   - Test plugin independently

## Security Best Practices

### 1. Principle of Least Privilege

Start with deny-all policies and grant only necessary permissions:

```html
<!-- Default deny -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">*</td>
    <td itemprop="resource">/*</td>
    <td><ul><li itemprop="method">*</li></ul></td>
    <td itemprop="permission">deny</td>
</tr>

<!-- Grant specific permissions -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">readers</td>
    <td itemprop="resource">/public/*</td>
    <td><ul><li itemprop="method">GET</li></ul></td>
    <td itemprop="permission">allow</td>
</tr>
```

### 2. Plugin Security

- **Validate Plugin Sources**: Only load trusted authorization plugins
- **Input Sanitization**: Plugins should validate all inputs
- **Error Handling**: Fail securely on plugin errors
- **Audit Logging**: Log all authorization decisions

### 3. Regular Review

- Periodically audit authorization rules and plugins
- Test authorization scenarios regularly
- Monitor for unusual access patterns
- Keep plugins updated

## Migration from Legacy System

### Backward Compatibility

The new plugin-based system is fully backward compatible:

- Existing HTML authorization files continue to work
- No configuration changes required
- Same rule format and behavior
- Automatic plugin loading for existing hosts

### Migration Path

1. **Current Setup**: Continue using file-based authorization
2. **Gradual Migration**: Add new hosts with different authorization plugins
3. **Custom Plugins**: Develop organization-specific authorization plugins
4. **Full Migration**: Migrate all hosts to appropriate authorization plugins

## See Also

- [Authentication System](authentication.md) - User identity verification
- [Plugin Architecture](../plugins/architecture.md) - Plugin development guide
- [Resource Selectors](../api/resource-selectors.md) - CSS selector resources
- [Security Guide](../guides/security.md) - Production security practices
- [Configuration](../guides/configuration.md) - Server configuration guide