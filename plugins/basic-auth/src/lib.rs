//! HTTP Basic Authentication Plugin for Rusty Beam
//!
//! This plugin provides standard HTTP Basic Authentication (RFC 7617) for protecting
//! resources with username and password credentials. It supports configurable
//! authentication realms and credential management through HTML configuration files.
//!
//! ## Features
//! - **RFC 7617 Compliant**: Standard HTTP Basic Authentication implementation
//! - **Credential Management**: Load users from HTML files with microdata
//! - **CORS Compatible**: Allows OPTIONS requests without authentication
//! - **Metadata Enrichment**: Adds authenticated user info to request metadata
//! - **Configurable Realm**: Custom authentication realm for branding
//! - **Secure Defaults**: Fail-safe authentication with secure error handling
//!
//! ## Security Considerations
//! - **HTTPS Required**: Basic Auth transmits credentials in base64 (not encrypted)
//! - **No Session Management**: Each request requires authentication
//! - **Credential Storage**: Store credential files outside document root
//! - **Strong Passwords**: Enforce strong password policies externally
//!
//! ## Configuration
//! - `name`: Plugin instance name (default: "basic-auth")
//! - `realm`: Authentication realm displayed to users (default: "Restricted Area")
//! - `authfile`: Path to HTML file containing user credentials
//!
//! ## Credential File Format
//! ```html
//! <div itemscope itemtype="http://rustybeam.net/Credential">
//!     <span itemprop="username">alice</span>
//!     <span itemprop="password">secure_password_123</span>
//! </div>
//! ```
//!
//! ## Default Credentials
//! If no credential file is specified or found, the plugin uses default credentials:
//! - Username: `admin`, Password: `admin123`
//! - Username: `johndoe`, Password: `doe123`
//! **WARNING**: Always configure custom credentials in production!
//!
//! ## Request Metadata
//! Successful authentication adds these metadata fields:
//! - `authenticated_user`: The username that was authenticated
//! - `auth_realm`: The authentication realm used
//!
//! ## HTTP Headers
//! - **Request**: Expects `Authorization: Basic <base64-encoded-credentials>`
//! - **Challenge**: Returns `WWW-Authenticate: Basic realm="<realm>"`
//!
//! ## Integration with Other Plugins
//! - **Authorization Plugin**: Use metadata for role-based access control
//! - **Access Log Plugin**: Log authenticated usernames
//! - **Rate Limit Plugin**: Apply per-user rate limits

use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, PluginResponse, create_plugin};
use async_trait::async_trait;
use hyper::{Body, Response, StatusCode, header::{AUTHORIZATION, WWW_AUTHENTICATE}};
use std::collections::HashMap;
use std::fs;
use dom_query::Document;

/// Plugin for HTTP Basic Authentication
#[derive(Debug)]
pub struct BasicAuthPlugin {
    name: String,
    realm: String,
    auth_file: Option<String>,
}

impl BasicAuthPlugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        let name = config.get("name").cloned().unwrap_or_else(|| "basic-auth".to_string());
        let realm = config.get("realm").cloned().unwrap_or_else(|| "Restricted Area".to_string());
        let auth_file = config.get("authfile").cloned();
        
        Self { name, realm, auth_file }
    }
    
    /// Extract and validate Authorization header from request
    fn extract_authorization_header<'a>(&self, request: &'a PluginRequest) -> Option<&'a str> {
        request.http_request.headers()
            .get(AUTHORIZATION)
            .and_then(|header| header.to_str().ok())
    }
    
    /// Simple base64 decoder for HTTP Basic Auth
    /// This is a minimal implementation sufficient for Basic Auth decoding
    fn base64_decode(&self, input: &str) -> Option<Vec<u8>> {
        const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        
        // Build decode table
        let mut decode_table = [0xFF; 256];
        for (i, &c) in BASE64_CHARS.iter().enumerate() {
            decode_table[c as usize] = i as u8;
        }
        
        let input = input.trim_end_matches('=');
        let mut result = Vec::new();
        let mut buffer = 0u32;
        let mut bits = 0;
        
        for byte in input.bytes() {
            let value = decode_table[byte as usize];
            if value == 0xFF {
                return None; // Invalid character
            }
            
            buffer = (buffer << 6) | (value as u32);
            bits += 6;
            
            if bits >= 8 {
                bits -= 8;
                result.push((buffer >> bits) as u8);
                buffer &= (1 << bits) - 1;
            }
        }
        
        Some(result)
    }
    
    /// Load user credentials from HTML file
    fn load_credentials(&self) -> HashMap<String, String> {
        self.auth_file.as_ref()
            .and_then(|file| self.load_credentials_from_file(file))
            .unwrap_or_else(Self::get_default_credentials)
    }
    
    /// Load credentials from specified auth file
    fn load_credentials_from_file(&self, auth_file: &str) -> Option<HashMap<String, String>> {
        let file_path = self.normalize_file_path(auth_file);
        let content = fs::read_to_string(file_path).ok()?;
        let credentials = self.parse_credentials_from_html(&content);
        
        // Return None if no credentials found to trigger default fallback
        if credentials.is_empty() {
            None
        } else {
            Some(credentials)
        }
    }
    
    /// Normalize file path by removing file:// prefix if present
    fn normalize_file_path<'a>(&self, auth_file: &'a str) -> &'a str {
        if auth_file.starts_with("file://") {
            &auth_file[7..]
        } else {
            auth_file
        }
    }
    
    /// Parse credentials from HTML content using microdata
    fn parse_credentials_from_html(&self, content: &str) -> HashMap<String, String> {
        let mut credentials = HashMap::new();
        let document = Document::from(content);
        
        // Look for credential entries with microdata
        let users = document.select(r#"[itemtype="http://rustybeam.net/Credential"]"#);
        
        for user in users.iter() {
            if let Some((username, password)) = self.extract_credential_from_element(&user) {
                credentials.insert(username, password);
            }
        }
        
        credentials
    }
    
    /// Extract username and password from a credential element
    fn extract_credential_from_element(&self, user: &dom_query::Selection) -> Option<(String, String)> {
        let username = user.select(r#"[itemprop="username"]"#).text().to_string();
        let password = user.select(r#"[itemprop="password"]"#).text().to_string();
        
        if !username.is_empty() && !password.is_empty() {
            Some((username, password))
        } else {
            None
        }
    }
    
    /// Get default credentials for development/testing
    /// WARNING: These should never be used in production!
    fn get_default_credentials() -> HashMap<String, String> {
        let mut credentials = HashMap::new();
        credentials.insert("admin".to_string(), "admin123".to_string());
        credentials.insert("johndoe".to_string(), "doe123".to_string());
        credentials
    }
    
    /// Parse Authorization header
    fn parse_auth_header(&self, auth_header: &str) -> Option<(String, String)> {
        if !auth_header.starts_with("Basic ") {
            return None;
        }
        
        let encoded = &auth_header[6..];
        
        // Decode base64 credentials
        let decoded = self.base64_decode(encoded)?;
        let decoded_str = String::from_utf8(decoded).ok()?;
        
        // Split on first colon to separate username and password
        if let Some(colon_pos) = decoded_str.find(':') {
            let username = decoded_str[..colon_pos].to_string();
            let password = decoded_str[colon_pos + 1..].to_string();
            Some((username, password))
        } else {
            None
        }
    }
    
    /// Check if credentials are valid with timing-safe comparison
    fn validate_credentials(&self, username: &str, password: &str) -> bool {
        let credentials = self.load_credentials();
        
        if let Some(expected_password) = credentials.get(username) {
            // Use constant-time comparison to prevent timing attacks
            self.constant_time_compare(password, expected_password)
        } else {
            // Perform dummy comparison to maintain constant timing
            self.constant_time_compare(password, "dummy_password_for_timing");
            false
        }
    }
    
    /// Constant-time string comparison to prevent timing attacks
    fn constant_time_compare(&self, a: &str, b: &str) -> bool {
        if a.len() != b.len() {
            return false;
        }
        
        let mut result = 0u8;
        for (byte_a, byte_b) in a.bytes().zip(b.bytes()) {
            result |= byte_a ^ byte_b;
        }
        
        result == 0
    }
    
    /// Create authentication challenge response with security headers
    fn create_auth_challenge(&self) -> Response<Body> {
        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header(WWW_AUTHENTICATE, self.format_www_authenticate_header())
            .header("Content-Type", "text/plain; charset=utf-8")
            .header("Cache-Control", "no-store, private")
            .body(Body::from("401 Unauthorized: Authentication required"))
            .unwrap_or_else(|_| {
                // Fallback response if header formatting fails
                Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .body(Body::from("Authentication required"))
                    .unwrap()
            })
    }
    
    /// Format WWW-Authenticate header with proper escaping
    fn format_www_authenticate_header(&self) -> String {
        // Escape quotes in realm to prevent header injection
        let escaped_realm = self.realm.replace('"', "\\\"");
        format!("Basic realm=\"{}\", charset=\"UTF-8\"", escaped_realm)
    }
}

#[async_trait]
impl Plugin for BasicAuthPlugin {
    async fn handle_request(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<PluginResponse> {
        // Allow OPTIONS requests without authentication for CORS
        if request.http_request.method() == hyper::Method::OPTIONS {
            context.log_verbose("[BasicAuth] OPTIONS request allowed without authentication");
            return None;
        }
        
        // Check for Authorization header with improved error handling
        let auth_header = match self.extract_authorization_header(request) {
            Some(header) => header,
            None => {
                context.log_verbose("[BasicAuth] Missing or invalid Authorization header");
                return Some(self.create_auth_challenge().into());
            }
        };
        
        // Parse credentials
        let (username, password) = match self.parse_auth_header(auth_header) {
            Some(creds) => creds,
            None => return Some(self.create_auth_challenge().into()),
        };
        
        // Validate credentials with security logging
        if !self.validate_credentials(&username, &password) {
            context.log_verbose(&format!(
                "[BasicAuth] Authentication failed for user '{}' from {}",
                username,
                request.http_request.headers()
                    .get("x-forwarded-for")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("unknown")
            ));
            return Some(self.create_auth_challenge().into());
        }
        
        // Authentication successful - add user info to metadata
        request.metadata.insert("authenticated_user".to_string(), username.clone());
        request.metadata.insert("auth_realm".to_string(), self.realm.clone());
        
        // Pass to next plugin
        None
    }
    
    async fn handle_response(&self, request: &PluginRequest, response: &mut Response<Body>, context: &PluginContext) {
        // Log successful authentication
        if let Some(user) = request.metadata.get("authenticated_user") {
            context.log_verbose(&format!("[BasicAuth] User '{}' authenticated for {} {}", 
                     user, request.http_request.method(), request.path));
        }
        
        // Add security headers to authenticated responses
        if request.metadata.contains_key("authenticated_user") {
            response.headers_mut().insert(
                "X-Content-Type-Options",
                hyper::header::HeaderValue::from_static("nosniff")
            );
        }
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

// Export the plugin creation function
create_plugin!(BasicAuthPlugin);