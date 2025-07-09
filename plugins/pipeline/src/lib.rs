use async_trait::async_trait;
use hyper::{Body, Response};
use rusty_beam_plugin_api::{create_plugin, Plugin, PluginContext, PluginRequest, PluginResponse};
use std::collections::HashMap;
use std::sync::Arc;

/// A plugin that executes nested plugins in sequence
#[derive(Debug)]
pub struct PipelinePlugin {
    nested_plugins: Vec<Arc<dyn Plugin>>,
}

impl PipelinePlugin {
    pub fn new(_config: HashMap<String, String>) -> Self {
        // For now, we don't support nested plugins in the dynamic version
        // This would require a more complex plugin loading mechanism
        Self {
            nested_plugins: Vec::new(),
        }
    }
}

#[async_trait]
impl Plugin for PipelinePlugin {
    async fn handle_request(
        &self,
        request: &mut PluginRequest,
        context: &PluginContext,
    ) -> Option<PluginResponse> {
        // Execute nested plugins in order
        for plugin in &self.nested_plugins {
            if let Some(response) = plugin.handle_request(request, context).await {
                return Some(response);
            }
        }
        None
    }

    async fn handle_response(
        &self,
        request: &PluginRequest,
        response: &mut Response<Body>,
        context: &PluginContext,
    ) {
        // Call handle_response on all nested plugins
        for plugin in &self.nested_plugins {
            plugin.handle_response(request, response, context).await;
        }
    }

    fn name(&self) -> &str {
        "pipeline"
    }
}

// Export the plugin
create_plugin!(PipelinePlugin);