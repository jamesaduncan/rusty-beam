use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, PluginResponse, create_plugin};
use async_trait::async_trait;
use hyper::{Body, Response, StatusCode, Method};
use std::collections::HashMap;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::path::Path;

/// Plugin that handles PATCH requests to reload server configuration
#[derive(Debug)]
pub struct ConfigReloadPlugin {
    name: String,
}

impl ConfigReloadPlugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        let name = config.get("name").cloned().unwrap_or_else(|| "config-reload".to_string());
        
        Self { name }
    }
    
    /// Canonicalize a path for comparison
    fn canonicalize_path(path: &str) -> Option<String> {
        Path::new(path)
            .canonicalize()
            .ok()
            .and_then(|p| p.to_str().map(|s| s.to_string()))
    }
    
    /// Check if the request is for the actual config file
    fn is_config_file_request(&self, request_path: &str, host_root: &str, config_file_path: &str) -> bool {
        // Construct the full path of the requested file
        let request_full_path = if request_path.starts_with('/') {
            format!("{}{}", host_root, request_path)
        } else {
            format!("{}/{}", host_root, request_path)
        };
        
        // Canonicalize both paths for comparison
        let request_canonical = Self::canonicalize_path(&request_full_path);
        let config_canonical = Self::canonicalize_path(config_file_path);
        
        match (request_canonical, config_canonical) {
            (Some(req), Some(cfg)) => req == cfg,
            _ => false,
        }
    }
}

#[async_trait]
impl Plugin for ConfigReloadPlugin {
    async fn handle_request(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<PluginResponse> {
        // Get the actual config file path from server metadata
        let config_file_path = context.server_metadata.get("config_file_path")?;
        
        // Get the host root directory
        let host_root = context.host_config.get("hostRoot")
            .map(|s| s.as_str())
            .unwrap_or(".");
        
        // Check if this request is for the config file
        if !self.is_config_file_request(&request.path, host_root, config_file_path) {
            return None; // Not our concern, pass to next plugin
        }
        
        match *request.http_request.method() {
            Method::PATCH => {
                // Check if body is empty (reload signal)
                let content_length = request.http_request.headers()
                    .get("content-length")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);
                    
                if content_length == 0 {
                    // Send SIGHUP to self
                    match kill(Pid::this(), Signal::SIGHUP) {
                        Ok(_) => {
                            Some(Response::builder()
                                .status(StatusCode::ACCEPTED)
                                .header("Content-Type", "text/plain")
                                .body(Body::from("Configuration reload initiated"))
                                .unwrap()
                                .into())
                        }
                        Err(e) => {
                            Some(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .header("Content-Type", "text/plain")
                                .body(Body::from(format!("Failed to send reload signal: {}", e)))
                                .unwrap()
                                .into())
                        }
                    }
                } else {
                    // PATCH with body - pass through to next plugin for future implementation
                    None
                }
            }
            Method::OPTIONS => {
                // Report PATCH as available for the config file
                Some(Response::builder()
                    .status(StatusCode::OK)
                    .header("Allow", "GET, PUT, DELETE, OPTIONS, PATCH, HEAD, POST")
                    .header("Accept-Ranges", "selector")
                    .body(Body::empty())
                    .unwrap()
                    .into())
            }
            _ => None, // Pass through to next plugin
        }
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

// Export the plugin creation function
create_plugin!(ConfigReloadPlugin);