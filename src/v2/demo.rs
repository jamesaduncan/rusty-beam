//! Demonstration of the v2 architecture
//! 
//! This module shows how the v2 plugin system would work in practice

use super::examples::{BasicAuthPlugin, FileServerPlugin, LoggingPlugin};
use super::pipeline::Pipeline;
use super::plugin::PluginContext;
use hyper::{Body, Method, Request};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Create a demo pipeline with basic plugins
pub fn create_demo_pipeline() -> Pipeline {
    let mut pipeline = Pipeline::new("demo-host".to_string());
    
    // Add plugins in the order they should execute
    pipeline.add_plugin(Box::new(LoggingPlugin::new()));      // 1. Log all requests
    pipeline.add_plugin(Box::new(BasicAuthPlugin::new()));    // 2. Authenticate if needed
    pipeline.add_plugin(Box::new(FileServerPlugin::new()));   // 3. Serve files
    // Note: LoggingPlugin will also handle responses via handle_response
    
    pipeline
}

/// Create a demo plugin context
pub fn create_demo_context(host_name: &str) -> PluginContext {
    let mut server_config = HashMap::new();
    server_config.insert("serverRoot".to_string(), "./examples/files".to_string());
    server_config.insert("bindAddress".to_string(), "127.0.0.1".to_string());
    server_config.insert("bindPort".to_string(), "3000".to_string());
    
    let mut host_config = HashMap::new();
    host_config.insert("hostRoot".to_string(), format!("./examples/{}", host_name));
    
    let plugin_config = HashMap::new(); // No plugin-specific config for this demo
    
    PluginContext {
        plugin_config,
        host_config,
        server_config,
        host_name: host_name.to_string(),
        request_id: Uuid::new_v4().to_string(),
        shared_state: Arc::new(RwLock::new(HashMap::new())),
    }
}

/// Demonstration function showing how a request would be processed
pub async fn demo_request_processing() {
    println!("=== Rusty Beam v2 Architecture Demo ===\n");
    
    // Create a demo pipeline
    let pipeline = create_demo_pipeline();
    println!("Created pipeline with {} plugins", pipeline.len());
    
    // Create some demo requests
    let requests = vec![
        Request::builder()
            .method(Method::GET)
            .uri("/index.html")
            .body(Body::empty())
            .unwrap(),
        Request::builder()
            .method(Method::GET)
            .uri("/admin/dashboard.html")
            .body(Body::empty())
            .unwrap(),
        Request::builder()
            .method(Method::GET)
            .uri("/admin/dashboard.html")
            .header("authorization", "Basic YWRtaW46cGFzc3dvcmQ=") // admin:password
            .body(Body::empty())
            .unwrap(),
    ];
    
    // Process each request through the pipeline
    for (i, request) in requests.into_iter().enumerate() {
        println!("--- Request {} ---", i + 1);
        println!("Method: {}", request.method());
        println!("URI: {}", request.uri());
        
        let context = create_demo_context("localhost");
        let result = pipeline.process(request, context).await;
        
        println!("Response Status: {}", result.response.status());
        println!("Plugins Executed: {}", result.plugins_executed);
        if let Some(generated_at) = result.response_generated_at {
            println!("Response Generated At Plugin: {}", generated_at);
        }
        println!();
    }
    
    println!("=== Demo Complete ===");
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::StatusCode;
    
    #[tokio::test]
    async fn test_demo_pipeline_creation() {
        let pipeline = create_demo_pipeline();
        assert_eq!(pipeline.len(), 3);
        assert!(!pipeline.is_empty());
    }
    
    #[test]
    fn test_demo_context_creation() {
        let context = create_demo_context("test-host");
        assert_eq!(context.host_name, "test-host");
        assert_eq!(context.get_config("serverRoot"), Some("./examples/files"));
        assert_eq!(context.get_config("hostRoot"), Some("./examples/test-host"));
    }
    
    #[tokio::test]
    async fn test_demo_public_request() {
        let pipeline = create_demo_pipeline();
        let request = Request::builder()
            .method(Method::GET)
            .uri("/index.html")
            .body(Body::empty())
            .unwrap();
        let context = create_demo_context("localhost");
        
        let result = pipeline.process(request, context).await;
        
        // Should be 200 for /index.html since our demo file server serves it
        assert_eq!(result.response.status(), StatusCode::OK);
        assert_eq!(result.plugins_executed, 3); // All plugins executed
        assert_eq!(result.response_generated_at, Some(2)); // FileServer generated response
    }
    
    #[tokio::test]
    async fn test_demo_admin_request_no_auth() {
        let pipeline = create_demo_pipeline();
        let request = Request::builder()
            .method(Method::GET)
            .uri("/admin/dashboard.html")
            .body(Body::empty())
            .unwrap();
        let context = create_demo_context("localhost");
        
        let result = pipeline.process(request, context).await;
        
        // Should be 401 because /admin requires auth but none provided
        assert_eq!(result.response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(result.plugins_executed, 2); // Logging + BasicAuth
        assert_eq!(result.response_generated_at, Some(1)); // BasicAuth generated response
    }
    
    #[tokio::test]
    async fn test_demo_admin_request_with_auth() {
        let pipeline = create_demo_pipeline();
        let request = Request::builder()
            .method(Method::GET)
            .uri("/admin/dashboard.html")
            .header("authorization", "Basic YWRtaW46cGFzc3dvcmQ=")
            .body(Body::empty())
            .unwrap();
        let context = create_demo_context("localhost");
        
        let result = pipeline.process(request, context).await;
        
        // Should be 404 (file not found) since auth passed but file doesn't exist
        assert_eq!(result.response.status(), StatusCode::NOT_FOUND);
        assert_eq!(result.plugins_executed, 3); // All plugins executed
        assert_eq!(result.response_generated_at, Some(2)); // FileServer generated 404
    }
}