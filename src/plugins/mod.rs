pub mod basic_auth;

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
pub struct UserInfo {
    #[allow(dead_code)] // Public API for plugin consumers
    pub username: String,
    #[allow(dead_code)] // Public API for plugin consumers
    pub roles: Vec<String>,
}

#[async_trait]
pub trait AuthPlugin: Send + Sync + Debug {
    async fn authenticate(&self, req: &Request<Body>) -> AuthResult;
    #[allow(dead_code)] // Public API for plugin identification
    fn name(&self) -> &'static str;
    fn requires_authentication(&self, path: &str) -> bool;
}

#[derive(Debug)]
pub struct PluginManager {
    server_wide_plugins: Vec<Box<dyn AuthPlugin>>,
    host_plugins: HashMap<String, Vec<Box<dyn AuthPlugin>>>,
}

impl PluginManager {
    pub fn new() -> Self {
        PluginManager {
            server_wide_plugins: Vec::new(),
            host_plugins: HashMap::new(),
        }
    }

    #[allow(dead_code)] // Public API for server-wide plugin support
    pub fn add_server_wide_plugin(&mut self, plugin: Box<dyn AuthPlugin>) {
        self.server_wide_plugins.push(plugin);
    }

    pub fn add_host_plugin(&mut self, host: String, plugin: Box<dyn AuthPlugin>) {
        self.host_plugins.entry(host).or_insert_with(Vec::new).push(plugin);
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
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
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