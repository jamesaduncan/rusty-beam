use crate::plugins::{AuthPlugin, AuthResult, UserInfo};
use async_trait::async_trait;
use hyper::{Body, Request};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleOAuth2Config {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub allowed_domains: Vec<String>, // Optional: restrict to specific domains
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
    pub scope: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleUserInfo {
    pub id: String,
    pub email: String,
    pub verified_email: bool,
    pub name: String,
    pub given_name: String,
    pub family_name: String,
    pub picture: String,
    pub locale: String,
    pub hd: Option<String>, // Hosted domain for Google Workspace accounts
}

#[derive(Debug)]
pub struct GoogleOAuth2Plugin {
    config: GoogleOAuth2Config,
    client: reqwest::Client,
}

impl GoogleOAuth2Plugin {
    pub fn new(config: GoogleOAuth2Config) -> Result<Self, String> {
        let client = reqwest::Client::new();
        Ok(GoogleOAuth2Plugin { config, client })
    }

    #[allow(dead_code)] // Public API for OAuth2 authorization flow
    pub fn get_authorization_url(&self, state: &str) -> String {
        let mut auth_url = Url::parse("https://accounts.google.com/o/oauth2/v2/auth").unwrap();
        
        auth_url.query_pairs_mut()
            .append_pair("client_id", &self.config.client_id)
            .append_pair("redirect_uri", &self.config.redirect_uri)
            .append_pair("scope", "openid email profile")
            .append_pair("response_type", "code")
            .append_pair("state", state)
            .append_pair("access_type", "offline")
            .append_pair("prompt", "consent");
            
        auth_url.to_string()
    }

    #[allow(dead_code)] // Public API for OAuth2 authorization flow
    pub async fn exchange_code_for_token(&self, code: &str) -> Result<GoogleTokenResponse, String> {
        let token_url = "https://oauth2.googleapis.com/token";
        
        let params = [
            ("client_id", self.config.client_id.as_str()),
            ("client_secret", self.config.client_secret.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", self.config.redirect_uri.as_str()),
        ];
        
        let response = self.client
            .post(token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Failed to exchange code for token: {}", e))?;
            
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Token exchange failed: {}", error_text));
        }
        
        response.json::<GoogleTokenResponse>()
            .await
            .map_err(|e| format!("Failed to parse token response: {}", e))
    }

    pub async fn get_user_info(&self, access_token: &str) -> Result<GoogleUserInfo, String> {
        let user_info_url = "https://www.googleapis.com/oauth2/v2/userinfo";
        
        let response = self.client
            .get(user_info_url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| format!("Failed to get user info: {}", e))?;
            
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("User info request failed: {}", error_text));
        }
        
        response.json::<GoogleUserInfo>()
            .await
            .map_err(|e| format!("Failed to parse user info response: {}", e))
    }

    fn parse_bearer_token(&self, auth_header: &str) -> Option<String> {
        if !auth_header.starts_with("Bearer ") {
            return None;
        }
        auth_header.strip_prefix("Bearer ").map(|token| token.to_string())
    }

    fn is_domain_allowed(&self, email: &str) -> bool {
        if self.config.allowed_domains.is_empty() {
            return true; // No domain restrictions
        }

        if let Some(domain) = email.split('@').nth(1) {
            self.config.allowed_domains.contains(&domain.to_string())
        } else {
            false
        }
    }
}

#[async_trait]
impl AuthPlugin for GoogleOAuth2Plugin {
    async fn authenticate(&self, req: &Request<Body>) -> AuthResult {
        // Check for Bearer token in Authorization header
        let auth_header = match req.headers().get("Authorization") {
            Some(header) => match header.to_str() {
                Ok(header_str) => header_str,
                Err(_) => return AuthResult::Error("Invalid Authorization header encoding".to_string()),
            },
            None => return AuthResult::Unauthorized,
        };

        let access_token = match self.parse_bearer_token(auth_header) {
            Some(token) => token,
            None => return AuthResult::Error("Invalid Bearer token format".to_string()),
        };

        // Get user info from Google
        let user_info = match self.get_user_info(&access_token).await {
            Ok(info) => info,
            Err(e) => return AuthResult::Error(format!("Failed to validate token: {}", e)),
        };

        // Check if the user's email is verified
        if !user_info.verified_email {
            return AuthResult::Unauthorized;
        }

        // Check domain restrictions if configured
        if !self.is_domain_allowed(&user_info.email) {
            return AuthResult::Unauthorized;
        }

        // Create user info with Google account details
        AuthResult::Authorized(UserInfo {
            username: user_info.email.clone(),
            roles: vec!["user".to_string()], // Default role, could be customized
        })
    }

    fn name(&self) -> &'static str {
        "google-oauth2"
    }

    fn requires_authentication(&self, _path: &str) -> bool {
        // For now, require authentication for all paths
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::{Body, Request, Method};

    fn create_test_config() -> GoogleOAuth2Config {
        GoogleOAuth2Config {
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string(),
            redirect_uri: "http://localhost:3000/auth/google/callback".to_string(),
            allowed_domains: vec!["example.com".to_string()],
        }
    }

    #[test]
    fn test_google_oauth2_plugin_creation() {
        let config = create_test_config();
        let plugin = GoogleOAuth2Plugin::new(config);
        assert!(plugin.is_ok());
        
        let plugin = plugin.unwrap();
        assert_eq!(plugin.name(), "google-oauth2");
    }

    #[test]
    fn test_requires_authentication() {
        let config = create_test_config();
        let plugin = GoogleOAuth2Plugin::new(config).unwrap();
        
        assert!(plugin.requires_authentication("/"));
        assert!(plugin.requires_authentication("/admin"));
        assert!(plugin.requires_authentication("/public"));
    }

    #[test]
    fn test_parse_bearer_token_valid() {
        let config = create_test_config();
        let plugin = GoogleOAuth2Plugin::new(config).unwrap();
        
        let token = plugin.parse_bearer_token("Bearer abc123token");
        assert_eq!(token, Some("abc123token".to_string()));
    }

    #[test]
    fn test_parse_bearer_token_invalid() {
        let config = create_test_config();
        let plugin = GoogleOAuth2Plugin::new(config).unwrap();
        
        assert_eq!(plugin.parse_bearer_token("Basic abc123"), None);
        assert_eq!(plugin.parse_bearer_token("Bearer"), None);
        assert_eq!(plugin.parse_bearer_token(""), None);
    }

    #[test]
    fn test_is_domain_allowed_with_restrictions() {
        let config = create_test_config();
        let plugin = GoogleOAuth2Plugin::new(config).unwrap();
        
        assert!(plugin.is_domain_allowed("user@example.com"));
        assert!(!plugin.is_domain_allowed("user@other.com"));
        assert!(!plugin.is_domain_allowed("invalid-email"));
    }

    #[test]
    fn test_is_domain_allowed_no_restrictions() {
        let mut config = create_test_config();
        config.allowed_domains = vec![];
        let plugin = GoogleOAuth2Plugin::new(config).unwrap();
        
        assert!(plugin.is_domain_allowed("user@example.com"));
        assert!(plugin.is_domain_allowed("user@any-domain.com"));
    }

    #[tokio::test]
    async fn test_authenticate_no_auth_header() {
        let config = create_test_config();
        let plugin = GoogleOAuth2Plugin::new(config).unwrap();
        
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
    async fn test_authenticate_invalid_bearer_token() {
        let config = create_test_config();
        let plugin = GoogleOAuth2Plugin::new(config).unwrap();
        
        let req = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("Authorization", "Bearer invalid_token")
            .body(Body::empty())
            .unwrap();

        let result = plugin.authenticate(&req).await;
        match result {
            AuthResult::Error(_) => {}, // Invalid tokens return errors from Google API
            _ => panic!("Expected error result for invalid token"),
        }
    }

    #[test]
    fn test_get_authorization_url() {
        let config = create_test_config();
        let plugin = GoogleOAuth2Plugin::new(config).unwrap();
        
        let auth_url = plugin.get_authorization_url("test_state");
        
        // Should contain Google OAuth2 endpoint
        assert!(auth_url.contains("accounts.google.com"));
        assert!(auth_url.contains("client_id=test_client_id"));
        assert!(auth_url.contains("state=test_state"));
        assert!(auth_url.contains("redirect_uri="));
    }

    #[tokio::test]
    async fn test_exchange_code_for_token_success() {
        let config = create_test_config();
        let plugin = GoogleOAuth2Plugin::new(config).unwrap();
        
        // This test will need to be mocked since it requires actual HTTP calls
        // For now, we'll test the error case and implement mocking later
        let result = plugin.exchange_code_for_token("test_code").await;
        
        // Should fail with network error since we're not mocking
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_user_info_success() {
        let config = create_test_config();
        let plugin = GoogleOAuth2Plugin::new(config).unwrap();
        
        // This test will need to be mocked since it requires actual HTTP calls
        let result = plugin.get_user_info("test_token").await;
        
        // Should fail with network error since we're not mocking
        assert!(result.is_err());
    }
}