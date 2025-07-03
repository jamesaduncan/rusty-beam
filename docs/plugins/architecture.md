# Plugin Architecture

Rusty-beam uses a dynamic plugin system for authentication that allows extending functionality without modifying the core server. Plugins are loaded as dynamic libraries (.so, .dylib, .dll) and communicate with the server through a well-defined C FFI interface.

## Overview

The plugin architecture provides:

- **Dynamic Loading**: Plugins are loaded at runtime
- **Language Agnostic**: Any language that can create C-compatible libraries
- **Hot Swappable**: Plugins can be updated without server restart (future feature)
- **Host Isolation**: Different plugins per virtual host
- **Extensible**: Easy to add new authentication methods

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ Rusty-beam      │───►│ Plugin Manager  │───►│ Dynamic Plugin  │
│ Core Server     │    │                 │    │ (.so/.dylib)    │
└─────────────────┘    └─────────────────┘    └─────────────────┘
        │                        │                        │
        │                        │                        │
        ▼                        ▼                        ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ HTTP Request    │    │ Plugin Registry │    │ Authentication  │
│ Processing      │    │                 │    │ Logic           │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Plugin Lifecycle

### 1. Discovery and Loading

```rust
// Server startup
1. Read configuration from config.html
2. Discover plugins from plugin-path entries
3. Load dynamic libraries (.so/.dylib/.dll)
4. Initialize plugins with configuration
5. Register plugins with Plugin Manager
```

### 2. Request Processing

```rust
// Per-request flow
1. Receive HTTP request
2. Determine host and path
3. Check if authentication required (plugin.requires_authentication)
4. If required, call plugin.authenticate()
5. Process authentication result
6. Continue with authorization if authenticated
```

### 3. Plugin Shutdown

```rust
// Server shutdown
1. Call plugin.destroy() for each plugin
2. Unload dynamic libraries
3. Clean up resources
```

## Plugin Interface

### Core Trait (Rust Side)

```rust
#[async_trait]
pub trait AuthPlugin: Send + Sync {
    /// Authenticate a request
    async fn authenticate(&self, req: &Request<Body>) -> AuthResult;
    
    /// Get plugin name
    fn name(&self) -> &'static str;
    
    /// Check if authentication is required for a path
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

### C FFI Interface

Plugins must implement these C-compatible functions:

```c
// Plugin creation and destruction
void* plugin_create(
    const char** config_keys,
    const char** config_values,
    size_t config_count
);

void plugin_destroy(void* plugin);

// Authentication interface
CAuthResult plugin_authenticate(
    void* plugin,
    const CHttpRequest* request
);

// Plugin metadata
const char* plugin_name();

int plugin_requires_auth(void* plugin, const char* path);
```

### Data Structures

```c
// HTTP request representation
typedef struct {
    const char* method;
    const char* uri;
    const char** headers;
    size_t headers_count;
    const char* body;
    size_t body_length;
} CHttpRequest;

// Authentication result
typedef struct {
    int result_type; // 0=Authorized, 1=Unauthorized, 2=Error
    CUserInfo user_info;
    const char* error_message;
} CAuthResult;

// User information
typedef struct {
    const char* username;
    const char** roles;
    size_t roles_count;
} CUserInfo;
```

## Plugin Configuration

### Configuration Format

Plugins are configured in `config.html` using HTML microdata:

```html
<tbody itemprop="host" itemscope itemtype="http://rustybeam.net/HostConfig">
    <!-- Host configuration -->
    <tr>
        <td>Host Name</td>
        <td itemprop="hostName">localhost</td>
    </tr>
    
    <!-- Plugin definition -->
    <tr id="plugin-basic-auth" itemprop="plugin" itemscope itemtype="http://rustybeam.net/PluginConfig">
        <td>Plugin</td>
        <td itemprop="plugin-path">plugins/lib/libbasic_auth.so</td>
    </tr>
    
    <!-- Plugin configuration parameters -->
    <tr>
        <td>Auth File</td>
        <td itemref="plugin-basic-auth" itemprop="authFile">./localhost/auth/users.html</td>
    </tr>
    
    <!-- Additional plugin config -->
    <tr>
        <td>Custom Setting</td>
        <td itemref="plugin-basic-auth" itemprop="customSetting">value</td>
    </tr>
</tbody>
```

### Configuration Parameters

Common configuration parameters:

| Parameter | Description | Example |
|-----------|-------------|---------|
| `plugin-path` | Path to plugin library | `plugins/lib/libbasic_auth.so` |
| `authFile` | User data file | `./localhost/auth/users.html` |
| `clientId` | OAuth client ID | `app.googleusercontent.com` |
| `clientSecret` | OAuth client secret | `secret123` |
| `allowedDomains` | Allowed email domains | `example.com,trusted.org` |

### Host-Specific Plugins

Each host can have different plugins:

```html
<!-- Host 1: Basic Auth -->
<tbody itemprop="host">
    <td itemprop="hostName">localhost</td>
    <tr itemprop="plugin">
        <td itemprop="plugin-path">plugins/lib/libbasic_auth.so</td>
    </tr>
</tbody>

<!-- Host 2: OAuth2 -->
<tbody itemprop="host">
    <td itemprop="hostName">api.example.com</td>
    <tr itemprop="plugin">
        <td itemprop="plugin-path">plugins/lib/libgoogle_oauth2.so</td>
    </tr>
</tbody>
```

## Plugin Manager

### Responsibilities

The Plugin Manager handles:

1. **Plugin Discovery**: Finding plugin libraries
2. **Dynamic Loading**: Loading and initializing plugins
3. **Request Routing**: Directing requests to appropriate plugins
4. **Error Handling**: Managing plugin errors gracefully
5. **Resource Management**: Cleanup and memory management

### Implementation

```rust
pub struct PluginManager {
    host_plugins: HashMap<String, Vec<Box<dyn AuthPlugin>>>,
    server_wide_plugins: Vec<Box<dyn AuthPlugin>>,
}

impl PluginManager {
    pub fn new() -> Self { /* ... */ }
    
    pub fn add_host_plugin(&mut self, host: String, plugin: Box<dyn AuthPlugin>) {
        self.host_plugins.entry(host).or_default().push(plugin);
    }
    
    pub async fn authenticate_request(
        &self, 
        req: &Request<Body>, 
        host: &str, 
        path: &str
    ) -> AuthResult {
        // Check host-specific plugins first
        if let Some(plugins) = self.host_plugins.get(host) {
            for plugin in plugins {
                if plugin.requires_authentication(path) {
                    return plugin.authenticate(req).await;
                }
            }
        }
        
        // Check server-wide plugins
        for plugin in &self.server_wide_plugins {
            if plugin.requires_authentication(path) {
                return plugin.authenticate(req).await;
            }
        }
        
        // No authentication required - anonymous access
        AuthResult::Authorized(UserInfo {
            username: "anonymous".to_string(),
            roles: vec!["anonymous".to_string()],
        })
    }
}
```

## Dynamic Plugin Loading

### Plugin Registry

```rust
pub struct PluginRegistry;

impl PluginRegistry {
    pub fn create_plugin(
        plugin_path: &str, 
        config: &HashMap<String, String>
    ) -> Result<Box<dyn AuthPlugin>, String> {
        // Auto-detect library file
        let library_path = Self::resolve_library_path(plugin_path)?;
        
        // Load dynamic library
        DynamicPluginRegistry::load_plugin(&library_path, config)
    }
    
    fn resolve_library_path(plugin_path: &str) -> Result<String, String> {
        // Check for direct library path
        if plugin_path.contains(".so") || plugin_path.contains(".dylib") || plugin_path.contains(".dll") {
            return Ok(plugin_path.to_string());
        }
        
        // Auto-discovery logic
        let plugin_name = plugin_path.replace("-", "_");
        let candidates = [
            format!("plugins/lib/lib{}.so", plugin_name),
            format!("plugins/lib/lib{}.dylib", plugin_name),
            format!("plugins/lib/{}.dll", plugin_name),
        ];
        
        for candidate in &candidates {
            if Path::new(candidate).exists() {
                return Ok(candidate.clone());
            }
        }
        
        Err(format!("No library found for plugin: {}", plugin_path))
    }
}
```

### Dynamic Library Wrapper

```rust
pub struct DynamicPlugin {
    _library: Library, // Keep library alive
    plugin_instance: *mut c_void,
    destroy_fn: PluginDestroyFn,
    authenticate_fn: PluginAuthenticateFn,
    name_fn: PluginNameFn,
    requires_auth_fn: PluginRequiresAuthFn,
    name_cache: String,
}

impl DynamicPlugin {
    pub fn load(library_path: &str, config: &HashMap<String, String>) -> Result<Self, String> {
        unsafe {
            // Load the dynamic library
            let library = Library::new(library_path)?;
            
            // Get function pointers
            let create_fn: Symbol<PluginCreateFn> = library.get(b"plugin_create")?;
            let destroy_fn: Symbol<PluginDestroyFn> = library.get(b"plugin_destroy")?;
            let authenticate_fn: Symbol<PluginAuthenticateFn> = library.get(b"plugin_authenticate")?;
            let name_fn: Symbol<PluginNameFn> = library.get(b"plugin_name")?;
            let requires_auth_fn: Symbol<PluginRequiresAuthFn> = library.get(b"plugin_requires_auth")?;
            
            // Convert config to C format
            let (key_ptrs, value_ptrs, _strings) = Self::convert_config(config);
            
            // Create plugin instance
            let plugin_instance = create_fn(
                key_ptrs.as_ptr(),
                value_ptrs.as_ptr(),
                config.len()
            );
            
            if plugin_instance.is_null() {
                return Err("Plugin creation failed".to_string());
            }
            
            // Cache plugin name
            let name_ptr = name_fn();
            let name_cstr = CStr::from_ptr(name_ptr);
            let name_cache = name_cstr.to_string_lossy().into_owned();
            
            Ok(DynamicPlugin {
                _library: library,
                plugin_instance,
                destroy_fn: *destroy_fn,
                authenticate_fn: *authenticate_fn,
                name_fn: *name_fn,
                requires_auth_fn: *requires_auth_fn,
                name_cache,
            })
        }
    }
}
```

## Error Handling

### Plugin Loading Errors

```rust
pub enum PluginError {
    LibraryNotFound(String),
    SymbolNotFound(String),
    InitializationFailed(String),
    ConfigurationError(String),
    RuntimeError(String),
}

impl From<PluginError> for String {
    fn from(error: PluginError) -> String {
        match error {
            PluginError::LibraryNotFound(path) => {
                format!("Plugin library not found: {}", path)
            },
            PluginError::SymbolNotFound(symbol) => {
                format!("Required symbol not found: {}", symbol)
            },
            PluginError::InitializationFailed(msg) => {
                format!("Plugin initialization failed: {}", msg)
            },
            PluginError::ConfigurationError(msg) => {
                format!("Plugin configuration error: {}", msg)
            },
            PluginError::RuntimeError(msg) => {
                format!("Plugin runtime error: {}", msg)
            },
        }
    }
}
```

### Graceful Degradation

When plugin loading fails:

1. **Log Error**: Detailed error logging for debugging
2. **Continue Starting**: Server continues without the failed plugin
3. **Fallback Behavior**: Default to no authentication or alternative plugin
4. **Health Monitoring**: Track plugin health and availability

```rust
// Example error handling during startup
for (host_name, host_config) in &CONFIG.hosts {
    for plugin_config in &host_config.plugins {
        match PluginRegistry::create_plugin(&plugin_config.plugin_path, &plugin_config.config) {
            Ok(plugin) => {
                manager.add_host_plugin(host_name.clone(), plugin);
                println!("✓ Loaded plugin: {} for host {}", plugin_config.plugin_path, host_name);
            }
            Err(e) => {
                eprintln!("✗ Failed to load plugin '{}' for host {}: {}", 
                    plugin_config.plugin_path, host_name, e);
                // Continue without this plugin
            }
        }
    }
}
```

## Security Considerations

### Plugin Sandboxing

- **Memory Safety**: Plugins run in the same process but should handle memory carefully
- **Resource Limits**: Consider implementing resource limits for plugins
- **Input Validation**: Validate all data passed to/from plugins
- **Error Isolation**: Plugin errors shouldn't crash the main server

### Trust Model

- **Signed Plugins**: Consider requiring signed plugin libraries in production
- **Plugin Directory**: Restrict plugin loading to specific directories
- **Configuration Validation**: Validate plugin configuration parameters
- **Audit Logging**: Log all plugin operations

## Performance Considerations

### Plugin Efficiency

- **Initialization Cost**: Plugin loading happens at startup
- **Request Overhead**: Authentication happens per request
- **Memory Usage**: Plugins share server memory space
- **Caching**: Implement caching for expensive operations

### Optimization Strategies

```rust
// Cache expensive operations
pub struct CachedPlugin {
    inner: Box<dyn AuthPlugin>,
    cache: Arc<Mutex<HashMap<String, (AuthResult, Instant)>>>,
    cache_ttl: Duration,
}

impl CachedPlugin {
    async fn authenticate(&self, req: &Request<Body>) -> AuthResult {
        let cache_key = self.generate_cache_key(req);
        
        // Check cache first
        if let Some((result, timestamp)) = self.cache.lock().unwrap().get(&cache_key) {
            if timestamp.elapsed() < self.cache_ttl {
                return result.clone();
            }
        }
        
        // Call underlying plugin
        let result = self.inner.authenticate(req).await;
        
        // Cache result
        self.cache.lock().unwrap().insert(cache_key, (result.clone(), Instant::now()));
        
        result
    }
}
```

## Debugging and Monitoring

### Debug Output

Enable debug logging for plugin operations:

```bash
RUST_LOG=debug cargo run
```

### Plugin Health Checks

```rust
pub trait PluginHealth {
    fn health_check(&self) -> PluginHealthStatus;
}

pub enum PluginHealthStatus {
    Healthy,
    Degraded(String),
    Unhealthy(String),
}
```

### Metrics Collection

```rust
pub struct PluginMetrics {
    pub authentication_requests: u64,
    pub successful_authentications: u64,
    pub failed_authentications: u64,
    pub average_response_time: Duration,
    pub error_count: u64,
}
```

## Built-in Plugins

### 1. Basic Authentication Plugin

- **Path**: `plugins/lib/libbasic_auth.so`
- **Type**: HTTP Basic Authentication
- **Configuration**: User file path
- **Features**: bcrypt/plaintext passwords, role support

### 2. Google OAuth2 Plugin

- **Path**: `plugins/lib/libgoogle_oauth2.so`
- **Type**: Google OAuth2
- **Configuration**: Client ID, secret, allowed domains
- **Features**: Domain filtering, automatic user provisioning

## Future Enhancements

### Planned Features

1. **Hot Reloading**: Reload plugins without server restart
2. **Plugin Versioning**: Support multiple plugin versions
3. **Plugin Dependencies**: Declare and manage plugin dependencies
4. **Plugin Marketplace**: Central repository for community plugins
5. **Advanced Caching**: Distributed caching for plugin results

### Extension Points

1. **Authorization Plugins**: Custom authorization logic
2. **Logging Plugins**: Custom audit and monitoring
3. **Transformation Plugins**: Content manipulation
4. **Storage Plugins**: Alternative backends

## See Also

- [Writing Plugins](writing-plugins.md) - Creating custom plugins
- [Basic Auth Plugin](basic-auth.md) - HTTP Basic Authentication
- [Google OAuth2 Plugin](google-oauth2.md) - Google OAuth2 integration
- [Authentication System](../auth/authentication.md) - Authentication overview