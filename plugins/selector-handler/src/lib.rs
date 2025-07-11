use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, PluginResponse, create_plugin};
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
    
    /// Handle special HTML elements that require preservation of structure
    fn needs_special_handling(&self, content: &str) -> bool {
        let trimmed = content.trim();
        trimmed.starts_with("<td") || trimmed.starts_with("<tr") || 
        trimmed.starts_with("<th") || trimmed.starts_with("<tbody") ||
        trimmed.starts_with("<thead") || trimmed.starts_with("<tfoot") ||
        trimmed.starts_with("<body") || trimmed.starts_with("<li") ||
        trimmed.starts_with("<ul") || trimmed.starts_with("<ol") ||
        trimmed.starts_with("<option") || trimmed.starts_with("<select")
    }
    
    /// Apply content to an element using special handling for table elements
    fn apply_content_with_special_handling(
        &self,
        document: &Document,
        selector: &str,
        new_content: &str,
        operation: &str  // "replace" or "append"
    ) -> (String, String) {
        let final_element = document.select(selector).first();
        
        if self.needs_special_handling(new_content) {
            // Create a temporary unique marker
            let marker = format!("__RUSTY_BEAM_{}_MARKER_{}__", operation.to_uppercase(), std::process::id());
            
            match operation {
                "replace" => {
                    final_element.replace_with_html(marker.clone());
                },
                "append" => {
                    final_element.append_html(marker.clone());
                },
                _ => panic!("Invalid operation: {}", operation)
            }
            
            // Get the document HTML and replace the marker with our content
            let document_html = document.html().to_string();
            let modified_html = document_html.replace(&marker, new_content);
            
            // Parse the modified HTML to get both full doc and the updated element
            let new_doc = Document::from(modified_html);
            let updated_element = new_doc.select(selector).first();
            let updated_html = updated_element.html().to_string().trim_end().to_string();
            
            (new_doc.html().to_string().trim_end().to_string(), updated_html)
        } else {
            // For non-special elements, use regular operations
            match operation {
                "replace" => final_element.replace_with_html(new_content),
                "append" => final_element.append_html(new_content),
                _ => panic!("Invalid operation: {}", operation)
            }
            
            // Get the updated element HTML after operation
            let updated_element = document.select(selector).first();
            let updated_html = updated_element.html().to_string().trim_end().to_string();
            
            (document.html().to_string().trim_end().to_string(), updated_html)
        }
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
        let mut file_path = format!("{}{}", root_dir, request.path);
        
        if context.verbose {
            println!("[selector-handler] GET request - root_dir: {}, request.path: {}, initial file_path: {}", root_dir, request.path, file_path);
        }
        
        // If path ends with '/', append 'index.html'
        if file_path.ends_with('/') {
            file_path.push_str("index.html");
        }
        
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
        
        // Check if file exists first
        if !path.exists() {
            return Some(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "text/plain")
                .body(Body::from("File not found"))
                .unwrap());
        }
        
        // Check if file exists first
        if !path.exists() {
            return Some(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "text/plain")
                .body(Body::from("File not found"))
                .unwrap());
        }
        
        // Only process HTML files
        // Extract just the filename from the full path for checking
        let filename = Path::new(&file_path).file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&file_path);
        if !self.is_html_file(filename) {
            // Return 416 Range Not Satisfiable for non-HTML files with selector
            return Some(Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header("Content-Type", "text/plain")
                .header("Content-Range", format!("selector {}", selector))
                .body(Body::from("Range Not Satisfiable: CSS selectors can only be used with HTML files"))
                .unwrap());
        }
        
        match fs::read_to_string(path) {
            Ok(html_content) => {
                if context.verbose {
                    println!("[selector-handler] Successfully read file: {}", file_path);
                }
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
                    .status(StatusCode::PARTIAL_CONTENT)
                    .header("Content-Type", "text/html")
                    .header("Content-Range", format!("selector {}", selector))
                    .body(Body::from(trimmed_output))
                    .unwrap())
            }
            Err(e) => {
                if context.verbose {
                    println!("[selector-handler] Failed to read file {}: {}", file_path, e);
                }
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
        let mut file_path = format!("{}{}", root_dir, request.path);
        
        // If path ends with '/', append 'index.html'
        if file_path.ends_with('/') {
            file_path.push_str("index.html");
        }
        
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
        
        // Check if file exists first
        if !path.exists() {
            return Some(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "text/plain")
                .body(Body::from("File not found"))
                .unwrap());
        }
        
        // Only process HTML files
        // Extract just the filename from the full path for checking
        let filename = Path::new(&file_path).file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&file_path);
        if !self.is_html_file(filename) {
            // Return 416 Range Not Satisfiable for non-HTML files with selector
            return Some(Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header("Content-Type", "text/plain")
                .header("Content-Range", format!("selector {}", selector))
                .body(Body::from("Range Not Satisfiable: CSS selectors can only be used with HTML files"))
                .unwrap());
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
                let (final_content_string, updated_element_html) = {
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
                    
                    // Use shared method for handling special elements
                    self.apply_content_with_special_handling(&document, selector, &new_content, "replace")
                };
                
                // Write the modified HTML back to the file
                match fs::write(path, final_content_string) {
                    Ok(_) => {
                        // Set metadata for other plugins (like WebSocket) to use
                        request.set_metadata("applied_selector".to_string(), selector.to_string());
                        request.set_metadata("selected_content".to_string(), updated_element_html.clone());
                        request.set_metadata("posted_content".to_string(), new_content.clone());
                        
                        // Return just the updated element HTML, not the entire document
                        Some(Response::builder()
                            .status(StatusCode::PARTIAL_CONTENT)
                            .header("Content-Type", "text/html")
                            .header("Content-Range", format!("selector {}", selector))
                            .body(Body::from(updated_element_html))
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
        let mut file_path = format!("{}{}", root_dir, request.path);
        
        // If path ends with '/', append 'index.html'
        if file_path.ends_with('/') {
            file_path.push_str("index.html");
        }
        
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
        
        // Check if file exists first
        if !path.exists() {
            return Some(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "text/plain")
                .body(Body::from("File not found"))
                .unwrap());
        }
        
        // Only process HTML files
        // Extract just the filename from the full path for checking
        let filename = Path::new(&file_path).file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&file_path);
        if !self.is_html_file(filename) {
            // Return 416 Range Not Satisfiable for non-HTML files with selector
            return Some(Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header("Content-Type", "text/plain")
                .header("Content-Range", format!("selector {}", selector))
                .body(Body::from("Range Not Satisfiable: CSS selectors can only be used with HTML files"))
                .unwrap());
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
                let (final_content_string, updated_element_html) = {
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
                    
                    // Use shared method for handling special elements
                    self.apply_content_with_special_handling(&document, selector, &new_content, "append")
                };
                
                // Write the modified HTML back to the file
                match fs::write(path, final_content_string) {
                    Ok(_) => {
                        // Set metadata for other plugins (like WebSocket) to use
                        request.set_metadata("applied_selector".to_string(), selector.to_string());
                        request.set_metadata("selected_content".to_string(), updated_element_html.clone());
                        request.set_metadata("posted_content".to_string(), new_content.clone());
                        
                        // For POST, return just the posted content, not the entire target element
                        Some(Response::builder()
                            .status(StatusCode::PARTIAL_CONTENT)
                            .header("Content-Type", "text/html")
                            .header("Content-Range", format!("selector {}", selector))
                            .body(Body::from(new_content))
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
    
    async fn handle_selector_delete(&self, request: &mut PluginRequest, selector: &str, context: &PluginContext) -> Option<Response<Body>> {
        // Use host-specific root if available, otherwise fall back to plugin config
        let root_dir = context.host_config.get("hostRoot")
            .unwrap_or(&self.root_dir);
        let mut file_path = format!("{}{}", root_dir, request.path);
        
        // If path ends with '/', append 'index.html'
        if file_path.ends_with('/') {
            file_path.push_str("index.html");
        }
        
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
        
        // Check if file exists first
        if !path.exists() {
            return Some(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "text/plain")
                .body(Body::from("File not found"))
                .unwrap());
        }
        
        // Check if file exists first
        if !path.exists() {
            return Some(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "text/plain")
                .body(Body::from("File not found"))
                .unwrap());
        }
        
        // Only process HTML files
        // Extract just the filename from the full path for checking
        let filename = Path::new(&file_path).file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&file_path);
        if !self.is_html_file(filename) {
            // Return 416 Range Not Satisfiable for non-HTML files with selector
            return Some(Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header("Content-Type", "text/plain")
                .header("Content-Range", format!("selector {}", selector))
                .body(Body::from("Range Not Satisfiable: CSS selectors can only be used with HTML files"))
                .unwrap());
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
                    
                    // Get the content before removing
                    let removed_content = document.select(selector).first().html().to_string();
                    document.select(selector).first().remove();
                    
                    (document.html().to_string(), removed_content)
                };
                
                // Write the modified HTML back to the file
                match fs::write(path, final_content_string.0.clone()) {
                    Ok(_) => {
                        // Set metadata for other plugins (like WebSocket) to use
                        request.set_metadata("applied_selector".to_string(), selector.to_string());
                        request.set_metadata("selected_content".to_string(), final_content_string.1);
                        
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
    async fn handle_request(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<PluginResponse> {
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
            Method::GET => self.handle_selector_get(request, &selector, context).await.map(|r| r.into()),
            Method::PUT => self.handle_selector_put(request, &selector, context).await.map(|r| r.into()),
            Method::POST => self.handle_selector_post(request, &selector, context).await.map(|r| r.into()),
            Method::DELETE => self.handle_selector_delete(request, &selector, context).await.map(|r| r.into()),
            _ => {
                Some(Response::builder()
                    .status(StatusCode::METHOD_NOT_ALLOWED)
                    .body(Body::from("Method not allowed for selector operations"))
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
create_plugin!(SelectorHandlerPlugin);