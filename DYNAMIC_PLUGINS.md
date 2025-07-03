# Dynamic Plugin System

Rusty-beam uses a pure dynamic plugin system with shared libraries (`.so`, `.dylib`, `.dll`). All plugins must be compiled as dynamic libraries and loaded at runtime.

## Loading Plugins

The system supports two ways to load plugins:

### 1. Direct Library Path
Load plugins by specifying the full path to the compiled library:
```html
<td itemprop="pluginPath">plugins/lib/libbasic_auth.so</td>
```

### 2. Auto-Discovery
Specify a plugin name and let the system find the corresponding library:
```html
<td itemprop="pluginPath">./plugins/basic-auth</td>
```
This will automatically look for:
- `plugins/lib/libbasic_auth.so` (Linux)
- `plugins/lib/libbasic_auth.dylib` (macOS)
- `plugins/lib/basic_auth.dll` (Windows)

## Building Plugins

### 1. Build All Plugins
```bash
./build-plugins.sh
```

### 2. Build Individual Plugins
```bash
# Basic Auth Plugin
cd plugins/basic-auth
cargo build --release

# Google OAuth2 Plugin  
cd plugins/google-oauth2
cargo build --release
```

Built libraries will be available in `plugins/lib/`:
- `libbasic_auth.so` - Basic HTTP Authentication
- `libgoogle_oauth2.so` - Google OAuth2 Authentication

## Plugin Development

### Creating a New Plugin

1. **Create Plugin Directory**
```bash
mkdir -p plugins/my-plugin/src
```

2. **Create Cargo.toml**
```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
libc = "0.2"
# Add other dependencies as needed
```

3. **Implement Plugin Interface**
Your plugin must export these C functions:

```rust
#[no_mangle]
pub unsafe extern "C" fn plugin_create(
    config_keys: *const *const c_char,
    config_values: *const *const c_char,
    config_count: usize,
) -> *mut std::ffi::c_void {
    // Initialize plugin with configuration
    // Return non-null pointer on success
}

#[no_mangle]
pub unsafe extern "C" fn plugin_destroy(plugin: *mut std::ffi::c_void) {
    // Clean up plugin resources
}

#[no_mangle]
pub unsafe extern "C" fn plugin_authenticate(
    plugin: *mut std::ffi::c_void,
    request: *const CHttpRequest,
) -> CAuthResult {
    // Perform authentication
    // Return CAuthResult with appropriate status
}

#[no_mangle]
pub unsafe extern "C" fn plugin_name() -> *const c_char {
    // Return plugin name as null-terminated C string
}

#[no_mangle]
pub unsafe extern "C" fn plugin_requires_auth(
    plugin: *mut std::ffi::c_void,
    path: *const c_char,
) -> c_int {
    // Return 1 if path requires authentication, 0 otherwise
}
```

### C Data Structures

Plugins must use these C-compatible structures:

```rust
#[repr(C)]
pub struct CAuthResult {
    pub result_type: c_int, // 0 = Authorized, 1 = Unauthorized, 2 = Error
    pub user_info: CUserInfo,
    pub error_message: *const c_char,
}

#[repr(C)]
pub struct CUserInfo {
    pub username: *const c_char,
    pub roles: *const *const c_char,
    pub roles_count: usize,
}

#[repr(C)]
pub struct CHttpRequest {
    pub method: *const c_char,
    pub uri: *const c_char,
    pub headers: *const *const c_char,
    pub headers_count: usize,
    pub body: *const c_char,
    pub body_length: usize,
}
```

## Configuration Example

See `config.html` for a complete configuration example. The configuration shows:

- How to specify plugin paths (either direct library paths or plugin names for auto-discovery)
- How to pass configuration parameters to plugins
- How to configure multiple hosts with different plugins

## Benefits

1. **True Plugin Architecture** - No plugins are compiled into the server binary
2. **Independent Development** - Plugins can be developed and versioned separately
3. **Runtime Loading** - No need to recompile the server for new plugins
4. **Language Flexibility** - Any language that can export C functions can create plugins
5. **Distribution** - Plugins can be distributed as binary libraries
6. **Security** - Plugin code runs in separate memory space
7. **Performance** - Native code performance, no FFI overhead after loading
8. **Smaller Server Binary** - Core server is lightweight without embedded plugins