use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use serde::{Deserialize, Serialize};
use url::Url;

// C-compatible structures (matching the server's definitions)
#[repr(C)]
pub struct CUserInfo {
    pub username: *const c_char,
    pub roles: *const *const c_char,
    pub roles_count: usize,
}

#[repr(C)]
pub struct CAuthResult {
    pub result_type: c_int, // 0 = Authorized, 1 = Unauthorized, 2 = Error
    pub user_info: CUserInfo,
    pub error_message: *const c_char,
}

#[repr(C)]
pub struct CHttpRequest {
    pub method: *const c_char,
    pub uri: *const c_char,
    pub headers: *const *const c_char,
    pub headers_count: usize,
    pub body: *const c_char,
    pub body_length: usize,
}

// OAuth2 structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleOAuth2Config {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub allowed_domains: Vec<String>,
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
    pub hd: Option<String>,
}

// Plugin state
struct GoogleOAuth2Plugin {
    config: GoogleOAuth2Config,
    client: reqwest::Client,
}

impl GoogleOAuth2Plugin {
    fn new(config: GoogleOAuth2Config) -> Result<Self, String> {
        let client = reqwest::Client::new();
        Ok(GoogleOAuth2Plugin { config, client })
    }

    fn get_authorization_url(&self, state: &str) -> String {
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

    async fn get_user_info(&self, access_token: &str) -> Result<GoogleUserInfo, String> {
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
            return true;
        }

        if let Some(domain) = email.split('@').nth(1) {
            self.config.allowed_domains.contains(&domain.to_string())
        } else {
            false
        }
    }

    fn authenticate(&self, request: &CHttpRequest) -> CAuthResult {
        // For this simplified example, we'll assume the Bearer token is passed somehow
        // In a real implementation, you'd parse the Authorization header from the request
        
        // Create a dummy unauthorized result for now
        CAuthResult {
            result_type: 1, // Unauthorized
            user_info: CUserInfo {
                username: std::ptr::null(),
                roles: std::ptr::null(),
                roles_count: 0,
            },
            error_message: std::ptr::null(),
        }
    }
}

// Global plugin instance
static mut PLUGIN_INSTANCE: Option<GoogleOAuth2Plugin> = None;

// Plugin FFI functions
#[no_mangle]
pub unsafe extern "C" fn plugin_create(
    config_keys: *const *const c_char,
    config_values: *const *const c_char,
    config_count: usize,
) -> *mut std::ffi::c_void {
    // Parse configuration
    let mut config_map = HashMap::new();
    
    for i in 0..config_count {
        let key_ptr = *config_keys.add(i);
        let value_ptr = *config_values.add(i);
        
        let key_cstr = CStr::from_ptr(key_ptr);
        let value_cstr = CStr::from_ptr(value_ptr);
        
        if let (Ok(key), Ok(value)) = (key_cstr.to_str(), value_cstr.to_str()) {
            config_map.insert(key.to_string(), value.to_string());
        }
    }
    
    // Extract required configuration
    let client_id = match config_map.get("clientId") {
        Some(id) => id.clone(),
        None => {
            eprintln!("Missing clientId configuration");
            return std::ptr::null_mut();
        }
    };
    
    let client_secret = match config_map.get("clientSecret") {
        Some(secret) => secret.clone(),
        None => {
            eprintln!("Missing clientSecret configuration");
            return std::ptr::null_mut();
        }
    };
    
    let redirect_uri = match config_map.get("redirectUri") {
        Some(uri) => uri.clone(),
        None => {
            eprintln!("Missing redirectUri configuration");
            return std::ptr::null_mut();
        }
    };
    
    let allowed_domains = config_map.get("allowedDomains")
        .map(|domains| domains.split(',').map(|d| d.trim().to_string()).collect())
        .unwrap_or_default();
    
    let config = GoogleOAuth2Config {
        client_id,
        client_secret,
        redirect_uri,
        allowed_domains,
    };
    
    // Create plugin instance
    match GoogleOAuth2Plugin::new(config) {
        Ok(plugin) => {
            PLUGIN_INSTANCE = Some(plugin);
            1 as *mut std::ffi::c_void
        }
        Err(e) => {
            eprintln!("Failed to create Google OAuth2 plugin: {}", e);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn plugin_destroy(_plugin: *mut std::ffi::c_void) {
    PLUGIN_INSTANCE = None;
}

#[no_mangle]
pub unsafe extern "C" fn plugin_authenticate(
    _plugin: *mut std::ffi::c_void,
    request: *const CHttpRequest,
) -> CAuthResult {
    if let Some(ref plugin) = PLUGIN_INSTANCE {
        plugin.authenticate(&*request)
    } else {
        CAuthResult {
            result_type: 2, // Error
            user_info: CUserInfo {
                username: std::ptr::null(),
                roles: std::ptr::null(),
                roles_count: 0,
            },
            error_message: b"Plugin not initialized\0".as_ptr() as *const c_char,
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn plugin_name() -> *const c_char {
    b"google-oauth2\0".as_ptr() as *const c_char
}

#[no_mangle]
pub unsafe extern "C" fn plugin_requires_auth(
    _plugin: *mut std::ffi::c_void,
    _path: *const c_char,
) -> c_int {
    1 // Always require authentication for now
}