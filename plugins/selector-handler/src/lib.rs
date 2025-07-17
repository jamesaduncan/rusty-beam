use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, PluginResponse, create_plugin};
use async_trait::async_trait;
use hyper::{Body, Response, StatusCode, Method, header::RANGE};
use std::collections::HashMap;
use std::path::Path;
use std::fs;
use dom_query::Document;
use regex::Regex;

// Constants
const DEFAULT_PLUGIN_NAME: &str = "selector-handler";
const DEFAULT_ROOT_DIR: &str = ".";
const INDEX_FILE_NAME: &str = "index.html";
const MARKER_PREFIX: &str = "__RUSTY_BEAM_";
const MARKER_SUFFIX: &str = "_MARKER_";

// Error messages
const ERROR_NO_ELEMENTS_MATCHED: &str = "No elements matched the selector";
const ERROR_FILE_NOT_FOUND: &str = "File not found";
const ERROR_ACCESS_DENIED: &str = "Access denied";
const ERROR_INVALID_REQUEST_BODY: &str = "Invalid request body";
const ERROR_RANGE_NOT_SATISFIABLE: &str = "Range Not Satisfiable: CSS selectors can only be used with HTML files";
const ERROR_METHOD_NOT_ALLOWED: &str = "Method not allowed for selector operations";

// Content types
const CONTENT_TYPE_HTML: &str = "text/html";
const CONTENT_TYPE_PLAIN: &str = "text/plain";

/// Plugin for CSS selector-based HTML manipulation
#[derive(Debug)]
pub struct SelectorHandlerPlugin {
    name: String,
    root_dir: String,
}

impl SelectorHandlerPlugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        let name = config.get("name").cloned().unwrap_or_else(|| DEFAULT_PLUGIN_NAME.to_string());
        let root_dir = config.get("root_dir").cloned().unwrap_or_else(|| DEFAULT_ROOT_DIR.to_string());
        
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
            let marker = format!("{}{}{}{}__{}", 
                MARKER_PREFIX, 
                operation.to_uppercase(), 
                MARKER_SUFFIX, 
                std::process::id(), 
                "__"
            );
            
            match operation {
                "replace" => {
                    final_element.replace_with_html(marker.clone());
                },
                "append" => {
                    final_element.append_html(marker.clone());
                },
                _ => unreachable!("Invalid operation: {}", operation)
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
                _ => unreachable!("Invalid operation: {}", operation)
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
    
    /// Build file path from request
    fn build_file_path(&self, request: &PluginRequest, context: &PluginContext) -> String {
        let root_dir = context.host_config.get("hostRoot")
            .unwrap_or(&self.root_dir);
        let mut file_path = format!("{}{}", root_dir, request.path);
        
        // If path ends with '/', append 'index.html'
        if file_path.ends_with('/') {
            file_path.push_str(INDEX_FILE_NAME);
        }
        
        file_path
    }
    
    /// Perform security check on file path
    fn check_path_security(&self, file_path: &str, context: &PluginContext) -> Result<(), Response<Body>> {
        let path = Path::new(file_path);
        let root_dir = context.host_config.get("hostRoot")
            .unwrap_or(&self.root_dir);
        
        if let Ok(canonical) = path.canonicalize() {
            let root_canonical = Path::new(root_dir).canonicalize()
                .unwrap_or_else(|_| Path::new(".").to_path_buf());
            if !canonical.starts_with(&root_canonical) {
                return Err(Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .body(Body::from(ERROR_ACCESS_DENIED))
                    .unwrap());
            }
        }
        Ok(())
    }
    
    /// Check if file exists
    fn check_file_exists(&self, file_path: &str) -> Result<(), Response<Body>> {
        let path = Path::new(file_path);
        if !path.exists() {
            return Err(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", CONTENT_TYPE_PLAIN)
                .body(Body::from(ERROR_FILE_NOT_FOUND))
                .unwrap());
        }
        Ok(())
    }
    
    /// Validate that file is HTML
    fn validate_html_file(&self, file_path: &str, selector: &str) -> Result<(), Response<Body>> {
        let filename = Path::new(file_path).file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(file_path);
            
        if !self.is_html_file(filename) {
            return Err(Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header("Content-Type", CONTENT_TYPE_PLAIN)
                .header("Content-Range", format!("selector {}", selector))
                .body(Body::from(ERROR_RANGE_NOT_SATISFIABLE))
                .unwrap());
        }
        Ok(())
    }
    
    /// Common file validation logic
    fn validate_file_for_selector(
        &self, 
        file_path: &str, 
        selector: &str, 
        context: &PluginContext
    ) -> Result<(), Response<Body>> {
        self.check_path_security(file_path, context)?;
        self.check_file_exists(file_path)?;
        self.validate_html_file(file_path, selector)?;
        Ok(())
    }
    
    async fn handle_selector_get(&self, request: &PluginRequest, selector: &str, context: &PluginContext) -> Option<Response<Body>> {
        // Handle empty selector
        if selector.is_empty() {
            return Some(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", CONTENT_TYPE_PLAIN)
                .body(Body::from(ERROR_NO_ELEMENTS_MATCHED))
                .unwrap());
        }
        
        let file_path = self.build_file_path(request, context);
        context.log_verbose(&format!("[selector-handler] GET request - file_path: {}", file_path));
        
        // Validate file
        if let Err(response) = self.validate_file_for_selector(&file_path, selector, context) {
            return Some(response);
        }
        
        match fs::read_to_string(&file_path) {
            Ok(html_content) => {
                context.log_verbose(&format!("[selector-handler] Successfully read file: {}", file_path));
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
                    .header("Content-Type", CONTENT_TYPE_HTML)
                    .header("Content-Range", format!("selector {}", selector))
                    .body(Body::from(trimmed_output))
                    .unwrap())
            }
            Err(e) => {
                context.log_verbose(&format!("[selector-handler] Failed to read file {}: {}", file_path, e));
                Some(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header("Content-Type", "text/plain")
                    .body(Body::from("File not found"))
                    .unwrap())
            }
        }
    }
    
    async fn handle_selector_put(&self, request: &mut PluginRequest, selector: &str, context: &PluginContext) -> Option<Response<Body>> {
        let file_path = self.build_file_path(request, context);
        context.log_verbose(&format!("[selector-handler] PUT request - file_path: {}", file_path));
        
        // Validate file
        if let Err(response) = self.validate_file_for_selector(&file_path, selector, context) {
            return Some(response);
        }
        
        // Get new content from request body
        let new_content = match self.get_request_body(request).await {
            Ok(content) => content,
            Err(_) => {
                return Some(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("Content-Type", CONTENT_TYPE_PLAIN)
                    .body(Body::from(ERROR_INVALID_REQUEST_BODY))
                    .unwrap());
            }
        };
        
        match fs::read_to_string(&file_path) {
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
                match fs::write(&file_path, final_content_string) {
                    Ok(_) => {
                        // Set metadata for other plugins (like WebSocket) to use
                        request.set_metadata("applied_selector".to_string(), selector.to_string());
                        request.set_metadata("selected_content".to_string(), updated_element_html.clone());
                        request.set_metadata("posted_content".to_string(), new_content.clone());
                        
                        // Return just the updated element HTML, not the entire document
                        Some(Response::builder()
                            .status(StatusCode::PARTIAL_CONTENT)
                            .header("Content-Type", CONTENT_TYPE_HTML)
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
        let file_path = self.build_file_path(request, context);
        context.log_verbose(&format!("[selector-handler] POST request - file_path: {}", file_path));
        
        // Validate file
        if let Err(response) = self.validate_file_for_selector(&file_path, selector, context) {
            return Some(response);
        }
        
        // Get new content from request body
        let new_content = match self.get_request_body(request).await {
            Ok(content) => content,
            Err(_) => {
                return Some(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("Content-Type", CONTENT_TYPE_PLAIN)
                    .body(Body::from(ERROR_INVALID_REQUEST_BODY))
                    .unwrap());
            }
        };
        
        match fs::read_to_string(&file_path) {
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
                match fs::write(&file_path, final_content_string) {
                    Ok(_) => {
                        // Set metadata for other plugins (like WebSocket) to use
                        request.set_metadata("applied_selector".to_string(), selector.to_string());
                        request.set_metadata("selected_content".to_string(), updated_element_html.clone());
                        request.set_metadata("posted_content".to_string(), new_content.clone());
                        
                        // For POST, return just the posted content, not the entire target element
                        Some(Response::builder()
                            .status(StatusCode::PARTIAL_CONTENT)
                            .header("Content-Type", CONTENT_TYPE_HTML)
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
        let file_path = self.build_file_path(request, context);
        context.log_verbose(&format!("[selector-handler] DELETE request - file_path: {}", file_path));
        
        // Validate file
        if let Err(response) = self.validate_file_for_selector(&file_path, selector, context) {
            return Some(response);
        }
        
        match fs::read_to_string(&file_path) {
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
                match fs::write(&file_path, final_content_string.0.clone()) {
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
                    .body(Body::from(ERROR_METHOD_NOT_ALLOWED))
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