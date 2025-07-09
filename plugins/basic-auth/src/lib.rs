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
    
    /// Load user credentials from HTML file
    fn load_credentials(&self) -> HashMap<String, String> {
        let mut credentials = HashMap::new();
        
        if let Some(auth_file) = &self.auth_file {
            // Handle file:// URLs
            let file_path = if auth_file.starts_with("file://") {
                &auth_file[7..]
            } else {
                auth_file
            };
            
            if let Ok(content) = fs::read_to_string(file_path) {
                let document = Document::from(content.as_str());
                
                // Look for user entries with microdata
                let users = document.select(r#"[itemtype="http://rustybeam.net/User"]"#);
                
                for user in users.iter() {
                    let username = user.select(r#"[itemprop="username"]"#).text().to_string();
                    let password = user.select(r#"[itemprop="password"]"#).text().to_string();
                    
                    if !username.is_empty() && !password.is_empty() {
                        credentials.insert(username, password);
                    }
                }
            }
        }
        
        // Default credentials if no file or file is empty
        if credentials.is_empty() {
            credentials.insert("admin".to_string(), "admin123".to_string());
            credentials.insert("johndoe".to_string(), "doe123".to_string());
        }
        
        credentials
    }
    
    /// Parse Authorization header
    fn parse_auth_header(&self, auth_header: &str) -> Option<(String, String)> {
        if !auth_header.starts_with("Basic ") {
            return None;
        }
        
        let encoded = &auth_header[6..];
        let decoded = base64_decode(encoded)?;
        let decoded_str = String::from_utf8(decoded).ok()?;
        
        if let Some(colon_pos) = decoded_str.find(':') {
            let username = decoded_str[..colon_pos].to_string();
            let password = decoded_str[colon_pos + 1..].to_string();
            Some((username, password))
        } else {
            None
        }
    }
    
    /// Check if credentials are valid
    fn validate_credentials(&self, username: &str, password: &str) -> bool {
        let credentials = self.load_credentials();
        
        if let Some(expected_password) = credentials.get(username) {
            expected_password == password
        } else {
            false
        }
    }
    
    /// Create authentication challenge response
    fn create_auth_challenge(&self) -> Response<Body> {
        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header(WWW_AUTHENTICATE, format!("Basic realm=\"{}\"", self.realm))
            .header("Content-Type", "text/plain")
            .body(Body::from("Authentication required"))
            .unwrap()
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
        
        // Check for Authorization header
        let auth_header = match request.http_request.headers().get(AUTHORIZATION) {
            Some(header) => match header.to_str() {
                Ok(header_str) => header_str,
                Err(_) => return Some(self.create_auth_challenge().into()),
            },
            None => return Some(self.create_auth_challenge().into()),
        };
        
        // Parse credentials
        let (username, password) = match self.parse_auth_header(auth_header) {
            Some(creds) => creds,
            None => return Some(self.create_auth_challenge().into()),
        };
        
        // Validate credentials
        if !self.validate_credentials(&username, &password) {
            return Some(self.create_auth_challenge().into());
        }
        
        // Authentication successful - add user info to metadata
        request.metadata.insert("authenticated_user".to_string(), username.clone());
        request.metadata.insert("auth_realm".to_string(), self.realm.clone());
        
        // Pass to next plugin
        None
    }
    
    async fn handle_response(&self, request: &PluginRequest, _response: &mut Response<Body>, context: &PluginContext) {
        // Log authentication attempts
        if let Some(user) = request.metadata.get("authenticated_user") {
            context.log_verbose(&format!("[BasicAuth] User '{}' authenticated for {} {}", 
                     user, request.http_request.method(), request.path));
        }
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

// Simple base64 decoder
fn base64_decode(input: &str) -> Option<Vec<u8>> {
    use std::collections::HashMap;
    
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    
    let mut chars_map = HashMap::new();
    for (i, &c) in CHARS.iter().enumerate() {
        chars_map.insert(c, i);
    }
    
    let input = input.trim_end_matches('=');
    let mut result = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0;
    
    for byte in input.bytes() {
        if let Some(&value) = chars_map.get(&byte) {
            buffer = (buffer << 6) | (value as u32);
            bits += 6;
            
            if bits >= 8 {
                bits -= 8;
                result.push((buffer >> bits) as u8);
                buffer &= (1 << bits) - 1;
            }
        } else {
            return None;
        }
    }
    
    Some(result)
}

// Export the plugin creation function
create_plugin!(BasicAuthPlugin);