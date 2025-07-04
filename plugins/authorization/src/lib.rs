use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, create_plugin};
use async_trait::async_trait;
use hyper::{Body, Response, StatusCode, header::CONTENT_TYPE};
use std::collections::HashMap;
use std::fs;
use dom_query::Document;

/// Plugin for resource authorization
#[derive(Debug)]
pub struct AuthorizationPlugin {
    name: String,
    auth_file: Option<String>,
}

impl AuthorizationPlugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        let name = config.get("name").cloned().unwrap_or_else(|| "authorization".to_string());
        let auth_file = config.get("authfile").cloned();
        
        Self { name, auth_file }
    }
    
    /// Load authorization rules from HTML file
    fn load_auth_rules(&self) -> HashMap<String, Vec<String>> {
        let mut rules = HashMap::new();
        
        if let Some(auth_file) = &self.auth_file {
            // Handle file:// URLs
            let file_path = if auth_file.starts_with("file://") {
                &auth_file[7..]
            } else {
                auth_file
            };
            
            if let Ok(content) = fs::read_to_string(file_path) {
                let document = Document::from(content.as_str());
                
                // Look for authorization rules with microdata
                // Expected format: <div itemscope itemtype="http://rustybeam.net/AuthRule">
                //                    <span itemprop="path">/admin/*</span>
                //                    <span itemprop="users">admin,moderator</span>
                //                  </div>
                let auth_rules = document.select(r#"[itemtype="http://rustybeam.net/AuthRule"]"#);
                
                for rule in auth_rules.iter() {
                    let path_pattern = rule.select(r#"[itemprop="path"]"#).text().to_string();
                    let users_str = rule.select(r#"[itemprop="users"]"#).text().to_string();
                    
                    if !path_pattern.is_empty() && !users_str.is_empty() {
                        let users: Vec<String> = users_str
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .collect();
                        
                        rules.insert(path_pattern, users);
                    }
                }
            }
        }
        
        rules
    }
    
    /// Check if a path matches a pattern (simple glob matching)
    fn path_matches_pattern(&self, path: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        
        if pattern.ends_with("/*") {
            let prefix = &pattern[..pattern.len() - 2];
            return path.starts_with(prefix);
        }
        
        if pattern.contains('*') {
            // Simple wildcard matching
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                let (prefix, suffix) = (parts[0], parts[1]);
                return path.starts_with(prefix) && path.ends_with(suffix);
            }
        }
        
        path == pattern
    }
    
    /// Check if user is authorized to access path
    fn is_authorized(&self, user: &str, path: &str) -> bool {
        let rules = self.load_auth_rules();
        
        // Check each rule to see if it applies to this path
        for (pattern, allowed_users) in rules {
            if self.path_matches_pattern(path, &pattern) {
                return allowed_users.contains(&user.to_string());
            }
        }
        
        // Default policy: allow if no rules match
        true
    }
    
    /// Create access denied response
    fn create_access_denied(&self, user: &str, path: &str) -> Response<Body> {
        Response::builder()
            .status(StatusCode::FORBIDDEN)
            .header(CONTENT_TYPE, "text/html")
            .body(Body::from(format!(
                r#"<!DOCTYPE html>
<html>
<head><title>403 Forbidden</title></head>
<body>
<h1>403 Forbidden</h1>
<p>User '{}' does not have permission to access '{}'.</p>
<p>Contact your administrator if you believe this is an error.</p>
</body>
</html>"#, user, path
            )))
            .unwrap()
    }
}

#[async_trait]
impl Plugin for AuthorizationPlugin {
    async fn handle_request(&self, request: &mut PluginRequest, _context: &PluginContext) -> Option<Response<Body>> {
        // Check if user is authenticated (should be set by BasicAuth plugin)
        let user = match request.metadata.get("authenticated_user") {
            Some(user) => user.clone(),
            None => {
                // No authenticated user - let it pass through
                // (BasicAuth plugin should handle this)
                return None;
            }
        };
        
        // Check authorization
        if !self.is_authorized(&user, &request.path) {
            return Some(self.create_access_denied(&user, &request.path));
        }
        
        // Authorization successful - add authorization info to metadata
        request.metadata.insert("authorized".to_string(), "true".to_string());
        request.metadata.insert("authorized_user".to_string(), user);
        
        // Pass to next plugin
        None
    }
    
    async fn handle_response(&self, request: &PluginRequest, _response: &mut Response<Body>, _context: &PluginContext) {
        // Log authorization decisions
        if let Some(user) = request.metadata.get("authorized_user") {
            println!("[Authorization] User '{}' authorized for {} {}", 
                     user, request.http_request.method(), request.path);
        }
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

// Export the plugin creation function
create_plugin!(AuthorizationPlugin);