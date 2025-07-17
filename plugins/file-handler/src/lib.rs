//! File Handler Plugin for Rusty Beam
//!
//! This plugin provides comprehensive file system operations via HTTP methods:
//!
//! ## HTTP Methods Supported
//! - **GET**: Serve files and directories (with index.html fallback)
//! - **HEAD**: Return file metadata and headers without body content
//! - **PUT**: Create or update files (follows REST semantics)
//! - **POST**: Append content to existing files
//! - **DELETE**: Remove files from the filesystem
//! - **OPTIONS**: Return allowed methods and capabilities
//!
//! ## Features
//! - Content-type detection based on file extensions
//! - Directory traversal protection (canonicalization)
//! - Automatic index.html serving for directories
//! - Proper HTTP status codes (201 Created, 200 OK, etc.)
//! - Host-specific document root support
//! - RFC 7231 compliant HTTP semantics

use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, PluginResponse, create_plugin};
use async_trait::async_trait;
use hyper::{Body, Response, StatusCode, Method};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;

/// File Handler Plugin for serving and manipulating files via HTTP
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
    
    /// Determines the appropriate Content-Type header based on file extension
    fn get_content_type(path: &Path) -> &'static str {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("html") => "text/html; charset=utf-8",
            Some("css") => "text/css; charset=utf-8", 
            Some("js") => "application/javascript; charset=utf-8",
            Some("mjs") => "application/javascript; charset=utf-8",
            Some("json") => "application/json; charset=utf-8",
            Some("txt") => "text/plain; charset=utf-8",
            Some("png") => "image/png",
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("gif") => "image/gif",
            _ => "application/octet-stream",
        }
    }
    
    /// Builds a file path from the root directory and request path
    /// Automatically appends index.html for directory paths
    fn build_file_path(&self, context: &PluginContext, request_path: &str) -> String {
        // Use host-specific root if available, otherwise fall back to plugin config
        let root_dir = context.host_config.get("hostRoot")
            .unwrap_or(&self.root_dir);
        let mut file_path = format!("{}{}", root_dir, request_path);
        
        // If path ends with '/', append 'index.html'
        if file_path.ends_with('/') {
            file_path.push_str("index.html");
        }
        
        file_path
    }
    
    /// Validates that the resolved path is within the allowed root directory
    /// Returns Ok(canonical_path) if valid, Err(response) if access should be denied
    fn validate_path_security(
        &self, 
        context: &PluginContext, 
        path: &Path
    ) -> Result<PathBuf, Response<Body>> {
        let root_dir = context.host_config.get("hostRoot")
            .unwrap_or(&self.root_dir);
            
        match path.canonicalize() {
            Ok(canonical) => {
                let root_canonical = Path::new(root_dir)
                    .canonicalize()
                    .unwrap_or_else(|_| Path::new(".").to_path_buf());
                    
                if canonical.starts_with(&root_canonical) {
                    Ok(canonical)
                } else {
                    Err(Response::builder()
                        .status(StatusCode::FORBIDDEN)
                        .body(Body::from("Access denied"))
                        .unwrap())
                }
            }
            Err(_) => {
                Err(Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .body(Body::from("Access denied"))
                    .unwrap())
            }
        }
    }
    
    /// Creates a standardized error response
    fn create_error_response(status: StatusCode, message: &str) -> Response<Body> {
        Response::builder()
            .status(status)
            .header("Content-Type", "text/plain")
            .body(Body::from(message.to_string()))
            .unwrap()
    }
    
    /// Handles GET requests to serve files and directories
    async fn handle_get(&self, request: &PluginRequest, context: &PluginContext) -> Option<Response<Body>> {
        let file_path = self.build_file_path(context, &request.path);
        let path = Path::new(&file_path);
        
        // Validate path security
        if let Err(error_response) = self.validate_path_security(context, path) {
            return Some(error_response);
        }
        
        // Try to serve the requested file
        match self.serve_file(path) {
            Ok(response) => Some(response),
            Err(_) => self.try_serve_directory_index(path),
        }
    }
    
    /// Attempts to serve a file directly
    fn serve_file(&self, path: &Path) -> Result<Response<Body>, std::io::Error> {
        let contents = fs::read(path)?;
        let content_type = Self::get_content_type(path);
        
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", content_type)
            .body(Body::from(contents))
            .unwrap())
    }
    
    /// Attempts to serve index.html from a directory, or returns 404
    fn try_serve_directory_index(&self, path: &Path) -> Option<Response<Body>> {
        if path.is_dir() {
            let index_path = path.join("index.html");
            match fs::read(&index_path) {
                Ok(contents) => {
                    Some(Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type", "text/html; charset=utf-8")
                        .body(Body::from(contents))
                        .unwrap())
                }
                Err(_) => Some(Self::create_error_response(StatusCode::NOT_FOUND, "File not found"))
            }
        } else {
            Some(Self::create_error_response(StatusCode::NOT_FOUND, "File not found"))
        }
    }
    
    /// Handles PUT requests to create or update files
    async fn handle_put(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<Response<Body>> {
        let file_path = self.build_file_path(context, &request.path);
        let path = Path::new(&file_path);
        
        // Check if file exists before writing to determine correct status code
        let file_existed = path.exists();
        
        // Validate parent directory security (for file creation)
        if let Some(parent) = path.parent() {
            if let Err(error_response) = self.validate_path_security(context, parent) {
                return Some(error_response);
            }
        }
        
        // Get request body
        let body_bytes = match request.get_body().await {
            Ok(bytes) => bytes,
            Err(_) => {
                return Some(Self::create_error_response(
                    StatusCode::BAD_REQUEST, 
                    "Failed to read request body"
                ));
            }
        };
        
        // Write the file
        match self.write_file_safely(path, &body_bytes) {
            Ok(_) => {
                // RFC 7231: 201 for new resources, 200 for updates
                let status = if file_existed { StatusCode::OK } else { StatusCode::CREATED };
                Some(Response::builder()
                    .status(status)
                    .header("Content-Type", "text/plain")
                    .body(Body::from("File uploaded successfully"))
                    .unwrap())
            }
            Err(e) => {
                Some(Self::create_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Failed to write file: {}", e)
                ))
            }
        }
    }
    
    /// Safely writes file content, creating parent directories as needed
    fn write_file_safely(&self, path: &Path, content: &[u8]) -> Result<(), std::io::Error> {
        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::write(path, content)
    }
    
    /// Handles POST requests to append content to files
    async fn handle_post(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<Response<Body>> {
        let file_path = self.build_file_path(context, &request.path);
        let path = Path::new(&file_path);
        
        // Validate path security
        if let Some(parent) = path.parent() {
            if let Err(error_response) = self.validate_path_security(context, parent) {
                return Some(error_response);
            }
        }
        
        // Get request body
        let body_bytes = match request.get_body().await {
            Ok(bytes) => bytes,
            Err(_) => {
                return Some(Self::create_error_response(
                    StatusCode::BAD_REQUEST,
                    "Failed to read request body"
                ));
            }
        };
        
        // Append content to the file (create if it doesn't exist)
        match self.append_to_file(path, &body_bytes) {
            Ok(_) => {
                Some(Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "text/plain")
                    .body(Body::from("Content appended successfully"))
                    .unwrap())
            }
            Err(e) => {
                Some(Self::create_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Failed to append to file: {}", e)
                ))
            }
        }
    }
    
    /// Appends content to a file, creating it if it doesn't exist
    fn append_to_file(&self, path: &Path, content: &[u8]) -> Result<(), std::io::Error> {
        use std::io::Write;
        
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
            
        file.write_all(content)
    }
    
    /// Handles HEAD requests to return file metadata without body
    async fn handle_head(&self, request: &PluginRequest, context: &PluginContext) -> Option<Response<Body>> {
        let file_path = self.build_file_path(context, &request.path);
        let path = Path::new(&file_path);
        
        // Validate path security
        if let Err(error_response) = self.validate_path_security(context, path) {
            return Some(error_response);
        }
        
        // HEAD should return same headers as GET but without body
        match fs::metadata(path) {
            Ok(metadata) => {
                let content_type = Self::get_content_type(path);
                
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
    
    /// Handles DELETE requests to remove files
    async fn handle_delete(&self, request: &PluginRequest, context: &PluginContext) -> Option<Response<Body>> {
        let file_path = self.build_file_path(context, &request.path);
        let path = Path::new(&file_path);
        
        // Validate path security
        if let Err(error_response) = self.validate_path_security(context, path) {
            return Some(error_response);
        }
        
        match fs::remove_file(path) {
            Ok(_) => {
                Some(Response::builder()
                    .status(StatusCode::NO_CONTENT)
                    .body(Body::empty())
                    .unwrap())
            }
            Err(_) => {
                Some(Self::create_error_response(
                    StatusCode::NOT_FOUND,
                    "File not found"
                ))
            }
        }
    }
}

#[async_trait]
impl Plugin for FileHandlerPlugin {
    async fn handle_request(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<PluginResponse> {
        match *request.http_request.method() {
            Method::GET => self.handle_get(request, context).await.map(|r| r.into()),
            Method::HEAD => self.handle_head(request, context).await.map(|r| r.into()),
            Method::PUT => self.handle_put(request, context).await.map(|r| r.into()),
            Method::POST => self.handle_post(request, context).await.map(|r| r.into()),
            Method::DELETE => self.handle_delete(request, context).await.map(|r| r.into()),
            Method::OPTIONS => {
                Some(Response::builder()
                    .status(StatusCode::OK)
                    .header("Allow", "GET, PUT, DELETE, OPTIONS, POST, HEAD")
                    .header("Accept-Ranges", "selector")
                    .body(Body::empty())
                    .unwrap()
                    .into())
            }
            _ => {
                Some(Response::builder()
                    .status(StatusCode::METHOD_NOT_ALLOWED)
                    .body(Body::from("Method not allowed"))
                    .unwrap()
                    .into())
            }
        }
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

// Export the plugin creation function
create_plugin!(FileHandlerPlugin);