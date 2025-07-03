use crate::config::{AuthConfig, AuthorizationRule, Permission, User};
use regex::Regex;

pub struct AuthorizedUser {
    pub username: String,
    pub roles: Vec<String>,
}

#[allow(dead_code)] // Legacy authorization system, replaced by plugin architecture
pub struct AuthorizationEngine {
    auth_config: AuthConfig,
}

#[allow(dead_code)] // Legacy authorization system, replaced by plugin architecture
impl AuthorizationEngine {
    pub fn new(auth_config: AuthConfig) -> Self {
        Self {
            auth_config,
        }
    }

    #[allow(dead_code)] // Used in authentication workflows and tests
    pub fn get_user(&self, username: &str) -> Option<&User> {
        self.auth_config.users.iter().find(|u| u.username == username)
    }

    pub fn authorize(&self, user: &AuthorizedUser, resource: &str, method: &str) -> bool {
        // Find all matching rules, sorted by specificity
        let mut matching_rules = Vec::new();
        
        for rule in &self.auth_config.authorization_rules {
            if self.rule_matches(rule, user, resource, method) {
                matching_rules.push(rule);
            }
        }
        
        // Sort by specificity (most specific first)
        matching_rules.sort_by_key(|b| std::cmp::Reverse(self.rule_specificity(b)));
        
        // Apply the first (most specific) matching rule
        if let Some(rule) = matching_rules.first() {
            rule.permission == Permission::Allow
        } else {
            // Default deny if no rules match
            false
        }
    }

    fn rule_matches(&self, rule: &AuthorizationRule, user: &AuthorizedUser, resource: &str, method: &str) -> bool {
        // Check method match
        if !rule.methods.contains(&method.to_uppercase()) {
            return false;
        }

        // Check user/role match
        if !self.user_matches(rule, user) {
            return false;
        }

        // Check resource match
        self.resource_matches(&rule.resource, resource, user)
    }

    fn user_matches(&self, rule: &AuthorizationRule, user: &AuthorizedUser) -> bool {
        // Wildcard match
        if rule.username == "*" {
            return true;
        }

        // Direct username match
        if rule.username == user.username {
            return true;
        }

        // Role match
        if user.roles.contains(&rule.username) {
            return true;
        }

        // Variable match (e.g., :username)
        if rule.username.starts_with(':') {
            let var_name = &rule.username[1..];
            if var_name == "username" {
                return true; // :username always matches the current user
            }
        }

        false
    }

    fn resource_matches(&self, pattern: &str, resource: &str, user: &AuthorizedUser) -> bool {
        // Handle selector requests
        let (resource_path, selector) = self.parse_selector_request(resource);
        let (pattern_path, pattern_selector) = self.parse_selector_request(pattern);

        // Check if selectors match
        if pattern_selector.is_some() && selector.is_some() {
            if pattern_selector != selector {
                return false;
            }
        } else if pattern_selector.is_some() != selector.is_some() {
            return false;
        }

        // Check path match
        self.path_matches(&pattern_path, &resource_path, user)
    }

    fn parse_selector_request(&self, resource: &str) -> (String, Option<String>) {
        if let Some(hash_pos) = resource.find("#(selector=") {
            let path = resource[..hash_pos].to_string();
            let selector_start = hash_pos + 11; // length of "#(selector="
            if let Some(end_pos) = resource[selector_start..].find(')') {
                let selector = resource[selector_start..selector_start + end_pos].to_string();
                return (path, Some(selector));
            }
        }
        (resource.to_string(), None)
    }

    fn path_matches(&self, pattern: &str, path: &str, user: &AuthorizedUser) -> bool {
        // First replace path variables with actual values before escaping
        let mut substituted_pattern = pattern.to_string();
        
        // Replace :username with the actual username
        substituted_pattern = substituted_pattern.replace(":username", &user.username);
        
        // Convert to regex pattern, escaping special chars
        let mut regex_pattern = regex::escape(&substituted_pattern);
        
        // Handle wildcards (after escaping)
        regex_pattern = regex_pattern.replace(r"\*", ".*");
        
        // Ensure full string match
        let full_pattern = format!("^{}$", regex_pattern);
        
        if let Ok(regex) = Regex::new(&full_pattern) {
            regex.is_match(path)
        } else {
            false
        }
    }

    fn rule_specificity(&self, rule: &AuthorizationRule) -> i32 {
        let mut specificity = 0i32;

        // More specific users get higher priority
        if rule.username != "*" {
            specificity += 1000;
        }

        // Path specificity - count segments and wildcards
        let path_segments = rule.resource.split('/').count() as i32;
        specificity += path_segments * 10; // More path segments = more specific
        
        // Penalty for wildcards
        let wildcard_count = rule.resource.matches('*').count() as i32;
        specificity -= wildcard_count * 5;
        
        // Bonus for exact paths (no wildcards)
        if !rule.resource.contains('*') {
            specificity += 100;
        }

        // Exact method matches get higher priority
        if rule.methods.len() == 1 {
            specificity += 5;
        }

        // Selector-based rules get higher priority
        if rule.resource.contains("#(selector=") {
            specificity += 50;
        }

        specificity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AuthConfig, AuthorizationRule, Permission, User};

    fn create_test_config() -> AuthConfig {
        AuthConfig {
            users: vec![
                User {
                    username: "admin".to_string(),
                    password: "admin123".to_string(),
                    roles: vec!["administrators".to_string(), "user".to_string()],
                    encryption: "plaintext".to_string(),
                },
                User {
                    username: "johndoe".to_string(),
                    password: "doe123".to_string(),
                    roles: vec!["user".to_string(), "editor".to_string()],
                    encryption: "plaintext".to_string(),
                },
            ],
            authorization_rules: vec![
                AuthorizationRule {
                    username: "*".to_string(),
                    resource: "/*".to_string(),
                    methods: vec!["GET".to_string()],
                    permission: Permission::Allow,
                },
                AuthorizationRule {
                    username: "*".to_string(),
                    resource: "/*".to_string(),
                    methods: vec!["PUT".to_string(), "POST".to_string(), "DELETE".to_string()],
                    permission: Permission::Deny,
                },
                AuthorizationRule {
                    username: "administrators".to_string(),
                    resource: "/admin/*".to_string(),
                    methods: vec!["GET".to_string(), "PUT".to_string(), "POST".to_string(), "DELETE".to_string()],
                    permission: Permission::Allow,
                },
                AuthorizationRule {
                    username: "*".to_string(),
                    resource: "/users/*".to_string(),
                    methods: vec!["GET".to_string(), "PUT".to_string(), "POST".to_string(), "DELETE".to_string()],
                    permission: Permission::Deny,
                },
                AuthorizationRule {
                    username: ":username".to_string(),
                    resource: "/users/:username/*".to_string(),
                    methods: vec!["GET".to_string(), "PUT".to_string(), "POST".to_string(), "DELETE".to_string()],
                    permission: Permission::Allow,
                },
            ],
        }
    }

    #[test]
    fn test_basic_authorization() {
        let config = create_test_config();
        let engine = AuthorizationEngine::new(config);
        
        let user = AuthorizedUser {
            username: "johndoe".to_string(),
            roles: vec!["user".to_string(), "editor".to_string()],
        };

        // Should allow GET to any resource
        assert!(engine.authorize(&user, "/test.html", "GET"));
        
        // Should deny PUT to general resources
        assert!(!engine.authorize(&user, "/test.html", "PUT"));
    }

    #[test]
    fn test_admin_authorization() {
        let config = create_test_config();
        let engine = AuthorizationEngine::new(config);
        
        let admin = AuthorizedUser {
            username: "admin".to_string(),
            roles: vec!["administrators".to_string(), "user".to_string()],
        };

        // Should allow admin access to admin resources
        assert!(engine.authorize(&admin, "/admin/config.html", "GET"));
        assert!(engine.authorize(&admin, "/admin/config.html", "PUT"));
        assert!(engine.authorize(&admin, "/admin/config.html", "DELETE"));
    }

    #[test]
    fn test_user_specific_resources() {
        let config = create_test_config();
        let engine = AuthorizationEngine::new(config);
        
        let user = AuthorizedUser {
            username: "johndoe".to_string(),
            roles: vec!["user".to_string()],
        };

        // Should allow access to own user directory
        assert!(engine.authorize(&user, "/users/johndoe/profile.html", "GET"));
        assert!(engine.authorize(&user, "/users/johndoe/profile.html", "PUT"));
        
        // Should deny access to other user's directory
        assert!(!engine.authorize(&user, "/users/admin/profile.html", "GET"));
    }

    #[test]
    fn test_selector_resources() {
        let config = create_test_config();
        let engine = AuthorizationEngine::new(config);
        
        let _user = AuthorizedUser {
            username: "johndoe".to_string(),
            roles: vec!["user".to_string()],
        };

        // Test selector parsing
        let (path, selector) = engine.parse_selector_request("/test.html#(selector=div.content)");
        assert_eq!(path, "/test.html");
        assert_eq!(selector, Some("div.content".to_string()));
    }
}