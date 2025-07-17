use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, PluginResponse, create_plugin};
use async_trait::async_trait;
use hyper::{Body, Response, StatusCode, Method, header::{CONTENT_TYPE, LOCATION, SET_COOKIE, COOKIE}};
use std::collections::HashMap;
use oauth2::{
    AuthUrl, ClientId, ClientSecret, CsrfToken,
    RedirectUrl, Scope, TokenUrl, basic::BasicClient,
    AuthorizationCode,
};
use serde::{Deserialize, Serialize};
use cookie::{Cookie, SameSite, time};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use std::env;

// Constants
const DEFAULT_PLUGIN_NAME: &str = "oauth2";
const DEFAULT_PROVIDER: &str = "google";
const GITHUB_PROVIDER: &str = "github";
const GOOGLE_PROVIDER: &str = "google";

// OAuth2 Provider URLs
const GITHUB_AUTH_URL: &str = "https://github.com/login/oauth/authorize";
const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const GITHUB_USER_INFO_URL: &str = "https://api.github.com/user";
const GITHUB_EMAIL_URL: &str = "https://api.github.com/user/emails";

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USER_INFO_URL: &str = "https://www.googleapis.com/oauth2/v2/userinfo";

// OAuth2 Scopes
const GITHUB_SCOPE: &str = "user:email";
const GOOGLE_EMAIL_SCOPE: &str = "email";
const GOOGLE_PROFILE_SCOPE: &str = "profile";

// Cookie names
const SESSION_COOKIE_NAME: &str = "session_id";
const STATE_COOKIE_NAME: &str = "oauth2_state";
const RETURN_TO_COOKIE_NAME: &str = "oauth2_return_to";

// Paths
const AUTH_PATH_PREFIX: &str = "/auth/";
const LOGOUT_PATH: &str = "/auth/logout";
const USER_INFO_PATH: &str = "/auth/user";
const DEFAULT_REDIRECT_PATH: &str = "/";

// HTTP User-Agent
const USER_AGENT: &str = "Rusty-Beam-OAuth2";

// Error messages
const ERROR_MISSING_CLIENT_ID_ENV: &str = "Error: 'clientIdEnv' configuration parameter is required for OAuth2Plugin";
const ERROR_MISSING_CLIENT_SECRET_ENV: &str = "Error: 'clientSecretEnv' configuration parameter is required for OAuth2Plugin";
const ERROR_MISSING_REDIRECT_URI_ENV: &str = "Error: 'redirectUriEnv' configuration parameter is required for OAuth2Plugin";
const ERROR_OAUTH_NOT_CONFIGURED: &str = "OAuth2 not configured";
const ERROR_OAUTH_FAILED: &str = "OAuth2 authentication failed";
const ERROR_INVALID_STATE: &str = "Invalid state parameter";
const ERROR_MISSING_CODE: &str = "Missing authorization code";
const ERROR_USER_INFO_FAILED: &str = "Failed to fetch user information";

// Content types
const CONTENT_TYPE_JSON: &str = "application/json";
const CONTENT_TYPE_HTML: &str = "text/html";

// Role assignment
const DEFAULT_USER_ROLE: &str = "user";

/// OAuth2 Authentication Plugin
#[derive(Debug)]
pub struct OAuth2Plugin {
    name: String,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    client_id_env: String,
    client_secret_env: String,
    redirect_uri_env: String,
    login_path: String,
    provider: String,
    auth_url: String,
    token_url: String,
    user_info_url: String,
    sessions: Arc<RwLock<HashMap<String, SessionData>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionData {
    email: String,
    name: String,
    picture: Option<String>,
    provider: String,  // Add provider identification
    created_at: std::time::SystemTime,
}

#[derive(Debug, Deserialize)]
struct GoogleUserInfo {
    email: String,
    name: String,
    picture: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubUserInfo {
    email: Option<String>,
    name: Option<String>,
    login: String,
    avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubEmail {
    email: String,
    primary: bool,
    verified: bool,
}

impl OAuth2Plugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        let name = config.get("name").cloned().unwrap_or_else(|| DEFAULT_PLUGIN_NAME.to_string());
        
        // Get environment variable names from config - these are now required
        let client_id_env = config.get("clientIdEnv").cloned()
            .expect(ERROR_MISSING_CLIENT_ID_ENV);
        let client_secret_env = config.get("clientSecretEnv").cloned()
            .expect(ERROR_MISSING_CLIENT_SECRET_ENV);
        let redirect_uri_env = config.get("redirectUriEnv").cloned()
            .expect(ERROR_MISSING_REDIRECT_URI_ENV);
        
        // Read values from environment variables for security
        let client_id = env::var(&client_id_env).unwrap_or_default();
        let client_secret = env::var(&client_secret_env).unwrap_or_default();
        let redirect_uri = env::var(&redirect_uri_env).unwrap_or_default();
        
        // Get login path from config with default
        let login_path = config.get("loginPath").cloned()
            .unwrap_or_else(|| format!("/auth/{}/login", name));
        
        // Get provider from config with default based on name
        let provider = config.get("provider").cloned()
            .unwrap_or_else(|| {
                if name.contains(GITHUB_PROVIDER) {
                    GITHUB_PROVIDER.to_string()
                } else {
                    DEFAULT_PROVIDER.to_string()
                }
            });
        
        // Set OAuth2 URLs based on provider
        let (auth_url, token_url, user_info_url) = match provider.as_str() {
            GITHUB_PROVIDER => (
                GITHUB_AUTH_URL.to_string(),
                GITHUB_TOKEN_URL.to_string(),
                GITHUB_USER_INFO_URL.to_string(),
            ),
            GOOGLE_PROVIDER | _ => (
                GOOGLE_AUTH_URL.to_string(),
                GOOGLE_TOKEN_URL.to_string(),
                GOOGLE_USER_INFO_URL.to_string(),
            ),
        };
        
        Self {
            name,
            client_id,
            client_secret,
            redirect_uri,
            client_id_env,
            client_secret_env,
            redirect_uri_env,
            login_path,
            provider,
            auth_url,
            token_url,
            user_info_url,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    fn create_oauth_client(&self) -> Result<BasicClient, String> {
        let mut missing = Vec::new();
        if self.client_id.is_empty() {
            missing.push(format!("client_id (env var: {})", self.client_id_env));
        }
        if self.client_secret.is_empty() {
            missing.push(format!("client_secret (env var: {})", self.client_secret_env));
        }
        if self.redirect_uri.is_empty() {
            missing.push(format!("redirect_uri (env var: {})", self.redirect_uri_env));
        }
        
        if !missing.is_empty() {
            return Err(format!("OAuth2 configuration error: Missing {}. Please set the corresponding environment variables.", missing.join(", ")));
        }
        
        let auth_url = AuthUrl::new(self.auth_url.clone())
            .map_err(|e| format!("Invalid auth URL: {}", e))?;
        
        let token_url = TokenUrl::new(self.token_url.clone())
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
impl Plugin for OAuth2Plugin {
    async fn handle_request(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<PluginResponse> {
        // Check if user is authenticated via session for ALL requests
        if let Some(session_id) = self.get_session_id_from_request(request) {
            let sessions = self.sessions.read().await;
            if let Some(session_data) = sessions.get(&session_id) {
                // Only set authenticated_user metadata if this session belongs to our provider
                if session_data.provider == self.provider {
                    request.metadata.insert("authenticated_user".to_string(), session_data.email.clone());
                    // Automatically assign "user" role to any authenticated user
                    request.metadata.insert("authenticated_user_roles".to_string(), DEFAULT_USER_ROLE.to_string());
                    context.log_verbose(&format!("[OAuth2-{}] User {} authenticated via session with role: user", self.provider, session_data.email));
                } else {
                    context.log_verbose(&format!("[OAuth2-{}] Session belongs to different provider: {}", self.provider, session_data.provider));
                }
            }
        }
        
        // Only handle specific auth endpoints
        if !request.path.starts_with(AUTH_PATH_PREFIX) {
            return None;
        }
        
        let callback_path = self.get_callback_path();
        
        match request.http_request.method() {
            &Method::GET if request.path == self.login_path => Some(self.handle_login(request, context).await.into()),
            &Method::GET if request.path == callback_path => Some(self.handle_callback(request, context).await.into()),
            &Method::POST if request.path == LOGOUT_PATH => {
                // Only handle logout if we have a session for this user
                if let Some(session_id) = self.get_session_id_from_request(request) {
                    if self.sessions.read().await.contains_key(&session_id) {
                        context.log_verbose(&format!("[OAuth2-{}] Handling logout for session {}", self.provider, session_id));
                        Some(self.handle_logout(request, context).await.into())
                    } else {
                        context.log_verbose(&format!("[OAuth2-{}] No session found for logout, passing through", self.provider));
                        None
                    }
                } else {
                    // No session cookie, but we can still handle the logout to be helpful
                    context.log_verbose(&format!("[OAuth2-{}] No session cookie for logout, handling anyway", self.provider));
                    Some(self.handle_logout(request, context).await.into())
                }
            },
            &Method::GET if request.path == USER_INFO_PATH => {
                // Only respond if we have a valid session for this request
                if let Some(session_id) = self.get_session_id_from_request(request) {
                    if let Some(session_data) = self.sessions.read().await.get(&session_id) {
                        if session_data.provider == self.provider {
                            // We have a valid session - return user info
                            context.log_verbose(&format!("[OAuth2-{}] Returning user info for {}", self.provider, session_data.email));
                            Some(self.handle_user_info(session_data).await.into())
                        } else {
                            // Session belongs to different provider
                            context.log_verbose(&format!("[OAuth2-{}] Session belongs to different provider: {}", self.provider, session_data.provider));
                            None
                        }
                    } else {
                        // No session with this ID in our storage
                        context.log_verbose(&format!("[OAuth2-{}] No session found for id: {}", self.provider, session_id));
                        None
                    }
                } else {
                    // No session cookie at all
                    context.log_verbose(&format!("[OAuth2-{}] No session cookie in request", self.provider));
                    None
                }
            },
            _ => None,
        }
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

impl OAuth2Plugin {
    fn get_callback_path(&self) -> String {
        // Extract path from redirect URI
        if let Ok(url) = url::Url::parse(&self.redirect_uri) {
            url.path().to_string()
        } else {
            // Fallback if URI parsing fails
            format!("/auth/{}/callback", self.name)
        }
    }
    
    fn get_session_id_from_request(&self, request: &PluginRequest) -> Option<String> {
        // Parse cookies from request
        request.http_request.headers()
            .get(COOKIE)
            .and_then(|value| value.to_str().ok())
            .and_then(|cookies| {
                cookies.split(';')
                    .filter_map(|cookie| {
                        let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
                        if parts.len() == 2 && parts[0] == SESSION_COOKIE_NAME {
                            Some(parts[1].to_string())
                        } else {
                            None
                        }
                    })
                    .next()
            })
    }
    
    async fn handle_login(&self, request: &PluginRequest, context: &PluginContext) -> Response<Body> {
        context.log_verbose("[OAuth2] Handling login request");
        
        let client = match self.create_oauth_client() {
            Ok(client) => client,
            Err(e) => {
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(format!("OAuth2 configuration error: {}", e)))
                    .unwrap();
            }
        };
        
        // Generate CSRF token with provider-specific scopes
        let mut auth_builder = client.authorize_url(CsrfToken::new_random);
        
        // Add provider-specific scopes
        match self.provider.as_str() {
            GITHUB_PROVIDER => {
                auth_builder = auth_builder.add_scope(Scope::new(GITHUB_SCOPE.to_string()));
            }
            GOOGLE_PROVIDER | _ => {
                auth_builder = auth_builder
                    .add_scope(Scope::new(GOOGLE_EMAIL_SCOPE.to_string()))
                    .add_scope(Scope::new(GOOGLE_PROFILE_SCOPE.to_string()));
            }
        }
        
        let (auth_url, csrf_token) = auth_builder.url();
        
        // Store CSRF token in cookie
        let csrf_cookie = Cookie::build(STATE_COOKIE_NAME, csrf_token.secret())
            .http_only(true)
            .same_site(SameSite::Lax)
            .path("/")
            .finish();
        
        // Store return_to if provided, or clear it if not
        let mut headers = vec![
            (LOCATION, auth_url.to_string()),
            (SET_COOKIE, csrf_cookie.to_string()),
        ];
        
        if let Some(return_to) = request.http_request.uri().query()
            .and_then(|q| url::form_urlencoded::parse(q.as_bytes())
                .find(|(k, _)| k == "return_to")
                .map(|(_, v)| v.to_string()))
        {
            context.log_verbose(&format!("[OAuth2-{}] Login: Setting return_to cookie to {}", self.provider, return_to));
            let return_cookie = Cookie::build(RETURN_TO_COOKIE_NAME, return_to)
                .http_only(true)
                .same_site(SameSite::Lax)
                .path("/")
                .finish();
            headers.push((SET_COOKIE, return_cookie.to_string()));
        } else {
            context.log_verbose(&format!("[OAuth2-{}] Login: No return_to specified, clearing cookie", self.provider));
            // Clear any existing return_to cookie if no return_to is specified
            let clear_cookie = Cookie::build(RETURN_TO_COOKIE_NAME, "")
                .http_only(true)
                .same_site(SameSite::Lax)
                .path("/")
                .max_age(time::Duration::seconds(0))
                .finish();
            headers.push((SET_COOKIE, clear_cookie.to_string()));
        }
        
        let mut response = Response::builder()
            .status(StatusCode::FOUND);
        
        for (name, value) in headers {
            response = response.header(name, value);
        }
        
        response.body(Body::empty()).unwrap()
    }
    
    /// Extract callback parameters from query string
    fn extract_callback_parameters(&self, request: &PluginRequest) -> Result<(AuthorizationCode, String), Response<Body>> {
        let query = request.http_request.uri().query().unwrap_or("");
        let params: HashMap<String, String> = url::form_urlencoded::parse(query.as_bytes())
            .into_owned()
            .collect();
        
        let code = match params.get("code") {
            Some(code) => AuthorizationCode::new(code.clone()),
            None => {
                return Err(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from(ERROR_MISSING_CODE))
                    .unwrap());
            }
        };
        
        let state = match params.get("state") {
            Some(state) => state.clone(),
            None => {
                return Err(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("Missing state parameter"))
                    .unwrap());
            }
        };
        
        Ok((code, state))
    }
    
    /// Validate CSRF token
    fn validate_csrf_token(&self, request: &PluginRequest, state: &str) -> Result<(), Response<Body>> {
        let stored_state = self.get_cookie_value(request, STATE_COOKIE_NAME);
        if stored_state.as_deref() != Some(state) {
            return Err(Response::builder()
                .status(StatusCode::FORBIDDEN)
                .body(Body::from(if stored_state.is_none() {
                    "Missing state cookie"
                } else {
                    ERROR_INVALID_STATE
                }))
                .unwrap());
        }
        Ok(())
    }
    
    /// Create and validate OAuth client
    fn create_and_validate_oauth_client(&self, context: &PluginContext) -> Result<BasicClient, Response<Body>> {
        match self.create_oauth_client() {
            Ok(client) => Ok(client),
            Err(e) => {
                context.log_verbose(&format!("[OAuth2] Failed to create client: {}", e));
                Err(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(format!("OAuth2 configuration error: {}", e)))
                    .unwrap())
            }
        }
    }
    
    /// Exchange authorization code for access token
    async fn exchange_authorization_code(&self, code: &str, context: &PluginContext) -> Result<String, Response<Body>> {
        match self.exchange_code_for_token(code, context).await {
            Ok(token) => Ok(token),
            Err(e) => {
                context.log_verbose(&format!("[OAuth2-{}] Token exchange failed: {}", self.provider, e));
                Err(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(format!("Failed to exchange authorization code: {}", e)))
                    .unwrap())
            }
        }
    }
    
    /// Retrieve user information using access token
    async fn retrieve_user_information(&self, access_token: &str, context: &PluginContext) -> Result<SessionData, Response<Body>> {
        match self.fetch_user_info(access_token, context).await {
            Ok(data) => Ok(data),
            Err(e) => {
                context.log_verbose(&format!("[OAuth2] Failed to fetch user info: {}", e));
                Err(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(ERROR_USER_INFO_FAILED))
                    .unwrap())
            }
        }
    }
    
    /// Create user session
    async fn create_user_session(&self, session_data: SessionData, context: &PluginContext) -> String {
        let session_id = Uuid::new_v4().to_string();
        context.log_verbose(&format!("[OAuth2] Created session for user: {}", session_data.email));
        
        // Store session
        self.sessions.write().await.insert(session_id.clone(), session_data);
        session_id
    }
    
    /// Build callback response with session cookie and cleanup
    fn build_callback_response(&self, request: &PluginRequest, context: &PluginContext, session_id: String) -> Response<Body> {
        // Create session cookie
        let session_cookie = self.create_session_cookie(SESSION_COOKIE_NAME, &session_id, request);
        
        // Get return_to URL
        let return_to = self.get_cookie_value(request, RETURN_TO_COOKIE_NAME);
        context.log_verbose(&format!("[OAuth2-{}] Callback: return_to cookie value = {:?}", self.provider, return_to));
        let return_to = return_to.unwrap_or_else(|| DEFAULT_REDIRECT_PATH.to_string());
        context.log_verbose(&format!("[OAuth2-{}] Callback: redirecting to {}", self.provider, return_to));
        
        Response::builder()
            .status(StatusCode::FOUND)
            .header(LOCATION, return_to)
            .header(SET_COOKIE, session_cookie.to_string())
            .header(SET_COOKIE, self.create_expired_cookie(STATE_COOKIE_NAME).to_string())
            .header(SET_COOKIE, self.create_expired_cookie(RETURN_TO_COOKIE_NAME).to_string())
            .body(Body::empty())
            .unwrap()
    }
    
    async fn handle_callback(&self, request: &PluginRequest, context: &PluginContext) -> Response<Body> {
        context.log_verbose(&format!("[OAuth2-{}] Handling callback request", self.provider));
        
        // 1. Extract and validate parameters
        let (code, state) = match self.extract_callback_parameters(request) {
            Ok(params) => params,
            Err(response) => return response,
        };
        
        // 2. Validate CSRF token
        if let Err(response) = self.validate_csrf_token(request, &state) {
            return response;
        }
        
        // 3. Create OAuth client
        let _client = match self.create_and_validate_oauth_client(context) {
            Ok(client) => client,
            Err(response) => return response,
        };
        
        // 4. Exchange code for token
        let access_token = match self.exchange_authorization_code(code.secret(), context).await {
            Ok(token) => token,
            Err(response) => return response,
        };
        
        // 5. Get user information
        let session_data = match self.retrieve_user_information(&access_token, context).await {
            Ok(data) => data,
            Err(response) => return response,
        };
        
        // 6. Create session
        let session_id = self.create_user_session(session_data, context).await;
        
        // 7. Build response
        self.build_callback_response(request, context, session_id)
    }
    
    async fn handle_logout(&self, request: &PluginRequest, context: &PluginContext) -> Response<Body> {
        context.log_verbose("[OAuth2] Handling logout request");
        
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
    
    async fn handle_user_info(&self, session_data: &SessionData) -> Response<Body> {
        // Return HTML with microdata about the authenticated user
        let html = format!(r#"<!DOCTYPE html>
<html>
<head>
    <title>User Information</title>
    <meta charset="UTF-8">
</head>
<body>
    <div itemscope itemtype="https://schema.org/Person">
        <span itemprop="email">{}</span>
        <span itemprop="name">{}</span>{}
    </div>
</body>
</html>"#,
            html_escape(&session_data.email),
            html_escape(&session_data.name),
            if let Some(picture) = &session_data.picture {
                format!("\n        <link itemprop=\"image\" href=\"{}\">", html_escape(picture))
            } else {
                String::new()
            }
        );
        
        Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(html))
            .unwrap()
    }
    
    /// Creates a secure HTTP-only cookie with standard settings
    fn create_secure_cookie<'a>(&self, name: &'a str, value: &'a str) -> Cookie<'a> {
        Cookie::build(name, value)
            .http_only(true)
            .same_site(SameSite::Lax)
            .path("/")
            .finish()
    }
    
    /// Creates a secure session cookie with HTTPS detection
    fn create_session_cookie<'a>(&self, name: &'a str, value: &'a str, request: &PluginRequest) -> Cookie<'a> {
        Cookie::build(name, value)
            .http_only(true)
            .same_site(SameSite::Lax)
            .secure(request.http_request.uri().scheme_str() == Some("https"))
            .path("/")
            .finish()
    }
    
    /// Creates a cookie that immediately expires (for clearing)
    fn create_expired_cookie<'a>(&self, name: &'a str) -> Cookie<'a> {
        Cookie::build(name, "")
            .http_only(true)
            .same_site(SameSite::Lax)
            .path("/")
            .max_age(time::Duration::seconds(0))
            .finish()
    }
    
    /// Extracts cookie value from request headers
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
    
    async fn exchange_code_for_token(&self, code: &str, context: &PluginContext) -> Result<String, String> {
        context.log_verbose(&format!("[OAuth2] Exchanging code for token with {}", self.provider));
        
        // Create form data
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &self.redirect_uri),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
        ];
        
        let body = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(&params)
            .finish();
        
        let token_url = self.token_url.clone();
        
        context.log_verbose(&format!("[OAuth2-{}] Making synchronous HTTP request for token exchange", self.provider));
        
        // Use block_in_place to run blocking code without needing a runtime handle
        let response_result = tokio::task::block_in_place(move || {
            ureq::post(&token_url)
                .set("Content-Type", "application/x-www-form-urlencoded")
                .set("Accept", "application/json")
                .send_string(&body)
        });
        
        context.log_verbose(&format!("[OAuth2-{}] HTTP request completed", self.provider));
        
        let response = response_result
            .map_err(|e| format!("Token exchange failed: {}", e))?;
        
        let token_response: serde_json::Value = response.into_json()
            .map_err(|e| format!("Failed to parse token response: {}", e))?;
        
        token_response.get("access_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| "No access token in response".to_string())
    }
    
    /// Fetches user information from OAuth2 provider
    async fn fetch_user_info(&self, access_token: &str, context: &PluginContext) -> Result<SessionData, String> {
        match self.provider.as_str() {
            "github" => self.fetch_github_user_info(access_token, context).await,
            "google" | _ => self.fetch_google_user_info(access_token).await,
        }
    }
    
    /// Makes an authenticated HTTP GET request to an OAuth2 API endpoint
    fn make_oauth_request(&self, url: &str, access_token: &str, include_user_agent: bool) -> Result<ureq::Response, String> {
        let url = url.to_string();
        let access_token = access_token.to_string();
        
        tokio::task::block_in_place(move || {
            let mut request = ureq::get(&url)
                .set("Authorization", &format!("Bearer {}", access_token))
                .set("Accept", "application/json");
            
            if include_user_agent {
                request = request.set("User-Agent", USER_AGENT);
            }
            
            request.call()
        })
        .map_err(|e| format!("HTTP request failed: {}", e))
    }
    
    /// Fetches user information from GitHub
    async fn fetch_github_user_info(&self, access_token: &str, context: &PluginContext) -> Result<SessionData, String> {
        // Get basic user info
        let user_info_response = self.make_oauth_request(&self.user_info_url, access_token, true)?;
        let user_info: GitHubUserInfo = user_info_response.into_json()
            .map_err(|e| format!("Failed to parse user info: {}", e))?;
        
        // Get email (GitHub might not include it in user endpoint)
        let email = match user_info.email {
            Some(email) => email,
            None => self.fetch_github_primary_email(access_token, context).await?,
        };
        
        Ok(SessionData {
            email,
            name: user_info.name.unwrap_or(user_info.login),
            picture: user_info.avatar_url,
            provider: self.provider.clone(),
            created_at: std::time::SystemTime::now(),
        })
    }
    
    /// Fetches primary verified email from GitHub emails endpoint
    async fn fetch_github_primary_email(&self, access_token: &str, context: &PluginContext) -> Result<String, String> {
        context.log_verbose("[OAuth2] Fetching email from GitHub emails endpoint");
        
        let emails_response = self.make_oauth_request(GITHUB_EMAIL_URL, access_token, true)?;
        let emails: Vec<GitHubEmail> = emails_response.into_json()
            .map_err(|e| format!("Failed to parse emails: {}", e))?;
        
        emails.iter()
            .find(|e| e.primary && e.verified)
            .or_else(|| emails.iter().find(|e| e.verified))
            .map(|e| e.email.clone())
            .ok_or_else(|| "No verified email found".to_string())
    }
    
    /// Fetches user information from Google
    async fn fetch_google_user_info(&self, access_token: &str) -> Result<SessionData, String> {
        let user_info_response = self.make_oauth_request(&self.user_info_url, access_token, false)?;
        let user_info: GoogleUserInfo = user_info_response.into_json()
            .map_err(|e| format!("Failed to parse user info: {}", e))?;
        
        Ok(SessionData {
            email: user_info.email,
            name: user_info.name,
            picture: user_info.picture,
            provider: self.provider.clone(),
            created_at: std::time::SystemTime::now(),
        })
    }
}

// Export the plugin creation function
// Helper function to escape HTML
fn html_escape(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&#39;".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

create_plugin!(OAuth2Plugin);

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::Request;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    
    fn create_test_plugin() -> OAuth2Plugin {
        // Set test environment variables
        env::set_var("TEST_CLIENT_ID", "test_client_id");
        env::set_var("TEST_CLIENT_SECRET", "test_client_secret");
        env::set_var("TEST_REDIRECT_URI", "http://localhost:3000/auth/google/callback");
        
        let mut config = HashMap::new();
        config.insert("name".to_string(), "google".to_string());
        config.insert("clientIdEnv".to_string(), "TEST_CLIENT_ID".to_string());
        config.insert("clientSecretEnv".to_string(), "TEST_CLIENT_SECRET".to_string());
        config.insert("redirectUriEnv".to_string(), "TEST_REDIRECT_URI".to_string());
        OAuth2Plugin::new(config)
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
        assert!(location.contains(GOOGLE_AUTH_URL));
        assert!(location.contains("client_id=test_client_id"));
        assert!(location.contains("redirect_uri="));
        assert!(location.contains("response_type=code"));
        // OAuth2 crate encodes scopes with + instead of %20
        assert!(location.contains("scope=email+profile"));
        
        let set_cookie = response.headers().get(SET_COOKIE).unwrap().to_str().unwrap();
        assert!(set_cookie.contains(&format!("{}=", STATE_COOKIE_NAME)));
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
        assert_eq!(body, ERROR_MISSING_CODE);
    }
    
    #[tokio::test]
    async fn test_callback_invalid_state() {
        let plugin = create_test_plugin();
        let context = create_test_context();
        let mut request = create_test_request(
            "GET", 
            "/auth/google/callback?code=test_code&state=test_state",
            vec![("cookie", &format!("{}=different_state", STATE_COOKIE_NAME))]
        );
        
        // Debug the request path
        println!("Request path: {}", request.path);
        let response = plugin.handle_request(&mut request, &context).await
            .expect("Plugin should handle /auth/google/callback");
        let response_status = response.response.status();
        let response_body = response.response.into_body();
        
        assert_eq!(response_status, StatusCode::FORBIDDEN);
        
        let body = hyper::body::to_bytes(response_body).await.unwrap();
        assert_eq!(body, ERROR_INVALID_STATE);
    }
    
    #[tokio::test]
    async fn test_logout() {
        let plugin = create_test_plugin();
        let context = create_test_context();
        
        // Add a test session to the plugin
        let session_id = "test_session";
        let session_data = SessionData {
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            picture: Some("https://example.com/picture.jpg".to_string()),
            provider: plugin.provider.clone(),
            created_at: std::time::SystemTime::now(),
        };
        plugin.sessions.write().await.insert(session_id.to_string(), session_data);
        
        let mut request = create_test_request(
            "POST",
            "/auth/logout",
            vec![("cookie", &format!("{}={}", SESSION_COOKIE_NAME, session_id))]
        );
        
        let response = plugin.handle_request(&mut request, &context).await.unwrap();
        let response = response.response;
        
        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(response.headers().get(LOCATION).unwrap(), DEFAULT_REDIRECT_PATH);
        
        let set_cookie = response.headers().get(SET_COOKIE).unwrap().to_str().unwrap();
        assert!(set_cookie.contains(&format!("{}=; Max-Age=0", SESSION_COOKIE_NAME)));
    }
    
    #[tokio::test]
    async fn test_user_no_session() {
        let plugin = create_test_plugin();
        let context = create_test_context();
        let mut request = create_test_request("GET", "/auth/user", vec![]);
        
        // Should return None (pass through) when no session
        let response = plugin.handle_request(&mut request, &context).await;
        assert!(response.is_none());
    }
    
    #[tokio::test]
    async fn test_user_with_session() {
        let plugin = create_test_plugin();
        let context = create_test_context();
        
        // Add a test session
        let session_id = "test_session_id";
        let session_data = SessionData {
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            picture: Some("https://example.com/picture.jpg".to_string()),
            provider: plugin.provider.clone(),
            created_at: std::time::SystemTime::now(),
        };
        plugin.sessions.write().await.insert(session_id.to_string(), session_data);
        
        // Test /auth/user with valid session
        let mut request = create_test_request(
            "GET",
            "/auth/user",
            vec![("cookie", &format!("{}={}", SESSION_COOKIE_NAME, session_id))]
        );
        
        let response = plugin.handle_request(&mut request, &context).await.unwrap();
        let response = response.response;
        
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers().get(CONTENT_TYPE).unwrap(), "text/html; charset=utf-8");
        
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        
        // Check HTML contains user info with microdata
        assert!(body_str.contains(r#"itemtype="https://schema.org/Person""#));
        assert!(body_str.contains(r#"<span itemprop="email">test@example.com</span>"#));
        assert!(body_str.contains(r#"<span itemprop="name">Test User</span>"#));
        assert!(body_str.contains(r#"<link itemprop="image" href="https://example.com/picture.jpg">"#));
    }
    
    #[tokio::test]
    async fn test_session_authentication_metadata() {
        let plugin = create_test_plugin();
        let context = create_test_context();
        
        // Add a test session
        let session_id = "test_session_id";
        let session_data = SessionData {
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            picture: None,
            provider: plugin.provider.clone(),
            created_at: std::time::SystemTime::now(),
        };
        plugin.sessions.write().await.insert(session_id.to_string(), session_data);
        
        // Test non-auth path with session - should set metadata
        let mut request = create_test_request(
            "GET",
            "/some/path",
            vec![("cookie", &format!("session_id={}", session_id))]
        );
        
        let response = plugin.handle_request(&mut request, &context).await;
        assert!(response.is_none()); // Plugin passes through
        assert_eq!(request.metadata.get("authenticated_user").unwrap(), "test@example.com");
        assert_eq!(request.metadata.get("authenticated_user_roles").unwrap(), "user");
    }
}