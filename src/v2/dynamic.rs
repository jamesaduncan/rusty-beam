//! Dynamic library plugin support for C-style shared libraries

use super::plugin::{Plugin, PluginRequest, PluginContext};
use super::loader::{CPluginABI, PluginLoadError};
use async_trait::async_trait;
use hyper::{Body, Response};
use std::collections::HashMap;
use std::ffi::CStr;

/// Dynamic library plugin wrapper
pub struct DynamicPluginWrapper {
    /// The loaded library handle
    _library: libloading::Library,
    /// Plugin instance pointer from the C library
    plugin_ptr: *mut std::ffi::c_void,
    /// Plugin ABI function pointers
    abi: CPluginABI,
    /// Plugin name
    name: String,
}

// Mark as Send and Sync - we ensure thread safety through the C ABI
unsafe impl Send for DynamicPluginWrapper {}
unsafe impl Sync for DynamicPluginWrapper {}

impl DynamicPluginWrapper {
    /// Load a dynamic library plugin
    pub fn new(
        library_path: &str,
        config: HashMap<String, String>,
    ) -> Result<Self, PluginLoadError> {
        unsafe {
            // Load the library
            let library = libloading::Library::new(library_path)
                .map_err(|e| PluginLoadError::DynamicLibraryError(
                    format!("Failed to load library {}: {}", library_path, e)
                ))?;
            
            // Get required symbols
            let init: libloading::Symbol<unsafe extern "C" fn(*const u8, usize) -> *mut std::ffi::c_void> = 
                library.get(b"plugin_init")
                    .map_err(|e| PluginLoadError::DynamicLibraryError(
                        format!("Missing plugin_init: {}", e)
                    ))?;
            
            let handle_request: libloading::Symbol<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize, *mut u8, *mut usize) -> i32> = 
                library.get(b"plugin_handle_request")
                    .map_err(|e| PluginLoadError::DynamicLibraryError(
                        format!("Missing plugin_handle_request: {}", e)
                    ))?;
            
            let handle_response: libloading::Symbol<unsafe extern "C" fn(*mut std::ffi::c_void, *const u8, usize, *mut u8, usize)> = 
                library.get(b"plugin_handle_response")
                    .map_err(|e| PluginLoadError::DynamicLibraryError(
                        format!("Missing plugin_handle_response: {}", e)
                    ))?;
            
            let get_name: libloading::Symbol<unsafe extern "C" fn(*mut std::ffi::c_void) -> *const u8> = 
                library.get(b"plugin_get_name")
                    .map_err(|e| PluginLoadError::DynamicLibraryError(
                        format!("Missing plugin_get_name: {}", e)
                    ))?;
            
            let destroy: libloading::Symbol<unsafe extern "C" fn(*mut std::ffi::c_void)> = 
                library.get(b"plugin_destroy")
                    .map_err(|e| PluginLoadError::DynamicLibraryError(
                        format!("Missing plugin_destroy: {}", e)
                    ))?;
            
            // Create ABI struct
            let abi = CPluginABI {
                init: *init,
                handle_request: *handle_request,
                handle_response: *handle_response,
                get_name: *get_name,
                destroy: *destroy,
            };
            
            // Serialize config to JSON
            let config_json = serde_json::to_string(&config)
                .map_err(|e| PluginLoadError::PluginInitError(
                    format!("Failed to serialize config: {}", e)
                ))?;
            
            let config_bytes = config_json.as_bytes();
            
            // Initialize the plugin
            let plugin_ptr = (abi.init)(config_bytes.as_ptr(), config_bytes.len());
            
            if plugin_ptr.is_null() {
                return Err(PluginLoadError::PluginInitError(
                    "Plugin initialization returned null".to_string()
                ));
            }
            
            // Get plugin name
            let name_ptr = (abi.get_name)(plugin_ptr);
            let name = if name_ptr.is_null() {
                "dynamic-plugin".to_string()
            } else {
                CStr::from_ptr(name_ptr as *const std::ffi::c_char)
                    .to_string_lossy()
                    .to_string()
            };
            
            Ok(DynamicPluginWrapper {
                _library: library,
                plugin_ptr,
                abi,
                name,
            })
        }
    }
}

impl Drop for DynamicPluginWrapper {
    fn drop(&mut self) {
        unsafe {
            (self.abi.destroy)(self.plugin_ptr);
        }
    }
}

#[async_trait]
impl Plugin for DynamicPluginWrapper {
    async fn handle_request(&self, request: &mut PluginRequest, _context: &PluginContext) -> Option<Response<Body>> {
        // Serialize request data
        let request_data = serde_json::json!({
            "method": request.http_request.method().as_str(),
            "path": request.path,
            "headers": request.http_request.headers()
                .iter()
                .map(|(k, v)| (k.as_str(), v.to_str().unwrap_or("")))
                .collect::<HashMap<_, _>>(),
            "metadata": request.metadata,
        });
        
        let request_json = serde_json::to_string(&request_data).ok()?;
        let request_bytes = request_json.as_bytes();
        
        // Prepare response buffer
        let mut response_buffer = vec![0u8; 65536]; // 64KB buffer
        let mut response_len = response_buffer.len();
        
        unsafe {
            let result = (self.abi.handle_request)(
                self.plugin_ptr,
                request_bytes.as_ptr(),
                request_bytes.len(),
                response_buffer.as_mut_ptr(),
                &mut response_len,
            );
            
            if result == 0 {
                // No response generated
                return None;
            }
            
            // Parse response
            response_buffer.truncate(response_len);
            
            if let Ok(response_str) = std::str::from_utf8(&response_buffer) {
                if let Ok(response_data) = serde_json::from_str::<serde_json::Value>(response_str) {
                    // Build HTTP response from JSON
                    let status = response_data["status"].as_u64().unwrap_or(200) as u16;
                    let body = response_data["body"].as_str().unwrap_or("");
                    
                    let mut builder = Response::builder()
                        .status(status);
                    
                    // Add headers
                    if let Some(headers) = response_data["headers"].as_object() {
                        for (key, value) in headers {
                            if let Some(value_str) = value.as_str() {
                                builder = builder.header(key, value_str);
                            }
                        }
                    }
                    
                    return Some(builder.body(Body::from(body.to_string())).unwrap());
                }
            }
        }
        
        None
    }
    
    async fn handle_response(&self, request: &PluginRequest, response: &mut Response<Body>, _context: &PluginContext) {
        // Serialize request data
        let request_data = serde_json::json!({
            "path": request.path,
            "metadata": request.metadata,
        });
        
        let request_json = serde_json::to_string(&request_data).unwrap_or_default();
        let request_bytes = request_json.as_bytes();
        
        // Serialize response data
        let response_data = serde_json::json!({
            "status": response.status().as_u16(),
            "headers": response.headers()
                .iter()
                .map(|(k, v)| (k.as_str(), v.to_str().unwrap_or("")))
                .collect::<HashMap<_, _>>(),
        });
        
        let response_json = serde_json::to_string(&response_data).unwrap_or_default();
        let mut response_bytes = response_json.into_bytes();
        
        unsafe {
            (self.abi.handle_response)(
                self.plugin_ptr,
                request_bytes.as_ptr(),
                request_bytes.len(),
                response_bytes.as_mut_ptr(),
                response_bytes.len(),
            );
        }
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

impl std::fmt::Debug for DynamicPluginWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynamicPluginWrapper")
            .field("name", &self.name)
            .field("plugin_ptr", &self.plugin_ptr)
            .finish()
    }
}