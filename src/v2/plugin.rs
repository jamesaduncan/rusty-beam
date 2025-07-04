use hyper::{Body, Request, Response};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;

/// Data that flows between plugins during request processing
#[derive(Debug)]
pub struct PluginRequest {
    /// The original HTTP request
    pub http_request: Request<Body>,
    /// The decoded URI path
    pub path: String,
    /// The canonicalized file system path (if applicable)
    pub canonical_path: Option<String>,
    /// Plugin-to-plugin metadata and state
    pub metadata: HashMap<String, String>,
}

impl PluginRequest {
    pub fn new(http_request: Request<Body>, path: String) -> Self {
        Self {
            http_request,
            path,
            canonical_path: None,
            metadata: HashMap::new(),
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
}

/// Configuration context available to plugins
#[derive(Debug, Clone)]
pub struct PluginContext {
    /// Plugin-specific configuration
    pub plugin_config: HashMap<String, String>,
    /// Host-level configuration
    pub host_config: HashMap<String, String>,
    /// Server-level configuration  
    pub server_config: HashMap<String, String>,
    /// Host name for this request
    pub host_name: String,
    /// Unique identifier for this request
    pub request_id: String,
    /// Shared state accessible to all plugins
    pub shared_state: Arc<RwLock<HashMap<String, String>>>,
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
}

/// Core plugin trait that all plugins must implement
#[async_trait]
pub trait Plugin: Send + Sync + std::fmt::Debug {
    /// Handle incoming request, optionally generating a response
    /// If this returns Some(response), the request phase stops and response phase begins
    async fn handle_request(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<Response<Body>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::{Method, StatusCode};

    #[test]
    fn test_plugin_request_metadata() {
        let req = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(Body::empty())
            .unwrap();
            
        let mut plugin_req = PluginRequest::new(req, "/test".to_string());
        
        assert_eq!(plugin_req.get_metadata("foo"), None);
        
        plugin_req.set_metadata("foo".to_string(), "bar".to_string());
        assert_eq!(plugin_req.get_metadata("foo"), Some("bar"));
    }
    
    #[test]
    fn test_plugin_context_config_hierarchy() {
        let context = PluginContext {
            plugin_config: {
                let mut map = HashMap::new();
                map.insert("key1".to_string(), "plugin_value".to_string());
                map
            },
            host_config: {
                let mut map = HashMap::new();
                map.insert("key2".to_string(), "host_value".to_string());
                map.insert("key1".to_string(), "host_override".to_string()); // Should be overridden
                map
            },
            server_config: {
                let mut map = HashMap::new();
                map.insert("key3".to_string(), "server_value".to_string());
                map.insert("key1".to_string(), "server_override".to_string()); // Should be overridden
                map.insert("key2".to_string(), "server_override2".to_string()); // Should be overridden
                map
            },
            host_name: "localhost".to_string(),
            request_id: "req_123".to_string(),
            shared_state: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Plugin config takes precedence
        assert_eq!(context.get_config("key1"), Some("plugin_value"));
        
        // Host config is used when plugin config doesn't have the key
        assert_eq!(context.get_config("key2"), Some("host_value"));
        
        // Server config is used when neither plugin nor host config have the key
        assert_eq!(context.get_config("key3"), Some("server_value"));
        
        // Non-existent key returns None
        assert_eq!(context.get_config("nonexistent"), None);
        
        // get_config_or provides default
        assert_eq!(context.get_config_or("nonexistent", "default"), "default");
    }

    // Mock plugin for testing
    #[derive(Debug)]
    struct MockPlugin {
        name: String,
    }
    
    impl MockPlugin {
        fn new(name: &str) -> Self {
            Self { name: name.to_string() }
        }
    }
    
    #[async_trait]
    impl Plugin for MockPlugin {
        async fn handle_request(&self, _request: &mut PluginRequest, _context: &PluginContext) -> Option<Response<Body>> {
            None
        }
        
        fn name(&self) -> &str {
            &self.name
        }
    }
    
    #[tokio::test]
    async fn test_plugin_trait() {
        let plugin = MockPlugin::new("test-plugin");
        assert_eq!(plugin.name(), "test-plugin");
        
        let req = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(Body::empty())
            .unwrap();
            
        let mut plugin_req = PluginRequest::new(req, "/test".to_string());
        let context = PluginContext {
            plugin_config: HashMap::new(),
            host_config: HashMap::new(),
            server_config: HashMap::new(),
            host_name: "localhost".to_string(),
            request_id: "req_123".to_string(),
            shared_state: Arc::new(RwLock::new(HashMap::new())),
        };
        
        let result = plugin.handle_request(&mut plugin_req, &context).await;
        assert!(result.is_none());
    }
}