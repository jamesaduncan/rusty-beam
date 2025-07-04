use super::plugin::{Plugin, PluginRequest, PluginContext};
use hyper::{Body, Request, Response, StatusCode};
use std::sync::Arc;
use uuid::Uuid;

/// Pipeline execution result
#[derive(Debug)]
pub struct PipelineResult {
    pub response: Response<Body>,
    pub plugins_executed: usize,
    pub response_generated_at: Option<usize>,
}

/// A collection of plugins that process requests in sequence
#[derive(Debug)]
pub struct Pipeline {
    plugins: Vec<Box<dyn Plugin>>,
    name: String,
}

impl Pipeline {
    pub fn new(name: String) -> Self {
        Self {
            plugins: Vec::new(),
            name,
        }
    }
    
    /// Add a plugin to the end of the pipeline
    pub fn add_plugin(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push(plugin);
    }
    
    /// Get the name of this pipeline
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Get the number of plugins in this pipeline
    pub fn len(&self) -> usize {
        self.plugins.len()
    }
    
    /// Check if the pipeline is empty
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
    
    /// Process a request through this pipeline
    pub async fn process(&self, http_request: Request<Body>, context: PluginContext) -> PipelineResult {
        let path = http_request.uri().path().to_string();
        let mut plugin_request = PluginRequest::new(http_request, path);
        
        // Phase 1: Request processing - call handle_request on plugins until one generates a response
        let mut response: Option<Response<Body>> = None;
        let mut response_generated_at: Option<usize> = None;
        
        for (index, plugin) in self.plugins.iter().enumerate() {
            match plugin.handle_request(&mut plugin_request, &context).await {
                Some(generated_response) => {
                    response = Some(generated_response);
                    response_generated_at = Some(index);
                    break;
                }
                None => continue,
            }
        }
        
        // If no plugin generated a response, return 404
        let mut final_response = response.unwrap_or_else(|| {
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "text/plain")
                .body(Body::from("Not Found"))
                .unwrap()
        });
        
        // Phase 2: Response processing - call handle_response on remaining plugins
        if let Some(generated_at) = response_generated_at {
            // Call handle_response on plugins that come after the one that generated the response
            for plugin in self.plugins.iter().skip(generated_at + 1) {
                plugin.handle_response(&plugin_request, &mut final_response, &context).await;
            }
        } else if !self.plugins.is_empty() {
            // If no plugin generated a response, still call handle_response on all plugins
            // This allows logging plugins to record 404s
            for plugin in &self.plugins {
                plugin.handle_response(&plugin_request, &mut final_response, &context).await;
            }
        }
        
        PipelineResult {
            response: final_response,
            plugins_executed: response_generated_at.map(|i| i + 1).unwrap_or(self.plugins.len()),
            response_generated_at,
        }
    }
}

/// Nested pipeline that can contain other pipelines or plugins
#[derive(Debug)]
pub enum PipelineItem {
    Plugin(Box<dyn Plugin>),
    Pipeline(Pipeline),
}

/// Pipeline that supports nested sub-pipelines
#[derive(Debug)]
pub struct NestedPipeline {
    items: Vec<PipelineItem>,
    name: String,
}

impl NestedPipeline {
    pub fn new(name: String) -> Self {
        Self {
            items: Vec::new(),
            name,
        }
    }
    
    /// Add a plugin to the pipeline
    pub fn add_plugin(&mut self, plugin: Box<dyn Plugin>) {
        self.items.push(PipelineItem::Plugin(plugin));
    }
    
    /// Add a nested pipeline
    pub fn add_pipeline(&mut self, pipeline: Pipeline) {
        self.items.push(PipelineItem::Pipeline(pipeline));
    }
    
    /// Process a request through this nested pipeline
    pub async fn process(&self, http_request: Request<Body>, context: PluginContext) -> PipelineResult {
        let path = http_request.uri().path().to_string();
        let mut plugin_request = PluginRequest::new(http_request, path);
        
        // Phase 1: Request processing
        let mut response: Option<Response<Body>> = None;
        let mut response_generated_at: Option<usize> = None;
        let mut plugins_executed = 0;
        
        for (index, item) in self.items.iter().enumerate() {
            match item {
                PipelineItem::Plugin(plugin) => {
                    plugins_executed += 1;
                    match plugin.handle_request(&mut plugin_request, &context).await {
                        Some(generated_response) => {
                            response = Some(generated_response);
                            response_generated_at = Some(index);
                            break;
                        }
                        None => continue,
                    }
                }
                PipelineItem::Pipeline(nested_pipeline) => {
                    // For nested pipelines, we reconstruct the request and process it
                    let nested_request = Request::builder()
                        .method(plugin_request.http_request.method())
                        .uri(plugin_request.http_request.uri())
                        .version(plugin_request.http_request.version());
                    
                    // Copy headers
                    let mut nested_request = nested_request;
                    for (key, value) in plugin_request.http_request.headers() {
                        nested_request = nested_request.header(key, value);
                    }
                    
                    // Note: We lose the body here in this simple implementation
                    // A full implementation would need to handle body cloning/sharing
                    let nested_request = nested_request.body(Body::empty()).unwrap();
                    
                    let nested_result = nested_pipeline.process(nested_request, context.clone()).await;
                    plugins_executed += nested_result.plugins_executed;
                    
                    if nested_result.response.status() != StatusCode::NOT_FOUND {
                        response = Some(nested_result.response);
                        response_generated_at = Some(index);
                        break;
                    }
                }
            }
        }
        
        // If no item generated a response, return 404
        let mut final_response = response.unwrap_or_else(|| {
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "text/plain")
                .body(Body::from("Not Found"))
                .unwrap()
        });
        
        // Phase 2: Response processing - call handle_response on remaining items
        if let Some(generated_at) = response_generated_at {
            for item in self.items.iter().skip(generated_at + 1) {
                match item {
                    PipelineItem::Plugin(plugin) => {
                        plugin.handle_response(&plugin_request, &mut final_response, &context).await;
                    }
                    PipelineItem::Pipeline(nested_pipeline) => {
                        // For nested pipelines in response phase, call handle_response on all their plugins
                        for plugin in &nested_pipeline.plugins {
                            plugin.handle_response(&plugin_request, &mut final_response, &context).await;
                        }
                    }
                }
            }
        } else if !self.items.is_empty() {
            // If no item generated a response, still call handle_response on all items
            for item in &self.items {
                match item {
                    PipelineItem::Plugin(plugin) => {
                        plugin.handle_response(&plugin_request, &mut final_response, &context).await;
                    }
                    PipelineItem::Pipeline(nested_pipeline) => {
                        for plugin in &nested_pipeline.plugins {
                            plugin.handle_response(&plugin_request, &mut final_response, &context).await;
                        }
                    }
                }
            }
        }
        
        PipelineResult {
            response: final_response,
            plugins_executed,
            response_generated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::plugin::{Plugin, PluginRequest, PluginContext};
    use async_trait::async_trait;
    use hyper::{Method, StatusCode};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    // Mock plugins for testing
    #[derive(Debug)]
    struct MockPlugin {
        name: String,
        should_respond: bool,
        response_body: String,
    }
    
    impl MockPlugin {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                should_respond: false,
                response_body: "Mock response".to_string(),
            }
        }
        
        fn with_response(mut self, body: &str) -> Self {
            self.should_respond = true;
            self.response_body = body.to_string();
            self
        }
    }
    
    #[async_trait]
    impl Plugin for MockPlugin {
        async fn handle_request(&self, request: &mut PluginRequest, _context: &PluginContext) -> Option<Response<Body>> {
            // Add plugin name to metadata
            request.set_metadata(format!("{}_called", self.name), "true".to_string());
            
            if self.should_respond {
                Some(Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::from(self.response_body.clone()))
                    .unwrap())
            } else {
                None
            }
        }
        
        async fn handle_response(&self, _request: &PluginRequest, _response: &mut Response<Body>, _context: &PluginContext) {
            // Mock response processing - in real plugins this would do logging, etc.
        }
        
        fn name(&self) -> &str {
            &self.name
        }
    }
    
    fn create_test_context() -> PluginContext {
        PluginContext {
            plugin_config: HashMap::new(),
            host_config: HashMap::new(),
            server_config: HashMap::new(),
            host_name: "localhost".to_string(),
            request_id: Uuid::new_v4().to_string(),
            shared_state: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    #[tokio::test]
    async fn test_empty_pipeline_returns_404() {
        let pipeline = Pipeline::new("test".to_string());
        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(Body::empty())
            .unwrap();
            
        let result = pipeline.process(request, create_test_context()).await;
        assert_eq!(result.response.status(), StatusCode::NOT_FOUND);
        assert_eq!(result.plugins_executed, 0);
        assert_eq!(result.response_generated_at, None);
    }
    
    #[tokio::test]
    async fn test_pipeline_no_response_generated() {
        let mut pipeline = Pipeline::new("test".to_string());
        pipeline.add_plugin(Box::new(MockPlugin::new("plugin1")));
        pipeline.add_plugin(Box::new(MockPlugin::new("plugin2")));
        
        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(Body::empty())
            .unwrap();
            
        let result = pipeline.process(request, create_test_context()).await;
        assert_eq!(result.response.status(), StatusCode::NOT_FOUND);
        assert_eq!(result.plugins_executed, 2);
        assert_eq!(result.response_generated_at, None);
    }
    
    #[tokio::test]
    async fn test_pipeline_response_generated() {
        let mut pipeline = Pipeline::new("test".to_string());
        pipeline.add_plugin(Box::new(MockPlugin::new("plugin1")));
        pipeline.add_plugin(Box::new(MockPlugin::new("plugin2").with_response("Hello")));
        pipeline.add_plugin(Box::new(MockPlugin::new("plugin3")));
        
        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(Body::empty())
            .unwrap();
            
        let result = pipeline.process(request, create_test_context()).await;
        assert_eq!(result.response.status(), StatusCode::OK);
        assert_eq!(result.plugins_executed, 2); // plugin1 and plugin2
        assert_eq!(result.response_generated_at, Some(1)); // plugin2 (0-indexed)
    }
}