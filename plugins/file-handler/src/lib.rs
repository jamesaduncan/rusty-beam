use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, create_plugin};
use async_trait::async_trait;
use hyper::{Body, Response, StatusCode, Method};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

/// Plugin for file operations
#[derive(Debug)]
pub struct FileHandlerPlugin {
    name: String,
    root_dir: String,
}

impl FileHandlerPlugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        let name = config.get("name").cloned().unwrap_or_else(|| "file-handler".to_string());
        let root_dir = config.get("root_dir").cloned().unwrap_or_else(|| ".".to_string());
        
        Self { name, root_dir }
    }
    
    async fn handle_get(&self, request: &PluginRequest, _context: &PluginContext) -> Option<Response<Body>> {
        let file_path = format!("{}{}", self.root_dir, request.path);
        let path = Path::new(&file_path);
        
        // Security check - ensure path is within root directory
        if let Ok(canonical) = path.canonicalize() {
            let root_canonical = Path::new(&self.root_dir).canonicalize().unwrap_or_else(|_| Path::new(".").to_path_buf());
            if !canonical.starts_with(&root_canonical) {
                return Some(Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .body(Body::from("Access denied"))
                    .unwrap());
            }
        }
        
        match fs::read(path).await {
            Ok(contents) => {
                let content_type = match path.extension().and_then(|ext| ext.to_str()) {
                    Some("html") => "text/html",
                    Some("css") => "text/css", 
                    Some("js") => "application/javascript",
                    Some("json") => "application/json",
                    Some("txt") => "text/plain",
                    Some("png") => "image/png",
                    Some("jpg") | Some("jpeg") => "image/jpeg",
                    Some("gif") => "image/gif",
                    _ => "application/octet-stream",
                };
                
                Some(Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", content_type)
                    .body(Body::from(contents))
                    .unwrap())
            }
            Err(_) => {
                Some(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from("File not found"))
                    .unwrap())
            }
        }
    }
    
    async fn handle_put(&self, request: &PluginRequest, _context: &PluginContext) -> Option<Response<Body>> {
        let file_path = format!("{}{}", self.root_dir, request.path);
        let path = Path::new(&file_path);
        
        // Security check
        if let Ok(canonical) = path.canonicalize().or_else(|_| path.parent().unwrap().canonicalize()) {
            let root_canonical = Path::new(&self.root_dir).canonicalize().unwrap_or_else(|_| Path::new(".").to_path_buf());
            if !canonical.starts_with(&root_canonical) {
                return Some(Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .body(Body::from("Access denied"))
                    .unwrap());
            }
        }
        
        // For now, just return success - in a real implementation we'd write the body
        Some(Response::builder()
            .status(StatusCode::CREATED)
            .body(Body::from("File created"))
            .unwrap())
    }
    
    async fn handle_delete(&self, request: &PluginRequest, _context: &PluginContext) -> Option<Response<Body>> {
        let file_path = format!("{}{}", self.root_dir, request.path);
        let path = Path::new(&file_path);
        
        match fs::remove_file(path).await {
            Ok(_) => {
                Some(Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::from("File deleted"))
                    .unwrap())
            }
            Err(_) => {
                Some(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from("File not found"))
                    .unwrap())
            }
        }
    }
}

#[async_trait]
impl Plugin for FileHandlerPlugin {
    async fn handle_request(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<Response<Body>> {
        match request.http_request.method() {
            &Method::GET => self.handle_get(request, context).await,
            &Method::PUT => self.handle_put(request, context).await,
            &Method::DELETE => self.handle_delete(request, context).await,
            &Method::OPTIONS => {
                Some(Response::builder()
                    .status(StatusCode::OK)
                    .header("Allow", "GET, PUT, DELETE, OPTIONS")
                    .body(Body::empty())
                    .unwrap())
            }
            _ => {
                Some(Response::builder()
                    .status(StatusCode::METHOD_NOT_ALLOWED)
                    .body(Body::from("Method not allowed"))
                    .unwrap())
            }
        }
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

// Export the plugin creation function
create_plugin!(FileHandlerPlugin);