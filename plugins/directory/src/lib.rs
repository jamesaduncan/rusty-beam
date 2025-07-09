use async_trait::async_trait;
use hyper::{Body, Response};
use rusty_beam_plugin_api::{create_plugin, Plugin, PluginContext, PluginRequest, PluginResponse};
use std::collections::HashMap;
use std::sync::Arc;

/// A plugin that executes nested plugins only if the request path matches a directory
#[derive(Debug)]
pub struct DirectoryPlugin {
    directory: String,
    #[allow(dead_code)]
    nested_plugins: Vec<Arc<dyn Plugin>>,
}

impl DirectoryPlugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        // Get the directory configuration
        let directory = config
            .get("directory")
            .map(|d| {
                // Handle file:// URLs by extracting just the directory part
                if d.starts_with("file://") {
                    // Extract the path after the host root
                    // e.g., "file://./examples/localhost/admin" -> "/admin"
                    if let Some(last_part) = d.rsplit('/').next() {
                        format!("/{}", last_part)
                    } else {
                        d.clone()
                    }
                } else {
                    d.clone()
                }
            })
            .unwrap_or_else(|| "/".to_string());

        // For now, we don't support nested plugins in the dynamic version
        // This would require a more complex plugin loading mechanism
        Self {
            directory,
            nested_plugins: Vec::new(),
        }
    }
}

#[async_trait]
impl Plugin for DirectoryPlugin {
    async fn handle_request(
        &self,
        request: &mut PluginRequest,
        _context: &PluginContext,
    ) -> Option<PluginResponse> {
        // Check if the request path matches the configured directory
        let normalized_dir = self.directory.trim_end_matches('/');
        let normalized_path = request.path.trim_end_matches('/');

        // Check if path matches exactly or starts with directory followed by /
        let matches = normalized_path == normalized_dir
            || request.path.starts_with(&format!("{}/", normalized_dir));

        if !matches {
            // Path doesn't match, pass through to next plugin
            return None;
        }

        // Path matches, but we have no nested plugins in the dynamic version
        // This plugin acts as a filter only
        None
    }

    async fn handle_response(
        &self,
        request: &PluginRequest,
        _response: &mut Response<Body>,
        _context: &PluginContext,
    ) {
        // Check if the request path matches the configured directory
        let normalized_dir = self.directory.trim_end_matches('/');
        let normalized_path = request.path.trim_end_matches('/');

        // Check if path matches exactly or starts with directory followed by /
        let matches = normalized_path == normalized_dir
            || request.path.starts_with(&format!("{}/", normalized_dir));

        if matches {
            // Path matches, but we have no nested plugins to call
        }
    }

    fn name(&self) -> &str {
        "directory"
    }
}

// Export the plugin
create_plugin!(DirectoryPlugin);