//! Configuration Reload Plugin for Rusty Beam
//!
//! This plugin enables hot reloading of server configuration files via HTTP PATCH requests.
//! It monitors requests to the configuration file and provides a mechanism to reload
//! the server configuration without restarting the process.
//!
//! ## Features
//! - **Hot Configuration Reload**: Reload server configuration via PATCH requests
//! - **Signal-Based Reload**: Uses SIGHUP signal to trigger configuration reload
//! - **Config File Detection**: Automatically detects requests to the configuration file
//! - **Zero-Downtime Reload**: Server remains running during configuration changes
//!
//! ## Usage
//! Send a PATCH request with an empty body to the configuration file to trigger a reload:
//!
//! ```bash
//! curl -X PATCH http://localhost:3000/config.html
//! ```
//!
//! The plugin will send a SIGHUP signal to the current process, causing the server
//! to reload its configuration from the original config file.
//!
//! ## Pipeline Integration
//! This plugin should be placed early in the pipeline to intercept config file
//! requests before they reach the file-handler plugin.

use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, PluginResponse, create_plugin};
use async_trait::async_trait;
use hyper::{Body, Response, StatusCode, Method};
use std::collections::HashMap;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::path::Path;

// Default configuration values
const DEFAULT_PLUGIN_NAME: &str = "config-reload";
const DEFAULT_HOST_ROOT: &str = ".";
const PATH_SEPARATOR: &str = "/";

// Configuration keys
const CONFIG_KEY_NAME: &str = "name";
const CONFIG_KEY_HOST_ROOT: &str = "hostRoot";
const CONFIG_KEY_CONFIG_FILE_PATH: &str = "config_file_path";

// HTTP headers
const HEADER_CONTENT_LENGTH: &str = "content-length";
const HEADER_CONTENT_TYPE: &str = "Content-Type";
const HEADER_ALLOW: &str = "Allow";
const HEADER_ACCEPT_RANGES: &str = "Accept-Ranges";

// HTTP header values
const CONTENT_TYPE_TEXT_PLAIN: &str = "text/plain";
const ALLOWED_METHODS: &str = "GET, PUT, DELETE, OPTIONS, PATCH, HEAD, POST";
const ACCEPT_RANGES_VALUE: &str = "selector";

// Response messages
const MSG_RELOAD_INITIATED: &str = "Configuration reload initiated";
const MSG_RELOAD_FAILED: &str = "Failed to send reload signal";

// HTTP status codes for empty body detection
const EMPTY_CONTENT_LENGTH: u64 = 0;

/// Configuration Reload Plugin for hot reloading server configuration
#[derive(Debug)]
pub struct ConfigReloadPlugin {
    name: String,
}

impl ConfigReloadPlugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        let name = config.get(CONFIG_KEY_NAME)
            .cloned()
            .unwrap_or_else(|| DEFAULT_PLUGIN_NAME.to_string());
        
        Self { name }
    }
    
    /// Canonicalizes a file path for secure comparison
    /// 
    /// This method resolves symbolic links and relative path components
    /// to ensure accurate path comparison between the request and config file.
    fn canonicalize_path(path: &str) -> Option<String> {
        Path::new(path)
            .canonicalize()
            .ok()
            .and_then(|p| p.to_str().map(|s| s.to_string()))
    }
    
    /// Determines if the request is targeting the server configuration file
    /// 
    /// This method constructs the full path of the requested file and compares
    /// it with the actual configuration file path using canonicalization.
    fn is_config_file_request(&self, request_path: &str, host_root: &str, config_file_path: &str) -> bool {
        let request_full_path = self.build_full_request_path(request_path, host_root);
        self.paths_match(&request_full_path, config_file_path)
    }
    
    /// Builds the full file system path for the requested resource
    fn build_full_request_path(&self, request_path: &str, host_root: &str) -> String {
        if request_path.starts_with(PATH_SEPARATOR) {
            format!("{}{}", host_root, request_path)
        } else {
            format!("{}{}{}", host_root, PATH_SEPARATOR, request_path)
        }
    }
    
    /// Compares two file paths after canonicalization
    fn paths_match(&self, path1: &str, path2: &str) -> bool {
        let canonical1 = Self::canonicalize_path(path1);
        let canonical2 = Self::canonicalize_path(path2);
        
        match (canonical1, canonical2) {
            (Some(p1), Some(p2)) => p1 == p2,
            _ => false,
        }
    }
}

#[async_trait]
impl Plugin for ConfigReloadPlugin {
    async fn handle_request(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<PluginResponse> {
        // Get the actual config file path from server metadata
        let config_file_path = context.server_metadata.get(CONFIG_KEY_CONFIG_FILE_PATH)?;
        
        // Get the host root directory
        let host_root = context.host_config.get(CONFIG_KEY_HOST_ROOT)
            .map(|s| s.as_str())
            .unwrap_or(DEFAULT_HOST_ROOT);
        
        // Check if this request is for the config file
        if !self.is_config_file_request(&request.path, host_root, config_file_path) {
            return None; // Not our concern, pass to next plugin
        }
        
        match *request.http_request.method() {
            Method::PATCH => {
                self.handle_patch_request(request)
            }
            Method::OPTIONS => {
                self.handle_options_request()
            }
            _ => None, // Pass through to next plugin
        }
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

impl ConfigReloadPlugin {
    /// Handles PATCH requests for configuration reload
    fn handle_patch_request(&self, request: &PluginRequest) -> Option<PluginResponse> {
        let content_length = self.get_content_length(request);
        
        if content_length == EMPTY_CONTENT_LENGTH {
            self.send_reload_signal()
        } else {
            // PATCH with body - pass through to next plugin for future implementation
            None
        }
    }
    
    /// Extracts the content length from the request headers
    fn get_content_length(&self, request: &PluginRequest) -> u64 {
        request.http_request.headers()
            .get(HEADER_CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(EMPTY_CONTENT_LENGTH)
    }
    
    /// Sends SIGHUP signal to trigger configuration reload
    fn send_reload_signal(&self) -> Option<PluginResponse> {
        match kill(Pid::this(), Signal::SIGHUP) {
            Ok(_) => {
                Some(Response::builder()
                    .status(StatusCode::ACCEPTED)
                    .header(HEADER_CONTENT_TYPE, CONTENT_TYPE_TEXT_PLAIN)
                    .body(Body::from(MSG_RELOAD_INITIATED))
                    .unwrap()
                    .into())
            }
            Err(e) => {
                Some(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(HEADER_CONTENT_TYPE, CONTENT_TYPE_TEXT_PLAIN)
                    .body(Body::from(format!("{}: {}", MSG_RELOAD_FAILED, e)))
                    .unwrap()
                    .into())
            }
        }
    }
    
    /// Handles OPTIONS requests for the configuration file
    fn handle_options_request(&self) -> Option<PluginResponse> {
        Some(Response::builder()
            .status(StatusCode::OK)
            .header(HEADER_ALLOW, ALLOWED_METHODS)
            .header(HEADER_ACCEPT_RANGES, ACCEPT_RANGES_VALUE)
            .body(Body::empty())
            .unwrap()
            .into())
    }
}

// Export the plugin creation function
create_plugin!(ConfigReloadPlugin);