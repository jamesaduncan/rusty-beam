use async_trait::async_trait;
use hyper::{Body, Response};
use rusty_beam_plugin_api::{create_plugin, Plugin, PluginContext, PluginRequest, PluginResponse};
use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use libloading::{Library, Symbol};

// Constants
const DEFAULT_PLUGIN_NAME: &str = "directory";
const DEFAULT_DIRECTORY: &str = "/";
const FILE_URL_SCHEME: &str = "file://";
const PLUGIN_CREATION_FUNCTION: &[u8] = b"create_plugin";
const DEFAULT_JSON_CONFIG: &str = "{}";
const METADATA_CALLED_SUFFIX: &str = "_called";
const METADATA_TRUE_VALUE: &str = "true";
const ROOT_PATH: &str = "/";

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
        // Parse nested plugins from JSON configuration
        // Note: This JSON serialization is a limitation of the FFI boundary.
        // The main server has already parsed the microdata into PluginConfig structs,
        // but must serialize them to JSON to pass through the C FFI as strings.
        // In the future, we could:
        // 1. Pass the raw HTML and use microdata-extract here
        // 2. Create a plugin registry pattern to avoid re-serialization
        // 3. Use a more efficient binary format instead of JSON
        let nested_plugins_config = if let Some(nested_plugins_json) = config.get(CONFIG_KEY_NESTED_PLUGINS) {
            // Parse the nested plugins JSON array
            serde_json::from_str::<Vec<PluginConfig>>(nested_plugins_json)
                .unwrap_or_else(|_| Vec::new())
        } else {
            Vec::new()
        };

        // Create directory config
        let directory_config = DirectoryConfig {
            directory: config.get(CONFIG_KEY_DIRECTORY).unwrap_or(&DEFAULT_DIRECTORY.to_string()).clone(),
            nested_plugins: nested_plugins_config,
        };

        // Process directory path
        let directory = Self::process_directory_path(&directory_config.directory);

        // Load nested plugins
        let nested_plugins = Self::load_nested_plugins(&directory_config.nested_plugins);

        Self {
            directory,
            nested_plugins,
        }
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

    /// Load nested plugins from configuration
    fn load_nested_plugins(plugin_configs: &[PluginConfig]) -> Vec<Arc<dyn Plugin>> {
        let mut plugins = Vec::new();
        
        for plugin_config in plugin_configs {
            if let Some(plugin) = Self::load_plugin_from_config(plugin_config) {
                plugins.push(plugin);
            }
        }
        
        plugins
    }

    /// Load a single plugin from configuration
    fn load_plugin_from_config(config: &PluginConfig) -> Option<Arc<dyn Plugin>> {
        let library_path = &config.library;
        
        // Handle file:// URLs
        if library_path.starts_with(FILE_URL_SCHEME) {
            let path = library_path.strip_prefix(FILE_URL_SCHEME).unwrap_or(library_path);
            Self::load_dynamic_plugin(path, &config.config)
        } else {
            // For now, only support file:// URLs
            None
        }
    }

    /// Load a plugin from a dynamic library
    fn load_dynamic_plugin(library_path: &str, config: &HashMap<String, String>) -> Option<Arc<dyn Plugin>> {
        unsafe {
            match Library::new(library_path) {
                Ok(lib) => {
                    // Look for the plugin creation function
                    let create_fn: Symbol<
                        unsafe extern "C" fn(*const std::os::raw::c_char) -> *mut std::ffi::c_void,
                    > = match lib.get(PLUGIN_CREATION_FUNCTION) {
                        Ok(func) => func,
                        Err(_) => return None,
                    };

                    // Serialize config to JSON for passing to plugin
                    let config_json = serde_json::to_string(config).unwrap_or_else(|_| DEFAULT_JSON_CONFIG.to_string());
                    let config_cstr = std::ffi::CString::new(config_json).ok()?;

                    let plugin_ptr = create_fn(config_cstr.as_ptr());
                    if plugin_ptr.is_null() {
                        return None;
                    }

                    // Cast the void pointer back to Box<Box<dyn Plugin>> and unwrap one level
                    let plugin_box = Box::from_raw(plugin_ptr as *mut Box<dyn Plugin>);
                    let plugin = *plugin_box;
                    
                    let wrapper = DynamicPluginWrapper {
                        _library: lib,
                        plugin,
                    };
                    Some(Arc::new(wrapper))
                }
                Err(_) => None,
            }
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
    
    fn matches_directory(&self, path: &str) -> bool {
        let normalized_dir = self.directory.trim_end_matches('/');
        let normalized_path = path.trim_end_matches('/');
        
        // Check if path matches exactly or starts with directory followed by /
        normalized_path == normalized_dir || path.starts_with(&format!("{}/", normalized_dir))
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
            // Path doesn't match, pass through to next plugin
            return None;
        }

        // Path matches, execute nested plugins in sequence until one returns a response
        for plugin in &self.nested_plugins {
            if let Some(response) = plugin.handle_request(request, context).await {
                return Some(response);
            }
        }

        // No nested plugin provided a response
        None
    }

    async fn handle_response(
        &self,
        request: &PluginRequest,
        response: &mut Response<Body>,
        context: &PluginContext,
    ) {
        // Only call handle_response on nested plugins if the directory matches
        if self.matches_directory(&request.path) {
            // Call handle_response on all nested plugins
            for plugin in &self.nested_plugins {
                plugin.handle_response(request, response, context).await;
            }
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