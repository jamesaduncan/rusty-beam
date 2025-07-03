use libc::{c_char, c_int, c_void, size_t};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::fs::OpenOptions;
use std::io::Write;
use std::ptr;

// C-compatible structures matching Rust FFI definitions
#[repr(C)]
pub struct CAccessLogEntry {
    pub remote_addr: *const c_char,
    pub timestamp: *const c_char,
    pub method: *const c_char,
    pub path: *const c_char,
    pub query: *const c_char,
    pub status_code: u16,
    pub response_size: u64,
    pub user_agent: *const c_char,
    pub referer: *const c_char,
    pub username: *const c_char,
}

// Plugin configuration structure
pub struct ApacheAccessLogConfig {
    log_file_path: String,
    format: String,
}

impl ApacheAccessLogConfig {
    fn format_log_entry(&self, entry: &CAccessLogEntry) -> String {
        unsafe {
            let remote_addr = if entry.remote_addr.is_null() {
                "-"
            } else {
                CStr::from_ptr(entry.remote_addr).to_str().unwrap_or("-")
            };
            
            let timestamp = if entry.timestamp.is_null() {
                "-"
            } else {
                CStr::from_ptr(entry.timestamp).to_str().unwrap_or("-")
            };
            
            let method = if entry.method.is_null() {
                "-"
            } else {
                CStr::from_ptr(entry.method).to_str().unwrap_or("-")
            };
            
            let path = if entry.path.is_null() {
                "-"
            } else {
                CStr::from_ptr(entry.path).to_str().unwrap_or("-")
            };
            
            let query = if entry.query.is_null() {
                ""
            } else {
                CStr::from_ptr(entry.query).to_str().unwrap_or("")
            };
            
            let user_agent = if entry.user_agent.is_null() {
                "-"
            } else {
                CStr::from_ptr(entry.user_agent).to_str().unwrap_or("-")
            };
            
            let referer = if entry.referer.is_null() {
                "-"
            } else {
                CStr::from_ptr(entry.referer).to_str().unwrap_or("-")
            };
            
            let username = if entry.username.is_null() {
                "-"
            } else {
                CStr::from_ptr(entry.username).to_str().unwrap_or("-")
            };
            
            let query_str = if query.is_empty() {
                String::new()
            } else {
                format!("?{}", query)
            };
            
            let response_size_str = if entry.response_size == 0 {
                "-".to_string()
            } else {
                entry.response_size.to_string()
            };
            
            match self.format.as_str() {
                "combined" => {
                    // Combined Log Format: %h %l %u %t "%r" %>s %b "%{Referer}i" "%{User-agent}i"
                    format!(
                        "{} - {} [{}] \"{} {}{}\" {} {} \"{}\" \"{}\"",
                        remote_addr,
                        username,
                        timestamp,
                        method,
                        path,
                        query_str,
                        entry.status_code,
                        response_size_str,
                        referer,
                        user_agent
                    )
                }
                _ => {
                    // Common Log Format (default): %h %l %u %t "%r" %>s %b
                    format!(
                        "{} - {} [{}] \"{} {}{}\" {} {}",
                        remote_addr,
                        username,
                        timestamp,
                        method,
                        path,
                        query_str,
                        entry.status_code,
                        response_size_str
                    )
                }
            }
        }
    }
}

// Plugin creation function
#[no_mangle]
pub extern "C" fn access_log_plugin_create(
    config_keys: *const *const c_char,
    config_values: *const *const c_char,
    config_count: size_t,
) -> *mut c_void {
    let mut config = HashMap::new();
    
    unsafe {
        for i in 0..config_count {
            let key_ptr = *config_keys.add(i);
            let value_ptr = *config_values.add(i);
            
            if !key_ptr.is_null() && !value_ptr.is_null() {
                if let (Ok(key), Ok(value)) = (
                    CStr::from_ptr(key_ptr).to_str(),
                    CStr::from_ptr(value_ptr).to_str(),
                ) {
                    config.insert(key.to_string(), value.to_string());
                }
            }
        }
    }
    
    let log_file_path = match config.get("log_file") {
        Some(path) => path.clone(),
        None => {
            eprintln!("Apache access log plugin: missing required 'log_file' configuration");
            return ptr::null_mut();
        }
    };
    
    let format = config.get("format").unwrap_or(&"common".to_string()).clone();
    
    // Create directory if it doesn't exist
    if let Some(parent) = std::path::Path::new(&log_file_path).parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("Apache access log plugin: failed to create log directory: {}", e);
            return ptr::null_mut();
        }
    }
    
    let plugin_config = ApacheAccessLogConfig {
        log_file_path,
        format,
    };
    
    Box::into_raw(Box::new(plugin_config)) as *mut c_void
}

// Plugin destruction function
#[no_mangle]
pub extern "C" fn access_log_plugin_destroy(plugin: *mut c_void) {
    if !plugin.is_null() {
        unsafe {
            let _ = Box::from_raw(plugin as *mut ApacheAccessLogConfig);
        }
    }
}

// Log access function
#[no_mangle]
pub extern "C" fn access_log_plugin_log_access(plugin: *mut c_void, entry: *const CAccessLogEntry) {
    if plugin.is_null() || entry.is_null() {
        return;
    }
    
    unsafe {
        let config = &*(plugin as *const ApacheAccessLogConfig);
        let entry_ref = &*entry;
        
        let log_line = format!("{}\n", config.format_log_entry(entry_ref));
        
        // Write to log file
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&config.log_file_path)
        {
            let _ = file.write_all(log_line.as_bytes());
            let _ = file.flush();
        } else {
            eprintln!("Apache access log plugin: failed to open log file: {}", config.log_file_path);
        }
    }
}

// Plugin name function
#[no_mangle]
pub extern "C" fn access_log_plugin_name() -> *const c_char {
    static NAME: &[u8] = b"apache-access-log\0";
    NAME.as_ptr() as *const c_char
}
