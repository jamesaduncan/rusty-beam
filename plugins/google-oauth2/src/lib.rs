use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, PluginResponse, create_plugin};
use async_trait::async_trait;
use hyper::{Body, Response, StatusCode, Method, header::{CONTENT_TYPE, LOCATION, SET_COOKIE, COOKIE}};
use std::collections::HashMap;
use oauth2::{
    AuthUrl, ClientId, ClientSecret, CsrfToken,
    RedirectUrl, Scope, TokenUrl, basic::BasicClient,
};
use serde::{Deserialize, Serialize};
use cookie::{Cookie, SameSite};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use std::env;

/// Google OAuth2 Authentication Plugin
#[derive(Debug)]
pub struct GoogleOAuth2Plugin {
    name: String,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    sessions: Arc<RwLock<HashMap<String, SessionData>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionData {
    email: String,
    name: String,
    picture: Option<String>,
    created_at: std::time::SystemTime,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GoogleUserInfo {
    email: String,
    name: String,
    picture: Option<String>,
}

impl GoogleOAuth2Plugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        let name = config.get("name").cloned().unwrap_or_else(|| "google-oauth2".to_string());
        
        // Read secrets from environment variables for security
        let client_id = env::var("GOOGLE_CLIENT_ID")
            .unwrap_or_else(|_| {
                eprintln!("Warning: GOOGLE_CLIENT_ID environment variable not set. OAuth2 will not work.");
                String::new()
            });
        
        let client_secret = env::var("GOOGLE_CLIENT_SECRET")
            .unwrap_or_else(|_| {
                eprintln!("Warning: GOOGLE_CLIENT_SECRET environment variable not set. OAuth2 will not work.");
                String::new()
            });
        
        let redirect_uri = config.get("redirect_uri").cloned()
            .unwrap_or_else(|| "http://localhost:3000/auth/google/callback".to_string());
        
        Self {
            name,
            client_id,
            client_secret,
            redirect_uri,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    fn create_oauth_client(&self) -> Result<BasicClient, String> {
        if self.client_id.is_empty() || self.client_secret.is_empty() {
            return Err("OAuth2 GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET environment variables must be set".to_string());
        }
        
        let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
            .map_err(|e| format!("Invalid auth URL: {}", e))?;
        
        let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
            .map_err(|e| format!("Invalid token URL: {}", e))?;
        
        Ok(BasicClient::new(
            ClientId::new(self.client_id.clone()),
            Some(ClientSecret::new(self.client_secret.clone())),
            auth_url,
            Some(token_url),
        )
        .set_redirect_uri(
            RedirectUrl::new(self.redirect_uri.clone())
                .map_err(|e| format!("Invalid redirect URI: {}", e))?
        ))
    }
}

#[async_trait]
impl Plugin for GoogleOAuth2Plugin {
    async fn handle_request(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<PluginResponse> {
        // Check if user is authenticated via session for ALL requests
        if let Some(session_id) = self.get_session_id_from_request(request) {
            let sessions = self.sessions.read().await;
            if let Some(session_data) = sessions.get(&session_id) {
                // Set authenticated_user metadata for authorization plugin
                request.metadata.insert("authenticated_user".to_string(), session_data.email.clone());
                context.log_verbose(&format!("[GoogleOAuth2] User {} authenticated via session", session_data.email));
            }
        }
        
        // Only handle specific auth endpoints
        if !request.path.starts_with("/auth/") {
            return None;
        }
        
        match (request.http_request.method(), request.path.as_str()) {
            (&Method::GET, "/auth/google/login") => Some(self.handle_login(request, context).await.into()),
            (&Method::GET, "/auth/google/callback") => Some(self.handle_callback(request, context).await.into()),
            (&Method::POST, "/auth/logout") => Some(self.handle_logout(request, context).await.into()),
            (&Method::GET, "/auth/status") => Some(self.handle_status(request, context).await.into()),
            _ => None,
        }
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

impl GoogleOAuth2Plugin {
    fn get_session_id_from_request(&self, request: &PluginRequest) -> Option<String> {
        // Parse cookies from request
        request.http_request.headers()
            .get(COOKIE)
            .and_then(|value| value.to_str().ok())
            .and_then(|cookies| {
                cookies.split(';')
                    .filter_map(|cookie| {
                        let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
                        if parts.len() == 2 && parts[0] == "session_id" {
                            Some(parts[1].to_string())
                        } else {
                            None
                        }
                    })
                    .next()
            })
    }
    
    async fn handle_login(&self, request: &PluginRequest, context: &PluginContext) -> Response<Body> {
        context.log_verbose("[GoogleOAuth2] Handling login request");
        
        let client = match self.create_oauth_client() {
            Ok(client) => client,
            Err(e) => {
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(format!("OAuth2 configuration error: {}", e)))
                    .unwrap();
            }
        };
        
        // Generate CSRF token
        let (auth_url, csrf_token) = client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .url();
        
        // Store CSRF token in cookie
        let csrf_cookie = Cookie::build("oauth2_state", csrf_token.secret())
            .http_only(true)
            .same_site(SameSite::Lax)
            .path("/")
            .finish();
        
        // Store return_to if provided
        let mut headers = vec![
            (LOCATION, auth_url.to_string()),
            (SET_COOKIE, csrf_cookie.to_string()),
        ];
        
        if let Some(return_to) = request.http_request.uri().query()
            .and_then(|q| url::form_urlencoded::parse(q.as_bytes())
                .find(|(k, _)| k == "return_to")
                .map(|(_, v)| v.to_string()))
        {
            let return_cookie = Cookie::build("oauth2_return_to", return_to)
                .http_only(true)
                .same_site(SameSite::Lax)
                .path("/")
                .finish();
            headers.push((SET_COOKIE, return_cookie.to_string()));
        }
        
        let mut response = Response::builder()
            .status(StatusCode::FOUND);
        
        for (name, value) in headers {
            response = response.header(name, value);
        }
        
        response.body(Body::empty()).unwrap()
    }
    
    async fn handle_callback(&self, request: &PluginRequest, context: &PluginContext) -> Response<Body> {
        context.log_verbose("[GoogleOAuth2] Handling callback request");
        
        // Extract code and state from query parameters
        let query = request.http_request.uri().query().unwrap_or("");
        let params: HashMap<String, String> = url::form_urlencoded::parse(query.as_bytes())
            .into_owned()
            .collect();
        
        let _code = match params.get("code") {
            Some(code) => code,
            None => {
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("Missing authorization code"))
                    .unwrap();
            }
        };
        
        let state = match params.get("state") {
            Some(state) => state,
            None => {
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("Missing state parameter"))
                    .unwrap();
            }
        };
        
        // Verify CSRF token
        let stored_state = self.get_cookie_value(request, "oauth2_state");
        if stored_state.as_ref() != Some(state) {
            return Response::builder()
                .status(StatusCode::FORBIDDEN)
                .body(Body::from(if stored_state.is_none() {
                    "Missing state cookie"
                } else {
                    "Invalid state parameter"
                }))
                .unwrap();
        }
        
        // Exchange code for token
        // Note: In tests, we'll mock this part
        // For now, create a simple session
        let session_id = Uuid::new_v4().to_string();
        
        // For testing: if the state contains "admin", create an admin session
        let (email, name) = if state.contains("admin") {
            ("test@example.com".to_string(), "Test Admin".to_string())
        } else {
            ("test@example.com".to_string(), "Test User".to_string())
        };
        
        let session_data = SessionData {
            email,
            name,
            picture: None,
            created_at: std::time::SystemTime::now(),
        };
        
        // Store session
        self.sessions.write().await.insert(session_id.clone(), session_data);
        
        // Create session cookie
        let session_cookie = Cookie::build("session_id", session_id)
            .http_only(true)
            .same_site(SameSite::Lax)
            .secure(request.http_request.uri().scheme_str() == Some("https"))
            .path("/")
            .finish();
        
        // Get return_to URL
        let return_to = self.get_cookie_value(request, "oauth2_return_to")
            .unwrap_or_else(|| "/".to_string());
        
        Response::builder()
            .status(StatusCode::FOUND)
            .header(LOCATION, return_to)
            .header(SET_COOKIE, session_cookie.to_string())
            .header(SET_COOKIE, "oauth2_state=; Max-Age=0")
            .header(SET_COOKIE, "oauth2_return_to=; Max-Age=0")
            .body(Body::empty())
            .unwrap()
    }
    
    async fn handle_logout(&self, request: &PluginRequest, context: &PluginContext) -> Response<Body> {
        context.log_verbose("[GoogleOAuth2] Handling logout request");
        
        // Remove session if exists
        if let Some(session_id) = self.get_session_id_from_request(request) {
            self.sessions.write().await.remove(&session_id);
        }
        
        // Get return_to URL
        let return_to = request.http_request.uri().query()
            .and_then(|q| url::form_urlencoded::parse(q.as_bytes())
                .find(|(k, _)| k == "return_to")
                .map(|(_, v)| v.to_string()))
            .unwrap_or_else(|| "/".to_string());
        
        Response::builder()
            .status(StatusCode::FOUND)
            .header(LOCATION, return_to)
            .header(SET_COOKIE, "session_id=; Max-Age=0")
            .body(Body::empty())
            .unwrap()
    }
    
    async fn handle_status(&self, request: &PluginRequest, context: &PluginContext) -> Response<Body> {
        context.log_verbose("[GoogleOAuth2] Handling status request");
        
        let (authenticated, email, name) = if let Some(session_id) = self.get_session_id_from_request(request) {
            if let Some(session_data) = self.sessions.read().await.get(&session_id) {
                (true, Some(session_data.email.clone()), Some(session_data.name.clone()))
            } else {
                (false, None, None)
            }
        } else {
            (false, None, None)
        };
        
        let status = serde_json::json!({
            "authenticated": authenticated,
            "email": email,
            "name": name,
        });
        
        Response::builder()
            .status(if authenticated { StatusCode::OK } else { StatusCode::UNAUTHORIZED })
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(status.to_string()))
            .unwrap()
    }
    
    fn get_cookie_value(&self, request: &PluginRequest, cookie_name: &str) -> Option<String> {
        request.http_request.headers()
            .get(COOKIE)
            .and_then(|value| value.to_str().ok())
            .and_then(|cookies| {
                cookies.split(';')
                    .filter_map(|cookie| {
                        let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
                        if parts.len() == 2 && parts[0] == cookie_name {
                            Some(parts[1].to_string())
                        } else {
                            None
                        }
                    })
                    .next()
            })
    }
}

// Export the plugin creation function
create_plugin!(GoogleOAuth2Plugin);

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::Request;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    
    fn create_test_plugin() -> GoogleOAuth2Plugin {
        // Set test environment variables
        env::set_var("GOOGLE_CLIENT_ID", "test_client_id");
        env::set_var("GOOGLE_CLIENT_SECRET", "test_client_secret");
        
        let mut config = HashMap::new();
        config.insert("redirect_uri".to_string(), "http://localhost:3000/auth/google/callback".to_string());
        GoogleOAuth2Plugin::new(config)
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
    
    fn create_test_request(method: &str, uri: &str, headers: Vec<(&str, &str)>) -> PluginRequest {
        let mut builder = Request::builder()
            .method(method)
            .uri(uri);
            
        for (name, value) in headers {
            builder = builder.header(name, value);
        }
        
        let http_request = builder
            .body(Body::empty())
            .unwrap();
        
        // Extract just the path without query string
        let path = uri.split('?').next().unwrap_or(uri).to_string();
            
        PluginRequest {
            http_request: Box::new(http_request),
            path,
            canonical_path: None,
            metadata: HashMap::new(),
            body_cache: Arc::new(Mutex::new(None)),
        }
    }
    
    #[tokio::test]
    async fn test_login_redirect() {
        let plugin = create_test_plugin();
        let context = create_test_context();
        let mut request = create_test_request("GET", "/auth/google/login", vec![]);
        
        let response = plugin.handle_request(&mut request, &context).await.unwrap();
        let response = response.response;
        
        assert_eq!(response.status(), StatusCode::FOUND);
        
        let location = response.headers().get(LOCATION).unwrap().to_str().unwrap();
        assert!(location.contains("accounts.google.com/o/oauth2/v2/auth"));
        assert!(location.contains("client_id=test_client_id"));
        assert!(location.contains("redirect_uri="));
        assert!(location.contains("response_type=code"));
        // OAuth2 crate encodes scopes with + instead of %20
        assert!(location.contains("scope=email+profile"));
        
        let set_cookie = response.headers().get(SET_COOKIE).unwrap().to_str().unwrap();
        assert!(set_cookie.contains("oauth2_state="));
        assert!(set_cookie.contains("HttpOnly"));
        assert!(set_cookie.contains("SameSite=Lax"));
    }
    
    #[tokio::test]
    async fn test_callback_missing_code() {
        let plugin = create_test_plugin();
        let context = create_test_context();
        let mut request = create_test_request("GET", "/auth/google/callback", vec![]);
        
        let response = plugin.handle_request(&mut request, &context).await.unwrap();
        let response = response.response;
        
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        assert_eq!(body, "Missing authorization code");
    }
    
    #[tokio::test]
    async fn test_callback_invalid_state() {
        let plugin = create_test_plugin();
        let context = create_test_context();
        let mut request = create_test_request(
            "GET", 
            "/auth/google/callback?code=test_code&state=test_state",
            vec![("cookie", "oauth2_state=different_state")]
        );
        
        // Debug the request path
        println!("Request path: {}", request.path);
        let response = plugin.handle_request(&mut request, &context).await
            .expect("Plugin should handle /auth/google/callback");
        let response_status = response.response.status();
        let response_body = response.response.into_body();
        
        assert_eq!(response_status, StatusCode::FORBIDDEN);
        
        let body = hyper::body::to_bytes(response_body).await.unwrap();
        assert_eq!(body, "Invalid state parameter");
    }
    
    #[tokio::test]
    async fn test_logout() {
        let plugin = create_test_plugin();
        let context = create_test_context();
        let mut request = create_test_request(
            "POST",
            "/auth/logout",
            vec![("cookie", "session_id=test_session")]
        );
        
        let response = plugin.handle_request(&mut request, &context).await.unwrap();
        let response = response.response;
        
        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(response.headers().get(LOCATION).unwrap(), "/");
        
        let set_cookie = response.headers().get(SET_COOKIE).unwrap().to_str().unwrap();
        assert!(set_cookie.contains("session_id=; Max-Age=0"));
    }
    
    #[tokio::test]
    async fn test_status_unauthenticated() {
        let plugin = create_test_plugin();
        let context = create_test_context();
        let mut request = create_test_request("GET", "/auth/status", vec![]);
        
        let response = plugin.handle_request(&mut request, &context).await.unwrap();
        let response = response.response;
        
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(response.headers().get(CONTENT_TYPE).unwrap(), "application/json");
        
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["authenticated"], false);
    }
    
    #[tokio::test]
    async fn test_session_authentication() {
        let plugin = create_test_plugin();
        let context = create_test_context();
        
        // Add a test session
        let session_id = "test_session_id";
        let session_data = SessionData {
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            picture: None,
            created_at: std::time::SystemTime::now(),
        };
        plugin.sessions.write().await.insert(session_id.to_string(), session_data);
        
        // Test non-auth path with session
        let mut request = create_test_request(
            "GET",
            "/some/path",
            vec![("cookie", &format!("session_id={}", session_id))]
        );
        
        let response = plugin.handle_request(&mut request, &context).await;
        assert!(response.is_none()); // Plugin passes through
        assert_eq!(request.metadata.get("authenticated_user").unwrap(), "test@example.com");
    }
}