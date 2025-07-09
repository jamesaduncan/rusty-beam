use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, PluginResponse, create_plugin};
use async_trait::async_trait;
use hyper::{Body, Response, header::HeaderValue};
use std::collections::HashMap;

/// Plugin for security headers (CSP, HSTS, etc.)
#[derive(Debug)]
pub struct SecurityHeadersPlugin {
    name: String,
    csp_policy: Option<String>,
    hsts_max_age: Option<u32>,
    hsts_include_subdomains: bool,
    hsts_preload: bool,
    frame_options: Option<String>,
    content_type_options: bool,
    referrer_policy: Option<String>,
    permissions_policy: Option<String>,
    xss_protection: Option<String>,
}

impl SecurityHeadersPlugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        let name = config.get("name").cloned().unwrap_or_else(|| "security-headers".to_string());
        
        let csp_policy = config.get("csp_policy").cloned()
            .or_else(|| Some("default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'".to_string()));
        
        let hsts_max_age = config.get("hsts_max_age")
            .and_then(|v| v.parse().ok())
            .or(Some(31536000)); // Default to 1 year
        
        let hsts_include_subdomains = config.get("hsts_include_subdomains")
            .map(|v| v.parse().unwrap_or(true))
            .unwrap_or(true);
        
        let hsts_preload = config.get("hsts_preload")
            .map(|v| v.parse().unwrap_or(false))
            .unwrap_or(false);
        
        let frame_options = config.get("frame_options").cloned()
            .or_else(|| Some("SAMEORIGIN".to_string()));
        
        let content_type_options = config.get("content_type_options")
            .map(|v| v.parse().unwrap_or(true))
            .unwrap_or(true);
        
        let referrer_policy = config.get("referrer_policy").cloned()
            .or_else(|| Some("strict-origin-when-cross-origin".to_string()));
        
        let permissions_policy = config.get("permissions_policy").cloned();
        
        let xss_protection = config.get("xss_protection").cloned()
            .or_else(|| Some("1; mode=block".to_string()));
        
        Self {
            name,
            csp_policy,
            hsts_max_age,
            hsts_include_subdomains,
            hsts_preload,
            frame_options,
            content_type_options,
            referrer_policy,
            permissions_policy,
            xss_protection,
        }
    }
    
    /// Check if the request is using HTTPS
    fn is_https_request(&self, request: &PluginRequest) -> bool {
        // Check X-Forwarded-Proto header (for proxy scenarios)
        if let Some(proto) = request.http_request.headers().get("x-forwarded-proto") {
            if let Ok(proto_str) = proto.to_str() {
                return proto_str.to_lowercase() == "https";
            }
        }
        
        // Check the URI scheme (though this might not be reliable in all setups)
        request.http_request.uri().scheme_str() == Some("https")
    }
    
    /// Add security headers to response
    fn add_security_headers(&self, request: &PluginRequest, response: &mut Response<Body>) {
        let headers = response.headers_mut();
        
        // Content Security Policy
        if let Some(csp) = &self.csp_policy {
            headers.insert("Content-Security-Policy", HeaderValue::from_str(csp).unwrap());
        }
        
        // HTTP Strict Transport Security (only over HTTPS)
        if self.is_https_request(request) {
            if let Some(max_age) = self.hsts_max_age {
                let mut hsts_value = format!("max-age={}", max_age);
                if self.hsts_include_subdomains {
                    hsts_value.push_str("; includeSubDomains");
                }
                if self.hsts_preload {
                    hsts_value.push_str("; preload");
                }
                headers.insert("Strict-Transport-Security", HeaderValue::from_str(&hsts_value).unwrap());
            }
        }
        
        // X-Frame-Options
        if let Some(frame_options) = &self.frame_options {
            headers.insert("X-Frame-Options", HeaderValue::from_str(frame_options).unwrap());
        }
        
        // X-Content-Type-Options
        if self.content_type_options {
            headers.insert("X-Content-Type-Options", HeaderValue::from_static("nosniff"));
        }
        
        // Referrer-Policy
        if let Some(referrer_policy) = &self.referrer_policy {
            headers.insert("Referrer-Policy", HeaderValue::from_str(referrer_policy).unwrap());
        }
        
        // Permissions-Policy (formerly Feature-Policy)
        if let Some(permissions_policy) = &self.permissions_policy {
            headers.insert("Permissions-Policy", HeaderValue::from_str(permissions_policy).unwrap());
        }
        
        // X-XSS-Protection (legacy, but still useful for older browsers)
        if let Some(xss_protection) = &self.xss_protection {
            headers.insert("X-XSS-Protection", HeaderValue::from_str(xss_protection).unwrap());
        }
        
        // Server header removal/replacement (security by obscurity)
        headers.remove("server");
        headers.insert("Server", HeaderValue::from_static("rusty-beam"));
    }
}

#[async_trait]
impl Plugin for SecurityHeadersPlugin {
    async fn handle_request(&self, _request: &mut PluginRequest, _context: &PluginContext) -> Option<PluginResponse> {
        // Security headers are added during response phase
        None
    }
    
    async fn handle_response(&self, request: &PluginRequest, response: &mut Response<Body>, _context: &PluginContext) {
        self.add_security_headers(request, response);
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

// Export the plugin creation function
create_plugin!(SecurityHeadersPlugin);