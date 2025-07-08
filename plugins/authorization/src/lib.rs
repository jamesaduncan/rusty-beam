use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, create_plugin};
use async_trait::async_trait;
use hyper::{Body, Response, StatusCode, header::CONTENT_TYPE};
use std::collections::HashMap;
use std::fs;
use microdata_extract::MicrodataExtractor;

/// Plugin for resource authorization
#[derive(Debug)]
pub struct AuthorizationPlugin {
    name: String,
    auth_file: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuthorizationRule {
    pub username: String,
    pub resource: String,
    pub methods: Vec<String>,
    pub permission: Permission,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Permission {
    Allow,
    Deny,
}

#[derive(Debug, Clone)]
pub struct User {
    pub username: String,
    pub roles: Vec<String>,
}

impl AuthorizationPlugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        let name = config.get("name").cloned().unwrap_or_else(|| "authorization".to_string());
        let auth_file = config.get("authfile").cloned();
        
        Self { name, auth_file }
    }
    
    /// Load authorization configuration from HTML file
    fn load_auth_config(&self) -> Option<(Vec<User>, Vec<AuthorizationRule>)> {
        let auth_file = self.auth_file.as_ref()?;
        
        // Handle file:// URLs
        let file_path = if auth_file.starts_with("file://") {
            &auth_file[7..]
        } else {
            auth_file
        };
        
        let content = fs::read_to_string(file_path).ok()?;
        let extractor = MicrodataExtractor::new();
        let items = extractor.extract(&content).ok()?;
        
        let mut users = Vec::new();
        let mut authorization_rules = Vec::new();
        
        // Load users
        for item in &items {
            if item.item_type() == Some("http://rustybeam.net/User") {
                let username = item.get_property("username").unwrap_or_default();
                let roles = item.get_property_values("role");
                
                if !username.is_empty() {
                    users.push(User { username, roles });
                }
            }
        }
        
        // Load authorization rules
        for item in &items {
            if item.item_type() == Some("http://rustybeam.net/Authorization") {
                let username = item.get_property("username").unwrap_or_default();
                let resource = item.get_property("resource").unwrap_or_default();
                let permission_str = item.get_property("permission").unwrap_or_else(|| "deny".to_string());
                
                let permission = match permission_str.to_lowercase().as_str() {
                    "allow" => Permission::Allow,
                    _ => Permission::Deny,
                };
                
                let methods = item.get_property_values("method");
                
                if !username.is_empty() && !resource.is_empty() && !methods.is_empty() {
                    authorization_rules.push(AuthorizationRule {
                        username,
                        resource,
                        methods,
                        permission,
                    });
                }
            }
        }
        
        Some((users, authorization_rules))
    }
    
    /// Check if a resource matches a pattern
    fn resource_matches(&self, resource: &str, pattern: &str) -> bool {
        // Handle exact matches
        if resource == pattern {
            return true;
        }
        
        // Handle wildcard patterns
        if pattern.ends_with("/*") {
            let prefix = &pattern[..pattern.len() - 2];
            // Special case: "/" should match "/*" pattern
            if prefix.is_empty() && resource == "/" {
                return true;
            }
            if resource.starts_with(prefix) {
                return true;
            }
        }
        
        // Handle :parameter patterns (e.g., /users/:username/*)
        if pattern.contains(':') {
            let pattern_parts: Vec<&str> = pattern.split('/').collect();
            let resource_parts: Vec<&str> = resource.split('/').collect();
            
            if pattern_parts.len() != resource_parts.len() {
                // Check if pattern ends with /* and adjust comparison
                if pattern.ends_with("/*") && pattern_parts.len() - 1 <= resource_parts.len() {
                    for i in 0..pattern_parts.len() - 1 {
                        if !pattern_parts[i].starts_with(':') && pattern_parts[i] != resource_parts[i] {
                            return false;
                        }
                    }
                    return true;
                }
                return false;
            }
            
            for i in 0..pattern_parts.len() {
                if !pattern_parts[i].starts_with(':') && pattern_parts[i] != resource_parts[i] {
                    return false;
                }
            }
            return true;
        }
        
        false
    }
    
    /// Get user's roles
    fn get_user_roles(&self, username: &str, users: &[User]) -> Vec<String> {
        users.iter()
            .find(|u| u.username == username)
            .map(|u| u.roles.clone())
            .unwrap_or_default()
    }
    
    /// Check if user/role matches the rule
    fn username_matches(&self, rule_username: &str, actual_username: &str, user_roles: &[String]) -> bool {
        // Check wildcard
        if rule_username == "*" {
            return true;
        }
        
        // Check exact username match
        if rule_username == actual_username {
            return true;
        }
        
        // Check :username parameter (matches current user)
        if rule_username == ":username" {
            return true;
        }
        
        // Check role match (rule username might be a role name)
        if user_roles.contains(&rule_username.to_string()) {
            return true;
        }
        
        false
    }
    
    /// Check if user is authorized for the request
    fn is_authorized(&self, username: &str, resource: &str, method: &str) -> bool {
        let (users, rules) = match self.load_auth_config() {
            Some(config) => config,
            None => {
                eprintln!("[Authorization] Failed to load auth config, denying access");
                return false;
            }
        };
        
        let user_roles = self.get_user_roles(username, &users);
        let method_upper = method.to_uppercase();
        
        // Process rules to find the most specific match
        // Priority order: exact username > role > wildcard
        let mut best_match: Option<(usize, &AuthorizationRule)> = None;
        
        for rule in &rules {
            // Check if this rule applies to the method
            if !rule.methods.iter().any(|m| m.to_uppercase() == method_upper) {
                continue;
            }
            
            // Check if this rule applies to the resource
            if !self.resource_matches(resource, &rule.resource) {
                continue;
            }
            
            // Check if this rule applies to the user/role and determine priority
            let priority = if rule.username == username {
                3 // Exact username match - highest priority
            } else if rule.username == ":username" {
                2 // Current user parameter - high priority
            } else if user_roles.contains(&rule.username.to_string()) {
                1 // Role match - medium priority
            } else if rule.username == "*" {
                0 // Wildcard - lowest priority
            } else {
                continue; // Rule doesn't apply
            };
            
            // Update best match if this rule has higher priority
            match best_match {
                None => best_match = Some((priority, rule)),
                Some((best_priority, _)) => {
                    if priority > best_priority {
                        best_match = Some((priority, rule));
                    }
                }
            }
            
            eprintln!("[Authorization] Rule evaluated - User: {}, Resource: {}, Method: {}, Permission: {:?}, Priority: {}", 
                     rule.username, rule.resource, method, rule.permission, priority);
        }
        
        // Use the best matching rule or default deny
        let decision = match best_match {
            Some((_, rule)) => {
                eprintln!("[Authorization] Best match - User: {}, Resource: {}, Method: {}, Permission: {:?}", 
                         rule.username, rule.resource, method, rule.permission);
                rule.permission.clone()
            }
            None => Permission::Deny
        };
        
        eprintln!("[Authorization] Final decision for user '{}' accessing '{}' with {}: {:?}", 
                 username, resource, method, decision);
        
        decision == Permission::Allow
    }
    
    /// Create access denied response
    fn create_access_denied(&self, user: &str, resource: &str, method: &str) -> Response<Body> {
        Response::builder()
            .status(StatusCode::FORBIDDEN)
            .header(CONTENT_TYPE, "text/html")
            .body(Body::from(format!(
                r#"<!DOCTYPE html>
<html>
<head><title>403 Forbidden</title></head>
<body>
<h1>403 Forbidden</h1>
<p>User '{}' does not have permission to {} '{}'.</p>
<p>Contact your administrator if you believe this is an error.</p>
</body>
</html>"#, user, method, resource
            )))
            .unwrap()
    }
}

#[async_trait]
impl Plugin for AuthorizationPlugin {
    async fn handle_request(&self, request: &mut PluginRequest, _context: &PluginContext) -> Option<Response<Body>> {
        // Get the HTTP method
        let method = request.http_request.method().as_str();
        
        // Special handling for OPTIONS requests - always allow for CORS
        if method == "OPTIONS" {
            eprintln!("[Authorization] OPTIONS request allowed for CORS");
            request.metadata.insert("authorized".to_string(), "true".to_string());
            // OPTIONS may not have authenticated user, which is fine
            if let Some(user) = request.metadata.get("authenticated_user") {
                request.metadata.insert("authorized_user".to_string(), user.clone());
            }
            return None;
        }
        
        // Check if user is authenticated (should be set by BasicAuth plugin)
        let user = match request.metadata.get("authenticated_user") {
            Some(user) => user.clone(),
            None => {
                // No authenticated user - let it pass through
                // (BasicAuth plugin should handle authentication)
                return None;
            }
        };
        
        // Check authorization for other methods
        eprintln!("[Authorization] Checking authorization for user '{}' on path '{}' with method '{}'", user, request.path, method);
        if !self.is_authorized(&user, &request.path, method) {
            return Some(self.create_access_denied(&user, &request.path, method));
        }
        
        // Authorization successful - add authorization info to metadata
        request.metadata.insert("authorized".to_string(), "true".to_string());
        request.metadata.insert("authorized_user".to_string(), user.clone());
        
        eprintln!("[Authorization] Access granted for user '{}' to {} {}", user, method, request.path);
        
        // Pass to next plugin
        None
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

// Export the plugin creation function
create_plugin!(AuthorizationPlugin);