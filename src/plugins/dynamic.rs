use libloading::{Library, Symbol};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use hyper::{Body, Request};
use async_trait::async_trait;
use super::{AuthPlugin, AuthResult, UserInfo, AuthzPlugin, AuthzResult, AuthzRequest};

// C-compatible structures for FFI
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

#[repr(C)]
pub struct CAuthzRequest {
    pub user: CUserInfo,
    pub resource: *const c_char,
    pub method: *const c_char,
}

#[repr(C)]
pub struct CAuthzResult {
    pub result_type: c_int, // 0 = Authorized, 1 = Denied, 2 = Error
    pub error_message: *const c_char,
}

// Plugin function signatures for FFI
pub type PluginCreateFn = unsafe extern "C" fn(
    config_keys: *const *const c_char,
    config_values: *const *const c_char,
    config_count: usize,
) -> *mut std::ffi::c_void;

pub type PluginDestroyFn = unsafe extern "C" fn(plugin: *mut std::ffi::c_void);

pub type PluginAuthenticateFn = unsafe extern "C" fn(
    plugin: *mut std::ffi::c_void,
    request: *const CHttpRequest,
) -> CAuthResult;

pub type PluginNameFn = unsafe extern "C" fn() -> *const c_char;

pub type PluginRequiresAuthFn = unsafe extern "C" fn(
    plugin: *mut std::ffi::c_void,
    path: *const c_char,
) -> c_int;

pub type PluginAuthorizeFn = unsafe extern "C" fn(
    plugin: *mut std::ffi::c_void,
    request: *const CAuthzRequest,
) -> CAuthzResult;

pub type PluginHandlesResourceFn = unsafe extern "C" fn(
    plugin: *mut std::ffi::c_void,
    resource: *const c_char,
) -> c_int;

// Dynamic plugin wrapper
#[derive(Debug)]
pub struct DynamicPlugin {
    _library: Library, // Keep library alive
    plugin_instance: *mut std::ffi::c_void,
    destroy_fn: PluginDestroyFn,
    authenticate_fn: PluginAuthenticateFn,
    #[allow(dead_code)] // Used for plugin name caching during initialization
    name_fn: PluginNameFn,
    requires_auth_fn: PluginRequiresAuthFn,
    name_cache: String,
}

impl DynamicPlugin {
    pub fn load(library_path: &str, config: &HashMap<String, String>) -> Result<Self, String> {
        unsafe {
            let library = Library::new(library_path)
                .map_err(|e| format!("Failed to load plugin library {}: {}", library_path, e))?;

            // Load required functions
            let create_fn: Symbol<PluginCreateFn> = library
                .get(b"plugin_create")
                .map_err(|e| format!("Failed to find plugin_create function: {}", e))?;

            let destroy_fn: Symbol<PluginDestroyFn> = library
                .get(b"plugin_destroy")
                .map_err(|e| format!("Failed to find plugin_destroy function: {}", e))?;

            let authenticate_fn: Symbol<PluginAuthenticateFn> = library
                .get(b"plugin_authenticate")
                .map_err(|e| format!("Failed to find plugin_authenticate function: {}", e))?;

            let name_fn: Symbol<PluginNameFn> = library
                .get(b"plugin_name")
                .map_err(|e| format!("Failed to find plugin_name function: {}", e))?;

            let requires_auth_fn: Symbol<PluginRequiresAuthFn> = library
                .get(b"plugin_requires_auth")
                .map_err(|e| format!("Failed to find plugin_requires_auth function: {}", e))?;

            // Convert config to C-compatible format
            let mut keys: Vec<CString> = Vec::new();
            let mut values: Vec<CString> = Vec::new();
            let mut key_ptrs: Vec<*const c_char> = Vec::new();
            let mut value_ptrs: Vec<*const c_char> = Vec::new();

            for (key, value) in config {
                let key_cstring = CString::new(key.as_str()).unwrap();
                let value_cstring = CString::new(value.as_str()).unwrap();
                key_ptrs.push(key_cstring.as_ptr());
                value_ptrs.push(value_cstring.as_ptr());
                keys.push(key_cstring);
                values.push(value_cstring);
            }

            // Store function pointers
            let destroy_fn_ptr = *destroy_fn;
            let authenticate_fn_ptr = *authenticate_fn;
            let name_fn_ptr = *name_fn;
            let requires_auth_fn_ptr = *requires_auth_fn;

            // Create plugin instance
            let plugin_instance = create_fn(
                key_ptrs.as_ptr(),
                value_ptrs.as_ptr(),
                config.len(),
            );

            if plugin_instance.is_null() {
                return Err("Plugin creation failed".to_string());
            }

            // Cache the plugin name
            let name_ptr = name_fn_ptr();
            let name_cstr = CStr::from_ptr(name_ptr);
            let name_cache = name_cstr.to_string_lossy().into_owned();

            Ok(DynamicPlugin {
                _library: library,
                plugin_instance,
                destroy_fn: destroy_fn_ptr,
                authenticate_fn: authenticate_fn_ptr,
                name_fn: name_fn_ptr,
                requires_auth_fn: requires_auth_fn_ptr,
                name_cache,
            })
        }
    }

    fn convert_request_to_c(&self, req: &Request<Body>) -> Result<(CHttpRequest, Vec<CString>), String> {
        // Convert method
        let method = CString::new(req.method().as_str())
            .map_err(|_| "Invalid method string")?;
        
        // Convert URI
        let uri = CString::new(req.uri().to_string())
            .map_err(|_| "Invalid URI string")?;

        // Convert headers
        let mut header_strings = Vec::new();
        let mut header_ptrs = Vec::new();
        
        for (name, value) in req.headers() {
            let header_str = format!("{}: {}", name.as_str(), value.to_str().unwrap_or(""));
            let header_cstring = CString::new(header_str)
                .map_err(|_| "Invalid header string")?;
            header_ptrs.push(header_cstring.as_ptr());
            header_strings.push(header_cstring);
        }

        let empty_body = CString::new("").unwrap();

        let c_request = CHttpRequest {
            method: method.as_ptr(),
            uri: uri.as_ptr(),
            headers: header_ptrs.as_ptr(),
            headers_count: header_ptrs.len(),
            body: empty_body.as_ptr(),
            body_length: 0,
        };

        // Return both the request and the strings to keep them alive
        header_strings.push(method);
        header_strings.push(uri);
        header_strings.push(empty_body);

        Ok((c_request, header_strings))
    }

    fn convert_c_result(&self, c_result: CAuthResult) -> AuthResult {
        unsafe {
            match c_result.result_type {
                0 => {
                    // Authorized
                    let username_cstr = CStr::from_ptr(c_result.user_info.username);
                    let username = username_cstr.to_string_lossy().into_owned();
                    
                    let mut roles = Vec::new();
                    for i in 0..c_result.user_info.roles_count {
                        let role_ptr = *c_result.user_info.roles.add(i);
                        let role_cstr = CStr::from_ptr(role_ptr);
                        roles.push(role_cstr.to_string_lossy().into_owned());
                    }
                    
                    AuthResult::Authorized(UserInfo { username, roles })
                }
                1 => AuthResult::Unauthorized,
                2 => {
                    let error_cstr = CStr::from_ptr(c_result.error_message);
                    let error_msg = error_cstr.to_string_lossy().into_owned();
                    AuthResult::Error(error_msg)
                }
                _ => AuthResult::Error("Invalid result type from plugin".to_string()),
            }
        }
    }
}

impl Drop for DynamicPlugin {
    fn drop(&mut self) {
        unsafe {
            (self.destroy_fn)(self.plugin_instance);
        }
    }
}

#[async_trait]
impl AuthPlugin for DynamicPlugin {
    async fn authenticate(&self, req: &Request<Body>) -> AuthResult {
        // Convert request to C format
        let (c_request, _header_strings) = match self.convert_request_to_c(req) {
            Ok(result) => result,
            Err(e) => return AuthResult::Error(format!("Failed to convert request: {}", e)),
        };

        // Call plugin function
        let c_result = unsafe {
            (self.authenticate_fn)(self.plugin_instance, &c_request)
        };

        // Convert result back
        self.convert_c_result(c_result)
    }

    fn name(&self) -> &'static str {
        // We can't return a reference to our cached string as a static str,
        // so we'll need to leak it. In a real implementation, you might
        // want to handle this differently.
        Box::leak(self.name_cache.clone().into_boxed_str())
    }

    fn requires_authentication(&self, path: &str) -> bool {
        let path_cstring = CString::new(path).unwrap();
        let result = unsafe {
            (self.requires_auth_fn)(self.plugin_instance, path_cstring.as_ptr())
        };
        result != 0
    }
}

unsafe impl Send for DynamicPlugin {}
unsafe impl Sync for DynamicPlugin {}

// Dynamic authorization plugin wrapper
#[derive(Debug)]
pub struct DynamicAuthzPlugin {
    _library: Library, // Keep library alive
    plugin_instance: *mut std::ffi::c_void,
    destroy_fn: PluginDestroyFn,
    authorize_fn: PluginAuthorizeFn,
    #[allow(dead_code)] // Used for plugin name caching during initialization
    name_fn: PluginNameFn,
    handles_resource_fn: PluginHandlesResourceFn,
    name_cache: String,
}

impl DynamicAuthzPlugin {
    pub fn load(library_path: &str, config: &HashMap<String, String>) -> Result<Self, String> {
        unsafe {
            let library = Library::new(library_path)
                .map_err(|e| format!("Failed to load authz plugin library {}: {}", library_path, e))?;

            // Load required functions
            let create_fn: Symbol<PluginCreateFn> = library
                .get(b"authz_plugin_create")
                .map_err(|e| format!("Failed to find authz_plugin_create function: {}", e))?;

            let destroy_fn: Symbol<PluginDestroyFn> = library
                .get(b"authz_plugin_destroy")
                .map_err(|e| format!("Failed to find authz_plugin_destroy function: {}", e))?;

            let authorize_fn: Symbol<PluginAuthorizeFn> = library
                .get(b"authz_plugin_authorize")
                .map_err(|e| format!("Failed to find authz_plugin_authorize function: {}", e))?;

            let name_fn: Symbol<PluginNameFn> = library
                .get(b"authz_plugin_name")
                .map_err(|e| format!("Failed to find authz_plugin_name function: {}", e))?;

            let handles_resource_fn: Symbol<PluginHandlesResourceFn> = library
                .get(b"authz_plugin_handles_resource")
                .map_err(|e| format!("Failed to find authz_plugin_handles_resource function: {}", e))?;

            // Convert config to C-compatible format
            let mut keys: Vec<CString> = Vec::new();
            let mut values: Vec<CString> = Vec::new();
            let mut key_ptrs: Vec<*const c_char> = Vec::new();
            let mut value_ptrs: Vec<*const c_char> = Vec::new();

            for (key, value) in config {
                let key_cstring = CString::new(key.as_str()).unwrap();
                let value_cstring = CString::new(value.as_str()).unwrap();
                key_ptrs.push(key_cstring.as_ptr());
                value_ptrs.push(value_cstring.as_ptr());
                keys.push(key_cstring);
                values.push(value_cstring);
            }

            // Store function pointers
            let destroy_fn_ptr = *destroy_fn;
            let authorize_fn_ptr = *authorize_fn;
            let name_fn_ptr = *name_fn;
            let handles_resource_fn_ptr = *handles_resource_fn;

            // Create plugin instance
            let plugin_instance = create_fn(
                key_ptrs.as_ptr(),
                value_ptrs.as_ptr(),
                config.len(),
            );

            if plugin_instance.is_null() {
                return Err("Authorization plugin creation failed".to_string());
            }

            // Cache the plugin name
            let name_ptr = name_fn_ptr();
            let name_cstr = CStr::from_ptr(name_ptr);
            let name_cache = name_cstr.to_string_lossy().into_owned();

            Ok(DynamicAuthzPlugin {
                _library: library,
                plugin_instance,
                destroy_fn: destroy_fn_ptr,
                authorize_fn: authorize_fn_ptr,
                name_fn: name_fn_ptr,
                handles_resource_fn: handles_resource_fn_ptr,
                name_cache,
            })
        }
    }

    fn convert_authz_request_to_c(&self, request: &AuthzRequest) -> Result<(CAuthzRequest, Vec<CString>), String> {
        // Convert resource and method
        let resource = CString::new(request.resource.as_str())
            .map_err(|_| "Invalid resource string")?;
        let method = CString::new(request.method.as_str())
            .map_err(|_| "Invalid method string")?;

        // Convert user info
        let username = CString::new(request.user.username.as_str())
            .map_err(|_| "Invalid username string")?;

        // Convert roles
        let mut role_strings = Vec::new();
        let mut role_ptrs = Vec::new();
        
        for role in &request.user.roles {
            let role_cstring = CString::new(role.as_str())
                .map_err(|_| "Invalid role string")?;
            role_ptrs.push(role_cstring.as_ptr());
            role_strings.push(role_cstring);
        }

        let c_user_info = CUserInfo {
            username: username.as_ptr(),
            roles: role_ptrs.as_ptr(),
            roles_count: role_ptrs.len(),
        };

        let c_request = CAuthzRequest {
            user: c_user_info,
            resource: resource.as_ptr(),
            method: method.as_ptr(),
        };

        // Return both the request and the strings to keep them alive
        let mut all_strings = role_strings;
        all_strings.push(username);
        all_strings.push(resource);
        all_strings.push(method);

        Ok((c_request, all_strings))
    }

    fn convert_c_authz_result(&self, c_result: CAuthzResult) -> AuthzResult {
        unsafe {
            match c_result.result_type {
                0 => AuthzResult::Authorized,
                1 => AuthzResult::Denied,
                2 => {
                    let error_cstr = CStr::from_ptr(c_result.error_message);
                    let error_msg = error_cstr.to_string_lossy().into_owned();
                    AuthzResult::Error(error_msg)
                }
                _ => AuthzResult::Error("Invalid result type from authorization plugin".to_string()),
            }
        }
    }
}

impl Drop for DynamicAuthzPlugin {
    fn drop(&mut self) {
        unsafe {
            (self.destroy_fn)(self.plugin_instance);
        }
    }
}

#[async_trait]
impl AuthzPlugin for DynamicAuthzPlugin {
    async fn authorize(&self, request: &AuthzRequest) -> AuthzResult {
        // Convert request to C format
        let (c_request, _strings) = match self.convert_authz_request_to_c(request) {
            Ok(result) => result,
            Err(e) => return AuthzResult::Error(format!("Failed to convert authz request: {}", e)),
        };

        // Call plugin function
        let c_result = unsafe {
            (self.authorize_fn)(self.plugin_instance, &c_request)
        };

        // Convert result back
        self.convert_c_authz_result(c_result)
    }

    fn name(&self) -> &'static str {
        // We can't return a reference to our cached string as a static str,
        // so we'll need to leak it. In a real implementation, you might
        // want to handle this differently.
        Box::leak(self.name_cache.clone().into_boxed_str())
    }

    fn handles_resource(&self, resource: &str) -> bool {
        let resource_cstring = CString::new(resource).unwrap();
        let result = unsafe {
            (self.handles_resource_fn)(self.plugin_instance, resource_cstring.as_ptr())
        };
        result != 0
    }
}

unsafe impl Send for DynamicAuthzPlugin {}
unsafe impl Sync for DynamicAuthzPlugin {}

// Plugin registry for dynamic libraries
pub struct DynamicPluginRegistry;

impl DynamicPluginRegistry {
    pub fn load_plugin(library_path: &str, config: &HashMap<String, String>) -> Result<Box<dyn AuthPlugin>, String> {
        let plugin = DynamicPlugin::load(library_path, config)?;
        Ok(Box::new(plugin))
    }

    pub fn load_authz_plugin(library_path: &str, config: &HashMap<String, String>) -> Result<Box<dyn AuthzPlugin>, String> {
        let plugin = DynamicAuthzPlugin::load(library_path, config)?;
        Ok(Box::new(plugin))
    }
}