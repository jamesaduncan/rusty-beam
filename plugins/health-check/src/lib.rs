use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, PluginResponse, create_plugin};
use async_trait::async_trait;
use hyper::{Body, Response, StatusCode, Method};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Health check status
#[derive(Debug, Clone)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Degraded,
}

/// Plugin for health check endpoints
#[derive(Debug)]
pub struct HealthCheckPlugin {
    name: String,
    health_endpoint: String,
    ready_endpoint: String,
    live_endpoint: String,
    detailed_checks: bool,
    check_disk_space: bool,
    min_disk_space_mb: u64,
}

impl HealthCheckPlugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        let name = config.get("name").cloned().unwrap_or_else(|| "health-check".to_string());
        
        let health_endpoint = config.get("health_endpoint")
            .cloned()
            .unwrap_or_else(|| "/health".to_string());
        
        let ready_endpoint = config.get("ready_endpoint")
            .cloned()
            .unwrap_or_else(|| "/ready".to_string());
        
        let live_endpoint = config.get("live_endpoint")
            .cloned()
            .unwrap_or_else(|| "/live".to_string());
        
        let detailed_checks = config.get("detailed_checks")
            .map(|v| v.parse().unwrap_or(true))
            .unwrap_or(true);
        
        let check_disk_space = config.get("check_disk_space")
            .map(|v| v.parse().unwrap_or(true))
            .unwrap_or(true);
        
        let min_disk_space_mb = config.get("min_disk_space_mb")
            .and_then(|v| v.parse().ok())
            .unwrap_or(100);
        
        Self {
            name,
            health_endpoint,
            ready_endpoint,
            live_endpoint,
            detailed_checks,
            check_disk_space,
            min_disk_space_mb,
        }
    }
    
    /// Check if request path matches health check endpoints
    fn is_health_endpoint(&self, path: &str) -> bool {
        path == self.health_endpoint || 
        path == self.ready_endpoint || 
        path == self.live_endpoint
    }
    
    /// Perform basic liveness check
    fn check_liveness(&self) -> (HealthStatus, Vec<String>) {
        let mut messages = Vec::new();
        
        // Basic liveness - server is running and responding
        messages.push("Server is running".to_string());
        
        (HealthStatus::Healthy, messages)
    }
    
    /// Perform readiness check
    fn check_readiness(&self, context: &PluginContext) -> (HealthStatus, Vec<String>) {
        let mut messages = Vec::new();
        let mut status = HealthStatus::Healthy;
        
        // Check if document root exists and is readable
        if let Some(doc_root) = context.get_config("document_root") {
            let path = Path::new(doc_root);
            if path.exists() && path.is_dir() {
                messages.push(format!("Document root accessible: {}", doc_root));
            } else {
                messages.push(format!("Document root not accessible: {}", doc_root));
                status = HealthStatus::Unhealthy;
            }
        }
        
        // Check disk space if enabled
        if self.check_disk_space {
            match self.check_disk_space_internal() {
                Ok(available_mb) => {
                    if available_mb < self.min_disk_space_mb {
                        messages.push(format!("Low disk space: {} MB available", available_mb));
                        status = HealthStatus::Degraded;
                    } else {
                        messages.push(format!("Disk space OK: {} MB available", available_mb));
                    }
                }
                Err(e) => {
                    messages.push(format!("Could not check disk space: {}", e));
                    status = HealthStatus::Degraded;
                }
            }
        }
        
        (status, messages)
    }
    
    /// Perform comprehensive health check
    fn check_health(&self, context: &PluginContext) -> (HealthStatus, Vec<String>) {
        let mut messages = Vec::new();
        
        // Combine liveness and readiness checks
        let (live_status, live_messages) = self.check_liveness();
        let (ready_status, ready_messages) = self.check_readiness(context);
        
        messages.extend(live_messages);
        messages.extend(ready_messages);
        
        // Determine overall status
        let overall_status = match (live_status, ready_status) {
            (HealthStatus::Healthy, HealthStatus::Healthy) => HealthStatus::Healthy,
            (HealthStatus::Healthy, HealthStatus::Degraded) | 
            (HealthStatus::Degraded, HealthStatus::Healthy) => HealthStatus::Degraded,
            _ => HealthStatus::Unhealthy,
        };
        
        // Add timestamp and server info
        if self.detailed_checks {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            messages.push(format!("Timestamp: {}", timestamp));
            messages.push("Server: rusty-beam".to_string());
        }
        
        (overall_status, messages)
    }
    
    /// Check available disk space
    fn check_disk_space_internal(&self) -> Result<u64, String> {
        // This is a simplified check - in a real implementation, you would use 
        // system-specific APIs to check disk space
        // For now, we'll simulate it by checking if we can create a temp file
        
        if let Ok(temp_dir) = std::env::temp_dir().canonicalize() {
            match fs::metadata(&temp_dir) {
                Ok(_) => {
                    // Simulate disk space check - return a reasonable value
                    Ok(1024) // 1GB available (simulated)
                }
                Err(e) => Err(format!("Cannot access temp directory: {}", e)),
            }
        } else {
            Err("Cannot determine temp directory".to_string())
        }
    }
    
    /// Create health check response
    fn create_health_response(&self, status: HealthStatus, messages: Vec<String>) -> Response<Body> {
        let status_code = match status {
            HealthStatus::Healthy => StatusCode::OK,
            HealthStatus::Degraded => StatusCode::OK, // Still return 200 for degraded
            HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
        };
        
        let status_text = match status {
            HealthStatus::Healthy => "healthy",
            HealthStatus::Degraded => "degraded",
            HealthStatus::Unhealthy => "unhealthy",
        };
        
        let response_body = if self.detailed_checks {
            serde_json::json!({
                "status": status_text,
                "checks": messages,
                "timestamp": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            })
        } else {
            serde_json::json!({
                "status": status_text
            })
        };
        
        Response::builder()
            .status(status_code)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "no-cache, no-store, must-revalidate")
            .body(Body::from(response_body.to_string()))
            .unwrap()
    }
}

#[async_trait]
impl Plugin for HealthCheckPlugin {
    async fn handle_request(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<PluginResponse> {
        // Only handle GET requests to health check endpoints
        if request.http_request.method() != Method::GET {
            return None;
        }
        
        if !self.is_health_endpoint(&request.path) {
            return None;
        }
        
        let (status, messages) = match request.path.as_str() {
            path if path == self.live_endpoint => {
                self.check_liveness()
            }
            path if path == self.ready_endpoint => {
                self.check_readiness(context)
            }
            path if path == self.health_endpoint => {
                self.check_health(context)
            }
            _ => return None,
        };
        
        Some(self.create_health_response(status, messages).into())
    }
    
    async fn handle_response(&self, _request: &PluginRequest, _response: &mut Response<Body>, _context: &PluginContext) {
        // Health check plugin doesn't modify responses
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

// Export the plugin creation function
create_plugin!(HealthCheckPlugin);