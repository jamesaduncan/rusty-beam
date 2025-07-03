use libc::{c_char, c_int, c_void, size_t};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::fs::OpenOptions;
use std::io::Write;
use std::ptr;

// C-compatible structures matching Rust FFI definitions
#[repr(C)]
pub struct CErrorLogEntry {
    pub timestamp: *const c_char,
    pub level: *const c_char,
    pub client_addr: *const c_char,
    pub message: *const c_char,
    pub file: *const c_char,
    pub line: u32,
}

// Plugin configuration structure
pub struct ApacheErrorLogConfig {
    log_file_path: String,
}

impl ApacheErrorLogConfig {
    fn format_log_entry(&self, entry: &CErrorLogEntry) -> String {
        unsafe {
            let timestamp = if entry.timestamp.is_null() {
                "-"
            } else {
                CStr::from_ptr(entry.timestamp).to_str().unwrap_or("-")
            };
            
            let level = if entry.level.is_null() {
                "error"
            } else {
                CStr::from_ptr(entry.level).to_str().unwrap_or("error")
            };
            
            let client_addr = if entry.client_addr.is_null() {
                "-"
            } else {
                CStr::from_ptr(entry.client_addr).to_str().unwrap_or("-")
            };
            
            let message = if entry.message.is_null() {
                "-"
            } else {
                CStr::from_ptr(entry.message).to_str().unwrap_or("-")
            };
            
            let file_info = if entry.file.is_null() {
                String::new()
            } else {
                let file_str = CStr::from_ptr(entry.file).to_str().unwrap_or("");
                if entry.line > 0 {
                    format!(" [{}:{}]", file_str, entry.line)
                } else {
                    format!(" [{}]", file_str)
                }
            };
            
            // Apache Error Log Format: [timestamp] [level] [client client_addr] message [file:line]
            format!(
                "[{}] [{}] [client {}] {}{}",
                timestamp,
                level,
                client_addr,
                message,
                file_info
            )
        }
    }
}

// Plugin creation function
#[no_mangle]
pub extern "C" fn error_log_plugin_create(
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
            eprintln!("Apache error log plugin: missing required 'log_file' configuration");
            return ptr::null_mut();
        }
    };
    
    // Create directory if it doesn't exist
    if let Some(parent) = std::path::Path::new(&log_file_path).parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("Apache error log plugin: failed to create log directory: {}", e);
            return ptr::null_mut();
        }
    }
    
    let plugin_config = ApacheErrorLogConfig {
        log_file_path,
    };
    
    Box::into_raw(Box::new(plugin_config)) as *mut c_void
}

// Plugin destruction function
#[no_mangle]
pub extern "C" fn error_log_plugin_destroy(plugin: *mut c_void) {
    if !plugin.is_null() {
        unsafe {
            let _ = Box::from_raw(plugin as *mut ApacheErrorLogConfig);
        }
    }
}

// Log error function
#[no_mangle]
pub extern "C" fn error_log_plugin_log_error(plugin: *mut c_void, entry: *const CErrorLogEntry) {
    if plugin.is_null() || entry.is_null() {
        return;
    }
    
    unsafe {
        let config = &*(plugin as *const ApacheErrorLogConfig);
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
            eprintln!("Apache error log plugin: failed to open log file: {}", config.log_file_path);
        }
    }
}

// Plugin name function
#[no_mangle]
pub extern "C" fn error_log_plugin_name() -> *const c_char {
    static NAME: &[u8] = b"apache-error-log\0";
    NAME.as_ptr() as *const c_char
}
