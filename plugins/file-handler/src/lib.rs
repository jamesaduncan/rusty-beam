use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, create_plugin};
use async_trait::async_trait;
use hyper::{Body, Response, StatusCode, Method};
use std::collections::HashMap;
use std::path::Path;
use std::fs;

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
    
    async fn handle_get(&self, request: &PluginRequest, context: &PluginContext) -> Option<Response<Body>> {
        // Use host-specific root if available, otherwise fall back to plugin config
        let root_dir = context.host_config.get("hostRoot")
            .unwrap_or(&self.root_dir);
        let mut file_path = format!("{}{}", root_dir, request.path);
        
        // If path ends with '/', append 'index.html'
        if file_path.ends_with('/') {
            file_path.push_str("index.html");
        }
        
        let path = Path::new(&file_path);
        
        // Security check - ensure path is within root directory
        if let Ok(canonical) = path.canonicalize() {
            let root_canonical = Path::new(root_dir).canonicalize().unwrap_or_else(|_| Path::new(".").to_path_buf());
            if !canonical.starts_with(&root_canonical) {
                return Some(Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .body(Body::from("Access denied"))
                    .unwrap());
            }
        }
        
        match fs::read(path) {
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
    
    async fn handle_put(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<Response<Body>> {
        // Use host-specific root if available, otherwise fall back to plugin config
        let root_dir = context.host_config.get("hostRoot")
            .unwrap_or(&self.root_dir);
        let file_path = format!("{}{}", root_dir, request.path);
        let path = Path::new(&file_path);
        
        // Check if file exists before writing to determine correct status code
        let file_existed = path.exists();
        
        // Security check
        if let Some(parent) = path.parent() {
            if let Ok(canonical) = parent.canonicalize() {
                let root_canonical = Path::new(root_dir).canonicalize().unwrap_or_else(|_| Path::new(".").to_path_buf());
                if !canonical.starts_with(&root_canonical) {
                    return Some(Response::builder()
                        .status(StatusCode::FORBIDDEN)
                        .body(Body::from("Access denied"))
                        .unwrap());
                }
            }
        }
        
        // Get request body
        let body_bytes = match request.get_body().await {
            Ok(bytes) => bytes,
            Err(_) => {
                return Some(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("Failed to read request body"))
                    .unwrap());
            }
        };
        
        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        
        // Write the file
        match fs::write(path, &body_bytes) {
            Ok(_) => {
                // RFC 7231: 201 for new resources, 200 for updates
                let status = if file_existed { 
                    StatusCode::OK 
                } else { 
                    StatusCode::CREATED 
                };
                Some(Response::builder()
                    .status(status)
                    .header("Content-Type", "text/plain")
                    .body(Body::from("File uploaded successfully"))
                    .unwrap())
            }
            Err(e) => {
                Some(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(format!("Failed to write file: {}", e)))
                    .unwrap())
            }
        }
    }
    
    async fn handle_post(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<Response<Body>> {
        // Use host-specific root if available, otherwise fall back to plugin config
        let root_dir = context.host_config.get("hostRoot")
            .unwrap_or(&self.root_dir);
        let file_path = format!("{}{}", root_dir, request.path);
        let path = Path::new(&file_path);
        
        // Get request body
        let body_bytes = match request.get_body().await {
            Ok(bytes) => bytes,
            Err(_) => {
                return Some(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("Failed to read request body"))
                    .unwrap());
            }
        };
        
        // For POST, append to the file or create it if it doesn't exist
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            Ok(mut file) => {
                use std::io::Write;
                match file.write_all(&body_bytes) {
                    Ok(_) => {
                        Some(Response::builder()
                            .status(StatusCode::OK)
                            .header("Content-Type", "text/plain")
                            .body(Body::from("Content appended successfully"))
                            .unwrap())
                    }
                    Err(e) => {
                        Some(Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Body::from(format!("Failed to append to file: {}", e)))
                            .unwrap())
                    }
                }
            }
            Err(e) => {
                Some(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(format!("Failed to open file: {}", e)))
                    .unwrap())
            }
        }
    }
    
    async fn handle_head(&self, request: &PluginRequest, context: &PluginContext) -> Option<Response<Body>> {
        // Use host-specific root if available, otherwise fall back to plugin config
        let root_dir = context.host_config.get("hostRoot")
            .unwrap_or(&self.root_dir);
        let mut file_path = format!("{}{}", root_dir, request.path);
        
        // If path ends with '/', append 'index.html'
        if file_path.ends_with('/') {
            file_path.push_str("index.html");
        }
        
        let path = Path::new(&file_path);
        
        // HEAD should return same headers as GET but without body
        match fs::metadata(path) {
            Ok(metadata) => {
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
                    .header("Content-Length", metadata.len().to_string())
                    .body(Body::empty())
                    .unwrap())
            }
            Err(_) => {
                Some(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header("Content-Type", "text/plain")
                    .body(Body::empty())
                    .unwrap())
            }
        }
    }
    
    async fn handle_delete(&self, request: &PluginRequest, context: &PluginContext) -> Option<Response<Body>> {
        // Use host-specific root if available, otherwise fall back to plugin config
        let root_dir = context.host_config.get("hostRoot")
            .unwrap_or(&self.root_dir);
        let file_path = format!("{}{}", root_dir, request.path);
        let path = Path::new(&file_path);
        
        match fs::remove_file(path) {
            Ok(_) => {
                Some(Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "text/plain") 
                    .body(Body::from("File deleted successfully"))
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
        match *request.http_request.method() {
            Method::GET => self.handle_get(request, context).await,
            Method::HEAD => self.handle_head(request, context).await,
            Method::PUT => self.handle_put(request, context).await,
            Method::POST => self.handle_post(request, context).await,
            Method::DELETE => self.handle_delete(request, context).await,
            Method::OPTIONS => {
                Some(Response::builder()
                    .status(StatusCode::OK)
                    .header("Allow", "GET, PUT, DELETE, OPTIONS, POST, HEAD")
                    .header("Accept-Ranges", "selector")
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