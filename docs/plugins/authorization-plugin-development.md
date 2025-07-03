# Authorization Plugin Development

This guide covers developing custom authorization plugins for Rusty-beam. Authorization plugins control access to resources after successful authentication.

## Overview

Authorization plugins implement fine-grained access control by:

- **Evaluating permissions** for authenticated users
- **Handling resource patterns** with wildcards and selectors  
- **Supporting role-based access control**
- **Implementing custom authorization logic** from any source

## Plugin Development Process

### 1. Set Up Development Environment

```bash
# Create new Rust project
cargo new my-authz-plugin --lib
cd my-authz-plugin

# Add dependencies to Cargo.toml
[lib]
name = "my_authz_plugin"
crate-type = ["cdylib"]

[dependencies]
libc = "0.2"
# Add other dependencies as needed
```

### 2. Implement the FFI Interface

```rust
use libc::{c_char, c_int, c_void};
use std::collections::HashMap;
use std::ffi::{CStr, CString};

// C-compatible structures
#[repr(C)]
pub struct CUserInfo {
    pub username: *const c_char,
    pub roles: *const *const c_char,
    pub roles_count: usize,
}

#[repr(C)]
pub struct CAuthzRequest {
    pub user: CUserInfo,
    pub resource: *const c_char,
    pub method: *const c_char,
}

#[repr(C)]
pub struct CAuthzResult {
    pub result_type: c_int, // 0=Authorized, 1=Denied, 2=Error
    pub error_message: *const c_char,
}

// Your plugin implementation
pub struct MyAuthzPlugin {
    config: HashMap<String, String>,
    // Add your plugin-specific fields
}

impl MyAuthzPlugin {
    pub fn new(config: &HashMap<String, String>) -> Result<Self, String> {
        // Initialize your plugin with configuration
        Ok(MyAuthzPlugin {
            config: config.clone(),
        })
    }

    pub fn authorize(&self, username: &str, roles: &[String], resource: &str, method: &str) -> Result<bool, String> {
        // Implement your authorization logic here
        // Return Ok(true) for authorized, Ok(false) for denied, Err(_) for errors
        
        // Example: Simple role-based authorization
        if roles.contains(&"admin".to_string()) {
            return Ok(true);
        }
        
        if method == "GET" && resource.starts_with("/public/") {
            return Ok(true);
        }
        
        Ok(false) // Default deny
    }

    pub fn handles_resource(&self, resource: &str) -> bool {
        // Return true if this plugin should handle the resource
        // This allows multiple authorization plugins to coexist
        true // Handle all resources (most common)
    }
}

// FFI exports
static mut PLUGIN_INSTANCES: Vec<Box<MyAuthzPlugin>> = Vec::new();

#[no_mangle]
pub unsafe extern "C" fn authz_plugin_create(
    config_keys: *const *const c_char,
    config_values: *const *const c_char,
    config_count: usize,
) -> *mut c_void {
    let mut config = HashMap::new();
    
    for i in 0..config_count {
        let key_ptr = *config_keys.add(i);
        let value_ptr = *config_values.add(i);
        
        if let (Ok(key), Ok(value)) = (CStr::from_ptr(key_ptr).to_str(), CStr::from_ptr(value_ptr).to_str()) {
            config.insert(key.to_string(), value.to_string());
        }
    }
    
    match MyAuthzPlugin::new(&config) {
        Ok(plugin) => {
            let boxed_plugin = Box::new(plugin);
            let ptr = Box::into_raw(boxed_plugin) as *mut c_void;
            
            // Store in static vector to prevent deallocation
            PLUGIN_INSTANCES.push(Box::from_raw(ptr as *mut MyAuthzPlugin));
            
            ptr
        }
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn authz_plugin_destroy(plugin: *mut c_void) {
    if !plugin.is_null() {
        PLUGIN_INSTANCES.retain(|p| p.as_ref() as *const MyAuthzPlugin != plugin as *const MyAuthzPlugin);
    }
}

#[no_mangle]
pub unsafe extern "C" fn authz_plugin_authorize(
    plugin: *mut c_void,
    request: *const CAuthzRequest,
) -> CAuthzResult {
    if plugin.is_null() || request.is_null() {
        return CAuthzResult {
            result_type: 2, // Error
            error_message: CString::new("Null plugin or request").unwrap().into_raw(),
        };
    }

    let plugin_ref = &*(plugin as *const MyAuthzPlugin);
    let request_ref = &*request;

    // Convert C request to Rust
    let username = match CStr::from_ptr(request_ref.user.username).to_str() {
        Ok(s) => s,
        Err(_) => {
            return CAuthzResult {
                result_type: 2,
                error_message: CString::new("Invalid username").unwrap().into_raw(),
            };
        }
    };

    let resource = match CStr::from_ptr(request_ref.resource).to_str() {
        Ok(s) => s,
        Err(_) => {
            return CAuthzResult {
                result_type: 2,
                error_message: CString::new("Invalid resource").unwrap().into_raw(),
            };
        }
    };

    let method = match CStr::from_ptr(request_ref.method).to_str() {
        Ok(s) => s,
        Err(_) => {
            return CAuthzResult {
                result_type: 2,
                error_message: CString::new("Invalid method").unwrap().into_raw(),
            };
        }
    };

    // Convert roles
    let mut roles = Vec::new();
    for i in 0..request_ref.user.roles_count {
        let role_ptr = *request_ref.user.roles.add(i);
        if let Ok(role) = CStr::from_ptr(role_ptr).to_str() {
            roles.push(role.to_string());
        }
    }

    // Call authorization
    match plugin_ref.authorize(username, &roles, resource, method) {
        Ok(true) => CAuthzResult {
            result_type: 0, // Authorized
            error_message: std::ptr::null(),
        },
        Ok(false) => CAuthzResult {
            result_type: 1, // Denied
            error_message: std::ptr::null(),
        },
        Err(error) => CAuthzResult {
            result_type: 2, // Error
            error_message: CString::new(error).unwrap().into_raw(),
        },
    }
}

#[no_mangle]
pub unsafe extern "C" fn authz_plugin_name() -> *const c_char {
    CString::new("my-authz-plugin").unwrap().into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn authz_plugin_handles_resource(
    plugin: *mut c_void,
    resource: *const c_char,
) -> c_int {
    if plugin.is_null() || resource.is_null() {
        return 0;
    }

    let plugin_ref = &*(plugin as *const MyAuthzPlugin);
    let resource_str = match CStr::from_ptr(resource).to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    if plugin_ref.handles_resource(resource_str) { 1 } else { 0 }
}
```

### 3. Build the Plugin

```bash
# Build as dynamic library
cargo build --release

# The plugin will be in target/release/libmy_authz_plugin.so (or .dylib on macOS)
```

### 4. Configure the Plugin

Add the plugin to your host configuration in `config.html`:

```html
<tbody itemprop="host" itemscope itemtype="http://rustybeam.net/HostConfig">
    <tr>
        <td>Host Name</td>
        <td itemprop="hostName">api.example.com</td>
    </tr>
    <tr>
        <td>Host Root</td>
        <td itemprop="hostRoot">./api-files</td>
    </tr>
    <tr itemprop="plugin" itemscope itemtype="http://rustybeam.net/PluginConfig">
        <td>Authorization Plugin</td>
        <td itemprop="plugin-path">plugins/lib/libmy_authz_plugin.so</td>
    </tr>
    <!-- Plugin-specific configuration -->
    <tr>
        <td>Database URL</td>
        <td itemprop="database_url">postgresql://user:pass@localhost/authz_db</td>
    </tr>
</tbody>
```

## Authorization Patterns

### 1. Role-Based Authorization

```rust
pub fn authorize(&self, username: &str, roles: &[String], resource: &str, method: &str) -> Result<bool, String> {
    // Admin users get full access
    if roles.contains(&"admin".to_string()) {
        return Ok(true);
    }
    
    // Editor role permissions
    if roles.contains(&"editor".to_string()) {
        if method == "GET" || method == "PUT" || method == "POST" {
            return Ok(resource.starts_with("/content/"));
        }
    }
    
    // Viewer role permissions
    if roles.contains(&"viewer".to_string()) {
        if method == "GET" {
            return Ok(resource.starts_with("/public/") || resource.starts_with("/content/"));
        }
    }
    
    Ok(false)
}
```

### 2. Resource Pattern Matching

```rust
use regex::Regex;

pub fn authorize(&self, username: &str, roles: &[String], resource: &str, method: &str) -> Result<bool, String> {
    // Define authorization rules
    let rules = vec![
        (r"^/admin/.*", vec!["admin"], vec!["GET", "PUT", "POST", "DELETE"]),
        (r"^/api/v1/.*", vec!["api-user"], vec!["GET", "POST"]),
        (r"^/users/([^/]+)/.*", vec![], vec!["GET", "PUT"]), // User-specific paths
        (r"^/public/.*", vec![], vec!["GET"]), // Public read access
    ];
    
    for (pattern, required_roles, allowed_methods) in rules {
        let regex = Regex::new(pattern).unwrap();
        
        if regex.is_match(resource) && allowed_methods.contains(&method) {
            // Check if user has required role (empty means any authenticated user)
            if required_roles.is_empty() || roles.iter().any(|role| required_roles.contains(&role.as_str())) {
                return Ok(true);
            }
        }
    }
    
    Ok(false)
}
```

### 3. User-Specific Resources

```rust
pub fn authorize(&self, username: &str, roles: &[String], resource: &str, method: &str) -> Result<bool, String> {
    // Users can access their own resources
    if resource.starts_with(&format!("/users/{}/", username)) {
        return Ok(true);
    }
    
    // Handle dynamic user paths with regex
    let user_path_regex = Regex::new(r"^/users/([^/]+)/.*").unwrap();
    if let Some(captures) = user_path_regex.captures(resource) {
        let resource_owner = &captures[1];
        
        // Users can access their own resources
        if resource_owner == username {
            return Ok(true);
        }
        
        // Admins can access any user resource
        if roles.contains(&"admin".to_string()) {
            return Ok(true);
        }
        
        // Otherwise deny
        return Ok(false);
    }
    
    // Other authorization logic...
    Ok(false)
}
```

### 4. CSS Selector Authorization

```rust
pub fn authorize(&self, username: &str, roles: &[String], resource: &str, method: &str) -> Result<bool, String> {
    // Parse selector from resource
    if let Some(selector_start) = resource.find("#(selector=") {
        let path = &resource[..selector_start];
        let selector_part = &resource[selector_start + 11..];
        
        if let Some(selector_end) = selector_part.find(')') {
            let selector = &selector_part[..selector_end];
            
            // Element-level permissions
            match selector {
                ".public" => return Ok(true), // Public elements
                ".admin-panel" => return Ok(roles.contains(&"admin".to_string())),
                ".user-content" => return Ok(true), // Any authenticated user
                _ => return Ok(false),
            }
        }
    }
    
    // Regular resource authorization...
    Ok(false)
}
```

## Database Integration Examples

### PostgreSQL Authorization Plugin

```rust
use sqlx::{PgPool, Row};

pub struct PostgresAuthzPlugin {
    pool: PgPool,
}

impl PostgresAuthzPlugin {
    pub async fn new(config: &HashMap<String, String>) -> Result<Self, String> {
        let database_url = config.get("database_url")
            .ok_or("database_url required")?;
        
        let pool = PgPool::connect(database_url)
            .await
            .map_err(|e| format!("Database connection failed: {}", e))?;
        
        Ok(PostgresAuthzPlugin { pool })
    }

    pub async fn authorize(&self, username: &str, roles: &[String], resource: &str, method: &str) -> Result<bool, String> {
        let result = sqlx::query(
            "SELECT permission FROM authorization_rules 
             WHERE (username = $1 OR username = ANY($2) OR username = '*')
               AND ($3 ~ resource_pattern OR resource_pattern = '*')
               AND (method = $4 OR method = '*')
             ORDER BY 
               CASE WHEN username = $1 THEN 3
                    WHEN username = ANY($2) THEN 2
                    ELSE 1 END DESC,
               length(resource_pattern) DESC
             LIMIT 1"
        )
        .bind(username)
        .bind(roles)
        .bind(resource)
        .bind(method)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;
        
        match result {
            Some(row) => {
                let permission: String = row.get("permission");
                Ok(permission == "allow")
            }
            None => Ok(false), // Default deny
        }
    }
}
```

### Redis Cache Integration

```rust
use redis::{Client, Commands};

pub struct CachedAuthzPlugin {
    redis_client: Client,
    cache_ttl: u64,
}

impl CachedAuthzPlugin {
    pub fn authorize(&self, username: &str, roles: &[String], resource: &str, method: &str) -> Result<bool, String> {
        let cache_key = format!("authz:{}:{}:{}", username, resource, method);
        
        // Check cache first
        let mut conn = self.redis_client.get_connection()
            .map_err(|e| format!("Redis connection error: {}", e))?;
        
        if let Ok(cached_result) = conn.get::<String, Option<String>>(cache_key.clone()) {
            if let Some(result) = cached_result {
                return Ok(result == "allow");
            }
        }
        
        // Compute authorization
        let authorized = self.compute_authorization(username, roles, resource, method)?;
        
        // Cache the result
        let cache_value = if authorized { "allow" } else { "deny" };
        let _: Result<(), _> = conn.setex(cache_key, self.cache_ttl, cache_value);
        
        Ok(authorized)
    }
    
    fn compute_authorization(&self, username: &str, roles: &[String], resource: &str, method: &str) -> Result<bool, String> {
        // Your actual authorization logic here
        Ok(false)
    }
}
```

## Testing Authorization Plugins

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_access() {
        let config = HashMap::new();
        let plugin = MyAuthzPlugin::new(&config).unwrap();
        
        let result = plugin.authorize(
            "admin_user",
            &vec!["admin".to_string()],
            "/admin/config",
            "GET"
        );
        
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn test_user_specific_resources() {
        let config = HashMap::new();
        let plugin = MyAuthzPlugin::new(&config).unwrap();
        
        // User can access their own resources
        let result = plugin.authorize(
            "johndoe",
            &vec!["user".to_string()],
            "/users/johndoe/profile",
            "GET"
        );
        assert_eq!(result.unwrap(), true);
        
        // User cannot access other user's resources
        let result = plugin.authorize(
            "johndoe",
            &vec!["user".to_string()],
            "/users/janedoe/profile",
            "GET"
        );
        assert_eq!(result.unwrap(), false);
    }

    #[test]
    fn test_public_resources() {
        let config = HashMap::new();
        let plugin = MyAuthzPlugin::new(&config).unwrap();
        
        let result = plugin.authorize(
            "any_user",
            &vec!["user".to_string()],
            "/public/document.html",
            "GET"
        );
        
        assert_eq!(result.unwrap(), true);
    }
}
```

### Integration Tests

```bash
#!/bin/bash
# test-plugin.sh

# Build the plugin
cargo build --release

# Copy to plugins directory
cp target/release/libmy_authz_plugin.so ../plugins/lib/

# Start server with plugin
cd .. && cargo run &
SERVER_PID=$!

sleep 2

# Test authorization scenarios
echo "Testing admin access..."
status=$(curl -s -o /dev/null -w "%{http_code}" -u admin:admin123 "http://localhost:3000/admin/config")
if [ "$status" = "200" ]; then
    echo "✓ Admin access allowed"
else
    echo "✗ Admin access denied (got $status)"
fi

echo "Testing user access to admin..."
status=$(curl -s -o /dev/null -w "%{http_code}" -u user:password "http://localhost:3000/admin/config")
if [ "$status" = "403" ]; then
    echo "✓ User correctly denied admin access"
else
    echo "✗ User incorrectly allowed admin access (got $status)"
fi

echo "Testing public access..."
status=$(curl -s -o /dev/null -w "%{http_code}" -u user:password "http://localhost:3000/public/info.html")
if [ "$status" = "200" ]; then
    echo "✓ Public access allowed"
else
    echo "✗ Public access denied (got $status)"
fi

# Clean up
kill $SERVER_PID
```

## Best Practices

### 1. Security Considerations

```rust
pub fn authorize(&self, username: &str, roles: &[String], resource: &str, method: &str) -> Result<bool, String> {
    // Input validation
    if username.is_empty() || resource.is_empty() || method.is_empty() {
        return Err("Invalid input parameters".to_string());
    }
    
    // Sanitize resource path
    let sanitized_resource = resource.replace("../", "").replace("..\\", "");
    
    // Rate limiting (implement as needed)
    if self.is_rate_limited(username) {
        return Ok(false);
    }
    
    // Audit logging
    self.log_authorization_attempt(username, &sanitized_resource, method);
    
    // Your authorization logic...
    Ok(false)
}
```

### 2. Performance Optimization

```rust
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct OptimizedAuthzPlugin {
    rule_cache: Arc<RwLock<HashMap<String, bool>>>,
    compiled_rules: Vec<CompiledRule>,
}

impl OptimizedAuthzPlugin {
    pub fn authorize(&self, username: &str, roles: &[String], resource: &str, method: &str) -> Result<bool, String> {
        let cache_key = format!("{}:{}:{}:{}", username, roles.join(","), resource, method);
        
        // Check cache
        if let Ok(cache) = self.rule_cache.read() {
            if let Some(&result) = cache.get(&cache_key) {
                return Ok(result);
            }
        }
        
        // Compute authorization
        let result = self.compute_authorization(username, roles, resource, method)?;
        
        // Cache result
        if let Ok(mut cache) = self.rule_cache.write() {
            cache.insert(cache_key, result);
        }
        
        Ok(result)
    }
}
```

### 3. Error Handling

```rust
#[derive(Debug)]
pub enum AuthzError {
    DatabaseError(String),
    ConfigError(String),
    ValidationError(String),
    PermissionDenied,
}

impl std::fmt::Display for AuthzError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AuthzError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            AuthzError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            AuthzError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            AuthzError::PermissionDenied => write!(f, "Permission denied"),
        }
    }
}

pub fn authorize(&self, username: &str, roles: &[String], resource: &str, method: &str) -> Result<bool, AuthzError> {
    // Detailed error handling
    if username.is_empty() {
        return Err(AuthzError::ValidationError("Username cannot be empty".to_string()));
    }
    
    // Your authorization logic with specific error types
    Ok(false)
}
```

## Deployment

### 1. Building for Production

```bash
# Optimize for size and performance
cargo build --release --target x86_64-unknown-linux-gnu

# Strip debug symbols
strip target/x86_64-unknown-linux-gnu/release/libmy_authz_plugin.so

# Verify exports
objdump -T target/x86_64-unknown-linux-gnu/release/libmy_authz_plugin.so | grep authz_plugin
```

### 2. Installation

```bash
# Copy plugin to server
scp target/release/libmy_authz_plugin.so server:/opt/rusty-beam/plugins/lib/

# Update configuration
# Edit config.html to reference the new plugin

# Restart server
systemctl restart rusty-beam
```

### 3. Monitoring

```rust
use std::sync::atomic::{AtomicU64, Ordering};

pub struct MonitoredAuthzPlugin {
    request_count: AtomicU64,
    denied_count: AtomicU64,
    error_count: AtomicU64,
}

impl MonitoredAuthzPlugin {
    pub fn authorize(&self, username: &str, roles: &[String], resource: &str, method: &str) -> Result<bool, String> {
        self.request_count.fetch_add(1, Ordering::Relaxed);
        
        match self.do_authorize(username, roles, resource, method) {
            Ok(true) => Ok(true),
            Ok(false) => {
                self.denied_count.fetch_add(1, Ordering::Relaxed);
                Ok(false)
            }
            Err(e) => {
                self.error_count.fetch_add(1, Ordering::Relaxed);
                Err(e)
            }
        }
    }
    
    pub fn get_stats(&self) -> (u64, u64, u64) {
        (
            self.request_count.load(Ordering::Relaxed),
            self.denied_count.load(Ordering::Relaxed),
            self.error_count.load(Ordering::Relaxed),
        )
    }
}
```

## See Also

- [Authorization System](../auth/authorization.md) - Authorization concepts and usage
- [Plugin Architecture](architecture.md) - Overall plugin architecture
- [Authentication Plugin Development](authentication-plugin-development.md) - Authentication plugins
- [Configuration Guide](../guides/configuration.md) - Server configuration
- [Security Best Practices](../guides/security.md) - Production security guidelines