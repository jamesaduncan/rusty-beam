use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, create_plugin};
use async_trait::async_trait;
use hyper::{Body, Response};
use std::collections::HashMap;
use chrono::Utc;

/// Access log format styles
#[derive(Debug, Clone)]
enum LogFormat {
    Common,
    Combined,
    Json,
}

impl LogFormat {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "combined" => LogFormat::Combined,
            "json" => LogFormat::Json,
            _ => LogFormat::Common,
        }
    }
}

/// Plugin for HTTP request access logging
#[derive(Debug)]
pub struct AccessLogPlugin {
    name: String,
    log_file: Option<String>,
    format: LogFormat,
}

impl AccessLogPlugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        let name = config.get("name").cloned().unwrap_or_else(|| "access-log".to_string());
        let log_file = config.get("log_file").cloned();
        let format = LogFormat::from_str(&config.get("format").cloned().unwrap_or_else(|| "common".to_string()));
        
        Self { name, log_file, format }
    }
    
    /// Get remote IP address from request
    fn get_remote_ip(&self, request: &PluginRequest) -> String {
        // Try to get from X-Forwarded-For header first
        if let Some(forwarded) = request.http_request.headers().get("X-Forwarded-For") {
            if let Ok(forwarded_str) = forwarded.to_str() {
                return forwarded_str.split(',').next().unwrap_or("-").trim().to_string();
            }
        }
        
        // Try to get from X-Real-IP header
        if let Some(real_ip) = request.http_request.headers().get("X-Real-IP") {
            if let Ok(real_ip_str) = real_ip.to_str() {
                return real_ip_str.to_string();
            }
        }
        
        // Default to unknown
        "-".to_string()
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
        let timestamp = Utc::now().format("%d/%b/%Y:%H:%M:%S %z");
        let remote_ip = self.get_remote_ip(request);
        let user = request.metadata.get("authenticated_user").unwrap_or("-");
        let method = request.http_request.method().as_str();
        let uri = request.http_request.uri().to_string();
        let version = format!("{:?}", request.http_request.version());
        let status = response.status().as_u16();
        
        match self.format {
            LogFormat::Common => {
                // Common Log Format: host ident authuser [timestamp] "request" status size
                format!(r#"{} - {} [{}] "{} {} {}" {} {}"#, 
                    remote_ip, user, timestamp, method, uri, version, status, response_size)
            }
            LogFormat::Combined => {
                // Combined Log Format: common + "referer" "user-agent"
                let referer = self.get_referer(request);
                let user_agent = self.get_user_agent(request);
                format!(r#"{} - {} [{}] "{} {} {}" {} {} "{}" "{}""#, 
                    remote_ip, user, timestamp, method, uri, version, status, response_size, referer, user_agent)
            }
            LogFormat::Json => {
                // JSON format
                format!(r#"{{"timestamp":"{}","remote_ip":"{}","user":"{}","method":"{}","uri":"{}","version":"{}","status":{},"size":{},"user_agent":"{}","referer":"{}"}}"#,
                    timestamp, remote_ip, user, method, uri, version, status, response_size, 
                    self.get_user_agent(request), self.get_referer(request))
            }
        }
    }
    
    /// Write log entry to file or stdout
    fn write_log_entry(&self, log_entry: &str) {
        if let Some(_log_file) = &self.log_file {
            // In a real implementation, we'd write to the log file
            // For now, just print to stdout
            println!("[ACCESS] {}", log_entry);
        } else {
            println!("[ACCESS] {}", log_entry);
        }
    }
}

#[async_trait]
impl Plugin for AccessLogPlugin {
    async fn handle_request(&self, _request: &mut PluginRequest, _context: &PluginContext) -> Option<Response<Body>> {
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
        self.write_log_entry(&log_entry);
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

// Export the plugin creation function
create_plugin!(AccessLogPlugin);