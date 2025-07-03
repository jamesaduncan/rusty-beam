use crate::config::{load_config_from_html, load_auth_config_from_html};
use crate::auth::{AuthorizationEngine, AuthorizedUser};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_separate_authorization_file_loading() {
        // Test that we can load authorization config from a separate file
        let auth_config = load_auth_config_from_html("./localhost/auth/authorization-only.html");
        assert!(auth_config.is_some());
        
        let config = auth_config.unwrap();
        assert!(!config.authorization_rules.is_empty());
        
        // Verify the rules from our test file
        assert!(config.authorization_rules.iter().any(|rule| 
            rule.username == "*" && rule.resource == "/*" && rule.methods.contains(&"GET".to_string())
        ));
        
        assert!(config.authorization_rules.iter().any(|rule| 
            rule.username == "testuser" && rule.resource == "/test-resource/*"
        ));
    }

    #[test]
    fn test_config_loading_with_separate_auth_file() {
        // Test loading configuration that specifies a separate authorization file
        let config = load_config_from_html("test-config-separate-auth.html");
        
        // Should have localhost host configured
        assert!(config.hosts.contains_key("localhost"));
        let localhost_config = &config.hosts["localhost"];
        
        // Should have auth config loaded from the separate file
        assert!(localhost_config.auth_config.is_some());
        
        let auth_config = localhost_config.auth_config.as_ref().unwrap();
        
        // Should contain the rules from authorization-only.html
        assert!(auth_config.authorization_rules.iter().any(|rule| 
            rule.username == "testuser" && rule.resource == "/test-resource/*"
        ));
    }

    #[test]
    fn test_authorization_engine_with_any_user_source() {
        // Test that authorization engine works regardless of how the user was authenticated
        let auth_config = load_auth_config_from_html("./localhost/auth/authorization-only.html").unwrap();
        
        // Verify correct authorization rules are loaded
        
        let engine = AuthorizationEngine::new(auth_config);

        // Create users as if they came from different authentication plugins
        let user_from_basic_auth = AuthorizedUser {
            username: "testuser".to_string(),
            roles: vec!["user".to_string()],
        };

        let user_from_oauth = AuthorizedUser {
            username: "testuser".to_string(),
            roles: vec!["oauth_user".to_string()],
        };

        let user_from_custom_plugin = AuthorizedUser {
            username: "testuser".to_string(),
            roles: vec!["custom_role".to_string()],
        };

        // All should work the same way - authorization is independent of authentication method
        assert!(engine.authorize(&user_from_basic_auth, "/test-resource/file.txt", "PUT"));
        assert!(engine.authorize(&user_from_oauth, "/test-resource/file.txt", "PUT"));
        assert!(engine.authorize(&user_from_custom_plugin, "/test-resource/file.txt", "PUT"));

        // All should be denied for unauthorized operations
        assert!(!engine.authorize(&user_from_basic_auth, "/test-resource/file.txt", "DELETE"));
        assert!(!engine.authorize(&user_from_oauth, "/test-resource/file.txt", "DELETE"));
        assert!(!engine.authorize(&user_from_custom_plugin, "/test-resource/file.txt", "DELETE"));
    }

    #[test]
    fn test_per_host_authorization_files() {
        // Test that different hosts can have different authorization files
        let config = load_config_from_html("test-config-separate-auth.html");
        
        // localhost should have auth config
        assert!(config.hosts["localhost"].auth_config.is_some());
        
        // example.com should not have auth config (no authorization file specified)
        assert!(config.hosts["example.com"].auth_config.is_none());
        
        // This demonstrates per-host authorization file configuration
        let localhost_auth = config.hosts["localhost"].auth_config.as_ref().unwrap();
        
        // Verify it loaded the correct authorization rules
        assert!(localhost_auth.authorization_rules.iter().any(|rule| 
            rule.username == "testuser"
        ));
    }

    #[test]
    fn test_backwards_compatibility_with_auth_file() {
        // Test that the system still works with the old authFile configuration from plugins
        let config = load_config_from_html("config.html");
        
        // Should have localhost configured with auth from plugin authFile
        assert!(config.hosts.contains_key("localhost"));
        let localhost_config = &config.hosts["localhost"];
        assert!(localhost_config.auth_config.is_some());
        
        // Should have loaded authorization rules from the users.html file via plugin config
        let auth_config = localhost_config.auth_config.as_ref().unwrap();
        assert!(!auth_config.authorization_rules.is_empty());
    }
}