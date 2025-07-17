//! Error Handler Plugin for Rusty Beam
//!
//! This plugin provides comprehensive error page handling with customizable HTML templates
//! and error logging capabilities. It replaces default HTTP error responses with
//! user-friendly, branded error pages while maintaining proper HTTP status codes.
//!
//! ## Features
//! - **Custom Error Pages**: Load error pages from configurable HTML files
//! - **Template Variables**: Dynamic content substitution in error pages
//! - **Fallback Templates**: Built-in default templates when custom files aren't found
//! - **Error Logging**: Configurable error logging with request context
//! - **Multiple Error Codes**: Support for any HTTP error status code
//! - **Document Root Aware**: Automatically locates error pages in document root
//!
//! ## Configuration
//! - `error_page_{code}`: Path to HTML file for specific error code (e.g., `error_page_404=errors/not-found.html`)
//! - `log_errors`: Enable error logging (default: true)
//! - `error_template_dir`: Directory containing error templates (default: document root)
//!
//! ## Template Variables
//! Error page HTML files support these template variables:
//! - `{status_code}`: HTTP status code (e.g., 404, 500)
//! - `{reason}`: HTTP status reason phrase (e.g., "Not Found", "Internal Server Error")
//! - `{path}`: Request path that caused the error
//! - `{host}`: Server hostname
//! - `{timestamp}`: Error occurrence timestamp
//!
//! ## Example Error Page Template
//! ```html
//! <!DOCTYPE html>
//! <html>
//! <head><title>Error {status_code} - {reason}</title></head>
//! <body>
//!     <h1>{status_code} - {reason}</h1>
//!     <p>The requested path '{path}' could not be found on {host}.</p>
//!     <p><small>Generated at {timestamp}</small></p>
//! </body>
//! </html>
//! ```
//!
//! ## Default Error Pages
//! If no custom error pages are configured, the plugin will look for:
//! - `404.html` for Not Found errors
//! - `500.html` for Internal Server Error
//! - `403.html` for Forbidden errors

use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, PluginResponse, create_plugin};
use async_trait::async_trait;
use hyper::{Body, Response};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Plugin for custom error pages and error logging
#[derive(Debug)]
pub struct ErrorHandlerPlugin {
    name: String,
    error_pages: HashMap<u16, String>,
    error_template_dir: Option<String>,
    log_errors: bool,
}

/// Template variables for error page substitution
#[derive(Debug, Clone)]
struct ErrorPageVariables {
    status_code: u16,
    reason: String,
    path: String,
    host: String,
    timestamp: String,
}

impl ErrorHandlerPlugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        let name = Self::parse_string_config(&config, "name", "error-handler");
        let log_errors = Self::parse_boolean_config(&config, "log_errors", true);
        let error_template_dir = config.get("error_template_dir").cloned();
        let error_pages = Self::parse_error_page_mappings(&config);
        
        Self { 
            name, 
            error_pages, 
            error_template_dir,
            log_errors 
        }
    }
    
    /// Parse string configuration value with fallback default
    fn parse_string_config(config: &HashMap<String, String>, key: &str, default: &str) -> String {
        config.get(key).cloned().unwrap_or_else(|| default.to_string())
    }
    
    /// Parse boolean configuration value with fallback default  
    fn parse_boolean_config(config: &HashMap<String, String>, key: &str, default: bool) -> bool {
        config.get(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }
    
    /// Parse error page mappings from configuration
    fn parse_error_page_mappings(config: &HashMap<String, String>) -> HashMap<u16, String> {
        let mut error_pages = HashMap::new();
        
        // Parse error page mappings from config (e.g., error_page_404=custom-404.html)
        for (key, value) in config {
            if key.starts_with("error_page_") {
                if let Ok(status_code) = key[11..].parse::<u16>() {
                    error_pages.insert(status_code, value.clone());
                }
            }
        }
        
        // Set default error pages if none configured
        if error_pages.is_empty() {
            error_pages.extend(Self::default_error_page_mappings());
        }
        
        error_pages
    }
    
    /// Get default error page file mappings
    fn default_error_page_mappings() -> HashMap<u16, String> {
        [
            (404, "404.html".to_string()),
            (500, "500.html".to_string()),
            (403, "403.html".to_string()),
        ].into_iter().collect()
    }
    
    /// Load and process error page template with variable substitution
    fn load_error_page(&self, status_code: u16, variables: &ErrorPageVariables, context: &PluginContext) -> Option<String> {
        let template_path = self.get_error_page_path(status_code, context)?;
        let template_content = fs::read_to_string(&template_path).ok()?;
        
        Some(self.substitute_template_variables(&template_content, variables))
    }
    
    /// Get the full path to an error page template
    fn get_error_page_path(&self, status_code: u16, context: &PluginContext) -> Option<PathBuf> {
        let error_page_file = self.error_pages.get(&status_code)?;
        
        // Use error_template_dir if specified, otherwise use document root
        let base_dir = match &self.error_template_dir {
            Some(dir) => dir.as_str(),
            None => context.get_config("document_root").unwrap_or("./"),
        };
            
        let error_path = Path::new(base_dir).join(error_page_file);
        
        // Return path if file exists
        if error_path.exists() {
            Some(error_path)
        } else {
            None
        }
    }
    
    /// Substitute template variables in error page content
    fn substitute_template_variables(&self, content: &str, variables: &ErrorPageVariables) -> String {
        content
            .replace("{status_code}", &variables.status_code.to_string())
            .replace("{reason}", &variables.reason)
            .replace("{path}", &variables.path)
            .replace("{host}", &variables.host)
            .replace("{timestamp}", &variables.timestamp)
    }
    
    /// Generate default error page using built-in template
    fn generate_default_error_page(&self, variables: &ErrorPageVariables) -> String {
        let default_template = self.get_default_error_template();
        self.substitute_template_variables(&default_template, variables)
    }
    
    /// Get the built-in default error page template
    fn get_default_error_template(&self) -> String {
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Error {status_code} - {reason}</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            max-width: 600px;
            margin: 50px auto;
            padding: 20px;
            background-color: #f8f9fa;
            color: #343a40;
            line-height: 1.6;
        }
        .error-container {
            background-color: white;
            padding: 40px;
            border-radius: 8px;
            box-shadow: 0 4px 12px rgba(0,0,0,0.1);
            border-left: 4px solid #dc3545;
        }
        .error-code {
            font-size: 72px;
            font-weight: 700;
            color: #dc3545;
            margin: 0;
            line-height: 1;
        }
        h1 {
            color: #dc3545;
            margin: 20px 0 10px 0;
            font-size: 28px;
            font-weight: 600;
        }
        .error-details {
            background-color: #f8f9fa;
            padding: 15px;
            border-radius: 4px;
            margin: 20px 0;
            font-family: 'Courier New', monospace;
            font-size: 14px;
        }
        .footer {
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #dee2e6;
            color: #6c757d;
            font-size: 14px;
        }
        a {
            color: #007bff;
            text-decoration: none;
        }
        a:hover {
            text-decoration: underline;
        }
    </style>
</head>
<body>
    <div class="error-container">
        <div class="error-code">{status_code}</div>
        <h1>{reason}</h1>
        <p>The requested resource could not be found or accessed on this server.</p>
        
        <div class="error-details">
            <strong>Request Path:</strong> {path}<br>
            <strong>Server:</strong> {host}<br>
            <strong>Timestamp:</strong> {timestamp}
        </div>
        
        <p>If you believe this is an error, please contact the website administrator or try again later.</p>
        
        <div class="footer">
            <p><a href="/">← Return to Home</a></p>
            <p><small>Generated by Rusty Beam Server</small></p>
        </div>
    </div>
</body>
</html>"#.to_string()
    }
    
    /// Create error page variables for template substitution
    fn create_error_variables(
        &self, 
        status_code: u16, 
        reason: &str, 
        request: &PluginRequest, 
        context: &PluginContext
    ) -> ErrorPageVariables {
        ErrorPageVariables {
            status_code,
            reason: reason.to_string(),
            path: request.path.clone(),
            host: context.host_name.clone(),
            timestamp: self.get_current_timestamp(),
        }
    }
    
    /// Get current timestamp in human-readable format
    fn get_current_timestamp(&self) -> String {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => {
                let timestamp = duration.as_secs();
                format!("{}", timestamp) // Could be enhanced with proper date formatting
            }
            Err(_) => "Unknown".to_string(),
        }
    }
    
    /// Create error response with proper headers and error handling
    fn create_error_response(
        &self, 
        status_code: u16, 
        content: String, 
        original_status: hyper::StatusCode
    ) -> Response<Body> {
        Response::builder()
            .status(original_status)
            .header("Content-Type", "text/html; charset=utf-8")
            .header("Cache-Control", "no-cache, no-store, must-revalidate")
            .body(Body::from(content))
            .unwrap_or_else(|_| {
                // Fallback response if error page creation fails
                Response::builder()
                    .status(original_status)
                    .header("Content-Type", "text/plain")
                    .body(Body::from(format!("Error {}: {}", status_code, 
                        original_status.canonical_reason().unwrap_or("Unknown Error"))))
                    .unwrap()
            })
    }
    
    /// Log error details with enhanced context
    fn log_error(&self, variables: &ErrorPageVariables, context: &PluginContext) {
        if self.log_errors {
            context.log_verbose(&format!(
                "[ErrorHandler] {} {} for path: {} (host: {}) at {}", 
                variables.status_code, 
                variables.reason,
                variables.path, 
                variables.host,
                variables.timestamp
            ));
        }
    }
}

#[async_trait]
impl Plugin for ErrorHandlerPlugin {
    async fn handle_request(&self, _request: &mut PluginRequest, _context: &PluginContext) -> Option<PluginResponse> {
        // Error handler doesn't intercept requests, only handles responses
        None
    }
    
    async fn handle_response(&self, request: &PluginRequest, response: &mut Response<Body>, context: &PluginContext) {
        let status = response.status();
        let status_code = status.as_u16();
        
        // Handle error status codes (4xx and 5xx)
        if status_code >= 400 {
            let reason = status.canonical_reason().unwrap_or("Unknown Error");
            let variables = self.create_error_variables(status_code, reason, request, context);
            
            // Log the error
            self.log_error(&variables, context);
            
            // Try to load custom error page, fallback to default template
            let error_content = self.load_error_page(status_code, &variables, context)
                .unwrap_or_else(|| self.generate_default_error_page(&variables));
            
            // Replace response with error page
            *response = self.create_error_response(status_code, error_content, status);
        }
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

// Export the plugin creation function
create_plugin!(ErrorHandlerPlugin);