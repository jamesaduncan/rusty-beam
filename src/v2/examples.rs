//! Example plugins for testing the v2 architecture

use super::plugin::{Plugin, PluginRequest, PluginContext};
use async_trait::async_trait;
use hyper::{Body, Response, StatusCode};

/// Simple logging plugin that logs requests
#[derive(Debug)]
pub struct LoggingPlugin {
    name: String,
}

impl LoggingPlugin {
    pub fn new() -> Self {
        Self {
            name: "logging".to_string(),
        }
    }
}

#[async_trait]
impl Plugin for LoggingPlugin {
    async fn handle_request(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<Response<Body>> {
        println!("[{}] Request: {} {}", 
                context.host_name, 
                request.http_request.method(), 
                request.path);
        
        // Log plugin doesn't generate responses, just logs
        None
    }
    
    async fn handle_response(&self, request: &PluginRequest, response: &mut Response<Body>, context: &PluginContext) {
        println!("[{}] Response: {} {} -> {}", 
                context.host_name,
                request.http_request.method(), 
                request.path,
                response.status());
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

/// Simple file server plugin
#[derive(Debug)]
pub struct FileServerPlugin {
    name: String,
}

impl FileServerPlugin {
    pub fn new() -> Self {
        Self {
            name: "file-server".to_string(),
        }
    }
}

#[async_trait]
impl Plugin for FileServerPlugin {
    async fn handle_request(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<Response<Body>> {
        // Only handle GET requests
        if request.http_request.method() != hyper::Method::GET {
            return None;
        }
        
        // For demo purposes, just return a simple response
        // In a real implementation, this would read actual files
        let server_root = context.get_config_or("serverRoot", "./examples/files");
        let file_path = format!("{}{}", server_root, request.path);
        
        // Set canonical path for other plugins
        request.canonical_path = Some(file_path);
        
        // Simulate file serving - in a real implementation, you'd read the actual file
        if request.path == "/index.html" {
            Some(Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/html")
                .body(Body::from("<html><body>Welcome to Rusty Beam v2!</body></html>"))
                .unwrap())
        } else {
            // Return 404 for all other files in this demo
            Some(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "text/plain")
                .body(Body::from("File not found"))
                .unwrap())
        }
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

/// Simple authentication plugin
#[derive(Debug)]
pub struct BasicAuthPlugin {
    name: String,
}

impl BasicAuthPlugin {
    pub fn new() -> Self {
        Self {
            name: "basic-auth".to_string(),
        }
    }
}

#[async_trait]
impl Plugin for BasicAuthPlugin {
    async fn handle_request(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<Response<Body>> {
        // Check if path requires authentication
        if !request.path.starts_with("/admin") {
            // No auth required, let request continue
            return None;
        }
        
        // Check for Authorization header
        if let Some(auth_header) = request.http_request.headers().get("authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                if auth_str.starts_with("Basic ") {
                    // For demo purposes, accept any basic auth
                    request.set_metadata("authenticated_user".to_string(), "demo_user".to_string());
                    return None; // Allow request to continue
                }
            }
        }
        
        // No valid auth, return 401
        let realm = context.get_config_or("realm", "Rusty Beam");
        Some(Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header("WWW-Authenticate", format!("Basic realm=\"{}\"", realm))
            .header("Content-Type", "text/plain")
            .body(Body::from("Authentication required"))
            .unwrap())
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::{Method, Request};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use uuid::Uuid;
    
    fn create_test_context() -> PluginContext {
        PluginContext {
            plugin_config: HashMap::new(),
            host_config: HashMap::new(),
            server_config: {
                let mut config = HashMap::new();
                config.insert("serverRoot".to_string(), "./examples/files".to_string());
                config
            },
            host_name: "localhost".to_string(),
            request_id: Uuid::new_v4().to_string(),
            shared_state: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    #[tokio::test]
    async fn test_logging_plugin() {
        let plugin = LoggingPlugin::new();
        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(Body::empty())
            .unwrap();
        let mut plugin_request = PluginRequest::new(request, "/test".to_string());
        let context = create_test_context();
        
        let result = plugin.handle_request(&mut plugin_request, &context).await;
        assert!(result.is_none()); // Logging plugin doesn't generate responses
    }
    
    #[tokio::test]
    async fn test_basic_auth_plugin_no_auth_required() {
        let plugin = BasicAuthPlugin::new();
        let request = Request::builder()
            .method(Method::GET)
            .uri("/public")
            .body(Body::empty())
            .unwrap();
        let mut plugin_request = PluginRequest::new(request, "/public".to_string());
        let context = create_test_context();
        
        let result = plugin.handle_request(&mut plugin_request, &context).await;
        assert!(result.is_none()); // No auth required for /public
    }
    
    #[tokio::test]
    async fn test_basic_auth_plugin_auth_required() {
        let plugin = BasicAuthPlugin::new();
        let request = Request::builder()
            .method(Method::GET)
            .uri("/admin/test")
            .body(Body::empty())
            .unwrap();
        let mut plugin_request = PluginRequest::new(request, "/admin/test".to_string());
        let context = create_test_context();
        
        let result = plugin.handle_request(&mut plugin_request, &context).await;
        assert!(result.is_some()); // Auth required for /admin
        
        let response = result.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}