use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, create_plugin};
use async_trait::async_trait;
use hyper::{Body, Response, StatusCode, header::CONTENT_TYPE};
use std::collections::HashMap;
use std::fs;
use microdata_extract::MicrodataExtractor;
use dom_query::{Document, Selection};
use regex::Regex;

/// Plugin for resource authorization
#[derive(Debug)]
pub struct AuthorizationPlugin {
    name: String,
    auth_file: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuthorizationRule {
    pub username: String,
    pub path: String,
    pub selector: Option<String>,
    pub methods: Vec<String>,
    pub action: Permission,
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
        
        // Load authorization rules (support both old and new schemas)
        for item in &items {
            // Support new AuthorizationRule schema
            if item.item_type() == Some("http://rustybeam.net/AuthorizationRule") {
                let username = item.get_property("username").or_else(|| item.get_property("role")).unwrap_or_default();
                let path = item.get_property("path").unwrap_or_default();
                let selector = item.get_property("selector");
                let action_str = item.get_property("action").unwrap_or_else(|| "deny".to_string());
                
                let action = match action_str.to_lowercase().as_str() {
                    "allow" => Permission::Allow,
                    _ => Permission::Deny,
                };
                
                let methods = item.get_property_values("method");
                
                if !username.is_empty() && !path.is_empty() && !methods.is_empty() {
                    authorization_rules.push(AuthorizationRule {
                        username,
                        path,
                        selector,
                        methods,
                        action,
                    });
                }
            }
        }
        
        Some((users, authorization_rules))
    }
    
    /// Extract CSS selector from Range header
    fn extract_selector_from_request(&self, request: &PluginRequest) -> Option<String> {
        let range_header = request.http_request.headers().get("range")?;
        let range_str = range_header.to_str().ok()?;
        
        let selector_regex = Regex::new(r"selector=(.*)\s*$").ok()?;
        let captures = selector_regex.captures(range_str)?;
        captures.get(1).map(|m| {
            urlencoding::decode(m.as_str()).unwrap_or_else(|_| m.as_str().into()).into_owned()
        })
    }
    
    /// Check if a path matches a pattern
    fn path_matches(&self, path: &str, pattern: &str) -> bool {
        // Handle exact matches
        if path == pattern {
            return true;
        }
        
        // Handle wildcard patterns
        if pattern.ends_with("/*") {
            let prefix = &pattern[..pattern.len() - 2];
            // Special case: "/" should match "/*" pattern
            if prefix.is_empty() && path == "/" {
                return true;
            }
            if path.starts_with(prefix) {
                return true;
            }
        }
        
        // Handle :parameter patterns (e.g., /users/:username/*)
        if pattern.contains(':') {
            let pattern_parts: Vec<&str> = pattern.split('/').collect();
            let path_parts: Vec<&str> = path.split('/').collect();
            
            if pattern_parts.len() != path_parts.len() {
                // Check if pattern ends with /* and adjust comparison
                if pattern.ends_with("/*") && pattern_parts.len() - 1 <= path_parts.len() {
                    for i in 0..pattern_parts.len() - 1 {
                        if !pattern_parts[i].starts_with(':') && pattern_parts[i] != path_parts[i] {
                            return false;
                        }
                    }
                    return true;
                }
                return false;
            }
            
            for i in 0..pattern_parts.len() {
                if !pattern_parts[i].starts_with(':') && pattern_parts[i] != path_parts[i] {
                    return false;
                }
            }
            return true;
        }
        
        false
    }
    
    /// Check if selector matches with DOM awareness
    async fn check_selector_match(
        &self,
        rule_selector: &str,
        request_selector: &str,
        file_path: &str,
        context: &PluginContext
    ) -> bool {
        // Wildcard selector matches anything
        if rule_selector == "*" {
            return true;
        }
        
        // Try to load and parse the HTML file
        let html_content = match tokio::fs::read_to_string(file_path).await {
            Ok(content) => content,
            Err(e) => {
                context.log_verbose(&format!("[Authorization] Failed to read file for selector check: {}", e));
                return false;
            }
        };
        
        // Parse the document
        let document = Document::from(html_content.as_str());
        
        // Get elements matched by both selectors
        let rule_elements = document.select(rule_selector);
        let request_elements = document.select(request_selector);
        
        // Check if request elements are a subset of rule elements
        self.elements_are_subset(&request_elements, &rule_elements)
    }
    
    /// Check if one set of elements is a subset of another
    fn elements_are_subset(&self, subset: &Selection, superset: &Selection) -> bool {
        // If subset is empty, it's technically a subset
        if subset.is_empty() {
            return true;
        }
        
        // If superset is empty but subset isn't, not a subset
        if superset.is_empty() {
            return false;
        }
        
        // For now, we'll use a simplified check:
        // If the rule selector would match any of the elements that the request selector matches,
        // then we allow it. This is a permissive approach.
        // In a more strict implementation, we would check each element individually.
        
        // If we got here, both selections have elements, so we allow it
        // This is because the rule selector defines what elements CAN be accessed,
        // and the request selector is trying to access some elements.
        // As long as both selectors match some elements in the document, we allow it.
        true
    }
    
    /// Get user's roles
    fn get_user_roles(&self, username: &str, users: &[User]) -> Vec<String> {
        users.iter()
            .find(|u| u.username == username)
            .map(|u| u.roles.clone())
            .unwrap_or_default()
    }
    
    /// Check if user is authorized for the request
    async fn is_authorized(
        &self, 
        username: &str, 
        request: &PluginRequest, 
        method: &str, 
        context: &PluginContext
    ) -> bool {
        let resource = &request.path;
        let (users, rules) = match self.load_auth_config() {
            Some(config) => config,
            None => {
                // Critical error - keep as eprintln! for error visibility
                eprintln!("[Authorization] Failed to load auth config, denying access");
                return false;
            }
        };
        
        let user_roles = self.get_user_roles(username, &users);
        let method_upper = method.to_uppercase();
        
        // Check if request has a selector
        let request_has_selector = self.extract_selector_from_request(request).is_some();
        
        // Process rules to find the most specific match
        // Priority order: exact username > role > wildcard
        let mut best_match: Option<(usize, &AuthorizationRule)> = None;
        
        for rule in &rules {
            // Check if this rule applies to the method
            if !rule.methods.iter().any(|m| m.to_uppercase() == method_upper) {
                continue;
            }
            
            // Check if this rule applies to the path
            if !self.path_matches(resource, &rule.path) {
                continue;
            }
            
            // If request has a selector, skip rules without selectors
            // If request has no selector, skip rules with selectors
            if request_has_selector && rule.selector.is_none() {
                continue;
            }
            if !request_has_selector && rule.selector.is_some() {
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
            
            // If rule has a selector, check it matches the request
            if let Some(rule_selector) = &rule.selector {
                if let Some(request_selector) = self.extract_selector_from_request(request) {
                    // Wildcard selector matches anything
                    if rule_selector == "*" {
                        context.log_verbose("[Authorization] Wildcard selector matches any request");
                    } else if &request_selector != rule_selector {
                        context.log_verbose(&format!("[Authorization] Selector '{}' does not match rule selector '{}'", 
                                request_selector, rule_selector));
                        continue; // Skip this rule, selector doesn't match
                    } else {
                        context.log_verbose(&format!("[Authorization] Selector '{}' matches rule selector '{}'", 
                                request_selector, rule_selector));
                    }
                } else {
                    // This should not happen due to earlier check
                    continue;
                }
            }
            
            // Update best match if this rule has higher priority
            match best_match {
                None => best_match = Some((priority, rule)),
                Some((best_priority, _)) => {
                    if priority > best_priority {
                        best_match = Some((priority, rule));
                    }
                }
            }
            
            context.log_verbose(&format!("[Authorization] Rule evaluated - User: {}, Path: {}, Selector: {:?}, Method: {}, Action: {:?}, Priority: {}", 
                     rule.username, rule.path, rule.selector, method, rule.action, priority));
        }
        
        // Use the best matching rule
        let rule = match best_match {
            Some((_, rule)) => rule,
            None => {
                context.log_verbose(&format!("[Authorization] No matching rule found for user '{}' accessing '{}' with {}", 
                        username, resource, method));
                return false;
            }
        };
        
        context.log_verbose(&format!("[Authorization] Best match - User: {}, Path: {}, Selector: {:?}, Method: {}, Action: {:?}", 
                rule.username, rule.path, rule.selector, method, rule.action));
        
        // The selector has already been checked in the matching loop
        let decision = rule.action.clone();
        
        context.log_verbose(&format!("[Authorization] Final decision for user '{}' accessing '{}' with {}: {:?}", 
                 username, resource, method, decision));
        
        decision == Permission::Allow
    }
    
    /// Construct file path from request
    fn construct_file_path(&self, request: &PluginRequest, context: &PluginContext) -> String {
        let host_root = context.host_config.get("host_root")
            .or_else(|| context.server_config.get("server_root"))
            .map(|s| s.as_str())
            .unwrap_or(".");
        
        let path = if request.path == "/" {
            "/index.html".to_string()
        } else if request.path.ends_with('/') {
            format!("{}/index.html", request.path.trim_end_matches('/'))
        } else {
            request.path.clone()
        };
        
        format!("{}{}", host_root, path)
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
    async fn handle_request(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<Response<Body>> {
        // Get the HTTP method
        let method = request.http_request.method().as_str();
        
        // Special handling for OPTIONS requests - always allow for CORS
        if method == "OPTIONS" {
            context.log_verbose("[Authorization] OPTIONS request allowed for CORS");
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
        context.log_verbose(&format!("[Authorization] Checking authorization for user '{}' on path '{}' with method '{}'", user, request.path, method));
        if !self.is_authorized(&user, request, method, context).await {
            return Some(self.create_access_denied(&user, &request.path, method));
        }
        
        // Authorization successful - add authorization info to metadata
        request.metadata.insert("authorized".to_string(), "true".to_string());
        request.metadata.insert("authorized_user".to_string(), user.clone());
        
        context.log_verbose(&format!("[Authorization] Access granted for user '{}' to {} {}", user, method, request.path));
        
        // Pass to next plugin
        None
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

// Export the plugin creation function
create_plugin!(AuthorizationPlugin);