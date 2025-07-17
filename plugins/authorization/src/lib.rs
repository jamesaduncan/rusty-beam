//! Authorization Plugin for Rusty Beam
//!
//! This plugin provides role-based access control (RBAC) for HTTP requests by checking
//! user permissions against configured authorization rules. It integrates with
//! authentication plugins to determine access rights based on username, roles,
//! paths, HTTP methods, and CSS selectors.
//!
//! ## Features
//! - **Role-Based Access Control**: Define permissions based on users and roles
//! - **Path-Based Rules**: Control access to specific paths with wildcard support
//! - **Method-Specific Permissions**: Allow/deny specific HTTP methods
//! - **CSS Selector Authorization**: Fine-grained control over HTML elements
//! - **DOM-Aware Matching**: Validates selector permissions against actual HTML structure
//! - **Dynamic Username Placeholders**: Use `${username}` in selectors for user-specific matching
//! - **Priority-Based Rules**: More specific rules override general ones
//! - **OAuth Integration**: Works with OAuth2 plugin for external authentication
//!
//! ## Configuration
//! The plugin reads authorization rules from an HTML file containing microdata:
//! - Credentials define users and their roles
//! - Authorization rules specify who can access what
//!
//! ## Username Placeholders
//! Use `${username}` in selectors to create user-specific rules:
//! - `li:has(meta[content="${username}"])` - matches elements with user's username
//! - `${username}` supports whitespace variations: `${ username }`, `${username }`
//! - Multiple placeholders in one selector are supported
//! - Rules with username placeholders are skipped for anonymous users
//! - Usernames preserve special characters (@ . - _) for email addresses
//! - Quotes and backslashes are escaped to prevent CSS injection
//!
//! ## Rule Priority
//! 1. Exact username match (highest)
//! 2. :username (current authenticated user)
//! 3. Role match
//! 4. Wildcard (*) match (lowest)
//!
//! ## Integration
//! - Must be placed after authentication plugins (basic-auth, oauth2)
//! - Works with selector-handler for CSS selector validation
//! - Provides metadata for downstream plugins

use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, PluginResponse, create_plugin};
use async_trait::async_trait;
use hyper::{Body, Response, StatusCode, header::CONTENT_TYPE};
use std::collections::HashMap;
use std::fs;
use microdata_extract::MicrodataExtractor;
use dom_query::{Document, Selection};
use regex::Regex;

// Schema URLs
const SCHEMA_CREDENTIAL: &str = "https://rustybeam.net/schema/Credential";
const SCHEMA_AUTHORIZATION_RULE: &str = "https://rustybeam.net/schema/AuthorizationRule";

// Default values
const DEFAULT_ACTION: &str = "deny";
const DEFAULT_PLUGIN_NAME: &str = "authorization";

// Special usernames
const USERNAME_CURRENT: &str = ":username";
const USERNAME_WILDCARD: &str = "*";

// Rule priorities
const PRIORITY_EXACT_USERNAME: usize = 3;
const PRIORITY_CURRENT_USER: usize = 2;
const PRIORITY_ROLE_MATCH: usize = 1;
const PRIORITY_WILDCARD: usize = 0;

// Path constants
const PATH_SEPARATOR: char = '/';
const PATH_WILDCARD_SUFFIX: &str = "/*";
const PATH_PARAMETER_PREFIX: char = ':';

// Username placeholder constants
const USERNAME_PLACEHOLDER_PATTERN: &str = r"\$\{\s*username\s*\}";
const USERNAME_ANONYMOUS: &str = "*";

// File extensions
const HTML_EXTENSIONS: &[&str] = &[".html", ".htm"];

// Error HTML template
const ACCESS_DENIED_HTML: &str = r#"<!DOCTYPE html>
<html>
<head><title>403 Forbidden</title></head>
<body>
<h1>403 Forbidden</h1>
<p>User '{}' does not have permission to {} '{}'.</p>
<p>Contact your administrator if you believe this is an error.</p>
</body>
</html>"#;

/// Plugin for resource authorization with role-based access control
#[derive(Debug)]
pub struct AuthorizationPlugin {
    name: String,
    auth_file: Option<String>,
}

/// Authorization rule defining access permissions
#[derive(Debug, Clone)]
pub struct AuthorizationRule {
    /// Username, role name, "*" for all, or ":username" for current user
    pub username: String,
    /// Path pattern (supports wildcards and parameters)
    pub path: String,
    /// Optional CSS selector for fine-grained element access
    pub selector: Option<String>,
    /// HTTP methods this rule applies to
    pub methods: Vec<String>,
    /// Allow or deny action
    pub action: Permission,
}

/// Permission action for authorization rules
#[derive(Debug, Clone, PartialEq)]
pub enum Permission {
    /// Grant access
    Allow,
    /// Deny access
    Deny,
}

/// User definition with roles
#[derive(Debug, Clone)]
pub struct User {
    /// Unique username
    pub username: String,
    /// List of roles assigned to the user
    pub roles: Vec<String>,
}

impl AuthorizationPlugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        let name = config.get("name").cloned().unwrap_or_else(|| DEFAULT_PLUGIN_NAME.to_string());
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
        
        // Process each microdata item
        for item in &items {
            match item.item_type() {
                Some(SCHEMA_CREDENTIAL) => {
                    if let Some(user) = self.parse_user_credential(item) {
                        users.push(user);
                    }
                }
                Some(SCHEMA_AUTHORIZATION_RULE) => {
                    if let Some(rule) = self.parse_authorization_rule(item) {
                        authorization_rules.push(rule);
                    }
                }
                _ => {}
            }
        }
        
        Some((users, authorization_rules))
    }
    
    /// Parse user credential from microdata item
    fn parse_user_credential(&self, item: &microdata_extract::MicrodataItem) -> Option<User> {
        let username = item.get_property("username").unwrap_or_default();
        if username.is_empty() {
            return None;
        }
        
        let roles = item.get_property_values("role");
        Some(User { username, roles })
    }
    
    /// Parse authorization rule from microdata item
    fn parse_authorization_rule(&self, item: &microdata_extract::MicrodataItem) -> Option<AuthorizationRule> {
        // Support both "username" and "role" properties for backward compatibility
        let username = item.get_property("username")
            .or_else(|| item.get_property("role"))
            .unwrap_or_default();
        
        let path = item.get_property("path").unwrap_or_default();
        let methods = item.get_property_values("method");
        
        // Validate required fields
        if username.is_empty() || path.is_empty() || methods.is_empty() {
            return None;
        }
        
        let selector = item.get_property("selector")
            .filter(|s| !s.trim().is_empty());
        
        let action_str = item.get_property("action")
            .unwrap_or_else(|| DEFAULT_ACTION.to_string());
        
        let action = match action_str.to_lowercase().as_str() {
            "allow" => Permission::Allow,
            _ => Permission::Deny,
        };
        
        Some(AuthorizationRule {
            username,
            path,
            selector,
            methods,
            action,
        })
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
        // Try exact match first
        if self.matches_exact_path(path, pattern) {
            return true;
        }
        
        // Try wildcard match
        if self.matches_wildcard_path(path, pattern) {
            return true;
        }
        
        // Try parameter pattern match
        if self.matches_parameter_path(path, pattern) {
            return true;
        }
        
        false
    }
    
    /// Check for exact path match
    fn matches_exact_path(&self, path: &str, pattern: &str) -> bool {
        path == pattern
    }
    
    /// Check for wildcard path match
    fn matches_wildcard_path(&self, path: &str, pattern: &str) -> bool {
        if !pattern.ends_with(PATH_WILDCARD_SUFFIX) {
            return false;
        }
        
        let prefix = &pattern[..pattern.len() - PATH_WILDCARD_SUFFIX.len()];
        
        // Special case: "/" should match "/*" pattern
        if prefix.is_empty() && path == "/" {
            return true;
        }
        
        path.starts_with(prefix)
    }
    
    /// Check for parameter pattern match (e.g., /users/:id)
    fn matches_parameter_path(&self, path: &str, pattern: &str) -> bool {
        if !pattern.contains(PATH_PARAMETER_PREFIX) {
            return false;
        }
        
        let pattern_parts: Vec<&str> = pattern.split(PATH_SEPARATOR).collect();
        let path_parts: Vec<&str> = path.split(PATH_SEPARATOR).collect();
        
        // Handle patterns ending with /*
        if pattern.ends_with(PATH_WILDCARD_SUFFIX) {
            let required_parts = pattern_parts.len() - 1;
            if required_parts > path_parts.len() {
                return false;
            }
            
            for i in 0..required_parts {
                if !self.path_part_matches(path_parts[i], pattern_parts[i]) {
                    return false;
                }
            }
            return true;
        }
        
        // Exact part count match required
        if pattern_parts.len() != path_parts.len() {
            return false;
        }
        
        for i in 0..pattern_parts.len() {
            if !self.path_part_matches(path_parts[i], pattern_parts[i]) {
                return false;
            }
        }
        
        true
    }
    
    /// Check if a single path part matches a pattern part
    fn path_part_matches(&self, path_part: &str, pattern_part: &str) -> bool {
        pattern_part.starts_with(PATH_PARAMETER_PREFIX) || pattern_part == path_part
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
        if rule_selector == USERNAME_WILDCARD {
            return true;
        }
        
        // Validate file for selector checking
        let html_content = match self.validate_file_for_selector_check(file_path, context) {
            Ok(content) => content,
            Err(_) => {
                // Fallback to string comparison if file validation fails
                return rule_selector == request_selector;
            }
        };
        
        // Parse and compare selectors in DOM
        self.compare_selectors_in_dom(rule_selector, request_selector, &html_content, context)
    }
    
    /// Validate file exists and is suitable for selector checking
    fn validate_file_for_selector_check(
        &self,
        file_path: &str,
        context: &PluginContext
    ) -> Result<String, ()> {
        // Check file exists
        if !std::path::Path::new(file_path).exists() {
            context.log_verbose(&format!(
                "[Authorization] File not found for selector check: {}",
                file_path
            ));
            return Err(());
        }
        
        // Check if file is HTML
        if !self.is_html_file(file_path) {
            context.log_verbose(
                "[Authorization] Non-HTML file, using string comparison for selectors"
            );
            return Err(());
        }
        
        // Read file content
        match std::fs::read_to_string(file_path) {
            Ok(content) if !content.trim().is_empty() => Ok(content),
            Ok(_) => {
                context.log_verbose("[Authorization] Empty HTML file, skipping DOM parsing");
                Err(())
            }
            Err(e) => {
                context.log_verbose(&format!(
                    "[Authorization] Failed to read file for selector check: {}",
                    e
                ));
                Err(())
            }
        }
    }
    
    /// Check if file has HTML extension
    fn is_html_file(&self, file_path: &str) -> bool {
        HTML_EXTENSIONS.iter().any(|ext| file_path.ends_with(ext))
    }
    
    /// Compare selectors in parsed DOM
    fn compare_selectors_in_dom(
        &self,
        rule_selector: &str,
        request_selector: &str,
        html_content: &str,
        context: &PluginContext
    ) -> bool {
        let document = Document::from(html_content);
        
        // Get elements matched by both selectors
        let rule_elements = document.select(rule_selector);
        let request_elements = document.select(request_selector);
        
        context.log_verbose(&format!(
            "[Authorization] Rule selector '{}' matches {} elements",
            rule_selector, rule_elements.length()
        ));
        context.log_verbose(&format!(
            "[Authorization] Request selector '{}' matches {} elements",
            request_selector, request_elements.length()
        ));
        
        // Check if request elements are a subset of rule elements
        self.elements_are_subset(&request_elements, &rule_elements)
    }
    
    /// Check if a rule matches the request
    fn rule_matches_request(
        &self,
        rule: &AuthorizationRule,
        username: &str,
        user_roles: &[String],
        request: &PluginRequest,
        context: &PluginContext,
        check_method: Option<&str>
    ) -> Option<usize> {
        // Check method match
        if !self.check_method_match(rule, check_method) {
            return None;
        }
        
        // Check path match
        if !self.check_path_match(rule, request) {
            return None;
        }
        
        // Check selector compatibility
        if !self.check_selector_compatibility(rule, request) {
            return None;
        }
        
        // Early check for username placeholder in selector with anonymous user
        if let Some(selector) = &rule.selector {
            if let Ok(regex) = Regex::new(USERNAME_PLACEHOLDER_PATTERN) {
                if regex.is_match(selector) && (username == USERNAME_ANONYMOUS || username == USERNAME_WILDCARD) {
                    context.log_verbose(&format!(
                        "[Authorization] Skipping rule with username placeholder '{}' for anonymous user", 
                        selector
                    ));
                    return None;
                }
            }
        }
        
        // Calculate priority based on user match
        let priority = self.calculate_rule_priority(rule, username, user_roles)?;
        
        // Validate selector if present
        if !self.validate_selector_match(rule, request, context) {
            return None;
        }
        
        Some(priority)
    }
    
    /// Check if rule method matches request
    fn check_method_match(&self, rule: &AuthorizationRule, check_method: Option<&str>) -> bool {
        match check_method {
            Some(method) => {
                let method_upper = method.to_uppercase();
                rule.methods.iter().any(|m| m.to_uppercase() == method_upper)
            }
            None => true
        }
    }
    
    /// Check if rule path matches request path
    fn check_path_match(&self, rule: &AuthorizationRule, request: &PluginRequest) -> bool {
        self.path_matches(&request.path, &rule.path)
    }
    
    /// Check selector compatibility between rule and request
    fn check_selector_compatibility(&self, rule: &AuthorizationRule, request: &PluginRequest) -> bool {
        let request_has_selector = self.extract_selector_from_request(request).is_some();
        let rule_has_selector = rule.selector.is_some();
        
        // Both must have selectors or both must not have selectors
        request_has_selector == rule_has_selector
    }
    
    /// Calculate rule priority based on user matching
    fn calculate_rule_priority(
        &self,
        rule: &AuthorizationRule,
        username: &str,
        user_roles: &[String]
    ) -> Option<usize> {
        if rule.username == username {
            Some(PRIORITY_EXACT_USERNAME)
        } else if rule.username == USERNAME_CURRENT {
            Some(PRIORITY_CURRENT_USER)
        } else if user_roles.contains(&rule.username) {
            Some(PRIORITY_ROLE_MATCH)
        } else if rule.username == USERNAME_WILDCARD {
            Some(PRIORITY_WILDCARD)
        } else {
            None // Rule doesn't apply to this user
        }
    }
    
    /// Sanitizes a username for safe use in CSS selectors
    /// 
    /// Escapes characters that could break CSS selector syntax when used
    /// inside quoted attribute values. Since the username will be used in
    /// selectors like [content="${username}"], we need to escape quotes
    /// and backslashes that could break out of the quoted string.
    fn sanitize_username_for_css(&self, username: &str) -> String {
        username
            .replace('\\', "\\\\")  // Escape backslashes first
            .replace('"', "\\\"")   // Escape double quotes
            .replace('\'', "\\'")   // Escape single quotes
    }
    
    /// Replaces ${username} placeholders with the actual username
    /// 
    /// Handles variations like ${ username }, ${ username}, etc.
    /// Returns None if the selector contains username placeholders but user is anonymous.
    fn replace_username_placeholder(&self, selector: &str, username: &str) -> Option<String> {
        // Check if selector contains username placeholder
        let regex = match Regex::new(USERNAME_PLACEHOLDER_PATTERN) {
            Ok(r) => r,
            Err(_) => return Some(selector.to_string()), // Return original if regex fails
        };
        
        if regex.is_match(selector) {
            // Skip rule if user is anonymous and selector contains username placeholder
            if username == USERNAME_ANONYMOUS || username == USERNAME_WILDCARD {
                return None;
            }
            
            // Sanitize username and replace placeholder
            let sanitized_username = self.sanitize_username_for_css(username);
            Some(regex.replace_all(selector, &sanitized_username).to_string())
        } else {
            // No placeholder found, return original selector
            Some(selector.to_string())
        }
    }
    
    /// Validate selector match if both rule and request have selectors
    fn validate_selector_match(
        &self,
        rule: &AuthorizationRule,
        request: &PluginRequest,
        context: &PluginContext
    ) -> bool {
        match (&rule.selector, self.extract_selector_from_request(request)) {
            (Some(rule_selector), Some(request_selector)) => {
                // Get authenticated user for placeholder replacement
                let username = request.metadata.get("authenticated_user")
                    .cloned()
                    .unwrap_or_else(|| USERNAME_WILDCARD.to_string());
                
                // Replace ${username} placeholder in rule selector
                let processed_rule_selector = match self.replace_username_placeholder(rule_selector, &username) {
                    Some(selector) => selector,
                    None => {
                        // Rule contains username placeholder but user is anonymous - skip rule
                        context.log_verbose(&format!(
                            "[Authorization] Skipping rule with username placeholder '{}' for anonymous user", 
                            rule_selector
                        ));
                        return false;
                    }
                };
                
                let file_path = self.construct_file_path(request, context);
                let matches = self.check_selector_match(
                    &processed_rule_selector,
                    &request_selector,
                    &file_path,
                    context
                );
                
                if !matches {
                    context.log_verbose(&format!(
                        "[Authorization] Selector '{}' does not match rule selector '{}' (DOM-aware check)", 
                        request_selector, processed_rule_selector
                    ));
                }
                
                matches
            }
            _ => true // No selector validation needed
        }
    }
    
    /// Check if one set of elements is a subset of another
    fn elements_are_subset(&self, subset: &Selection, superset: &Selection) -> bool {
        // Empty subset is always valid
        if subset.is_empty() {
            return true;
        }
        
        // Non-empty subset with empty superset is invalid
        if superset.is_empty() {
            return false;
        }
        
        // Build set of superset element signatures
        let superset_signatures = self.build_element_signature_set(superset);
        
        // Check if all subset elements exist in superset
        self.all_elements_in_set(subset, &superset_signatures)
    }
    
    /// Build a set of element signatures for comparison
    fn build_element_signature_set(&self, selection: &Selection) -> std::collections::HashSet<String> {
        selection.iter()
            .map(|elem| elem.html().to_string())
            .collect()
    }
    
    /// Check if all elements in selection exist in the signature set
    fn all_elements_in_set(&self, selection: &Selection, signatures: &std::collections::HashSet<String>) -> bool {
        selection.iter()
            .all(|elem| signatures.contains(&elem.html().to_string()))
    }
    
    /// Get user's roles
    fn get_user_roles(&self, username: &str, users: &[User], metadata: &HashMap<String, String>) -> Vec<String> {
        // First try to find user in pre-configured list
        if let Some(user) = users.iter().find(|u| u.username == username) {
            return user.roles.clone();
        }
        
        // If not found, check metadata for roles assigned by auth plugins (e.g., OAuth2)
        if let Some(roles_str) = metadata.get("authenticated_user_roles") {
            // Parse comma-separated roles or single role
            roles_str.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Get all allowed methods for a user/path/selector combination
    fn get_allowed_methods(&self, username: &str, request: &PluginRequest, context: &PluginContext) -> Vec<String> {
        let (users, rules) = match self.load_auth_config() {
            Some(config) => config,
            None => {
                context.log_verbose("[Authorization] Failed to load auth config for OPTIONS");
                return vec![];
            }
        };
        
        let user_roles = self.get_user_roles(username, &users, &request.metadata);
        
        // Collect applicable rules with their priorities
        let mut applicable_rules: Vec<(usize, &AuthorizationRule)> = rules.iter()
            .filter_map(|rule| {
                self.rule_matches_request(rule, username, &user_roles, request, context, None)
                    .map(|priority| (priority, rule))
            })
            .collect();
        
        // Sort rules by priority (highest first)
        applicable_rules.sort_by(|a, b| b.0.cmp(&a.0));
        
        // Process rules to determine allowed methods
        let (allowed_methods, _) = self.process_rules_for_methods(&applicable_rules, context);
        
        // Always include OPTIONS itself
        let mut result: Vec<String> = allowed_methods.into_iter().collect();
        if !result.contains(&"OPTIONS".to_string()) {
            result.push("OPTIONS".to_string());
        }
        result.sort();
        
        context.log_verbose(&format!("[Authorization] Allowed methods for user '{}' on '{}': {:?}", 
            username, request.path, result));
        
        result
    }
    
    /// Process rules to determine allowed/denied methods
    fn process_rules_for_methods(
        &self,
        applicable_rules: &[(usize, &AuthorizationRule)],
        context: &PluginContext
    ) -> (std::collections::HashSet<String>, std::collections::HashSet<String>) {
        let mut allowed_methods = std::collections::HashSet::new();
        let mut denied_methods = std::collections::HashSet::new();
        let mut methods_processed = std::collections::HashSet::new();
        
        for (priority, rule) in applicable_rules {
            context.log_verbose(&format!(
                "[Authorization] Processing rule - User: {}, Path: {}, Methods: {:?}, Action: {:?}, Priority: {}", 
                rule.username, rule.path, rule.methods, rule.action, priority
            ));
            
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
        
        (allowed_methods, denied_methods)
    }
    
    /// Check if user is authorized for the request
    fn is_authorized(
        &self, 
        username: &str, 
        request: &PluginRequest, 
        method: &str, 
        context: &PluginContext
    ) -> bool {
        let (users, rules) = match self.load_auth_config() {
            Some(config) => config,
            None => {
                context.log_verbose("[Authorization] Failed to load auth config, denying access");
                return false;
            }
        };
        
        let user_roles = self.get_user_roles(username, &users, &request.metadata);
        
        // Find the best matching rule
        let best_match = self.find_best_matching_rule(
            &rules,
            username,
            &user_roles,
            request,
            method,
            context
        );
        
        match best_match {
            Some((_, rule)) => {
                context.log_verbose(&format!(
                    "[Authorization] Best match - User: {}, Path: {}, Selector: {:?}, Method: {}, Action: {:?}", 
                    rule.username, rule.path, rule.selector, method, rule.action
                ));
                
                let decision = rule.action == Permission::Allow;
                context.log_verbose(&format!(
                    "[Authorization] Final decision for user '{}' accessing '{}' with {}: {}", 
                    username, request.path, method, if decision { "ALLOW" } else { "DENY" }
                ));
                
                decision
            }
            None => {
                context.log_verbose(&format!(
                    "[Authorization] No matching rule found for user '{}' accessing '{}' with {}", 
                    username, request.path, method
                ));
                false
            }
        }
    }
    
    /// Find the best matching authorization rule
    fn find_best_matching_rule<'a>(
        &self,
        rules: &'a [AuthorizationRule],
        username: &str,
        user_roles: &[String],
        request: &PluginRequest,
        method: &str,
        context: &PluginContext
    ) -> Option<(usize, &'a AuthorizationRule)> {
        let mut best_match: Option<(usize, &AuthorizationRule)> = None;
        
        for rule in rules {
            if let Some(priority) = self.rule_matches_request(
                rule,
                username,
                user_roles,
                request,
                context,
                Some(method)
            ) {
                context.log_verbose(&format!(
                    "[Authorization] Rule evaluated - User: {}, Path: {}, Selector: {:?}, Method: {}, Action: {:?}, Priority: {}", 
                    rule.username, rule.path, rule.selector, method, rule.action, priority
                ));
                
                match best_match {
                    None => best_match = Some((priority, rule)),
                    Some((best_priority, _)) => {
                        if priority > best_priority {
                            best_match = Some((priority, rule));
                        }
                    }
                }
            }
        }
        
        best_match
    }
    
    /// Get host root from context
    fn get_host_root(&self, request: &PluginRequest, context: &PluginContext) -> String {
        context.host_config.get("host_root")
            .or_else(|| context.host_config.get("hostRoot"))
            .or_else(|| request.metadata.get("host_root"))
            .or_else(|| context.server_config.get("server_root"))
            .cloned()
            .unwrap_or_else(|| ".".to_string())
    }
    
    /// Normalize path to handle index files
    fn normalize_path(&self, path: &str) -> String {
        if path == "/" {
            "/index.html".to_string()
        } else if path.ends_with('/') {
            format!("{}/index.html", path.trim_end_matches('/'))
        } else {
            path.to_string()
        }
    }
    
    /// Construct file path from request
    fn construct_file_path(&self, request: &PluginRequest, context: &PluginContext) -> String {
        let host_root = self.get_host_root(request, context);
        let normalized_path = self.normalize_path(&request.path);
        
        let file_path = format!("{}{}", host_root, normalized_path);
        context.log_verbose(&format!("[Authorization] Constructed file path: {} (host_root: {}, path: {})", 
            file_path, host_root, normalized_path));
        file_path
    }
    
    /// Create access denied response
    fn create_access_denied(&self, user: &str, resource: &str, method: &str) -> Response<Body> {
        Response::builder()
            .status(StatusCode::FORBIDDEN)
            .header(CONTENT_TYPE, "text/html")
            .body(Body::from(ACCESS_DENIED_HTML
                .replace("{}", user)
                .replace("{}", method)
                .replace("{}", resource)))
            .unwrap()
    }
}

#[async_trait]
impl Plugin for AuthorizationPlugin {
    async fn handle_request(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<PluginResponse> {
        let method = request.http_request.method().as_str().to_string();
        
        // Handle OPTIONS requests for method discovery
        if method == "OPTIONS" {
            return Some(self.handle_options_request(request, context).await.into());
        }
        
        // Handle authorization check for other methods
        self.handle_authorization_check(request, &method, context)
            .map(|response| response.into())
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

impl AuthorizationPlugin {
    /// Handle OPTIONS request for method discovery
    async fn handle_options_request(
        &self,
        request: &PluginRequest,
        context: &PluginContext
    ) -> Response<Body> {
        // Extract selector from request if present
        let selector = self.extract_selector_from_request(request);
        
        // Log OPTIONS request with details
        match &selector {
            Some(sel) => {
                context.log_verbose(&format!(
                    "[Authorization] Processing OPTIONS request for method discovery - Path: '{}', Selector: '{}'",
                    request.path, sel
                ));
            }
            None => {
                context.log_verbose(&format!(
                    "[Authorization] Processing OPTIONS request for method discovery - Path: '{}'",
                    request.path
                ));
            }
        }
        
        // Get authenticated user or default to wildcard
        let user = request.metadata.get("authenticated_user")
            .cloned()
            .unwrap_or_else(|| USERNAME_WILDCARD.to_string());
        
        // Get allowed methods for this user/path/selector combination
        let allowed_methods = self.get_allowed_methods(&user, request, context);
        let allow_header = allowed_methods.join(", ");
        
        context.log_verbose(&format!("[Authorization] OPTIONS response - Allow: {}", allow_header));
        
        Response::builder()
            .status(StatusCode::OK)
            .header("Allow", allow_header)
            .header("Accept-Ranges", "selector")
            .header(CONTENT_TYPE, "text/plain")
            .body(Body::empty())
            .unwrap()
    }
    
    /// Handle authorization check for non-OPTIONS requests
    fn handle_authorization_check(
        &self,
        request: &mut PluginRequest,
        method: &str,
        context: &PluginContext
    ) -> Option<Response<Body>> {
        // Get authenticated user or default to wildcard
        let user = request.metadata.get("authenticated_user")
            .cloned()
            .unwrap_or_else(|| USERNAME_WILDCARD.to_string());
        
        // Extract selector for logging if present
        let selector = self.extract_selector_from_request(request);
        
        match &selector {
            Some(sel) => {
                context.log_verbose(&format!(
                    "[Authorization] Checking authorization for user '{}' - Method: '{}', Path: '{}', Selector: '{}'",
                    user, method, request.path, sel
                ));
            }
            None => {
                context.log_verbose(&format!(
                    "[Authorization] Checking authorization for user '{}' - Method: '{}', Path: '{}'",
                    user, method, request.path
                ));
            }
        }
        
        // Check if user is authorized
        if !self.is_authorized(&user, request, method, context) {
            return Some(self.create_access_denied(&user, &request.path, method));
        }
        
        // Set authorization metadata for downstream plugins
        self.set_authorization_metadata(request, &user);
        
        match &selector {
            Some(sel) => {
                context.log_verbose(&format!(
                    "[Authorization] Access granted for user '{}' - Method: '{}', Path: '{}', Selector: '{}'",
                    user, method, request.path, sel
                ));
            }
            None => {
                context.log_verbose(&format!(
                    "[Authorization] Access granted for user '{}' - Method: '{}', Path: '{}'",
                    user, method, request.path
                ));
            }
        }
        
        None // Pass to next plugin
    }
    
    /// Set authorization metadata in request
    fn set_authorization_metadata(&self, request: &mut PluginRequest, user: &str) {
        request.metadata.insert("authorized".to_string(), "true".to_string());
        request.metadata.insert("authorized_user".to_string(), user.to_string());
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
            server_metadata: HashMap::new(),
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
        
        let test_metadata = HashMap::new();
        
        let admin_roles = plugin.get_user_roles("admin", &users, &test_metadata);
        assert_eq!(admin_roles.len(), 2);
        assert!(admin_roles.contains(&"administrators".to_string()));
        
        let editor_roles = plugin.get_user_roles("editor", &users, &test_metadata);
        assert_eq!(editor_roles.len(), 1);
        assert!(editor_roles.contains(&"editors".to_string()));
        
        let unknown_roles = plugin.get_user_roles("unknown", &users, &test_metadata);
        assert_eq!(unknown_roles.len(), 0);
    }
    
    #[test]
    fn test_get_user_roles_from_metadata() {
        let plugin = create_test_plugin();
        let users = vec![]; // No pre-configured users
        
        // Test metadata role assignment
        let mut metadata = HashMap::new();
        metadata.insert("authenticated_user".to_string(), "oauth@example.com".to_string());
        metadata.insert("authenticated_user_roles".to_string(), "user".to_string());
        
        let roles = plugin.get_user_roles("oauth@example.com", &users, &metadata);
        assert_eq!(roles.len(), 1);
        assert!(roles.contains(&"user".to_string()));
        
        // Test multiple roles
        metadata.insert("authenticated_user_roles".to_string(), "user,editor".to_string());
        let multi_roles = plugin.get_user_roles("oauth@example.com", &users, &metadata);
        assert_eq!(multi_roles.len(), 2);
        assert!(multi_roles.contains(&"user".to_string()));
        assert!(multi_roles.contains(&"editor".to_string()));
        
        // Test that pre-configured users take precedence
        let configured_user = User {
            username: "oauth@example.com".to_string(),
            roles: vec!["admin".to_string()],
        };
        let users_with_config = vec![configured_user];
        let config_roles = plugin.get_user_roles("oauth@example.com", &users_with_config, &metadata);
        assert_eq!(config_roles.len(), 1);
        assert!(config_roles.contains(&"admin".to_string()));
        assert!(!config_roles.contains(&"user".to_string()));
    }
    
    #[test]
    fn test_username_placeholder_replacement() {
        let plugin = create_test_plugin();
        
        // Test basic replacement
        let selector = "li:has(meta[content=\"${username}\"])";
        let replaced = plugin.replace_username_placeholder(selector, "johndoe");
        assert_eq!(replaced, Some("li:has(meta[content=\"johndoe\"])".to_string()));
        
        // Test with whitespace variations
        let selector_spaces = "li:has(meta[content=\"${ username }\"])";
        let replaced_spaces = plugin.replace_username_placeholder(selector_spaces, "johndoe");
        assert_eq!(replaced_spaces, Some("li:has(meta[content=\"johndoe\"])".to_string()));
        
        // Test with asymmetric whitespace
        let selector_asymmetric = "li:has(meta[content=\"${username }\"])";
        let replaced_asymmetric = plugin.replace_username_placeholder(selector_asymmetric, "johndoe");
        assert_eq!(replaced_asymmetric, Some("li:has(meta[content=\"johndoe\"])".to_string()));
        
        // Test multiple replacements
        let selector_multiple = "li:has(meta[content=\"${username}\"]) .${username}-data";
        let replaced_multiple = plugin.replace_username_placeholder(selector_multiple, "johndoe");
        assert_eq!(replaced_multiple, Some("li:has(meta[content=\"johndoe\"]) .johndoe-data".to_string()));
        
        // Test with anonymous user - should return None
        let replaced_anon = plugin.replace_username_placeholder(selector, "*");
        assert_eq!(replaced_anon, None);
        
        // Test with wildcard user - should return None
        let replaced_wildcard = plugin.replace_username_placeholder(selector, USERNAME_WILDCARD);
        assert_eq!(replaced_wildcard, None);
        
        // Test without placeholder - should return original
        let selector_no_placeholder = "li:has(meta[content=\"static-value\"])";
        let replaced_no_placeholder = plugin.replace_username_placeholder(selector_no_placeholder, "johndoe");
        assert_eq!(replaced_no_placeholder, Some("li:has(meta[content=\"static-value\"])".to_string()));
        
        // Test with email address username
        let selector_email = "li:has(meta[content=\"${username}\"])";
        let replaced_email = plugin.replace_username_placeholder(selector_email, "jamesaduncan@mac.com");
        assert_eq!(replaced_email, Some("li:has(meta[content=\"jamesaduncan@mac.com\"])".to_string()));
        
        // Test with username containing quotes (should be escaped)
        let username_with_quotes = "user\"with'quotes";
        let replaced_quotes = plugin.replace_username_placeholder(selector, username_with_quotes);
        assert_eq!(replaced_quotes, Some("li:has(meta[content=\"user\\\"with\\'quotes\"])".to_string()));
    }
    
    #[test]
    fn test_username_sanitization() {
        let plugin = create_test_plugin();
        
        // Test normal username
        let sanitized = plugin.sanitize_username_for_css("johndoe");
        assert_eq!(sanitized, "johndoe");
        
        // Test email addresses - should be preserved
        let sanitized_email = plugin.sanitize_username_for_css("john@doe.com");
        assert_eq!(sanitized_email, "john@doe.com");
        
        // Test with allowed characters - should be preserved
        let sanitized_allowed = plugin.sanitize_username_for_css("john-doe_123");
        assert_eq!(sanitized_allowed, "john-doe_123");
        
        // Test with quotes - should be escaped
        let sanitized_quotes = plugin.sanitize_username_for_css("john\"doe'hack");
        assert_eq!(sanitized_quotes, "john\\\"doe\\'hack");
        
        // Test with backslashes - should be escaped
        let sanitized_backslash = plugin.sanitize_username_for_css("john\\doe");
        assert_eq!(sanitized_backslash, "john\\\\doe");
        
        // Test with CSS injection attempt - quotes and backslashes escaped
        let sanitized_injection = plugin.sanitize_username_for_css("john\\\"; alert('xss'); /*");
        assert_eq!(sanitized_injection, "john\\\\\\\"; alert(\\'xss\\'); /*");
    }
}