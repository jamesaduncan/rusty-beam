use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, create_plugin};
use async_trait::async_trait;
use hyper::{Body, Response, StatusCode, Method, header::{HeaderName, HeaderValue}};
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
        
        let allowed_origins = config.get("allowed_origins")
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_else(|| vec!["*".to_string()]);
        
        let allowed_methods = config.get("allowed_methods")
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_else(|| vec!["GET".to_string(), "POST".to_string(), "PUT".to_string(), "DELETE".to_string(), "OPTIONS".to_string()]);
        
        let allowed_headers = config.get("allowed_headers")
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_else(|| vec!["Content-Type".to_string(), "Authorization".to_string(), "X-Requested-With".to_string()]);
        
        let exposed_headers = config.get("exposed_headers")
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_else(|| vec![]);
        
        let allow_credentials = config.get("allow_credentials")
            .map(|v| v.parse().unwrap_or(false))
            .unwrap_or(false);
        
        let max_age = config.get("max_age")
            .and_then(|v| v.parse().ok());
        
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
    
    /// Check if origin is allowed
    fn is_origin_allowed(&self, origin: &str) -> bool {
        self.allowed_origins.contains(&"*".to_string()) || 
        self.allowed_origins.contains(&origin.to_string())
    }
    
    /// Get the appropriate Access-Control-Allow-Origin header value
    fn get_allowed_origin(&self, request_origin: Option<&str>) -> Option<String> {
        match request_origin {
            Some(origin) if self.is_origin_allowed(origin) => {
                if self.allow_credentials && self.allowed_origins.contains(&"*".to_string()) {
                    // If credentials are allowed, we cannot use "*", must specify exact origin
                    Some(origin.to_string())
                } else if self.allowed_origins.contains(&"*".to_string()) {
                    Some("*".to_string())
                } else {
                    Some(origin.to_string())
                }
            }
            _ => None,
        }
    }
    
    /// Create CORS preflight response
    fn create_preflight_response(&self, request: &PluginRequest) -> Response<Body> {
        let mut response = Response::builder()
            .status(StatusCode::NO_CONTENT);
        
        // Get origin from request
        let origin = request.http_request.headers()
            .get("origin")
            .and_then(|v| v.to_str().ok());
        
        // Set Access-Control-Allow-Origin
        if let Some(allowed_origin) = self.get_allowed_origin(origin) {
            response = response.header("Access-Control-Allow-Origin", allowed_origin);
        }
        
        // Set Access-Control-Allow-Methods
        if !self.allowed_methods.is_empty() {
            response = response.header("Access-Control-Allow-Methods", self.allowed_methods.join(", "));
        }
        
        // Set Access-Control-Allow-Headers
        if !self.allowed_headers.is_empty() {
            response = response.header("Access-Control-Allow-Headers", self.allowed_headers.join(", "));
        }
        
        // Set Access-Control-Allow-Credentials
        if self.allow_credentials {
            response = response.header("Access-Control-Allow-Credentials", "true");
        }
        
        // Set Access-Control-Max-Age
        if let Some(max_age) = self.max_age {
            response = response.header("Access-Control-Max-Age", max_age.to_string());
        }
        
        response.body(Body::empty()).unwrap()
    }
    
    /// Add CORS headers to response
    fn add_cors_headers(&self, response: &mut Response<Body>, request_origin: Option<&str>) {
        // Set Access-Control-Allow-Origin
        if let Some(allowed_origin) = self.get_allowed_origin(request_origin) {
            response.headers_mut().insert(
                "Access-Control-Allow-Origin",
                HeaderValue::from_str(&allowed_origin).unwrap()
            );
        }
        
        // Set Access-Control-Expose-Headers
        if !self.exposed_headers.is_empty() {
            response.headers_mut().insert(
                "Access-Control-Expose-Headers",
                HeaderValue::from_str(&self.exposed_headers.join(", ")).unwrap()
            );
        }
        
        // Set Access-Control-Allow-Credentials
        if self.allow_credentials {
            response.headers_mut().insert(
                "Access-Control-Allow-Credentials",
                HeaderValue::from_static("true")
            );
        }
    }
}

#[async_trait]
impl Plugin for CorsPlugin {
    async fn handle_request(&self, request: &mut PluginRequest, _context: &PluginContext) -> Option<Response<Body>> {
        // Handle CORS preflight requests (OPTIONS method)
        if request.http_request.method() == Method::OPTIONS {
            // Check if this is a CORS preflight request
            if request.http_request.headers().contains_key("Access-Control-Request-Method") {
                return Some(self.create_preflight_response(request));
            }
        }
        
        // Let request continue to other plugins
        None
    }
    
    async fn handle_response(&self, request: &PluginRequest, response: &mut Response<Body>, _context: &PluginContext) {
        // Get origin from request
        let origin = request.http_request.headers()
            .get("origin")
            .and_then(|v| v.to_str().ok());
        
        // Add CORS headers to response
        self.add_cors_headers(response, origin);
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

// Export the plugin creation function
create_plugin!(CorsPlugin);