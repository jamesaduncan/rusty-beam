use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, create_plugin};
use async_trait::async_trait;
use hyper::{Body, Response, StatusCode, Method, header::RANGE};
use std::collections::HashMap;
use std::path::Path;
use std::fs;
use dom_query::Document;
use regex::Regex;

/// Plugin for CSS selector-based HTML manipulation
#[derive(Debug)]
pub struct SelectorHandlerPlugin {
    name: String,
    root_dir: String,
}

impl SelectorHandlerPlugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        let name = config.get("name").cloned().unwrap_or_else(|| "selector-handler".to_string());
        let root_dir = config.get("root_dir").cloned().unwrap_or_else(|| ".".to_string());
        
        Self { name, root_dir }
    }
    
    /// Parse Range header for CSS selector
    fn parse_selector_from_range(&self, range_header: &str) -> Option<String> {
        let selector_regex = Regex::new(r"selector=(.*)").ok()?;
        let captures = selector_regex.captures(range_header)?;
        captures.get(1).map(|m| {
            // URL decode the selector value
            urlencoding::decode(m.as_str()).unwrap_or_else(|_| m.as_str().into()).into_owned()
        })
    }
    
    /// Check if file is HTML
    fn is_html_file(&self, path: &str) -> bool {
        path.ends_with(".html") || path.ends_with(".htm")
    }
    
    /// Get body content from request
    async fn get_request_body(&self, request: &mut PluginRequest) -> Result<String, String> {
        request.get_body_string().await
    }
    
    async fn handle_selector_get(&self, request: &PluginRequest, selector: &str, context: &PluginContext) -> Option<Response<Body>> {
        // Handle empty selector
        if selector.is_empty() {
            return Some(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "text/plain")
                .body(Body::from("No elements matched the selector"))
                .unwrap());
        }
        
        // Use host-specific root if available, otherwise fall back to plugin config
        let root_dir = context.host_config.get("hostRoot")
            .unwrap_or(&self.root_dir);
        let file_path = format!("{}{}", root_dir, request.path);
        let path = Path::new(&file_path);
        
        // Security check
        if let Ok(canonical) = path.canonicalize() {
            let root_canonical = Path::new(root_dir).canonicalize().unwrap_or_else(|_| Path::new(".").to_path_buf());
            if !canonical.starts_with(&root_canonical) {
                return Some(Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .body(Body::from("Access denied"))
                    .unwrap());
            }
        }
        
        // Only process HTML files
        if !self.is_html_file(&request.path) {
            return None; // Let file handler deal with non-HTML files
        }
        
        match fs::read_to_string(path) {
            Ok(html_content) => {
                let document = Document::from(html_content.as_str());
                
                // Validate selector first
                let element = document.try_select(selector);
                if element.is_none() {
                    return Some(Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .header("Content-Type", "text/plain")
                        .body(Body::from("No elements matched the selector"))
                        .unwrap());
                }
                
                let final_element = document.select(selector);
                let html_output = final_element.html().to_string();
                let trimmed_output = html_output.trim_end().to_string();
                
                Some(Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "text/html")
                    .body(Body::from(trimmed_output))
                    .unwrap())
            }
            Err(_) => {
                Some(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header("Content-Type", "text/plain")
                    .body(Body::from("File not found"))
                    .unwrap())
            }
        }
    }
    
    async fn handle_selector_put(&self, request: &mut PluginRequest, selector: &str, context: &PluginContext) -> Option<Response<Body>> {
        // Use host-specific root if available, otherwise fall back to plugin config
        let root_dir = context.host_config.get("hostRoot")
            .unwrap_or(&self.root_dir);
        let file_path = format!("{}{}", root_dir, request.path);
        let path = Path::new(&file_path);
        
        // Security check
        if let Ok(canonical) = path.canonicalize() {
            let root_canonical = Path::new(root_dir).canonicalize().unwrap_or_else(|_| Path::new(".").to_path_buf());
            if !canonical.starts_with(&root_canonical) {
                return Some(Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .body(Body::from("Access denied"))
                    .unwrap());
            }
        }
        
        // Only process HTML files
        if !self.is_html_file(&request.path) {
            return None; // Let file handler deal with non-HTML files
        }
        
        // Get new content from request body
        let new_content = match self.get_request_body(request).await {
            Ok(content) => content,
            Err(_) => {
                return Some(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("Content-Type", "text/plain")
                    .body(Body::from("Invalid request body"))
                    .unwrap());
            }
        };
        
        match fs::read_to_string(path) {
            Ok(html_content) => {
                // Do all DOM processing in a block to ensure it completes before async operations
                let final_content_string = {
                    let document = Document::from(html_content.as_str());
                    
                    // Validate selector first
                    let element = document.try_select(selector);
                    if element.is_none() {
                        return Some(Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .header("Content-Type", "text/plain")
                            .body(Body::from("No elements matched the selector"))
                            .unwrap());
                    }
                    
                    let final_element = document.select(selector).first();
                    
                    // Handle table elements specially (like in the original implementation)
                    if new_content.trim().starts_with("<td") || new_content.trim().starts_with("<tr") || 
                       new_content.trim().starts_with("<th") || new_content.trim().starts_with("<tbody") ||
                       new_content.trim().starts_with("<thead") || new_content.trim().starts_with("<tfoot") {
                        
                        // Create a temporary unique marker
                        let marker = format!("__RUSTY_BEAM_REPLACE_MARKER_{}__", std::process::id());
                        final_element.replace_with_html(marker.clone());
                        
                        // Get the document HTML and replace the marker with our content
                        let document_html = document.html().to_string();
                        let modified_html = document_html.replace(&marker, &new_content);
                        
                        // Parse the modified HTML and return it
                        let new_doc = Document::from(modified_html);
                        new_doc.html().to_string().trim_end().to_string()
                    } else {
                        final_element.replace_with_html(new_content);
                        document.html().to_string().trim_end().to_string()
                    }
                };
                
                // Write the modified HTML back to the file
                match fs::write(path, final_content_string.clone()) {
                    Ok(_) => {
                        Some(Response::builder()
                            .status(StatusCode::OK)
                            .header("Content-Type", "text/html")
                            .body(Body::from(final_content_string))
                            .unwrap())
                    }
                    Err(e) => {
                        Some(Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .header("Content-Type", "text/plain")
                            .body(Body::from(format!("Failed to write file: {}", e)))
                            .unwrap())
                    }
                }
            }
            Err(_) => {
                Some(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header("Content-Type", "text/plain")
                    .body(Body::from("File not found"))
                    .unwrap())
            }
        }
    }
    
    async fn handle_selector_post(&self, request: &mut PluginRequest, selector: &str, context: &PluginContext) -> Option<Response<Body>> {
        // Use host-specific root if available, otherwise fall back to plugin config
        let root_dir = context.host_config.get("hostRoot")
            .unwrap_or(&self.root_dir);
        let file_path = format!("{}{}", root_dir, request.path);
        let path = Path::new(&file_path);
        
        // Security check
        if let Ok(canonical) = path.canonicalize() {
            let root_canonical = Path::new(root_dir).canonicalize().unwrap_or_else(|_| Path::new(".").to_path_buf());
            if !canonical.starts_with(&root_canonical) {
                return Some(Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .body(Body::from("Access denied"))
                    .unwrap());
            }
        }
        
        // Only process HTML files
        if !self.is_html_file(&request.path) {
            return None;
        }
        
        // Get new content from request body
        let new_content = match self.get_request_body(request).await {
            Ok(content) => content,
            Err(_) => {
                return Some(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("Content-Type", "text/plain")
                    .body(Body::from("Invalid request body"))
                    .unwrap());
            }
        };
        
        match fs::read_to_string(path) {
            Ok(html_content) => {
                // Do all DOM processing in a block to ensure it completes before async operations
                let final_content_string = {
                    let document = Document::from(html_content.as_str());
                    
                    // Validate selector first
                    let element = document.try_select(selector);
                    if element.is_none() {
                        return Some(Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .header("Content-Type", "text/plain")
                            .body(Body::from("No elements matched the selector"))
                            .unwrap());
                    }
                    
                    let final_element = document.select(selector).first();
                    final_element.append_html(new_content);
                    document.html().to_string()
                };
                
                // Write the modified HTML back to the file
                match fs::write(path, final_content_string.clone()) {
                    Ok(_) => {
                        Some(Response::builder()
                            .status(StatusCode::OK)
                            .header("Content-Type", "text/html")
                            .body(Body::from(final_content_string))
                            .unwrap())
                    }
                    Err(e) => {
                        Some(Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .header("Content-Type", "text/plain")
                            .body(Body::from(format!("Failed to write file: {}", e)))
                            .unwrap())
                    }
                }
            }
            Err(_) => {
                Some(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header("Content-Type", "text/plain")
                    .body(Body::from("File not found"))
                    .unwrap())
            }
        }
    }
    
    async fn handle_selector_delete(&self, request: &PluginRequest, selector: &str, context: &PluginContext) -> Option<Response<Body>> {
        // Use host-specific root if available, otherwise fall back to plugin config
        let root_dir = context.host_config.get("hostRoot")
            .unwrap_or(&self.root_dir);
        let file_path = format!("{}{}", root_dir, request.path);
        let path = Path::new(&file_path);
        
        // Security check
        if let Ok(canonical) = path.canonicalize() {
            let root_canonical = Path::new(root_dir).canonicalize().unwrap_or_else(|_| Path::new(".").to_path_buf());
            if !canonical.starts_with(&root_canonical) {
                return Some(Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .body(Body::from("Access denied"))
                    .unwrap());
            }
        }
        
        // Only process HTML files
        if !self.is_html_file(&request.path) {
            return None;
        }
        
        match fs::read_to_string(path) {
            Ok(html_content) => {
                // Do all DOM processing in a block to ensure it completes before async operations
                let final_content_string = {
                    let document = Document::from(html_content.as_str());
                    
                    // Validate selector first
                    let element = document.try_select(selector);
                    if element.is_none() {
                        return Some(Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .header("Content-Type", "text/plain")
                            .body(Body::from("No elements matched the selector"))
                            .unwrap());
                    }
                    
                    document.select(selector).first().remove();
                    document.html().to_string()
                };
                
                // Write the modified HTML back to the file
                match fs::write(path, final_content_string.clone()) {
                    Ok(_) => {
                        Some(Response::builder()
                            .status(StatusCode::NO_CONTENT)
                            .header("Content-Type", "text/plain")
                            .body(Body::from(""))
                            .unwrap())
                    }
                    Err(e) => {
                        Some(Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .header("Content-Type", "text/plain")
                            .body(Body::from(format!("Failed to write file: {}", e)))
                            .unwrap())
                    }
                }
            }
            Err(_) => {
                Some(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header("Content-Type", "text/plain")
                    .body(Body::from("File not found"))
                    .unwrap())
            }
        }
    }
}

#[async_trait]
impl Plugin for SelectorHandlerPlugin {
    async fn handle_request(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<Response<Body>> {
        // Check for Range header with CSS selector
        let range_header = match request.http_request.headers().get(RANGE) {
            Some(header) => match header.to_str() {
                Ok(header_str) => header_str,
                Err(_) => return None, // Pass through if invalid header
            },
            None => return None, // No Range header, pass through
        };
        
        // Parse selector from Range header
        let selector = match self.parse_selector_from_range(range_header) {
            Some(sel) => sel,
            None => return None, // Not a selector range, pass through
        };
        
        match *request.http_request.method() {
            Method::GET => self.handle_selector_get(request, &selector, context).await,
            Method::PUT => self.handle_selector_put(request, &selector, context).await,
            Method::POST => self.handle_selector_post(request, &selector, context).await,
            Method::DELETE => self.handle_selector_delete(request, &selector, context).await,
            _ => {
                Some(Response::builder()
                    .status(StatusCode::METHOD_NOT_ALLOWED)
                    .body(Body::from("Method not allowed for selector operations"))
                    .unwrap())
            }
        }
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

// Export the plugin creation function
create_plugin!(SelectorHandlerPlugin);