use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, PluginResponse, create_plugin};
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
    fn check_selector_match(
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
        
        // Check if the file exists and is likely HTML
        let _metadata = match std::fs::metadata(file_path) {
            Ok(meta) => meta,
            Err(e) => {
                context.log_verbose(&format!("[Authorization] File not found for selector check: {} - {}", file_path, e));
                // If file doesn't exist, use string comparison as fallback
                return rule_selector == request_selector;
            }
        };
        
        // Skip DOM parsing for non-HTML files (check by extension)
        if !file_path.ends_with(".html") && !file_path.ends_with(".htm") {
            context.log_verbose(&format!("[Authorization] Non-HTML file, using string comparison for selectors"));
            return rule_selector == request_selector;
        }
        
        // Try to load and parse the HTML file
        let html_content = match std::fs::read_to_string(file_path) {
            Ok(content) => content,
            Err(e) => {
                context.log_verbose(&format!("[Authorization] Failed to read file for selector check: {}", e));
                // Fallback to string comparison
                return rule_selector == request_selector;
            }
        };
        
        // Skip empty files
        if html_content.trim().is_empty() {
            context.log_verbose("[Authorization] Empty HTML file, skipping DOM parsing");
            return rule_selector == request_selector;
        }
        
        // Parse the document
        let document = Document::from(html_content.as_str());
        
        // Get elements matched by both selectors
        let rule_elements = document.select(rule_selector);
        let request_elements = document.select(request_selector);
        
        context.log_verbose(&format!("[Authorization] Rule selector '{}' matches {} elements", 
            rule_selector, rule_elements.length()));
        context.log_verbose(&format!("[Authorization] Request selector '{}' matches {} elements", 
            request_selector, request_elements.length()));
        
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
        
        // For DOM-aware matching, we need to check if the elements selected by the request
        // are actually a subset of what's allowed by the rule.
        // If the rule allows "p", and the request is for ".private p", we need to check
        // if those specific p elements inside .private are in the set of all p elements.
        
        // Get unique identifiers for superset elements
        let mut superset_elements = std::collections::HashSet::new();
        for elem in superset.iter() {
            // Use the full HTML content as identifier
            let html = elem.html().to_string();
            superset_elements.insert(html);
        }
        
        // Check if all subset elements are in the superset
        for elem in subset.iter() {
            let html = elem.html().to_string();
            if !superset_elements.contains(&html) {
                return false;
            }
        }
        
        true
    }
    
    /// Get user's roles
    fn get_user_roles(&self, username: &str, users: &[User]) -> Vec<String> {
        users.iter()
            .find(|u| u.username == username)
            .map(|u| u.roles.clone())
            .unwrap_or_default()
    }
    
    /// Get all allowed methods for a user/path/selector combination
    fn get_allowed_methods(&self, username: &str, request: &PluginRequest, context: &PluginContext) -> Vec<String> {
        let resource = &request.path;
        let (users, rules) = match self.load_auth_config() {
            Some(config) => config,
            None => {
                context.log_verbose("[Authorization] Failed to load auth config for OPTIONS");
                return vec![];
            }
        };
        
        let user_roles = self.get_user_roles(username, &users);
        let request_has_selector = self.extract_selector_from_request(request).is_some();
        
        // Collect all methods that are explicitly allowed and denied
        let mut allowed_methods = std::collections::HashSet::new();
        let mut denied_methods = std::collections::HashSet::new();
        
        // Process rules in priority order
        let mut applicable_rules: Vec<(usize, &AuthorizationRule)> = vec![];
        
        for rule in &rules {
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
                    let file_path = self.construct_file_path(request, context);
                    let matches = self.check_selector_match(
                        rule_selector,
                        &request_selector,
                        &file_path,
                        context
                    );
                    
                    if !matches {
                        continue; // Skip this rule, selector doesn't match
                    }
                }
            }
            
            applicable_rules.push((priority, rule));
        }
        
        // Sort rules by priority (highest first)
        applicable_rules.sort_by(|a, b| b.0.cmp(&a.0));
        
        // Process rules to determine allowed methods
        // Higher priority rules override lower priority ones
        let mut methods_processed = std::collections::HashSet::new();
        
        for (priority, rule) in applicable_rules {
            context.log_verbose(&format!("[Authorization] OPTIONS checking rule - User: {}, Path: {}, Methods: {:?}, Action: {:?}, Priority: {}", 
                rule.username, rule.path, rule.methods, rule.action, priority));
            
            for method in &rule.methods {
                let method_upper = method.to_uppercase();
                
                // Skip if we've already processed this method at a higher priority
                if methods_processed.contains(&method_upper) {
                    continue;
                }
                
                methods_processed.insert(method_upper.clone());
                
                match rule.action {
                    Permission::Allow => {
                        allowed_methods.insert(method_upper.clone());
                        denied_methods.remove(&method_upper);
                    }
                    Permission::Deny => {
                        denied_methods.insert(method_upper.clone());
                        allowed_methods.remove(&method_upper);
                    }
                }
            }
        }
        
        // Always include OPTIONS itself
        allowed_methods.insert("OPTIONS".to_string());
        
        let mut result: Vec<String> = allowed_methods.into_iter().collect();
        result.sort();
        
        context.log_verbose(&format!("[Authorization] Allowed methods for user '{}' on '{}': {:?}", 
            username, resource, result));
        
        result
    }
    
    /// Check if user is authorized for the request
    fn is_authorized(
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
                    // Construct the file path for DOM parsing
                    let file_path = self.construct_file_path(request, context);
                    
                    // Use DOM-aware selector matching
                    let matches = self.check_selector_match(
                        rule_selector,
                        &request_selector,
                        &file_path,
                        context
                    );
                    
                    if !matches {
                        context.log_verbose(&format!("[Authorization] Selector '{}' does not match rule selector '{}' (DOM-aware check)", 
                                request_selector, rule_selector));
                        continue; // Skip this rule, selector doesn't match
                    } else {
                        context.log_verbose(&format!("[Authorization] Selector '{}' matches rule selector '{}' (DOM-aware check)", 
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
        // Try to get host_root from various sources
        let host_root = context.host_config.get("host_root")
            .or_else(|| context.host_config.get("hostRoot"))
            .or_else(|| request.metadata.get("host_root"))
            .or_else(|| context.server_config.get("server_root"))
            .map(|s| s.as_str())
            .unwrap_or(".");
        
        context.log_verbose(&format!("[Authorization] Available host_config keys: {:?}", 
            context.host_config.keys().collect::<Vec<_>>()));
        context.log_verbose(&format!("[Authorization] Available metadata keys: {:?}", 
            request.metadata.keys().collect::<Vec<_>>()));
        
        let path = if request.path == "/" {
            "/index.html".to_string()
        } else if request.path.ends_with('/') {
            format!("{}/index.html", request.path.trim_end_matches('/'))
        } else {
            request.path.clone()
        };
        
        let file_path = format!("{}{}", host_root, path);
        context.log_verbose(&format!("[Authorization] Constructed file path: {} (host_root: {}, path: {})", 
            file_path, host_root, path));
        file_path
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
    async fn handle_request(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<PluginResponse> {
        // Get the HTTP method
        let method = request.http_request.method().as_str();
        
        // Special handling for OPTIONS requests - discover allowed methods
        if method == "OPTIONS" {
            context.log_verbose("[Authorization] Processing OPTIONS request for method discovery");
            
            // Get authenticated user or treat as anonymous
            let user = request.metadata.get("authenticated_user")
                .cloned()
                .unwrap_or_else(|| "*".to_string());
            
            // Get allowed methods for this user/path/selector combination
            let allowed_methods = self.get_allowed_methods(&user, request, context);
            
            // Create OPTIONS response with Allow header
            let allow_header = if allowed_methods.is_empty() {
                "".to_string()
            } else {
                allowed_methods.join(", ")
            };
            
            context.log_verbose(&format!("[Authorization] OPTIONS response - Allow: {}", allow_header));
            
            let response = Response::builder()
                .status(StatusCode::OK)
                .header("Allow", allow_header)
                .header("Accept-Ranges", "selector")
                .header(CONTENT_TYPE, "text/plain")
                .body(Body::empty())
                .unwrap();
            
            return Some(response.into());
        }
        
        // Check if user is authenticated (should be set by BasicAuth plugin)
        let user = match request.metadata.get("authenticated_user") {
            Some(user) => user.clone(),
            None => {
                // No authenticated user - treat as anonymous ("*")
                "*".to_string()
            }
        };
        
        // Check authorization for other methods
        context.log_verbose(&format!("[Authorization] Checking authorization for user '{}' on path '{}' with method '{}'", user, request.path, method));
        if !self.is_authorized(&user, request, method, context) {
            return Some(self.create_access_denied(&user, &request.path, method).into());
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use hyper::Request;
    use tokio::sync::Mutex;
    
    fn create_test_plugin() -> AuthorizationPlugin {
        let mut config = HashMap::new();
        config.insert("name".to_string(), "test-auth".to_string());
        config.insert("authfile".to_string(), "file://tests/test-auth.html".to_string());
        AuthorizationPlugin::new(config)
    }
    
    fn create_test_context() -> PluginContext {
        PluginContext {
            plugin_config: HashMap::new(),
            server_config: HashMap::new(),
            host_config: HashMap::new(),
            host_name: "test-host".to_string(),
            request_id: "test-request".to_string(),
            runtime_handle: None,
            verbose: false,
        }
    }
    
    fn create_test_request(method: &str, path: &str, selector: Option<&str>) -> PluginRequest {
        let mut builder = Request::builder()
            .method(method)
            .uri(path);
            
        if let Some(sel) = selector {
            builder = builder.header("range", format!("selector={}", sel));
        }
        
        let http_request = builder
            .body(Body::empty())
            .unwrap();
            
        PluginRequest {
            http_request: Box::new(http_request),
            path: path.to_string(),
            canonical_path: None,
            metadata: HashMap::new(),
            body_cache: Arc::new(Mutex::new(None)),
        }
    }
    
    #[test]
    fn test_permission_enum() {
        assert_eq!(Permission::Allow, Permission::Allow);
        assert_eq!(Permission::Deny, Permission::Deny);
        assert_ne!(Permission::Allow, Permission::Deny);
    }
    
    #[test]
    fn test_extract_selector_from_request() {
        let plugin = create_test_plugin();
        
        // Test with selector
        let req = create_test_request("GET", "/", Some("#entries"));
        let selector = plugin.extract_selector_from_request(&req);
        assert_eq!(selector, Some("#entries".to_string()));
        
        // Test without selector
        let req_no_sel = create_test_request("GET", "/", None);
        let selector_none = plugin.extract_selector_from_request(&req_no_sel);
        assert_eq!(selector_none, None);
        
        // Test with complex selector
        let req_complex = create_test_request("GET", "/", Some("#entries .entry:nth-child(1)"));
        let selector_complex = plugin.extract_selector_from_request(&req_complex);
        assert_eq!(selector_complex, Some("#entries .entry:nth-child(1)".to_string()));
    }
    
    #[test]
    fn test_path_matches() {
        let plugin = create_test_plugin();
        
        // Exact match
        assert!(plugin.path_matches("/index.html", "/index.html"));
        
        // Wildcard at end
        assert!(plugin.path_matches("/admin/users.html", "/admin/*"));
        assert!(plugin.path_matches("/admin/", "/admin/*"));
        assert!(!plugin.path_matches("/user/file.html", "/admin/*"));
        
        // Root wildcard
        assert!(plugin.path_matches("/anything", "/*"));
        assert!(plugin.path_matches("/path/to/file.html", "/*"));
        
        // Parameter matching
        assert!(plugin.path_matches("/users/john/profile", "/users/:username/profile"));
        assert!(plugin.path_matches("/users/jane/profile", "/users/:username/profile"));
        assert!(!plugin.path_matches("/users/john/settings", "/users/:username/profile"));
        
        // Complex patterns - the implementation handles wildcard at the end
        assert!(plugin.path_matches("/api/v1/users/123", "/api/v1/users/:id"));
        assert!(plugin.path_matches("/api/v1/posts", "/api/v1/*"));
        assert!(!plugin.path_matches("/api/v2/users", "/api/v1/*"));
    }
    
    #[test]
    fn test_construct_file_path() {
        let plugin = create_test_plugin();
        let mut context = create_test_context();
        
        // Test with host_root
        context.host_config.insert("host_root".to_string(), "/var/www".to_string());
        let req = create_test_request("GET", "/test.html", None);
        let path = plugin.construct_file_path(&req, &context);
        assert_eq!(path, "/var/www/test.html");
        
        // Test with root path
        let req_root = create_test_request("GET", "/", None);
        let path_root = plugin.construct_file_path(&req_root, &context);
        assert_eq!(path_root, "/var/www/index.html");
        
        // Test with trailing slash
        let req_dir = create_test_request("GET", "/dir/", None);
        let path_dir = plugin.construct_file_path(&req_dir, &context);
        assert_eq!(path_dir, "/var/www/dir/index.html");
    }
    
    #[test]
    fn test_authorization_rule_creation() {
        let rule = AuthorizationRule {
            username: "testuser".to_string(),
            path: "/test/*".to_string(),
            selector: Some("#content".to_string()),
            methods: vec!["GET".to_string(), "POST".to_string()],
            action: Permission::Allow,
        };
        
        assert_eq!(rule.username, "testuser");
        assert_eq!(rule.path, "/test/*");
        assert_eq!(rule.selector, Some("#content".to_string()));
        assert_eq!(rule.methods.len(), 2);
        assert_eq!(rule.action, Permission::Allow);
    }
    
    #[test]
    fn test_user_creation() {
        let user = User {
            username: "john".to_string(),
            roles: vec!["editor".to_string(), "user".to_string()],
        };
        
        assert_eq!(user.username, "john");
        assert_eq!(user.roles.len(), 2);
        assert!(user.roles.contains(&"editor".to_string()));
        assert!(user.roles.contains(&"user".to_string()));
    }
    
    #[test]
    fn test_get_user_roles() {
        let plugin = create_test_plugin();
        let users = vec![
            User {
                username: "admin".to_string(),
                roles: vec!["administrators".to_string(), "users".to_string()],
            },
            User {
                username: "editor".to_string(),
                roles: vec!["editors".to_string()],
            },
        ];
        
        let admin_roles = plugin.get_user_roles("admin", &users);
        assert_eq!(admin_roles.len(), 2);
        assert!(admin_roles.contains(&"administrators".to_string()));
        
        let editor_roles = plugin.get_user_roles("editor", &users);
        assert_eq!(editor_roles.len(), 1);
        assert!(editor_roles.contains(&"editors".to_string()));
        
        let unknown_roles = plugin.get_user_roles("unknown", &users);
        assert_eq!(unknown_roles.len(), 0);
    }
}