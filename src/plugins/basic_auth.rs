use crate::plugins::{AuthPlugin, AuthResult, UserInfo};
use async_trait::async_trait;
use dom_query::Document;
use hyper::{Body, Request};
use std::collections::HashMap;
use std::fs;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;

#[derive(Debug, Clone)]
pub struct User {
    pub username: String,
    pub password: String,
    pub roles: Vec<String>,
    pub encryption: String,
}

#[derive(Debug)]
pub struct BasicAuthPlugin {
    users: HashMap<String, User>,
}

impl BasicAuthPlugin {
    pub fn new(auth_file_path: String) -> Result<Self, String> {
        let users = Self::load_users_from_html(&auth_file_path)?;
        Ok(BasicAuthPlugin {
            users,
        })
    }

    fn load_users_from_html(file_path: &str) -> Result<HashMap<String, User>, String> {
        let mut users = HashMap::new();

        let content = fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read auth file {}: {}", file_path, e))?;

        let document = Document::from(content);
        let user_elements = document.select("[itemscope][itemtype='http://rustybeam.net/User']");

        // Get all username, password, roles, and encryption elements
        let username_elements = document.select("[itemprop='username']");
        let password_elements = document.select("[itemprop='password']");
        let encryption_elements = document.select("[itemprop='encryption']");
        
        // Process each user (assuming same order)
        for i in 0..user_elements.length() {
            let username = if i < username_elements.length() {
                username_elements.get(i).unwrap().text().trim().to_string()
            } else {
                continue;
            };

            let password = if i < password_elements.length() {
                password_elements.get(i).unwrap().text().trim().to_string()
            } else {
                continue;
            };

            let encryption = if i < encryption_elements.length() {
                encryption_elements.get(i).unwrap().text().trim().to_string()
            } else {
                "plaintext".to_string()
            };
            
            // Get roles for this user - find li elements in the roles context
            let roles_elements = document.select("[itemprop='roles'] li");
            let mut roles = Vec::new();
            for j in 0..roles_elements.length() {
                let role = roles_elements.get(j).unwrap().text().trim().to_string();
                if !role.is_empty() {
                    roles.push(role);
                }
            }

            if !username.is_empty() && !password.is_empty() {
                let user = User {
                    username: username.clone(),
                    password,
                    roles,
                    encryption,
                };
                users.insert(username, user);
            }
        }

        Ok(users)
    }

    fn verify_password(&self, provided_password: &str, stored_password: &str, encryption: &str) -> bool {
        match encryption {
            "plaintext" => provided_password == stored_password,
            "bcrypt" => {
                bcrypt::verify(provided_password, stored_password)
                    .unwrap_or(false)
            }
            _ => {
                eprintln!("Unknown encryption type: {}", encryption);
                false
            }
        }
    }

    #[allow(dead_code)] // Public utility for password hashing
    pub fn hash_password(password: &str, encryption: &str) -> Result<String, String> {
        match encryption {
            "plaintext" => Ok(password.to_string()),
            "bcrypt" => {
                bcrypt::hash(password, bcrypt::DEFAULT_COST)
                    .map_err(|e| format!("Failed to hash password: {}", e))
            }
            _ => Err(format!("Unknown encryption type: {}", encryption)),
        }
    }

    fn parse_basic_auth_header(&self, auth_header: &str) -> Option<(String, String)> {
        if !auth_header.starts_with("Basic ") {
            return None;
        }

        let encoded = auth_header.strip_prefix("Basic ")?;
        let decoded = BASE64_STANDARD.decode(encoded).ok()?;
        let credentials = String::from_utf8(decoded).ok()?;
        
        let parts: Vec<&str> = credentials.splitn(2, ':').collect();
        if parts.len() == 2 {
            Some((parts[0].to_string(), parts[1].to_string()))
        } else {
            None
        }
    }
}

#[async_trait]
impl AuthPlugin for BasicAuthPlugin {
    async fn authenticate(&self, req: &Request<Body>) -> AuthResult {
        let auth_header = match req.headers().get("Authorization") {
            Some(header) => match header.to_str() {
                Ok(header_str) => header_str,
                Err(_) => return AuthResult::Error("Invalid Authorization header encoding".to_string()),
            },
            None => return AuthResult::Unauthorized,
        };

        let (username, password) = match self.parse_basic_auth_header(auth_header) {
            Some(credentials) => credentials,
            None => return AuthResult::Error("Invalid Basic Auth format".to_string()),
        };

        match self.users.get(&username) {
            Some(user) => {
                if self.verify_password(&password, &user.password, &user.encryption) {
                    AuthResult::Authorized(UserInfo {
                        username: user.username.clone(),
                        roles: user.roles.clone(),
                    })
                } else {
                    AuthResult::Unauthorized
                }
            }
            None => AuthResult::Unauthorized,
        }
    }

    fn name(&self) -> &'static str {
        "basic-auth"
    }

    fn requires_authentication(&self, _path: &str) -> bool {
        // For now, require authentication for all paths
        // This could be made configurable in the future
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::{Body, Request, Method};
    use std::io::Write;
    use tempfile::NamedTempFile;
    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
    use base64::Engine;

    fn create_test_users_file(users: &[(&str, &str, &[&str], &str)]) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        
        write!(file, r#"<!DOCTYPE html>
<html>
    <head>
        <title>Test Authentication file</title>
    </head>
    <body>
        <table id="users">
            <thead>
                <tr>
                    <td>Username</td>
                    <td>Password</td>
                    <td>Roles</td>
                    <td>Meta</td>
                </tr>
            </thead>
            <tbody>"#).unwrap();

        for (username, password, roles, encryption) in users {
            write!(file, r#"
                <tr itemscope itemtype="http://rustybeam.net/User">
                    <td itemprop="username">{}</td>
                    <td itemprop="password">{}</td>
                    <td itemprop="roles">
                        <ul>"#, username, password).unwrap();
            
            for role in *roles {
                write!(file, "<li>{}</li>", role).unwrap();
            }
            
            write!(file, r#"
                        </ul>
                    </td>
                    <td>
                        <ul>
                            <li itemprop="encryption">{}</li>
                        </ul>
                    </td>
                </tr>"#, encryption).unwrap();
        }

        write!(file, r#"
            </tbody>
        </table>
    </body>
</html>"#).unwrap();

        file.flush().unwrap();
        file
    }

    fn create_basic_auth_header(username: &str, password: &str) -> String {
        let credentials = format!("{}:{}", username, password);
        let encoded = BASE64_STANDARD.encode(credentials.as_bytes());
        format!("Basic {}", encoded)
    }

    #[test]
    fn test_basic_auth_plugin_creation() {
        let file = create_test_users_file(&[
            ("admin", "admin123", &["admin"], "plaintext"),
            ("user", "user123", &["user"], "plaintext"),
        ]);
        
        let plugin = BasicAuthPlugin::new(file.path().to_str().unwrap().to_string());
        assert!(plugin.is_ok());
        
        let plugin = plugin.unwrap();
        assert_eq!(plugin.users.len(), 2);
        assert!(plugin.users.contains_key("admin"));
        assert!(plugin.users.contains_key("user"));
    }

    #[test]
    fn test_basic_auth_plugin_invalid_file() {
        let plugin = BasicAuthPlugin::new("/nonexistent/file.html".to_string());
        assert!(plugin.is_err());
    }

    #[test]
    fn test_password_hashing_plaintext() {
        let result = BasicAuthPlugin::hash_password("test123", "plaintext");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test123");
    }

    #[test]
    fn test_password_hashing_bcrypt() {
        let result = BasicAuthPlugin::hash_password("test123", "bcrypt");
        assert!(result.is_ok());
        
        let hashed = result.unwrap();
        assert!(hashed.starts_with("$2b$"));
        assert_ne!(hashed, "test123");
    }

    #[test]
    fn test_password_hashing_unknown_type() {
        let result = BasicAuthPlugin::hash_password("test123", "unknown");
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_password_plaintext() {
        let file = create_test_users_file(&[
            ("admin", "admin123", &["admin"], "plaintext"),
        ]);
        
        let plugin = BasicAuthPlugin::new(file.path().to_str().unwrap().to_string()).unwrap();
        assert!(plugin.verify_password("admin123", "admin123", "plaintext"));
        assert!(!plugin.verify_password("wrong", "admin123", "plaintext"));
    }

    #[test]
    fn test_verify_password_bcrypt() {
        let file = create_test_users_file(&[
            ("admin", "$2b$12$7TzFqs.CAWoacMSlf7WOK.duP.vf1YwytKWKkDMTgLVgkmKjMZTW2", &["admin"], "bcrypt"),
        ]);
        
        let plugin = BasicAuthPlugin::new(file.path().to_str().unwrap().to_string()).unwrap();
        // This is the bcrypt hash for "admin123"
        assert!(plugin.verify_password("admin123", "$2b$12$7TzFqs.CAWoacMSlf7WOK.duP.vf1YwytKWKkDMTgLVgkmKjMZTW2", "bcrypt"));
        assert!(!plugin.verify_password("wrong", "$2b$12$7TzFqs.CAWoacMSlf7WOK.duP.vf1YwytKWKkDMTgLVgkmKjMZTW2", "bcrypt"));
    }

    #[test]
    fn test_parse_basic_auth_header_valid() {
        let file = create_test_users_file(&[]);
        let plugin = BasicAuthPlugin::new(file.path().to_str().unwrap().to_string()).unwrap();
        
        let auth_header = create_basic_auth_header("admin", "admin123");
        let result = plugin.parse_basic_auth_header(&auth_header);
        
        assert!(result.is_some());
        let (username, password) = result.unwrap();
        assert_eq!(username, "admin");
        assert_eq!(password, "admin123");
    }

    #[test]
    fn test_parse_basic_auth_header_invalid() {
        let file = create_test_users_file(&[]);
        let plugin = BasicAuthPlugin::new(file.path().to_str().unwrap().to_string()).unwrap();
        
        assert!(plugin.parse_basic_auth_header("Bearer token").is_none());
        assert!(plugin.parse_basic_auth_header("Basic invalid_base64!").is_none());
        assert!(plugin.parse_basic_auth_header("Basic dGVzdA==").is_none()); // "test" without colon
    }

    #[tokio::test]
    async fn test_authenticate_no_auth_header() {
        let file = create_test_users_file(&[
            ("admin", "admin123", &["admin"], "plaintext"),
        ]);
        
        let plugin = BasicAuthPlugin::new(file.path().to_str().unwrap().to_string()).unwrap();
        
        let req = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let result = plugin.authenticate(&req).await;
        match result {
            AuthResult::Unauthorized => {},
            _ => panic!("Expected unauthorized result"),
        }
    }

    #[tokio::test]
    async fn test_authenticate_invalid_credentials() {
        let file = create_test_users_file(&[
            ("admin", "admin123", &["admin"], "plaintext"),
        ]);
        
        let plugin = BasicAuthPlugin::new(file.path().to_str().unwrap().to_string()).unwrap();
        
        let auth_header = create_basic_auth_header("admin", "wrongpassword");
        let req = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("Authorization", auth_header)
            .body(Body::empty())
            .unwrap();

        let result = plugin.authenticate(&req).await;
        match result {
            AuthResult::Unauthorized => {},
            _ => panic!("Expected unauthorized result"),
        }
    }

    #[tokio::test]
    async fn test_authenticate_valid_credentials_plaintext() {
        let file = create_test_users_file(&[
            ("admin", "admin123", &["admin", "user"], "plaintext"),
        ]);
        
        let plugin = BasicAuthPlugin::new(file.path().to_str().unwrap().to_string()).unwrap();
        
        let auth_header = create_basic_auth_header("admin", "admin123");
        let req = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("Authorization", auth_header)
            .body(Body::empty())
            .unwrap();

        let result = plugin.authenticate(&req).await;
        match result {
            AuthResult::Authorized(user_info) => {
                assert_eq!(user_info.username, "admin");
                assert_eq!(user_info.roles, vec!["admin", "user"]);
            },
            _ => panic!("Expected authorized result"),
        }
    }

    #[tokio::test]
    async fn test_authenticate_valid_credentials_bcrypt() {
        // Using the bcrypt hash for "admin123"
        let file = create_test_users_file(&[
            ("admin", "$2b$12$7TzFqs.CAWoacMSlf7WOK.duP.vf1YwytKWKkDMTgLVgkmKjMZTW2", &["admin"], "bcrypt"),
        ]);
        
        let plugin = BasicAuthPlugin::new(file.path().to_str().unwrap().to_string()).unwrap();
        
        let auth_header = create_basic_auth_header("admin", "admin123");
        let req = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("Authorization", auth_header)
            .body(Body::empty())
            .unwrap();

        let result = plugin.authenticate(&req).await;
        match result {
            AuthResult::Authorized(user_info) => {
                assert_eq!(user_info.username, "admin");
                assert_eq!(user_info.roles, vec!["admin"]);
            },
            _ => panic!("Expected authorized result"),
        }
    }

    #[tokio::test]
    async fn test_authenticate_mixed_encryption_types() {
        // Test with mixed plaintext and bcrypt passwords
        let file = create_test_users_file(&[
            ("admin", "$2b$12$7TzFqs.CAWoacMSlf7WOK.duP.vf1YwytKWKkDMTgLVgkmKjMZTW2", &["admin"], "bcrypt"),
            ("user", "user123", &["user"], "plaintext"),
        ]);
        
        let plugin = BasicAuthPlugin::new(file.path().to_str().unwrap().to_string()).unwrap();
        
        // Test bcrypt user
        let auth_header = create_basic_auth_header("admin", "admin123");
        let req = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("Authorization", auth_header)
            .body(Body::empty())
            .unwrap();

        let result = plugin.authenticate(&req).await;
        match result {
            AuthResult::Authorized(user_info) => {
                assert_eq!(user_info.username, "admin");
            },
            _ => panic!("Expected authorized result for bcrypt user"),
        }

        // Test plaintext user
        let auth_header = create_basic_auth_header("user", "user123");
        let req = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("Authorization", auth_header)
            .body(Body::empty())
            .unwrap();

        let result = plugin.authenticate(&req).await;
        match result {
            AuthResult::Authorized(user_info) => {
                assert_eq!(user_info.username, "user");
            },
            _ => panic!("Expected authorized result for plaintext user"),
        }
    }

    #[tokio::test]
    async fn test_authenticate_nonexistent_user() {
        let file = create_test_users_file(&[
            ("admin", "admin123", &["admin"], "plaintext"),
        ]);
        
        let plugin = BasicAuthPlugin::new(file.path().to_str().unwrap().to_string()).unwrap();
        
        let auth_header = create_basic_auth_header("nonexistent", "password");
        let req = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("Authorization", auth_header)
            .body(Body::empty())
            .unwrap();

        let result = plugin.authenticate(&req).await;
        match result {
            AuthResult::Unauthorized => {},
            _ => panic!("Expected unauthorized result for nonexistent user"),
        }
    }

    #[test]
    fn test_requires_authentication() {
        let file = create_test_users_file(&[]);
        let plugin = BasicAuthPlugin::new(file.path().to_str().unwrap().to_string()).unwrap();
        
        // Current implementation requires authentication for all paths
        assert!(plugin.requires_authentication("/"));
        assert!(plugin.requires_authentication("/admin"));
        assert!(plugin.requires_authentication("/public"));
    }

    #[test]
    fn test_plugin_name() {
        let file = create_test_users_file(&[]);
        let plugin = BasicAuthPlugin::new(file.path().to_str().unwrap().to_string()).unwrap();
        
        assert_eq!(plugin.name(), "basic-auth");
    }
}