pub mod dynamic;

use dynamic::DynamicPluginRegistry;

use async_trait::async_trait;
use hyper::{Body, Request};
use std::collections::HashMap;
use std::fmt::Debug;

#[derive(Debug, Clone)]
#[allow(dead_code)] // Used in config loading
pub struct PluginConfig {
    pub plugin_path: String,
    pub config: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum AuthResult {
    Authorized(UserInfo),
    Unauthorized,
    Error(String),
}

#[derive(Debug, Clone)]
pub enum AuthzResult {
    Authorized,
    Denied,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct UserInfo {
    #[allow(dead_code)] // Public API for plugin consumers
    pub username: String,
    #[allow(dead_code)] // Public API for plugin consumers
    pub roles: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AuthzRequest {
    pub user: UserInfo,
    pub resource: String,
    pub method: String,
}

#[async_trait]
pub trait AuthPlugin: Send + Sync + Debug {
    async fn authenticate(&self, req: &Request<Body>) -> AuthResult;
    #[allow(dead_code)] // Public API for plugin identification
    fn name(&self) -> &'static str;
    fn requires_authentication(&self, path: &str) -> bool;
}

#[async_trait]
pub trait AuthzPlugin: Send + Sync + Debug {
    async fn authorize(&self, request: &AuthzRequest) -> AuthzResult;
    #[allow(dead_code)] // Public API for plugin identification
    fn name(&self) -> &'static str;
    fn handles_resource(&self, resource: &str) -> bool;
}

#[derive(Debug)]
pub struct PluginManager {
    server_wide_plugins: Vec<Box<dyn AuthPlugin>>,
    host_plugins: HashMap<String, Vec<Box<dyn AuthPlugin>>>,
    server_wide_authz_plugins: Vec<Box<dyn AuthzPlugin>>,
    host_authz_plugins: HashMap<String, Vec<Box<dyn AuthzPlugin>>>,
}

impl PluginManager {
    pub fn new() -> Self {
        PluginManager {
            server_wide_plugins: Vec::new(),
            host_plugins: HashMap::new(),
            server_wide_authz_plugins: Vec::new(),
            host_authz_plugins: HashMap::new(),
        }
    }

    #[allow(dead_code)] // Public API for server-wide plugin support
    pub fn add_server_wide_plugin(&mut self, plugin: Box<dyn AuthPlugin>) {
        self.server_wide_plugins.push(plugin);
    }

    pub fn add_host_plugin(&mut self, host: String, plugin: Box<dyn AuthPlugin>) {
        self.host_plugins.entry(host).or_default().push(plugin);
    }

    #[allow(dead_code)] // Public API for server-wide authorization plugin support
    pub fn add_server_wide_authz_plugin(&mut self, plugin: Box<dyn AuthzPlugin>) {
        self.server_wide_authz_plugins.push(plugin);
    }

    pub fn add_host_authz_plugin(&mut self, host: String, plugin: Box<dyn AuthzPlugin>) {
        self.host_authz_plugins.entry(host).or_default().push(plugin);
    }

    pub async fn authenticate_request(&self, req: &Request<Body>, host: &str, path: &str) -> AuthResult {
        // Check host-specific plugins first
        if let Some(plugins) = self.host_plugins.get(host) {
            for plugin in plugins {
                if plugin.requires_authentication(path) {
                    let result = plugin.authenticate(req).await;
                    match result {
                        AuthResult::Authorized(_) | AuthResult::Unauthorized | AuthResult::Error(_) => {
                            return result;
                        }
                    }
                }
            }
        }

        // Check server-wide plugins
        for plugin in &self.server_wide_plugins {
            if plugin.requires_authentication(path) {
                let result = plugin.authenticate(req).await;
                match result {
                    AuthResult::Authorized(_) | AuthResult::Unauthorized | AuthResult::Error(_) => {
                        return result;
                    }
                }
            }
        }

        // If no plugin requires authentication for this path, allow access
        AuthResult::Authorized(UserInfo {
            username: "anonymous".to_string(),
            roles: vec!["anonymous".to_string()],
        })
    }

    #[allow(dead_code)] // Public API for checking auth requirements
    pub fn requires_authentication(&self, host: &str, path: &str) -> bool {
        // Check host-specific plugins first
        if let Some(plugins) = self.host_plugins.get(host) {
            for plugin in plugins {
                if plugin.requires_authentication(path) {
                    return true;
                }
            }
        }

        // Check server-wide plugins
        for plugin in &self.server_wide_plugins {
            if plugin.requires_authentication(path) {
                return true;
            }
        }

        false
    }

    pub async fn authorize_request(&self, user: &UserInfo, resource: &str, method: &str, host: &str) -> AuthzResult {
        let request = AuthzRequest {
            user: user.clone(),
            resource: resource.to_string(),
            method: method.to_string(),
        };

        // Check host-specific authorization plugins first
        if let Some(plugins) = self.host_authz_plugins.get(host) {
            for plugin in plugins {
                if plugin.handles_resource(resource) {
                    let result = plugin.authorize(&request).await;
                    match result {
                        AuthzResult::Authorized | AuthzResult::Denied | AuthzResult::Error(_) => {
                            return result;
                        }
                    }
                }
            }
        }

        // Check server-wide authorization plugins
        for plugin in &self.server_wide_authz_plugins {
            if plugin.handles_resource(resource) {
                let result = plugin.authorize(&request).await;
                match result {
                    AuthzResult::Authorized | AuthzResult::Denied | AuthzResult::Error(_) => {
                        return result;
                    }
                }
            }
        }

        // If no plugin handles this resource, default to deny
        AuthzResult::Denied
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

// Plugin registry for dynamic plugin creation
pub struct PluginRegistry;

impl PluginRegistry {
    pub fn create_plugin(plugin_path: &str, config: &HashMap<String, String>) -> Result<Box<dyn AuthPlugin>, String> {
        // Check if this is a direct library path
        if plugin_path.contains(".so") || plugin_path.contains(".dylib") || plugin_path.contains(".dll") {
            return DynamicPluginRegistry::load_plugin(plugin_path, config);
        }
        
        // Extract plugin name from path
        let plugin_name = plugin_path
            .strip_prefix("./plugins/")
            .unwrap_or(plugin_path);
            
        // Try to load as dynamic library with standard naming conventions
        let library_extensions = ["so", "dylib", "dll"];
        let library_prefixes = ["lib", ""];
        
        for prefix in &library_prefixes {
            for ext in &library_extensions {
                let lib_path = format!("plugins/lib/{}{}.{}", prefix, plugin_name.replace("-", "_"), ext);
                if std::path::Path::new(&lib_path).exists() {
                    return DynamicPluginRegistry::load_plugin(&lib_path, config);
                }
            }
        }
        
        Err(format!("No dynamic library found for plugin: {}", plugin_name))
    }

    pub fn create_authz_plugin(plugin_path: &str, config: &HashMap<String, String>) -> Result<Box<dyn AuthzPlugin>, String> {
        // Check if this is a direct library path
        if plugin_path.contains(".so") || plugin_path.contains(".dylib") || plugin_path.contains(".dll") {
            return DynamicPluginRegistry::load_authz_plugin(plugin_path, config);
        }
        
        // Extract plugin name from path
        let plugin_name = plugin_path
            .strip_prefix("./plugins/")
            .unwrap_or(plugin_path);
            
        // Try to load as dynamic library with standard naming conventions
        let library_extensions = ["so", "dylib", "dll"];
        let library_prefixes = ["lib", ""];
        
        for prefix in &library_prefixes {
            for ext in &library_extensions {
                let lib_path = format!("plugins/lib/{}{}.{}", prefix, plugin_name.replace("-", "_"), ext);
                if std::path::Path::new(&lib_path).exists() {
                    return DynamicPluginRegistry::load_authz_plugin(&lib_path, config);
                }
            }
        }
        
        Err(format!("No dynamic library found for authz plugin: {}", plugin_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::{Body, Request, Method};

    // Mock plugin for testing
    #[derive(Debug)]
    struct MockAuthPlugin {
        #[allow(dead_code)] // Used for plugin identification in tests
        name: String,
        requires_auth: bool,
        should_authenticate: bool,
        error_message: Option<String>,
    }

    impl MockAuthPlugin {
        fn new(name: &str) -> Self {
            MockAuthPlugin {
                name: name.to_string(),
                requires_auth: true,
                should_authenticate: false,
                error_message: None,
            }
        }

        fn with_auth_required(mut self, required: bool) -> Self {
            self.requires_auth = required;
            self
        }

        fn with_authentication_result(mut self, should_auth: bool) -> Self {
            self.should_authenticate = should_auth;
            self
        }

        fn with_error(mut self, error: &str) -> Self {
            self.error_message = Some(error.to_string());
            self
        }
    }

    #[async_trait]
    impl AuthPlugin for MockAuthPlugin {
        async fn authenticate(&self, _req: &Request<Body>) -> AuthResult {
            if let Some(ref error) = self.error_message {
                return AuthResult::Error(error.clone());
            }

            if self.should_authenticate {
                AuthResult::Authorized(UserInfo {
                    username: "test_user".to_string(),
                    roles: vec!["user".to_string()],
                })
            } else {
                AuthResult::Unauthorized
            }
        }

        fn name(&self) -> &'static str {
            // For testing, we'll use a static string, but in practice this would be more dynamic
            "mock-auth"
        }

        fn requires_authentication(&self, _path: &str) -> bool {
            self.requires_auth
        }
    }

    #[tokio::test]
    async fn test_plugin_manager_new() {
        let manager = PluginManager::new();
        assert_eq!(manager.server_wide_plugins.len(), 0);
        assert_eq!(manager.host_plugins.len(), 0);
    }

    #[tokio::test]
    async fn test_add_server_wide_plugin() {
        let mut manager = PluginManager::new();
        let plugin = MockAuthPlugin::new("test1");
        
        manager.add_server_wide_plugin(Box::new(plugin));
        assert_eq!(manager.server_wide_plugins.len(), 1);
    }

    #[tokio::test]
    async fn test_add_host_plugin() {
        let mut manager = PluginManager::new();
        let plugin = MockAuthPlugin::new("test1");
        
        manager.add_host_plugin("localhost".to_string(), Box::new(plugin));
        assert_eq!(manager.host_plugins.len(), 1);
        assert!(manager.host_plugins.contains_key("localhost"));
        assert_eq!(manager.host_plugins.get("localhost").unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_requires_authentication_host_specific() {
        let mut manager = PluginManager::new();
        let plugin = MockAuthPlugin::new("test1").with_auth_required(true);
        
        manager.add_host_plugin("localhost".to_string(), Box::new(plugin));
        
        assert!(manager.requires_authentication("localhost", "/test"));
        assert!(!manager.requires_authentication("example.com", "/test"));
    }

    #[tokio::test]
    async fn test_requires_authentication_server_wide() {
        let mut manager = PluginManager::new();
        let plugin = MockAuthPlugin::new("test1").with_auth_required(true);
        
        manager.add_server_wide_plugin(Box::new(plugin));
        
        assert!(manager.requires_authentication("localhost", "/test"));
        assert!(manager.requires_authentication("example.com", "/test"));
    }

    #[tokio::test]
    async fn test_authenticate_request_authorized() {
        let mut manager = PluginManager::new();
        let plugin = MockAuthPlugin::new("test1")
            .with_auth_required(true)
            .with_authentication_result(true);
        
        manager.add_host_plugin("localhost".to_string(), Box::new(plugin));
        
        let req = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let result = manager.authenticate_request(&req, "localhost", "/test").await;
        
        match result {
            AuthResult::Authorized(user_info) => {
                assert_eq!(user_info.username, "test_user");
                assert_eq!(user_info.roles, vec!["user"]);
            }
            _ => panic!("Expected authorized result"),
        }
    }

    #[tokio::test]
    async fn test_authenticate_request_unauthorized() {
        let mut manager = PluginManager::new();
        let plugin = MockAuthPlugin::new("test1")
            .with_auth_required(true)
            .with_authentication_result(false);
        
        manager.add_host_plugin("localhost".to_string(), Box::new(plugin));
        
        let req = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let result = manager.authenticate_request(&req, "localhost", "/test").await;
        
        match result {
            AuthResult::Unauthorized => {},
            _ => panic!("Expected unauthorized result"),
        }
    }

    #[tokio::test]
    async fn test_authenticate_request_error() {
        let mut manager = PluginManager::new();
        let plugin = MockAuthPlugin::new("test1")
            .with_auth_required(true)
            .with_error("Test error");
        
        manager.add_host_plugin("localhost".to_string(), Box::new(plugin));
        
        let req = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let result = manager.authenticate_request(&req, "localhost", "/test").await;
        
        match result {
            AuthResult::Error(msg) => assert_eq!(msg, "Test error"),
            _ => panic!("Expected error result"),
        }
    }

    #[tokio::test]
    async fn test_authenticate_request_no_auth_required() {
        let mut manager = PluginManager::new();
        let plugin = MockAuthPlugin::new("test1").with_auth_required(false);
        
        manager.add_host_plugin("localhost".to_string(), Box::new(plugin));
        
        let req = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let result = manager.authenticate_request(&req, "localhost", "/test").await;
        
        match result {
            AuthResult::Authorized(user_info) => {
                assert_eq!(user_info.username, "anonymous");
                assert_eq!(user_info.roles, vec!["anonymous"]);
            }
            _ => panic!("Expected anonymous access"),
        }
    }

    #[test]
    fn test_plugin_registry_plugin_not_found() {
        let config = HashMap::new();
        let result = PluginRegistry::create_plugin("unknown-plugin", &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No dynamic library found for plugin"));
    }
    
    #[test]
    fn test_plugin_registry_dynamic_library_auto_discovery() {
        let config = HashMap::new();
        
        // Test plugin name that should try dynamic library loading
        let result = PluginRegistry::create_plugin("test-plugin", &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No dynamic library found for plugin"));
    }
    
    #[test] 
    fn test_plugin_registry_direct_library_path() {
        let config = HashMap::new();
        
        // Test direct library path (this will fail to load since the path doesn't exist)
        let result = PluginRegistry::create_plugin("nonexistent.so", &config);
        assert!(result.is_err());
        // Should try to load as dynamic library
        assert!(result.unwrap_err().contains("Failed to load plugin library"));
    }

    #[tokio::test]
    async fn test_host_specific_overrides_server_wide() {
        let mut manager = PluginManager::new();
        
        // Add server-wide plugin that denies access
        let server_plugin = MockAuthPlugin::new("server")
            .with_auth_required(true)
            .with_authentication_result(false);
        manager.add_server_wide_plugin(Box::new(server_plugin));
        
        // Add host-specific plugin that allows access
        let host_plugin = MockAuthPlugin::new("host")
            .with_auth_required(true)
            .with_authentication_result(true);
        manager.add_host_plugin("localhost".to_string(), Box::new(host_plugin));
        
        let req = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        // Host-specific plugin should take precedence
        let result = manager.authenticate_request(&req, "localhost", "/test").await;
        
        match result {
            AuthResult::Authorized(user_info) => {
                assert_eq!(user_info.username, "test_user");
            }
            _ => panic!("Expected host-specific plugin to authorize"),
        }
    }
}