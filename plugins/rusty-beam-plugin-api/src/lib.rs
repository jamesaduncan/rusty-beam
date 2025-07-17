//! Rusty Beam Plugin API
//!
//! This crate provides the core plugin API for Rusty Beam, enabling dynamic
//! extension of server functionality through a robust plugin system. All Rusty
//! Beam plugins implement the traits and use the types defined in this crate.
//!
//! ## Overview
//!
//! The plugin API provides:
//! - **Plugin Trait**: Core interface that all plugins must implement
//! - **Request/Response Types**: Data structures for plugin communication
//! - **Context System**: Configuration and runtime context for plugins
//! - **FFI Support**: C-compatible interface for dynamic loading
//! - **Async Support**: Full async/await compatibility with Tokio
//! - **Upgrade Handling**: Support for protocol upgrades (WebSocket, etc.)
//!
//! ## Plugin Lifecycle
//!
//! 1. **Loading**: Plugins are loaded dynamically at server startup
//! 2. **Request Phase**: Plugins process requests in pipeline order
//! 3. **Response Phase**: All plugins can modify the response
//! 4. **Upgrade Handling**: Optional protocol upgrade support
//!
//! ## Key Components
//!
//! - `Plugin`: The core trait all plugins implement
//! - `PluginRequest`: Request data passed between plugins
//! - `PluginResponse`: Response with optional upgrade handler
//! - `PluginContext`: Configuration and runtime context
//! - `create_plugin!`: Macro for FFI-compatible plugin creation
//!
//! ## Example Plugin
//!
//! ```rust
//! use rusty_beam_plugin_api::*;
//! use async_trait::async_trait;
//! use std::collections::HashMap;
//!
//! #[derive(Debug)]
//! struct MyPlugin {
//!     name: String,
//! }
//!
//! impl MyPlugin {
//!     pub fn new(config: HashMap<String, String>) -> Self {
//!         Self {
//!             name: config.get("name")
//!                 .cloned()
//!                 .unwrap_or_else(|| "my-plugin".to_string()),
//!         }
//!     }
//! }
//!
//! #[async_trait]
//! impl Plugin for MyPlugin {
//!     async fn handle_request(
//!         &self,
//!         request: &mut PluginRequest,
//!         context: &PluginContext,
//!     ) -> Option<PluginResponse> {
//!         // Process request
//!         None
//!     }
//!
//!     fn name(&self) -> &str {
//!         &self.name
//!     }
//! }
//!
//! // Export the plugin
//! create_plugin!(MyPlugin);
//! ```

use hyper::{Body, Request, Response};
use std::collections::HashMap;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::pin::Pin;
use std::future::Future;

/// Handler for upgraded connections (e.g., WebSocket, HTTP/2)
/// 
/// This type represents a closure that handles protocol upgrades. When a plugin
/// returns a response with status 101 (Switching Protocols), it can provide
/// an upgrade handler to manage the upgraded connection.
/// 
/// The handler receives the upgraded connection and returns a future that
/// completes when the upgraded protocol session ends.
pub type UpgradeHandler = Box<dyn FnOnce(hyper::upgrade::Upgraded) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send>> + Send>;

/// Enhanced plugin response that can optionally handle connection upgrades
/// 
/// This struct represents a plugin's response to a request. It includes the
/// standard HTTP response and an optional upgrade handler for protocol upgrades
/// like WebSocket connections.
/// 
/// # Examples
/// 
/// Basic response without upgrade:
/// ```rust
/// let response = Response::builder()
///     .status(200)
///     .body(Body::from("Hello"))
///     .unwrap();
/// let plugin_response = PluginResponse::from(response);
/// ```
/// 
/// Response with WebSocket upgrade:
/// ```rust
/// let plugin_response = PluginResponse {
///     response: switching_protocols_response,
///     upgrade: Some(Box::new(|upgraded| {
///         Box::pin(async move {
///             // Handle WebSocket connection
///             Ok(())
///         })
///     })),
/// };
/// ```
pub struct PluginResponse {
    /// The HTTP response to send
    pub response: Response<Body>,
    /// Optional handler for connection upgrade
    pub upgrade: Option<UpgradeHandler>,
}

impl From<Response<Body>> for PluginResponse {
    fn from(response: Response<Body>) -> Self {
        PluginResponse {
            response,
            upgrade: None,
        }
    }
}

/// Data that flows between plugins during request processing
/// 
/// This struct contains all the information plugins need to process a request.
/// It includes the original HTTP request, decoded path information, metadata
/// for inter-plugin communication, and a cache for the request body.
/// 
/// The request is passed through the plugin pipeline, allowing each plugin to:
/// - Read request information
/// - Add metadata for downstream plugins
/// - Extract and cache the request body
/// 
/// # Thread Safety
/// 
/// The body cache is protected by a mutex to ensure thread-safe access when
/// multiple plugins need to read the request body.
#[derive(Debug)]
pub struct PluginRequest {
    /// The original HTTP request (boxed to avoid lifetime issues)
    pub http_request: Box<Request<Body>>,
    /// The decoded URI path
    pub path: String,
    /// The canonicalized file system path (if applicable)
    pub canonical_path: Option<String>,
    /// Plugin-to-plugin metadata and state
    pub metadata: HashMap<String, String>,
    /// Cached request body (once extracted)
    pub body_cache: Arc<Mutex<Option<bytes::Bytes>>>,
}

impl PluginRequest {
    /// Create a new plugin request from an HTTP request
    pub fn new(http_request: Request<Body>, path: String) -> Self {
        Self {
            http_request: Box::new(http_request),
            path,
            canonical_path: None,
            metadata: HashMap::new(),
            body_cache: Arc::new(Mutex::new(None)),
        }
    }
    
    /// Get metadata value set by previous plugins
    /// 
    /// # Example
    /// 
    /// ```rust
    /// if let Some(user) = request.get_metadata("authenticated_user") {
    ///     println!("Request from user: {}", user);
    /// }
    /// ```
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|s| s.as_str())
    }
    
    /// Set metadata for downstream plugins
    /// 
    /// # Example
    /// 
    /// ```rust
    /// request.set_metadata("content_type".to_string(), "application/json".to_string());
    /// ```
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
    
    /// Check if metadata key exists
    pub fn has_metadata(&self, key: &str) -> bool {
        self.metadata.contains_key(key)
    }
    
    /// Get request method as a string
    pub fn method(&self) -> &str {
        self.http_request.method().as_str()
    }
    
    /// Check if this is a specific HTTP method
    pub fn is_method(&self, method: &hyper::Method) -> bool {
        self.http_request.method() == method
    }
    
    /// Extract the request body as bytes
    /// 
    /// This method extracts the request body and caches it for subsequent access.
    /// The first call will consume the body from the HTTP request, and subsequent
    /// calls will return the cached result.
    /// 
    /// # Errors
    /// 
    /// Returns an error if the body cannot be read (e.g., connection issues)
    /// 
    /// # Example
    /// 
    /// ```rust
    /// let body_bytes = request.get_body().await?;
    /// println!("Body size: {} bytes", body_bytes.len());
    /// ```
    pub async fn get_body(&mut self) -> Result<bytes::Bytes, String> {
        let mut cache = self.body_cache.lock().await;
        
        // Return cached body if already extracted
        if let Some(cached_body) = cache.as_ref() {
            return Ok(cached_body.clone());
        }
        
        // Extract body from the HTTP request
        let body = std::mem::replace(self.http_request.body_mut(), Body::empty());
        match hyper::body::to_bytes(body).await {
            Ok(bytes) => {
                *cache = Some(bytes.clone());
                Ok(bytes)
            }
            Err(e) => Err(format!("Failed to read request body: {}", e))
        }
    }
    
    /// Get the request body as a UTF-8 string
    pub async fn get_body_string(&mut self) -> Result<String, String> {
        let bytes = self.get_body().await?;
        String::from_utf8(bytes.to_vec())
            .map_err(|_| "Request body is not valid UTF-8".to_string())
    }
}

/// Configuration context available to plugins
/// 
/// This struct provides plugins with access to configuration at multiple levels
/// (plugin, host, server) and runtime information. It implements hierarchical
/// configuration lookup and provides logging utilities.
/// 
/// # Configuration Hierarchy
/// 
/// When looking up configuration values, the context checks in order:
/// 1. Plugin-specific configuration
/// 2. Host-level configuration
/// 3. Server-level configuration
/// 
/// This allows for flexible configuration with sensible defaults.
#[derive(Clone)]
pub struct PluginContext {
    /// Plugin-specific configuration
    pub plugin_config: HashMap<String, String>,
    /// Host-level configuration
    pub host_config: HashMap<String, String>,
    /// Server-level configuration  
    pub server_config: HashMap<String, String>,
    /// Server metadata (e.g., config file path)
    pub server_metadata: HashMap<String, String>,
    /// Host name for this request
    pub host_name: String,
    /// Unique identifier for this request
    pub request_id: String,
    /// Optional Tokio runtime handle for plugins that need async operations
    pub runtime_handle: Option<tokio::runtime::Handle>,
    /// Whether verbose logging is enabled
    pub verbose: bool,
}

impl std::fmt::Debug for PluginContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginContext")
            .field("plugin_config", &self.plugin_config)
            .field("host_config", &self.host_config)
            .field("server_config", &self.server_config)
            .field("server_metadata", &self.server_metadata)
            .field("host_name", &self.host_name)
            .field("request_id", &self.request_id)
            .field("runtime_handle", &self.runtime_handle.is_some())
            .field("verbose", &self.verbose)
            .finish()
    }
}

impl PluginContext {
    /// Get configuration value with hierarchical lookup
    /// 
    /// Searches for configuration in order:
    /// 1. Plugin-specific config
    /// 2. Host-level config  
    /// 3. Server-level config
    /// 
    /// # Example
    /// 
    /// ```rust
    /// if let Some(timeout) = context.get_config("timeout") {
    ///     let timeout_ms: u64 = timeout.parse().unwrap_or(5000);
    /// }
    /// ```
    pub fn get_config(&self, key: &str) -> Option<&str> {
        self.plugin_config.get(key)
            .or_else(|| self.host_config.get(key))
            .or_else(|| self.server_config.get(key))
            .map(|s| s.as_str())
    }
    
    /// Get configuration value with default fallback
    /// 
    /// # Example
    /// 
    /// ```rust
    /// let port = context.get_config_or("port", "8080");
    /// ```
    pub fn get_config_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.get_config(key).unwrap_or(default)
    }
    
    /// Get configuration value and parse to a specific type
    /// 
    /// # Example
    /// 
    /// ```rust
    /// let max_size: usize = context.get_config_parsed("max_size").unwrap_or(1024);
    /// ```
    pub fn get_config_parsed<T: std::str::FromStr>(&self, key: &str) -> Option<T> {
        self.get_config(key).and_then(|v| v.parse().ok())
    }
    
    /// Check if a configuration key exists at any level
    pub fn has_config(&self, key: &str) -> bool {
        self.plugin_config.contains_key(key) ||
        self.host_config.contains_key(key) ||
        self.server_config.contains_key(key)
    }
    
    /// Get the document root path from configuration
    pub fn document_root(&self) -> &str {
        self.get_config_or("document_root", "./")
    }
    
    /// Log a message if verbose mode is enabled
    /// 
    /// # Example
    /// 
    /// ```rust
    /// context.log_verbose("[MyPlugin] Processing request");
    /// ```
    pub fn log_verbose(&self, message: &str) {
        if self.verbose {
            println!("{}", message);
        }
    }
    
    /// Log a formatted message if verbose mode is enabled
    /// 
    /// # Example
    /// 
    /// ```rust
    /// context.log_verbose_fmt(format_args!("[MyPlugin] Status: {}", status));
    /// ```
    pub fn log_verbose_fmt(&self, args: std::fmt::Arguments) {
        if self.verbose {
            println!("{}", args);
        }
    }
    
    /// Log an error message (always logged, regardless of verbose setting)
    pub fn log_error(&self, message: &str) {
        eprintln!("[{}] ERROR: {}", self.request_id, message);
    }
}

/// Core plugin trait that all plugins must implement
/// 
/// This is the fundamental trait that defines a Rusty Beam plugin. Plugins can
/// intercept requests, generate responses, and modify responses from other plugins.
/// 
/// # Plugin Execution Model
/// 
/// 1. **Request Phase**: Plugins are called in order via `handle_request`
///    - First plugin to return `Some(response)` stops the chain
///    - Remaining plugins skip to response phase
/// 
/// 2. **Response Phase**: All plugins see the response via `handle_response`
///    - Plugins can modify headers, add logging, etc.
///    - Cannot replace the response entirely
/// 
/// # Thread Safety
/// 
/// Plugins must be `Send + Sync` as they may be called from multiple threads.
/// Use appropriate synchronization for any shared state.
#[async_trait]
pub trait Plugin: Send + Sync + std::fmt::Debug {
    /// Handle incoming request, optionally generating a response
    /// 
    /// This method is called during the request phase. If a plugin returns
    /// `Some(response)`, the request phase ends and the response phase begins.
    /// 
    /// # Arguments
    /// 
    /// * `request` - Mutable request data that can be modified
    /// * `context` - Configuration and runtime context
    /// 
    /// # Returns
    /// 
    /// * `None` - Continue to next plugin
    /// * `Some(response)` - Stop request phase and begin response phase
    async fn handle_request(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<PluginResponse> {
        // Default implementation does nothing
        let _ = (request, context);
        None
    }
    
    /// Handle response after it's been generated
    /// 
    /// This method is called during the response phase on all plugins that
    /// were part of the request processing pipeline. Plugins can modify the
    /// response headers and observe the response body.
    /// 
    /// # Arguments
    /// 
    /// * `request` - Immutable request data for context
    /// * `response` - Mutable response that can be modified
    /// * `context` - Configuration and runtime context
    async fn handle_response(&self, request: &PluginRequest, response: &mut Response<Body>, context: &PluginContext) {
        // Default implementation does nothing
        let _ = (request, response, context);
    }
    
    /// Plugin name for identification and logging
    /// 
    /// This should return a unique, descriptive name for the plugin.
    /// Used in logging and debugging.
    fn name(&self) -> &str;
}

// FFI Support
// These types and functions enable dynamic loading of plugins via C FFI

use std::ffi::CStr;
use std::os::raw::c_char;

/// Create plugin function signature - all plugins must export this
/// 
/// This is the C-compatible function signature that plugin libraries must
/// export as `create_plugin`. It receives configuration as a JSON string
/// and returns a raw pointer to the plugin instance.
/// 
/// # Safety
/// 
/// The returned pointer must be a valid `Box<Box<dyn Plugin>>` that the
/// caller will own and eventually free.
pub type CreatePluginFn = extern "C" fn(config: *const c_char) -> *mut std::ffi::c_void;

/// Helper function to parse JSON config from C string
/// 
/// Safely converts a C string containing JSON configuration into a HashMap.
/// Used by the `create_plugin!` macro to parse plugin configuration.
/// 
/// # Arguments
/// 
/// * `config_ptr` - Pointer to null-terminated C string containing JSON
/// 
/// # Returns
/// 
/// HashMap of configuration key-value pairs, or empty map on error
/// 
/// # Safety
/// 
/// This function is safe to call with null pointers (returns empty map)
pub fn parse_plugin_config(config_ptr: *const c_char) -> HashMap<String, String> {
    if config_ptr.is_null() {
        return HashMap::new();
    }
    
    unsafe {
        match CStr::from_ptr(config_ptr).to_str() {
            Ok(config_str) => {
                serde_json::from_str(config_str).unwrap_or_else(|e| {
                    eprintln!("[Plugin API] Failed to parse config JSON: {}", e);
                    HashMap::new()
                })
            }
            Err(e) => {
                eprintln!("[Plugin API] Invalid UTF-8 in config string: {}", e);
                HashMap::new()
            }
        }
    }
}

/// Macro to simplify plugin creation and FFI export
/// 
/// This macro generates the required `create_plugin` function that the plugin
/// loader expects. It handles configuration parsing and proper boxing for FFI.
/// 
/// # Usage
/// 
/// Place this at the end of your plugin module:
/// 
/// ```rust
/// create_plugin!(MyPlugin);
/// ```
/// 
/// This expands to a `create_plugin` function that:
/// 1. Parses JSON configuration from C string
/// 2. Creates a new plugin instance
/// 3. Boxes it properly for FFI safety
/// 4. Returns a raw pointer
/// 
/// # Requirements
/// 
/// Your plugin type must:
/// - Implement the `Plugin` trait
/// - Have a `new(config: HashMap<String, String>) -> Self` method
#[macro_export]
macro_rules! create_plugin {
    ($plugin_type:ty) => {
        #[no_mangle]
        pub extern "C" fn create_plugin(config: *const std::os::raw::c_char) -> *mut std::ffi::c_void {
            let config_map = rusty_beam_plugin_api::parse_plugin_config(config);
            let plugin = <$plugin_type>::new(config_map);
            // Box the plugin as a trait object first, then box again for FFI safety
            let boxed: Box<dyn rusty_beam_plugin_api::Plugin> = Box::new(plugin);
            Box::into_raw(Box::new(boxed)) as *mut std::ffi::c_void
        }
    };
}