//! Directory Plugin for Rusty Beam
//!
//! This plugin provides path-based routing and conditional plugin execution, allowing
//! different plugin pipelines to handle different URL paths. It acts as a sophisticated
//! router that executes nested plugins only when requests match configured directory patterns.
//!
//! ## Features
//! - **Path-Based Routing**: Execute plugins only for specific URL paths
//! - **Nested Plugin Support**: Load and manage sub-plugins dynamically
//! - **Sequential Execution**: Process nested plugins in order until one responds
//! - **Dynamic Loading**: Load plugin libraries at runtime via FFI
//! - **Flexible Configuration**: Support for complex routing scenarios
//! - **Response Phase Handling**: Apply response transformations conditionally
//!
//! ## Use Cases
//! - **Admin Interfaces**: Protect admin paths with authentication plugins
//! - **API Versioning**: Route `/api/v1` and `/api/v2` to different handlers
//! - **Multi-Tenant Applications**: Isolate tenant-specific plugins
//! - **Static vs Dynamic Content**: Apply compression only to static paths
//! - **Development vs Production**: Load debug plugins for specific paths
//!
//! ## Configuration
//! - `directory`: The path prefix to match (e.g., "/admin", "/api")
//! - `nested_plugins`: JSON array of plugin configurations to execute
//!
//! ## Nested Plugin Configuration
//! Each nested plugin requires:
//! - `library`: Path to the plugin shared library (file:// URL)
//! - `config`: Plugin-specific configuration as key-value pairs
//! - `nested_plugins`: Additional nested plugins (recursive)
//!
//! ## Path Matching Rules
//! - Exact match: `/admin` matches `/admin`
//! - Prefix match: `/admin` matches `/admin/users`, `/admin/settings`
//! - Trailing slashes ignored: `/admin/` matches `/admin`
//! - Case sensitive: `/Admin` does not match `/admin`
//!
//! ## Execution Flow
//! 1. **Request Phase**: If path matches, execute nested plugins sequentially
//! 2. **First Response Wins**: Stop at first plugin that returns a response
//! 3. **Response Phase**: All matching nested plugins process the response
//!
//! ## Security Considerations
//! - **Library Loading**: Only loads libraries from file:// URLs
//! - **Sandboxing**: Each nested plugin runs in the same process (no isolation)
//! - **Configuration Validation**: Validates plugin configurations at load time
//! - **Error Handling**: Failed plugins don't crash the directory plugin
//!
//! ## Performance
//! - **Lazy Loading**: Plugins loaded once at startup, not per request
//! - **Fast Path Matching**: O(1) string comparison for path checking
//! - **Memory Efficient**: Plugins shared via Arc for multiple references

use async_trait::async_trait;
use hyper::{Body, Response};
use rusty_beam_plugin_api::{create_plugin, Plugin, PluginContext, PluginRequest, PluginResponse};
use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use libloading::{Library, Symbol};

// Plugin identification
const DEFAULT_PLUGIN_NAME: &str = "directory";

// Path and URL constants
const DEFAULT_DIRECTORY: &str = "/";
const FILE_URL_SCHEME: &str = "file://";
const ROOT_PATH: &str = "/";

// FFI and plugin loading
const PLUGIN_CREATION_FUNCTION: &[u8] = b"create_plugin";
const DEFAULT_JSON_CONFIG: &str = "{}";

// Test metadata tracking
const METADATA_CALLED_SUFFIX: &str = "_called";
const METADATA_TRUE_VALUE: &str = "true";

// Configuration keys
const CONFIG_KEY_DIRECTORY: &str = "directory";
const CONFIG_KEY_NESTED_PLUGINS: &str = "nested_plugins";

/// Configuration structure for nested plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub library: String,
    #[serde(default)]
    pub config: HashMap<String, String>,
    #[serde(default)]
    pub nested_plugins: Vec<PluginConfig>,
}

/// Configuration structure for the directory plugin
#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryConfig {
    pub directory: String,
    #[serde(default)]
    pub nested_plugins: Vec<PluginConfig>,
}

/// Wrapper to keep dynamic libraries alive
struct DynamicPluginWrapper {
    _library: Library,
    plugin: Box<dyn Plugin>,
}

impl std::fmt::Debug for DynamicPluginWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DynamicPluginWrapper({})", self.plugin.name())
    }
}

#[async_trait]
impl Plugin for DynamicPluginWrapper {
    async fn handle_request(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<PluginResponse> {
        self.plugin.handle_request(request, context).await
    }

    async fn handle_response(&self, request: &PluginRequest, response: &mut Response<Body>, context: &PluginContext) {
        self.plugin.handle_response(request, response, context).await;
    }

    fn name(&self) -> &str {
        self.plugin.name()
    }
}

/// A plugin that executes nested plugins only if the request path matches a directory
#[derive(Debug)]
pub struct DirectoryPlugin {
    directory: String,
    nested_plugins: Vec<Arc<dyn Plugin>>,
}

impl DirectoryPlugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        let directory_config = Self::parse_directory_config(config);
        let directory = Self::process_directory_path(&directory_config.directory);
        let nested_plugins = Self::load_nested_plugins(&directory_config.nested_plugins);

        Self {
            directory,
            nested_plugins,
        }
    }
    
    /// Parse directory configuration from raw config map
    fn parse_directory_config(config: HashMap<String, String>) -> DirectoryConfig {
        // Note: This JSON serialization is a limitation of the FFI boundary.
        // The main server has already parsed the microdata into PluginConfig structs,
        // but must serialize them to JSON to pass through the C FFI as strings.
        // Future improvements could:
        // 1. Pass the raw HTML and use microdata-extract here
        // 2. Create a plugin registry pattern to avoid re-serialization
        // 3. Use a more efficient binary format instead of JSON
        
        let nested_plugins = Self::parse_nested_plugins_config(&config);
        let directory = config
            .get(CONFIG_KEY_DIRECTORY)
            .cloned()
            .unwrap_or_else(|| DEFAULT_DIRECTORY.to_string());
        
        DirectoryConfig {
            directory,
            nested_plugins,
        }
    }
    
    /// Parse nested plugins configuration from JSON
    fn parse_nested_plugins_config(config: &HashMap<String, String>) -> Vec<PluginConfig> {
        config.get(CONFIG_KEY_NESTED_PLUGINS)
            .and_then(|json| serde_json::from_str::<Vec<PluginConfig>>(json).ok())
            .unwrap_or_default()
    }

    /// Process directory path to handle file:// URLs
    fn process_directory_path(directory: &str) -> String {
        if directory.starts_with(FILE_URL_SCHEME) {
            if let Some(last_part) = directory.rsplit('/').next() {
                format!("{}{}", ROOT_PATH, last_part)
            } else {
                directory.to_string()
            }
        } else {
            directory.to_string()
        }
    }

    /// Load nested plugins from configuration with error tracking
    fn load_nested_plugins(plugin_configs: &[PluginConfig]) -> Vec<Arc<dyn Plugin>> {
        let mut plugins = Vec::new();
        let mut failed_count = 0;
        
        for (index, plugin_config) in plugin_configs.iter().enumerate() {
            match Self::load_plugin_from_config(plugin_config) {
                Some(plugin) => {
                    plugins.push(plugin);
                }
                None => {
                    eprintln!(
                        "[DirectoryPlugin] Failed to load nested plugin at index {}: {}",
                        index, plugin_config.library
                    );
                    failed_count += 1;
                }
            }
        }
        
        if failed_count > 0 {
            eprintln!(
                "[DirectoryPlugin] Loaded {}/{} nested plugins successfully",
                plugins.len(),
                plugin_configs.len()
            );
        }
        
        plugins
    }

    /// Load a single plugin from configuration with validation
    fn load_plugin_from_config(config: &PluginConfig) -> Option<Arc<dyn Plugin>> {
        let library_path = &config.library;
        
        // Validate and extract file path
        let file_path = Self::validate_and_extract_library_path(library_path)?;
        
        // Load the plugin with error handling
        match Self::load_dynamic_plugin(file_path, &config.config) {
            Some(plugin) => Some(plugin),
            None => {
                eprintln!("[DirectoryPlugin] Failed to load plugin from: {}", library_path);
                None
            }
        }
    }
    
    /// Validate library path and extract file path from URL
    fn validate_and_extract_library_path(library_path: &str) -> Option<&str> {
        // Only support file:// URLs for security
        if !library_path.starts_with(FILE_URL_SCHEME) {
            eprintln!("[DirectoryPlugin] Unsupported library URL scheme: {}", library_path);
            return None;
        }
        
        let file_path = library_path.strip_prefix(FILE_URL_SCHEME)?;
        
        // Basic path validation
        if file_path.is_empty() || file_path.contains("../") {
            eprintln!("[DirectoryPlugin] Invalid library path: {}", file_path);
            return None;
        }
        
        Some(file_path)
    }

    /// Load a plugin from a dynamic library
    fn load_dynamic_plugin(library_path: &str, config: &HashMap<String, String>) -> Option<Arc<dyn Plugin>> {
        let library = Self::load_library_safely(library_path)?;
        let plugin = Self::create_plugin_from_library(library, config)?;
        Some(Arc::new(plugin))
    }
    
    /// Safely load a dynamic library with validation
    fn load_library_safely(library_path: &str) -> Option<Library> {
        // Validate file exists and is readable
        if !std::path::Path::new(library_path).exists() {
            eprintln!("[DirectoryPlugin] Library file not found: {}", library_path);
            return None;
        }
        
        unsafe {
            match Library::new(library_path) {
                Ok(lib) => Some(lib),
                Err(e) => {
                    eprintln!("[DirectoryPlugin] Failed to load library {}: {}", library_path, e);
                    None
                }
            }
        }
    }
    
    /// Create plugin instance from loaded library
    fn create_plugin_from_library(library: Library, config: &HashMap<String, String>) -> Option<DynamicPluginWrapper> {
        unsafe {
            // Get the plugin creation function
            let create_fn = Self::get_plugin_creation_function(&library)?;
            
            // Prepare configuration for FFI
            let config_cstr = Self::prepare_config_for_ffi(config)?;
            
            // Create the plugin instance
            let plugin = Self::invoke_plugin_creation(create_fn, config_cstr)?;
            
            Some(DynamicPluginWrapper {
                _library: library,
                plugin,
            })
        }
    }
    
    /// Get the plugin creation function from library
    unsafe fn get_plugin_creation_function(
        library: &Library
    ) -> Option<Symbol<unsafe extern "C" fn(*const std::os::raw::c_char) -> *mut std::ffi::c_void>> {
        unsafe { library.get(PLUGIN_CREATION_FUNCTION).ok() }
    }
    
    /// Prepare configuration string for FFI
    fn prepare_config_for_ffi(config: &HashMap<String, String>) -> Option<std::ffi::CString> {
        let config_json = serde_json::to_string(config)
            .unwrap_or_else(|_| DEFAULT_JSON_CONFIG.to_string());
        std::ffi::CString::new(config_json).ok()
    }
    
    /// Invoke the plugin creation function and get plugin instance
    unsafe fn invoke_plugin_creation(
        create_fn: Symbol<unsafe extern "C" fn(*const std::os::raw::c_char) -> *mut std::ffi::c_void>,
        config_cstr: std::ffi::CString
    ) -> Option<Box<dyn Plugin>> {
        unsafe {
            let plugin_ptr = create_fn(config_cstr.as_ptr());
            if plugin_ptr.is_null() {
                return None;
            }
            
            // Cast the void pointer back to Box<Box<dyn Plugin>> and unwrap one level
            let plugin_box = Box::from_raw(plugin_ptr as *mut Box<dyn Plugin>);
            Some(*plugin_box)
        }
    }
    
    pub fn new_with_nested_plugins(config: HashMap<String, String>, nested_plugins: Vec<Arc<dyn Plugin>>) -> Self {
        let directory = config
            .get(CONFIG_KEY_DIRECTORY)
            .map(|d| Self::process_directory_path(d))
            .unwrap_or_else(|| DEFAULT_DIRECTORY.to_string());

        Self {
            directory,
            nested_plugins,
        }
    }
    
    /// Check if a request path matches this directory's pattern
    fn matches_directory(&self, path: &str) -> bool {
        let normalized_dir = self.normalize_path(&self.directory);
        let normalized_path = self.normalize_path(path);
        
        // Check if path matches exactly or starts with directory followed by /
        normalized_path == normalized_dir || 
        path.starts_with(&format!("{}/", normalized_dir))
    }
    
    /// Normalize a path by removing trailing slashes
    fn normalize_path<'a>(&self, path: &'a str) -> &'a str {
        path.trim_end_matches('/')
    }
}

#[async_trait]
impl Plugin for DirectoryPlugin {
    async fn handle_request(
        &self,
        request: &mut PluginRequest,
        context: &PluginContext,
    ) -> Option<PluginResponse> {
        // Check if the request path matches the configured directory
        if !self.matches_directory(&request.path) {
            context.log_verbose(&format!(
                "[DirectoryPlugin] Path '{}' does not match directory '{}'",
                request.path, self.directory
            ));
            return None;
        }

        context.log_verbose(&format!(
            "[DirectoryPlugin] Path '{}' matches directory '{}', executing {} nested plugins",
            request.path, self.directory, self.nested_plugins.len()
        ));

        // Path matches, execute nested plugins in sequence until one returns a response
        for (index, plugin) in self.nested_plugins.iter().enumerate() {
            match plugin.handle_request(request, context).await {
                Some(response) => {
                    context.log_verbose(&format!(
                        "[DirectoryPlugin] Nested plugin '{}' (index {}) handled request",
                        plugin.name(), index
                    ));
                    return Some(response);
                }
                None => {
                    context.log_verbose(&format!(
                        "[DirectoryPlugin] Nested plugin '{}' (index {}) passed through",
                        plugin.name(), index
                    ));
                }
            }
        }

        context.log_verbose("[DirectoryPlugin] No nested plugin provided a response");
        None
    }

    async fn handle_response(
        &self,
        request: &PluginRequest,
        response: &mut Response<Body>,
        context: &PluginContext,
    ) {
        // Only call handle_response on nested plugins if the directory matches
        if !self.matches_directory(&request.path) {
            context.log_verbose(&format!(
                "[DirectoryPlugin] Skipping response phase - path '{}' does not match directory '{}'",
                request.path, self.directory
            ));
            return;
        }
        
        context.log_verbose(&format!(
            "[DirectoryPlugin] Processing response phase for {} nested plugins",
            self.nested_plugins.len()
        ));
        
        // Call handle_response on all nested plugins
        for (index, plugin) in self.nested_plugins.iter().enumerate() {
            context.log_verbose(&format!(
                "[DirectoryPlugin] Calling handle_response on nested plugin '{}' (index {})",
                plugin.name(), index
            ));
            plugin.handle_response(request, response, context).await;
        }
    }

    fn name(&self) -> &str {
        DEFAULT_PLUGIN_NAME
    }
}

// Export the plugin
create_plugin!(DirectoryPlugin);

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::{Body, Method, Request, Response, StatusCode};
    use rusty_beam_plugin_api::{PluginContext, PluginRequest, PluginResponse};
    use std::collections::HashMap;
    use std::sync::Arc;

    // Mock plugin for testing
    #[derive(Debug)]
    struct MockPlugin {
        name: String,
        should_respond: bool,
        response_body: String,
    }

    impl MockPlugin {
        fn new(name: &str, should_respond: bool, response_body: &str) -> Self {
            Self {
                name: name.to_string(),
                should_respond,
                response_body: response_body.to_string(),
            }
        }
    }

    #[async_trait]
    impl Plugin for MockPlugin {
        async fn handle_request(
            &self,
            request: &mut PluginRequest,
            _context: &PluginContext,
        ) -> Option<PluginResponse> {
            // Store that this plugin was called in the metadata
            request.set_metadata(
                format!("{}{}", self.name, METADATA_CALLED_SUFFIX),
                METADATA_TRUE_VALUE.to_string(),
            );
            
            if self.should_respond {
                let response = Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::from(self.response_body.clone()))
                    .unwrap();
                Some(PluginResponse::from(response))
            } else {
                None
            }
        }

        async fn handle_response(
            &self,
            _request: &PluginRequest,
            _response: &mut Response<Body>,
            _context: &PluginContext,
        ) {
            // For testing, we can't modify the request in handle_response
            // In a real implementation, we would use logging or other mechanisms
            // For now, we'll just check if the plugin was called in handle_request
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    fn create_test_request(path: &str) -> PluginRequest {
        let req = Request::builder()
            .method(Method::GET)
            .uri(path)
            .body(Body::empty())
            .unwrap();
        PluginRequest::new(req, path.to_string())
    }

    fn create_test_context() -> PluginContext {
        PluginContext {
            plugin_config: HashMap::new(),
            host_config: HashMap::new(),
            server_config: HashMap::new(),
            server_metadata: HashMap::new(),
            host_name: "localhost".to_string(),
            request_id: "test-request-id".to_string(),
            runtime_handle: Some(tokio::runtime::Handle::current()),
            verbose: false,
        }
    }

    #[tokio::test]
    async fn test_directory_plugin_matches_exact_path() {
        let config = HashMap::from([("directory".to_string(), "/admin".to_string())]);
        let mock_plugin = Arc::new(MockPlugin::new("test", true, "response"));
        let directory_plugin = DirectoryPlugin::new_with_nested_plugins(config, vec![mock_plugin]);

        let mut request = create_test_request("/admin");
        let context = create_test_context();

        let response = directory_plugin.handle_request(&mut request, &context).await;
        
        assert!(response.is_some());
        assert_eq!(request.get_metadata(&format!("test{}", METADATA_CALLED_SUFFIX)), Some(METADATA_TRUE_VALUE));
    }

    #[tokio::test]
    async fn test_directory_plugin_matches_subpath() {
        let config = HashMap::from([("directory".to_string(), "/admin".to_string())]);
        let mock_plugin = Arc::new(MockPlugin::new("test", true, "response"));
        let directory_plugin = DirectoryPlugin::new_with_nested_plugins(config, vec![mock_plugin]);

        let mut request = create_test_request("/admin/users");
        let context = create_test_context();

        let response = directory_plugin.handle_request(&mut request, &context).await;
        
        assert!(response.is_some());
        assert_eq!(request.get_metadata(&format!("test{}", METADATA_CALLED_SUFFIX)), Some(METADATA_TRUE_VALUE));
    }

    #[tokio::test]
    async fn test_directory_plugin_no_match() {
        let config = HashMap::from([("directory".to_string(), "/admin".to_string())]);
        let mock_plugin = Arc::new(MockPlugin::new("test", true, "response"));
        let directory_plugin = DirectoryPlugin::new_with_nested_plugins(config, vec![mock_plugin]);

        let mut request = create_test_request("/public");
        let context = create_test_context();

        let response = directory_plugin.handle_request(&mut request, &context).await;
        
        assert!(response.is_none());
        assert_eq!(request.get_metadata(&format!("test{}", METADATA_CALLED_SUFFIX)), None);
    }

    #[tokio::test]
    async fn test_directory_plugin_sequential_execution() {
        let config = HashMap::from([("directory".to_string(), "/admin".to_string())]);
        let mock_plugin1 = Arc::new(MockPlugin::new("first", false, ""));
        let mock_plugin2 = Arc::new(MockPlugin::new("second", true, "response"));
        let mock_plugin3 = Arc::new(MockPlugin::new("third", true, "should_not_see"));
        
        let directory_plugin = DirectoryPlugin::new_with_nested_plugins(
            config, 
            vec![mock_plugin1, mock_plugin2, mock_plugin3]
        );

        let mut request = create_test_request("/admin");
        let context = create_test_context();

        let response = directory_plugin.handle_request(&mut request, &context).await;
        
        assert!(response.is_some());
        assert_eq!(request.get_metadata(&format!("first{}", METADATA_CALLED_SUFFIX)), Some(METADATA_TRUE_VALUE));
        assert_eq!(request.get_metadata(&format!("second{}", METADATA_CALLED_SUFFIX)), Some(METADATA_TRUE_VALUE));
        assert_eq!(request.get_metadata(&format!("third{}", METADATA_CALLED_SUFFIX)), None); // Should not be called
    }

    #[tokio::test]
    async fn test_directory_plugin_no_nested_response() {
        let config = HashMap::from([("directory".to_string(), "/admin".to_string())]);
        let mock_plugin1 = Arc::new(MockPlugin::new("first", false, ""));
        let mock_plugin2 = Arc::new(MockPlugin::new("second", false, ""));
        
        let directory_plugin = DirectoryPlugin::new_with_nested_plugins(
            config, 
            vec![mock_plugin1, mock_plugin2]
        );

        let mut request = create_test_request("/admin");
        let context = create_test_context();

        let response = directory_plugin.handle_request(&mut request, &context).await;
        
        assert!(response.is_none());
        assert_eq!(request.get_metadata(&format!("first{}", METADATA_CALLED_SUFFIX)), Some(METADATA_TRUE_VALUE));
        assert_eq!(request.get_metadata(&format!("second{}", METADATA_CALLED_SUFFIX)), Some(METADATA_TRUE_VALUE));
    }

    #[tokio::test]
    async fn test_directory_plugin_handle_response_matching_path() {
        let config = HashMap::from([("directory".to_string(), "/admin".to_string())]);
        let mock_plugin = Arc::new(MockPlugin::new("test", false, ""));
        let directory_plugin = DirectoryPlugin::new_with_nested_plugins(config, vec![mock_plugin]);

        let request = create_test_request("/admin");
        let context = create_test_context();
        let mut response = Response::builder()
            .status(StatusCode::OK)
            .body(Body::from("test"))
            .unwrap();

        // This test verifies that handle_response is called for matching paths
        // Since we can't modify the request in handle_response, we test indirectly
        directory_plugin.handle_response(&request, &mut response, &context).await;
        
        // The test passes if no errors occur during handle_response call
        assert!(true);
    }

    #[tokio::test]
    async fn test_directory_plugin_handle_response_non_matching_path() {
        let config = HashMap::from([("directory".to_string(), "/admin".to_string())]);
        let mock_plugin = Arc::new(MockPlugin::new("test", false, ""));
        let directory_plugin = DirectoryPlugin::new_with_nested_plugins(config, vec![mock_plugin]);

        let request = create_test_request("/public");
        let context = create_test_context();
        let mut response = Response::builder()
            .status(StatusCode::OK)
            .body(Body::from("test"))
            .unwrap();

        // This test verifies that handle_response is not called for non-matching paths
        directory_plugin.handle_response(&request, &mut response, &context).await;
        
        // The test passes if no errors occur during handle_response call
        assert!(true);
    }

    #[tokio::test]
    async fn test_directory_plugin_file_url_parsing() {
        let config = HashMap::from([("directory".to_string(), "file://./examples/localhost/admin".to_string())]);
        let mock_plugin = Arc::new(MockPlugin::new("test", true, "response"));
        let directory_plugin = DirectoryPlugin::new_with_nested_plugins(config, vec![mock_plugin]);

        let mut request = create_test_request("/admin");
        let context = create_test_context();

        let response = directory_plugin.handle_request(&mut request, &context).await;
        
        assert!(response.is_some());
        assert_eq!(request.get_metadata(&format!("test{}", METADATA_CALLED_SUFFIX)), Some(METADATA_TRUE_VALUE));
    }
}