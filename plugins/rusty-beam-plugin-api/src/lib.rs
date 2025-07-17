use hyper::{Body, Request, Response};
use std::collections::HashMap;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::pin::Pin;
use std::future::Future;

/// Handler for upgraded connections (e.g., WebSocket)
pub type UpgradeHandler = Box<dyn FnOnce(hyper::upgrade::Upgraded) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send>> + Send>;

/// Enhanced plugin response that can optionally handle connection upgrades
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
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|s| s.as_str())
    }
    
    /// Set metadata for downstream plugins
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
    
    /// Extract the request body as bytes
    /// This can only be called once per request, subsequent calls return cached result
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
            Err(_) => Err("Failed to read request body".to_string())
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
    /// Plugin config -> Host config -> Server config
    pub fn get_config(&self, key: &str) -> Option<&str> {
        self.plugin_config.get(key)
            .or_else(|| self.host_config.get(key))
            .or_else(|| self.server_config.get(key))
            .map(|s| s.as_str())
    }
    
    /// Get configuration value with default fallback
    pub fn get_config_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.get_config(key).unwrap_or(default)
    }
    
    /// Log a message if verbose mode is enabled
    pub fn log_verbose(&self, message: &str) {
        if self.verbose {
            println!("{}", message);
        }
    }
    
    /// Log a formatted message if verbose mode is enabled
    pub fn log_verbose_fmt(&self, args: std::fmt::Arguments) {
        if self.verbose {
            println!("{}", args);
        }
    }
}

/// Core plugin trait that all plugins must implement
#[async_trait]
pub trait Plugin: Send + Sync + std::fmt::Debug {
    /// Handle incoming request, optionally generating a response
    /// If this returns Some(response), the request phase stops and response phase begins
    /// The response can optionally include an upgrade handler for protocol upgrades
    async fn handle_request(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<PluginResponse> {
        // Default implementation does nothing
        let _ = (request, context);
        None
    }
    
    /// Handle response after it's been generated
    /// This is called on all remaining plugins after a response is generated
    /// Plugins can modify the response but cannot replace it entirely
    async fn handle_response(&self, request: &PluginRequest, response: &mut Response<Body>, context: &PluginContext) {
        // Default implementation does nothing
        let _ = (request, response, context);
    }
    
    /// Plugin name for identification and logging
    fn name(&self) -> &str;
}

/// C FFI interface for plugins
use std::ffi::CStr;
use std::os::raw::c_char;

/// Create plugin function signature - all plugins must export this
/// Returns a raw pointer to a trait object (Box<dyn Plugin>)
pub type CreatePluginFn = extern "C" fn(config: *const c_char) -> *mut std::ffi::c_void;

/// Helper function to parse JSON config from C string
pub fn parse_plugin_config(config_ptr: *const c_char) -> HashMap<String, String> {
    if config_ptr.is_null() {
        return HashMap::new();
    }
    
    unsafe {
        let config_str = CStr::from_ptr(config_ptr).to_str().unwrap_or("{}");
        serde_json::from_str(config_str).unwrap_or_else(|_| HashMap::new())
    }
}

/// Macro to simplify plugin creation
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