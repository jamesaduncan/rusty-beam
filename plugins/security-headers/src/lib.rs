//! Security Headers Plugin for Rusty Beam
//!
//! This plugin adds essential security headers to HTTP responses to protect against
//! common web vulnerabilities including XSS, clickjacking, MIME sniffing, and more.
//!
//! ## Security Headers Implemented
//! - **Content-Security-Policy (CSP)**: Prevents XSS attacks by controlling resource loading
//! - **Strict-Transport-Security (HSTS)**: Enforces HTTPS connections
//! - **X-Frame-Options**: Prevents clickjacking attacks
//! - **X-Content-Type-Options**: Prevents MIME sniffing attacks
//! - **Referrer-Policy**: Controls referrer information leakage
//! - **Permissions-Policy**: Controls browser feature permissions
//! - **X-XSS-Protection**: Legacy XSS protection for older browsers
//! - **Server**: Obscures server information
//!
//! ## Features
//! - Configurable policies for all security headers
//! - HTTPS-aware HSTS implementation
//! - Sensible security defaults
//! - Server header replacement for security by obscurity
//!
//! ## Pipeline Integration
//! This plugin operates in the response phase, adding headers after content
//! has been processed by other plugins like file-handler or selector-handler.

use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, PluginResponse, create_plugin};
use async_trait::async_trait;
use hyper::{Body, Response, header::HeaderValue};
use std::collections::HashMap;

// Default configuration values
const DEFAULT_PLUGIN_NAME: &str = "security-headers";
const DEFAULT_CSP_POLICY: &str = "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'";
const DEFAULT_HSTS_MAX_AGE: u32 = 31536000; // 1 year in seconds
const DEFAULT_FRAME_OPTIONS: &str = "SAMEORIGIN";
const DEFAULT_REFERRER_POLICY: &str = "strict-origin-when-cross-origin";
const DEFAULT_XSS_PROTECTION: &str = "1; mode=block";

// Configuration keys
const CONFIG_KEY_NAME: &str = "name";
const CONFIG_KEY_CSP_POLICY: &str = "csp_policy";
const CONFIG_KEY_HSTS_MAX_AGE: &str = "hsts_max_age";
const CONFIG_KEY_HSTS_INCLUDE_SUBDOMAINS: &str = "hsts_include_subdomains";
const CONFIG_KEY_HSTS_PRELOAD: &str = "hsts_preload";
const CONFIG_KEY_FRAME_OPTIONS: &str = "frame_options";
const CONFIG_KEY_CONTENT_TYPE_OPTIONS: &str = "content_type_options";
const CONFIG_KEY_REFERRER_POLICY: &str = "referrer_policy";
const CONFIG_KEY_PERMISSIONS_POLICY: &str = "permissions_policy";
const CONFIG_KEY_XSS_PROTECTION: &str = "xss_protection";

// Header names
const HEADER_X_FORWARDED_PROTO: &str = "x-forwarded-proto";
const HEADER_CONTENT_SECURITY_POLICY: &str = "Content-Security-Policy";
const HEADER_STRICT_TRANSPORT_SECURITY: &str = "Strict-Transport-Security";
const HEADER_X_FRAME_OPTIONS: &str = "X-Frame-Options";
const HEADER_X_CONTENT_TYPE_OPTIONS: &str = "X-Content-Type-Options";
const HEADER_REFERRER_POLICY: &str = "Referrer-Policy";
const HEADER_PERMISSIONS_POLICY: &str = "Permissions-Policy";
const HEADER_X_XSS_PROTECTION: &str = "X-XSS-Protection";
const HEADER_SERVER: &str = "server";

// Header values
const HEADER_VALUE_NOSNIFF: &str = "nosniff";
const HEADER_VALUE_RUSTY_BEAM: &str = "rusty-beam";
const PROTOCOL_HTTPS: &str = "https";
const HSTS_INCLUDE_SUBDOMAINS: &str = "; includeSubDomains";
const HSTS_PRELOAD: &str = "; preload";

/// Security Headers Plugin for adding essential security headers to HTTP responses
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
        let name = config.get(CONFIG_KEY_NAME)
            .cloned()
            .unwrap_or_else(|| DEFAULT_PLUGIN_NAME.to_string());
        
        let csp_policy = config.get(CONFIG_KEY_CSP_POLICY)
            .cloned()
            .or_else(|| Some(DEFAULT_CSP_POLICY.to_string()));
        
        let hsts_max_age = config.get(CONFIG_KEY_HSTS_MAX_AGE)
            .and_then(|v| v.parse().ok())
            .or(Some(DEFAULT_HSTS_MAX_AGE));
        
        let hsts_include_subdomains = config.get(CONFIG_KEY_HSTS_INCLUDE_SUBDOMAINS)
            .map(|v| v.parse().unwrap_or(true))
            .unwrap_or(true);
        
        let hsts_preload = config.get(CONFIG_KEY_HSTS_PRELOAD)
            .map(|v| v.parse().unwrap_or(false))
            .unwrap_or(false);
        
        let frame_options = config.get(CONFIG_KEY_FRAME_OPTIONS)
            .cloned()
            .or_else(|| Some(DEFAULT_FRAME_OPTIONS.to_string()));
        
        let content_type_options = config.get(CONFIG_KEY_CONTENT_TYPE_OPTIONS)
            .map(|v| v.parse().unwrap_or(true))
            .unwrap_or(true);
        
        let referrer_policy = config.get(CONFIG_KEY_REFERRER_POLICY)
            .cloned()
            .or_else(|| Some(DEFAULT_REFERRER_POLICY.to_string()));
        
        let permissions_policy = config.get(CONFIG_KEY_PERMISSIONS_POLICY).cloned();
        
        let xss_protection = config.get(CONFIG_KEY_XSS_PROTECTION)
            .cloned()
            .or_else(|| Some(DEFAULT_XSS_PROTECTION.to_string()));
        
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
    
    /// Determines if the request is using HTTPS by checking headers and URI scheme
    /// 
    /// This method checks both the X-Forwarded-Proto header (for proxy scenarios)
    /// and the URI scheme to determine if HTTPS is being used.
    fn is_https_request(&self, request: &PluginRequest) -> bool {
        // Check X-Forwarded-Proto header (for proxy scenarios)
        if let Some(proto) = request.http_request.headers().get(HEADER_X_FORWARDED_PROTO) {
            if let Ok(proto_str) = proto.to_str() {
                return proto_str.to_lowercase() == PROTOCOL_HTTPS;
            }
        }
        
        // Check the URI scheme (though this might not be reliable in all setups)
        request.http_request.uri().scheme_str() == Some(PROTOCOL_HTTPS)
    }
    
    /// Adds comprehensive security headers to the HTTP response
    /// 
    /// This method adds various security headers to protect against common
    /// web vulnerabilities including XSS, clickjacking, MIME sniffing, etc.
    fn add_security_headers(&self, request: &PluginRequest, response: &mut Response<Body>) {
        let headers = response.headers_mut();
        
        self.add_csp_header(headers);
        self.add_hsts_header(request, headers);
        self.add_frame_options_header(headers);
        self.add_content_type_options_header(headers);
        self.add_referrer_policy_header(headers);
        self.add_permissions_policy_header(headers);
        self.add_xss_protection_header(headers);
        self.add_server_header(headers);
    }
    
    /// Adds Content-Security-Policy header if configured
    fn add_csp_header(&self, headers: &mut hyper::HeaderMap) {
        if let Some(csp) = &self.csp_policy {
            headers.insert(HEADER_CONTENT_SECURITY_POLICY, HeaderValue::from_str(csp).unwrap());
        }
    }
    
    /// Adds Strict-Transport-Security header (only over HTTPS)
    fn add_hsts_header(&self, request: &PluginRequest, headers: &mut hyper::HeaderMap) {
        if self.is_https_request(request) {
            if let Some(max_age) = self.hsts_max_age {
                let hsts_value = self.build_hsts_value(max_age);
                headers.insert(HEADER_STRICT_TRANSPORT_SECURITY, HeaderValue::from_str(&hsts_value).unwrap());
            }
        }
    }
    
    /// Builds the HSTS header value with appropriate directives
    fn build_hsts_value(&self, max_age: u32) -> String {
        let mut hsts_value = format!("max-age={}", max_age);
        if self.hsts_include_subdomains {
            hsts_value.push_str(HSTS_INCLUDE_SUBDOMAINS);
        }
        if self.hsts_preload {
            hsts_value.push_str(HSTS_PRELOAD);
        }
        hsts_value
    }
    
    /// Adds X-Frame-Options header if configured
    fn add_frame_options_header(&self, headers: &mut hyper::HeaderMap) {
        if let Some(frame_options) = &self.frame_options {
            headers.insert(HEADER_X_FRAME_OPTIONS, HeaderValue::from_str(frame_options).unwrap());
        }
    }
    
    /// Adds X-Content-Type-Options header if enabled
    fn add_content_type_options_header(&self, headers: &mut hyper::HeaderMap) {
        if self.content_type_options {
            headers.insert(HEADER_X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static(HEADER_VALUE_NOSNIFF));
        }
    }
    
    /// Adds Referrer-Policy header if configured
    fn add_referrer_policy_header(&self, headers: &mut hyper::HeaderMap) {
        if let Some(referrer_policy) = &self.referrer_policy {
            headers.insert(HEADER_REFERRER_POLICY, HeaderValue::from_str(referrer_policy).unwrap());
        }
    }
    
    /// Adds Permissions-Policy header if configured
    fn add_permissions_policy_header(&self, headers: &mut hyper::HeaderMap) {
        if let Some(permissions_policy) = &self.permissions_policy {
            headers.insert(HEADER_PERMISSIONS_POLICY, HeaderValue::from_str(permissions_policy).unwrap());
        }
    }
    
    /// Adds X-XSS-Protection header if configured (legacy support)
    fn add_xss_protection_header(&self, headers: &mut hyper::HeaderMap) {
        if let Some(xss_protection) = &self.xss_protection {
            headers.insert(HEADER_X_XSS_PROTECTION, HeaderValue::from_str(xss_protection).unwrap());
        }
    }
    
    /// Replaces the Server header for security by obscurity
    fn add_server_header(&self, headers: &mut hyper::HeaderMap) {
        headers.remove(HEADER_SERVER);
        headers.insert(HEADER_SERVER, HeaderValue::from_static(HEADER_VALUE_RUSTY_BEAM));
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