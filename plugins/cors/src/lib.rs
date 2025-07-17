//! CORS Plugin for Rusty Beam
//!
//! This plugin provides Cross-Origin Resource Sharing (CORS) support to enable
//! controlled access to resources from different origins. It handles both simple
//! and preflight CORS requests according to the W3C CORS specification.
//!
//! ## Features
//! - **Preflight Request Handling**: Automatic OPTIONS request processing
//! - **Configurable Origins**: Support for specific origins or wildcard (*)
//! - **Method Control**: Customizable allowed HTTP methods
//! - **Header Management**: Control over allowed and exposed headers
//! - **Credentials Support**: Optional cookie/auth header support
//! - **Cache Control**: Configurable preflight cache duration
//!
//! ## Configuration
//! - `allowed_origins`: Comma-separated list of allowed origins (default: "*")
//! - `allowed_methods`: Comma-separated HTTP methods (default: "GET,POST,PUT,DELETE,OPTIONS")
//! - `allowed_headers`: Comma-separated request headers (default: "Content-Type,Authorization,X-Requested-With")
//! - `exposed_headers`: Comma-separated response headers to expose (default: none)
//! - `allow_credentials`: Enable credentials support (default: false)
//! - `max_age`: Preflight cache duration in seconds (default: none)
//!
//! ## CORS Headers
//! - **Access-Control-Allow-Origin**: Specifies allowed origins
//! - **Access-Control-Allow-Methods**: Lists allowed HTTP methods
//! - **Access-Control-Allow-Headers**: Lists allowed request headers
//! - **Access-Control-Expose-Headers**: Lists headers exposed to client
//! - **Access-Control-Allow-Credentials**: Enables credential support
//! - **Access-Control-Max-Age**: Preflight response cache duration
//!
//! ## Security Notes
//! When `allow_credentials` is true, wildcard (*) origins are automatically
//! replaced with the specific requesting origin for security compliance.

use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, PluginResponse, create_plugin};
use async_trait::async_trait;
use hyper::{Body, Response, StatusCode, Method, header::HeaderValue};
use std::collections::HashMap;

/// Plugin for Cross-Origin Resource Sharing (CORS) support
#[derive(Debug)]
pub struct CorsPlugin {
    name: String,
    allowed_origins: Vec<String>,
    allowed_methods: Vec<String>,
    allowed_headers: Vec<String>,
    exposed_headers: Vec<String>,
    allow_credentials: bool,
    max_age: Option<u32>,
}

impl CorsPlugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        let name = config.get("name").cloned().unwrap_or_else(|| "cors".to_string());
        
        let allowed_origins = Self::parse_comma_separated_config(
            &config, 
            "allowed_origins", 
            vec!["*".to_string()]
        );
        
        let allowed_methods = Self::parse_comma_separated_config(
            &config,
            "allowed_methods",
            Self::default_allowed_methods()
        );
        
        let allowed_headers = Self::parse_comma_separated_config(
            &config,
            "allowed_headers",
            Self::default_allowed_headers()
        );
        
        let exposed_headers = Self::parse_comma_separated_config(
            &config,
            "exposed_headers",
            vec![]
        );
        
        let allow_credentials = Self::parse_boolean_config(&config, "allow_credentials", false);
        let max_age = Self::parse_numeric_config(&config, "max_age");
        
        Self {
            name,
            allowed_origins,
            allowed_methods,
            allowed_headers,
            exposed_headers,
            allow_credentials,
            max_age,
        }
    }
    
    /// Parse comma-separated configuration value with fallback default
    fn parse_comma_separated_config(
        config: &HashMap<String, String>,
        key: &str,
        default: Vec<String>
    ) -> Vec<String> {
        config.get(key)
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or(default)
    }
    
    /// Parse boolean configuration value with fallback default
    fn parse_boolean_config(config: &HashMap<String, String>, key: &str, default: bool) -> bool {
        config.get(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }
    
    /// Parse numeric configuration value (returns None if invalid or missing)
    fn parse_numeric_config<T: std::str::FromStr>(config: &HashMap<String, String>, key: &str) -> Option<T> {
        config.get(key).and_then(|v| v.parse().ok())
    }
    
    /// Get default allowed HTTP methods
    fn default_allowed_methods() -> Vec<String> {
        vec![
            "GET".to_string(),
            "POST".to_string(),
            "PUT".to_string(),
            "DELETE".to_string(),
            "OPTIONS".to_string(),
        ]
    }
    
    /// Get default allowed request headers
    fn default_allowed_headers() -> Vec<String> {
        vec![
            "Content-Type".to_string(),
            "Authorization".to_string(),
            "X-Requested-With".to_string(),
        ]
    }
    
    /// Check if origin is allowed based on configuration
    fn is_origin_allowed(&self, origin: &str) -> bool {
        self.has_wildcard_origin() || self.allowed_origins.contains(&origin.to_string())
    }
    
    /// Check if wildcard origin (*) is configured
    fn has_wildcard_origin(&self) -> bool {
        self.allowed_origins.contains(&"*".to_string())
    }
    
    /// Get the appropriate Access-Control-Allow-Origin header value
    /// Handles credentials security by never returning "*" when credentials are enabled
    fn get_allowed_origin(&self, request_origin: Option<&str>) -> Option<String> {
        let origin = request_origin?;
        
        if !self.is_origin_allowed(origin) {
            return None;
        }
        
        // Security: When credentials are allowed, never use wildcard
        // Must specify exact origin for CORS security compliance
        if self.allow_credentials {
            Some(origin.to_string())
        } else if self.has_wildcard_origin() {
            Some("*".to_string())
        } else {
            Some(origin.to_string())
        }
    }
    
    /// Create CORS preflight response for OPTIONS requests
    fn create_preflight_response(&self, request: &PluginRequest) -> Response<Body> {
        let mut response = Response::builder()
            .status(StatusCode::NO_CONTENT);
        
        let origin = self.extract_origin_from_request(request);
        
        // Add common CORS headers
        response = self.add_cors_headers_to_builder(response, origin);
        
        // Add preflight-specific headers
        response = self.add_preflight_headers_to_builder(response);
        
        response.body(Body::empty())
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("Failed to create CORS preflight response"))
                    .unwrap()
            })
    }
    
    /// Add preflight-specific headers to response builder
    fn add_preflight_headers_to_builder(&self, mut response: hyper::http::response::Builder) -> hyper::http::response::Builder {
        // Set Access-Control-Allow-Methods
        if !self.allowed_methods.is_empty() {
            response = response.header("Access-Control-Allow-Methods", self.allowed_methods.join(", "));
        }
        
        // Set Access-Control-Allow-Headers
        if !self.allowed_headers.is_empty() {
            response = response.header("Access-Control-Allow-Headers", self.allowed_headers.join(", "));
        }
        
        // Set Access-Control-Max-Age
        if let Some(max_age) = self.max_age {
            response = response.header("Access-Control-Max-Age", max_age.to_string());
        }
        
        response
    }
    
    /// Add common CORS headers to response builder
    fn add_cors_headers_to_builder(&self, mut response: hyper::http::response::Builder, origin: Option<&str>) -> hyper::http::response::Builder {
        // Set Access-Control-Allow-Origin
        if let Some(allowed_origin) = self.get_allowed_origin(origin) {
            response = response.header("Access-Control-Allow-Origin", allowed_origin);
        }
        
        // Set Access-Control-Allow-Credentials
        if self.allow_credentials {
            response = response.header("Access-Control-Allow-Credentials", "true");
        }
        
        response
    }
    
    /// Extract origin header from request
    fn extract_origin_from_request<'a>(&self, request: &'a PluginRequest) -> Option<&'a str> {
        request.http_request.headers()
            .get("origin")
            .and_then(|v| v.to_str().ok())
    }
    
    /// Add CORS headers to existing response
    fn add_cors_headers(&self, response: &mut Response<Body>, request_origin: Option<&str>) {
        let headers = response.headers_mut();
        
        // Set Access-Control-Allow-Origin
        if let Some(allowed_origin) = self.get_allowed_origin(request_origin) {
            if let Ok(header_value) = HeaderValue::from_str(&allowed_origin) {
                headers.insert("Access-Control-Allow-Origin", header_value);
            }
        }
        
        // Set Access-Control-Expose-Headers
        if !self.exposed_headers.is_empty() {
            let exposed_headers_str = self.exposed_headers.join(", ");
            if let Ok(header_value) = HeaderValue::from_str(&exposed_headers_str) {
                headers.insert("Access-Control-Expose-Headers", header_value);
            }
        }
        
        // Set Access-Control-Allow-Credentials
        if self.allow_credentials {
            headers.insert(
                "Access-Control-Allow-Credentials",
                HeaderValue::from_static("true")
            );
        }
    }
}

#[async_trait]
impl Plugin for CorsPlugin {
    async fn handle_request(&self, request: &mut PluginRequest, _context: &PluginContext) -> Option<PluginResponse> {
        // Handle CORS preflight requests (OPTIONS method)
        if request.http_request.method() == Method::OPTIONS {
            // Check if this is a CORS preflight request
            if request.http_request.headers().contains_key("Access-Control-Request-Method") {
                return Some(self.create_preflight_response(request).into());
            }
        }
        
        // Let request continue to other plugins
        None
    }
    
    async fn handle_response(&self, request: &PluginRequest, response: &mut Response<Body>, _context: &PluginContext) {
        let origin = self.extract_origin_from_request(request);
        self.add_cors_headers(response, origin);
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

// Export the plugin creation function
create_plugin!(CorsPlugin);