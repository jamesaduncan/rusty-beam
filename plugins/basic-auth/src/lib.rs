use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::fs;
use dom_query::Document;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;

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

// Plugin state
struct BasicAuthPlugin {
    users: HashMap<String, User>,
}

#[derive(Debug, Clone)]
struct User {
    username: String,
    password: String,
    roles: Vec<String>,
    encryption: String,
}

impl BasicAuthPlugin {
    fn new(auth_file_path: &str) -> Result<Self, String> {
        let users = Self::load_users_from_html(auth_file_path)?;
        Ok(BasicAuthPlugin { users })
    }

    fn load_users_from_html(file_path: &str) -> Result<HashMap<String, User>, String> {
        let mut users = HashMap::new();

        let content = fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read auth file {}: {}", file_path, e))?;

        let document = Document::from(content);
        let user_elements = document.select("[itemscope][itemtype='http://rustybeam.net/User']");

        let username_elements = document.select("[itemprop='username']");
        let password_elements = document.select("[itemprop='password']");
        let encryption_elements = document.select("[itemprop='encryption']");
        
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

    fn authenticate(&self, request: &CHttpRequest) -> CAuthResult {
        // For this simplified example, we'll assume Authorization header is passed in a specific way
        // In a real implementation, you'd parse all headers
        
        // Create a dummy unauthorized result for now
        // In practice, you'd extract the Authorization header from the request
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

// Global plugin instance (in a real implementation, you might want better state management)
static mut PLUGIN_INSTANCE: Option<BasicAuthPlugin> = None;
static mut CONFIG_STRINGS: Vec<CString> = Vec::new();

// Plugin FFI functions
#[no_mangle]
pub unsafe extern "C" fn plugin_create(
    config_keys: *const *const c_char,
    config_values: *const *const c_char,
    config_count: usize,
) -> *mut std::ffi::c_void {
    // Parse configuration
    let mut config = HashMap::new();
    
    for i in 0..config_count {
        let key_ptr = *config_keys.add(i);
        let value_ptr = *config_values.add(i);
        
        let key_cstr = CStr::from_ptr(key_ptr);
        let value_cstr = CStr::from_ptr(value_ptr);
        
        if let (Ok(key), Ok(value)) = (key_cstr.to_str(), value_cstr.to_str()) {
            config.insert(key.to_string(), value.to_string());
        }
    }
    
    // Get auth file from config
    let auth_file = match config.get("authFile") {
        Some(file) => file,
        None => {
            eprintln!("Missing authFile configuration");
            return std::ptr::null_mut();
        }
    };
    
    // Create plugin instance
    match BasicAuthPlugin::new(auth_file) {
        Ok(plugin) => {
            PLUGIN_INSTANCE = Some(plugin);
            // Return a non-null pointer to indicate success
            // In practice, you might return a pointer to the actual instance
            1 as *mut std::ffi::c_void
        }
        Err(e) => {
            eprintln!("Failed to create basic auth plugin: {}", e);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn plugin_destroy(_plugin: *mut std::ffi::c_void) {
    PLUGIN_INSTANCE = None;
    CONFIG_STRINGS.clear();
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
    b"basic-auth\0".as_ptr() as *const c_char
}

#[no_mangle]
pub unsafe extern "C" fn plugin_requires_auth(
    _plugin: *mut std::ffi::c_void,
    _path: *const c_char,
) -> c_int {
    1 // Always require authentication for now
}