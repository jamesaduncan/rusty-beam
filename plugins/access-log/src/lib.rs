//! Access Log Plugin for Rusty Beam
//!
//! This plugin provides comprehensive HTTP access logging with multiple format
//! options and flexible output destinations. It supports standard log formats
//! (Common, Combined) as well as structured JSON logging for modern log analysis.
//!
//! ## Features
//! - **Multiple Log Formats**: Common, Combined, and JSON formats
//! - **Flexible Output**: Log to file or stdout
//! - **Real Client IP Detection**: Handles proxy headers (X-Forwarded-For, X-Real-IP)
//! - **Authenticated User Tracking**: Logs authenticated usernames when available
//! - **Automatic Directory Creation**: Creates log directories if they don't exist
//! - **Performance Optimized**: Minimal overhead on request processing
//! - **Error Resilience**: Continues serving even if logging fails
//!
//! ## Configuration
//! - `log_file`: Path to log file (e.g., "/var/log/rusty-beam/access.log")
//! - `format`: Log format - "common", "combined", or "json" (default: "common")
//! - `buffer_size`: Number of entries to buffer before writing (default: 1)
//! - `rotate_size_mb`: Rotate log when it reaches this size in MB (default: disabled)
//! - `rotate_daily`: Enable daily log rotation (default: false)
//!
//! ## Log Formats
//!
//! ### Common Log Format
//! ```
//! 127.0.0.1 - alice [10/Oct/2024:13:55:36 +0000] "GET /index.html HTTP/1.1" 200 2326
//! ```
//!
//! ### Combined Log Format
//! ```
//! 127.0.0.1 - alice [10/Oct/2024:13:55:36 +0000] "GET /index.html HTTP/1.1" 200 2326 "http://example.com/" "Mozilla/5.0"
//! ```
//!
//! ### JSON Format
//! ```json
//! {"timestamp":"10/Oct/2024:13:55:36 +0000","remote_ip":"127.0.0.1","user":"alice","method":"GET","uri":"/index.html","version":"HTTP/1.1","status":200,"size":2326,"user_agent":"Mozilla/5.0","referer":"http://example.com/"}
//! ```
//!
//! ## Integration with Other Plugins
//! - **Basic Auth Plugin**: Logs authenticated usernames
//! - **OAuth Plugin**: Logs OAuth user identities  
//! - **Rate Limit Plugin**: Can analyze logs for rate limiting decisions
//! - **Error Handler Plugin**: Access logs include error responses

use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, PluginResponse, create_plugin};
use async_trait::async_trait;
use hyper::{Body, Response};
use std::collections::HashMap;
use chrono::{Utc, Local};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Access log format styles
#[derive(Debug, Clone)]
enum LogFormat {
    /// Common Log Format (CLF)
    Common,
    /// Combined Log Format (CLF + referer + user-agent)
    Combined,
    /// JSON structured logging
    Json,
}

impl LogFormat {
    /// Parse log format from string
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "combined" => LogFormat::Combined,
            "json" => LogFormat::Json,
            "common" | _ => LogFormat::Common,
        }
    }
}

/// Buffered log entry for batch writing
#[derive(Debug)]
struct LogBuffer {
    entries: Vec<String>,
    max_size: usize,
}

/// Plugin for HTTP request access logging
#[derive(Debug)]
pub struct AccessLogPlugin {
    name: String,
    log_file: Option<PathBuf>,
    format: LogFormat,
    buffer: Mutex<LogBuffer>,
    rotate_size_bytes: Option<u64>,
    rotate_daily: bool,
}

impl AccessLogPlugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        let name = Self::parse_string_config(&config, "name", "access-log");
        let log_file = Self::parse_log_file_config(&config);
        let format = Self::parse_format_config(&config);
        let buffer_size = Self::parse_numeric_config(&config, "buffer_size", 1);
        let rotate_size_mb = Self::parse_numeric_config::<f64>(&config, "rotate_size_mb", 0.0);
        let rotate_daily = Self::parse_boolean_config(&config, "rotate_daily", false);
        
        // Create log directory if needed
        if let Some(ref log_path) = log_file {
            Self::ensure_log_directory_exists(log_path);
        }
        
        Self {
            name,
            log_file,
            format,
            buffer: Mutex::new(LogBuffer {
                entries: Vec::with_capacity(buffer_size),
                max_size: buffer_size,
            }),
            rotate_size_bytes: if rotate_size_mb > 0.0 {
                Some((rotate_size_mb * 1024.0 * 1024.0) as u64)
            } else {
                None
            },
            rotate_daily,
        }
    }
    
    /// Parse string configuration with default
    fn parse_string_config(config: &HashMap<String, String>, key: &str, default: &str) -> String {
        config.get(key).cloned().unwrap_or_else(|| default.to_string())
    }
    
    /// Parse numeric configuration with default
    fn parse_numeric_config<T: std::str::FromStr>(config: &HashMap<String, String>, key: &str, default: T) -> T {
        config.get(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }
    
    /// Parse boolean configuration with default
    fn parse_boolean_config(config: &HashMap<String, String>, key: &str, default: bool) -> bool {
        config.get(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }
    
    /// Parse log file path from configuration
    fn parse_log_file_config(config: &HashMap<String, String>) -> Option<PathBuf> {
        // Check for both "logfile" and "log_file" for backward compatibility
        config.get("log_file")
            .or_else(|| config.get("logfile"))
            .map(|path| {
                // Remove file:// prefix if present
                let clean_path = if path.starts_with("file://") {
                    path.strip_prefix("file://").unwrap()
                } else {
                    path
                };
                PathBuf::from(clean_path)
            })
    }
    
    /// Parse log format from configuration
    fn parse_format_config(config: &HashMap<String, String>) -> LogFormat {
        config.get("format")
            .map(|f| LogFormat::from_str(f))
            .unwrap_or(LogFormat::Common)
    }
    
    /// Ensure log directory exists
    fn ensure_log_directory_exists(log_path: &Path) {
        if let Some(parent) = log_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("[AccessLog] Failed to create log directory {:?}: {}", parent, e);
            }
        }
    }
    
    /// Get remote IP address from request with proxy header support
    fn get_remote_ip(&self, request: &PluginRequest) -> String {
        // Check various proxy headers in order of preference
        let headers = [
            "X-Forwarded-For",
            "X-Real-IP",
            "X-Client-IP",
            "CF-Connecting-IP", // Cloudflare
            "True-Client-IP",   // Cloudflare Enterprise
        ];
        
        for header_name in &headers {
            if let Some(ip) = self.extract_ip_from_header(request, header_name) {
                return ip;
            }
        }
        
        // Default to unknown
        "-".to_string()
    }
    
    /// Extract IP address from a specific header
    fn extract_ip_from_header(&self, request: &PluginRequest, header_name: &str) -> Option<String> {
        request.http_request.headers()
            .get(header_name)
            .and_then(|h| h.to_str().ok())
            .map(|value| {
                // X-Forwarded-For can contain multiple IPs, take the first
                value.split(',').next().unwrap_or(value).trim().to_string()
            })
            .filter(|ip| !ip.is_empty() && ip != "-")
    }
    
    /// Get user agent from request
    fn get_user_agent(&self, request: &PluginRequest) -> String {
        request.http_request.headers()
            .get("User-Agent")
            .and_then(|ua| ua.to_str().ok())
            .unwrap_or("-")
            .to_string()
    }
    
    /// Get referer from request
    fn get_referer(&self, request: &PluginRequest) -> String {
        request.http_request.headers()
            .get("Referer")
            .and_then(|ref_header| ref_header.to_str().ok())
            .unwrap_or("-")
            .to_string()
    }
    
    /// Format log entry based on configured format
    fn format_log_entry(&self, request: &PluginRequest, response: &Response<Body>, response_size: usize) -> String {
        let entry_data = self.collect_log_entry_data(request, response, response_size);
        
        match self.format {
            LogFormat::Common => self.format_common_log(&entry_data),
            LogFormat::Combined => self.format_combined_log(&entry_data),
            LogFormat::Json => self.format_json_log(&entry_data),
        }
    }
    
    /// Collect all data needed for log entry
    fn collect_log_entry_data(&self, request: &PluginRequest, response: &Response<Body>, response_size: usize) -> LogEntryData {
        LogEntryData {
            timestamp: Utc::now().format("%d/%b/%Y:%H:%M:%S %z").to_string(),
            remote_ip: self.get_remote_ip(request),
            user: request.get_metadata("authenticated_user").unwrap_or("-").to_string(),
            method: request.method().to_string(),
            uri: request.http_request.uri().to_string(),
            version: format!("{:?}", request.http_request.version()),
            status: response.status().as_u16(),
            size: response_size,
            user_agent: self.get_user_agent(request),
            referer: self.get_referer(request),
            request_time_ms: request.get_metadata("request_time_ms")
                .and_then(|t| t.parse::<u64>().ok())
                .unwrap_or(0),
        }
    }
    
    /// Format as Common Log Format
    fn format_common_log(&self, data: &LogEntryData) -> String {
        format!(r#"{} - {} [{}] "{} {} {}" {} {}"#,
            data.remote_ip, data.user, data.timestamp, 
            data.method, data.uri, data.version, 
            data.status, data.size)
    }
    
    /// Format as Combined Log Format
    fn format_combined_log(&self, data: &LogEntryData) -> String {
        format!(r#"{} - {} [{}] "{} {} {}" {} {} "{}" "{}""#,
            data.remote_ip, data.user, data.timestamp, 
            data.method, data.uri, data.version, 
            data.status, data.size, 
            data.referer, data.user_agent)
    }
    
    /// Format as JSON
    fn format_json_log(&self, data: &LogEntryData) -> String {
        // Use serde_json for proper escaping
        serde_json::json!({
            "timestamp": data.timestamp,
            "remote_ip": data.remote_ip,
            "user": data.user,
            "method": data.method,
            "uri": data.uri,
            "version": data.version,
            "status": data.status,
            "size": data.size,
            "user_agent": data.user_agent,
            "referer": data.referer,
            "request_time_ms": data.request_time_ms,
        }).to_string()
    }
    
    /// Add log entry to buffer and flush if needed
    fn buffer_log_entry(&self, log_entry: String) {
        let should_flush = {
            let mut buffer = self.buffer.lock().unwrap();
            buffer.entries.push(log_entry);
            buffer.entries.len() >= buffer.max_size
        };
        
        if should_flush {
            self.flush_buffer();
        }
    }
    
    /// Flush buffered log entries to file
    fn flush_buffer(&self) {
        let entries = {
            let mut buffer = self.buffer.lock().unwrap();
            std::mem::take(&mut buffer.entries)
        };
        
        if entries.is_empty() {
            return;
        }
        
        if let Some(log_file) = &self.log_file {
            // Check if we need to rotate the log
            if self.should_rotate_log(log_file) {
                self.rotate_log_file(log_file);
            }
            
            // Write entries to file
            self.write_entries_to_file(log_file, &entries);
        } else {
            // No log file configured, write to stdout
            for entry in entries {
                println!("{}", entry);
            }
        }
    }
    
    /// Check if log rotation is needed
    fn should_rotate_log(&self, log_file: &Path) -> bool {
        // Check size-based rotation
        if let Some(max_size) = self.rotate_size_bytes {
            if let Ok(metadata) = std::fs::metadata(log_file) {
                if metadata.len() >= max_size {
                    return true;
                }
            }
        }
        
        // Check daily rotation
        if self.rotate_daily {
            // Check if log file is from a previous day
            if let Ok(metadata) = std::fs::metadata(log_file) {
                if let Ok(modified) = metadata.modified() {
                    let modified_date = chrono::DateTime::<Local>::from(modified).date_naive();
                    let today = Local::now().date_naive();
                    if modified_date < today {
                        return true;
                    }
                }
            }
        }
        
        false
    }
    
    /// Rotate the log file
    fn rotate_log_file(&self, log_file: &Path) {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let rotated_name = format!(
            "{}.{}",
            log_file.to_string_lossy(),
            timestamp
        );
        
        if let Err(e) = std::fs::rename(log_file, &rotated_name) {
            eprintln!("[AccessLog] Failed to rotate log file: {}", e);
        }
    }
    
    /// Write entries to log file
    fn write_entries_to_file(&self, log_file: &Path, entries: &[String]) {
        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)
        {
            Ok(mut file) => {
                for entry in entries {
                    if let Err(e) = writeln!(file, "{}", entry) {
                        eprintln!("[AccessLog] Failed to write to log file {:?}: {}", log_file, e);
                    }
                }
            }
            Err(e) => {
                eprintln!("[AccessLog] Failed to open log file {:?}: {}", log_file, e);
                // Fallback to stdout
                for entry in entries {
                    println!("{}", entry);
                }
            }
        }
    }
}

#[async_trait]
impl Plugin for AccessLogPlugin {
    async fn handle_request(&self, _request: &mut PluginRequest, _context: &PluginContext) -> Option<PluginResponse> {
        // Access log plugin doesn't intercept requests, just logs them
        None
    }
    
    async fn handle_response(&self, request: &PluginRequest, response: &mut Response<Body>, _context: &PluginContext) {
        // Estimate response size (in a real implementation we'd track the actual bytes)
        let response_size = response.headers()
            .get("Content-Length")
            .and_then(|len| len.to_str().ok())
            .and_then(|len| len.parse().ok())
            .unwrap_or(0);
        
        let log_entry = self.format_log_entry(request, response, response_size);
        self.buffer_log_entry(log_entry);
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

/// Structured data for log entries
#[derive(Debug)]
struct LogEntryData {
    timestamp: String,
    remote_ip: String,
    user: String,
    method: String,
    uri: String,
    version: String,
    status: u16,
    size: usize,
    user_agent: String,
    referer: String,
    request_time_ms: u64,
}

/// Ensure buffer is flushed on drop
impl Drop for AccessLogPlugin {
    fn drop(&mut self) {
        self.flush_buffer();
    }
}

// Export the plugin creation function
create_plugin!(AccessLogPlugin);