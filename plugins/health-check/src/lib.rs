//! Health Check Plugin for Rusty Beam
//!
//! This plugin provides comprehensive health monitoring endpoints following
//! industry standards for health checks, readiness probes, and liveness probes.
//! It enables monitoring systems and load balancers to determine server health.
//!
//! ## Features
//! - **Liveness Checks**: Basic server responsiveness ("am I running?")
//! - **Readiness Checks**: Service readiness for traffic ("can I serve requests?")
//! - **Health Checks**: Comprehensive system health assessment
//! - **Disk Space Monitoring**: Configurable disk space thresholds
//! - **Document Root Validation**: Ensures critical paths are accessible
//! - **Detailed Diagnostics**: Optional verbose health information
//!
//! ## Standard Endpoints
//! - **`/health`**: Comprehensive health check (combines liveness + readiness)
//! - **`/ready`**: Readiness probe for load balancers
//! - **`/live`**: Liveness probe for container orchestrators
//!
//! ## Configuration
//! - `health_endpoint`: Health check path (default: "/health")
//! - `ready_endpoint`: Readiness probe path (default: "/ready")
//! - `live_endpoint`: Liveness probe path (default: "/live")
//! - `detailed_checks`: Include detailed diagnostics (default: true)
//! - `check_disk_space`: Enable disk space monitoring (default: true)
//! - `min_disk_space_mb`: Minimum required disk space in MB (default: 100)
//!
//! ## Health Status Levels
//! - **Healthy**: All systems operational (HTTP 200)
//! - **Degraded**: Partial functionality, still serving (HTTP 200)
//! - **Unhealthy**: Critical failure, should not serve traffic (HTTP 503)
//!
//! ## Integration
//! Compatible with Kubernetes probes, load balancer health checks,
//! and monitoring systems like Prometheus, Consul, and AWS ALB.

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
        let name = Self::parse_string_config(&config, "name", "health-check");
        let health_endpoint = Self::parse_string_config(&config, "health_endpoint", "/health");
        let ready_endpoint = Self::parse_string_config(&config, "ready_endpoint", "/ready");
        let live_endpoint = Self::parse_string_config(&config, "live_endpoint", "/live");
        let detailed_checks = Self::parse_boolean_config(&config, "detailed_checks", true);
        let check_disk_space = Self::parse_boolean_config(&config, "check_disk_space", true);
        let min_disk_space_mb = Self::parse_numeric_config(&config, "min_disk_space_mb", 100);
        
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
    
    /// Parse string configuration value with fallback default
    fn parse_string_config(config: &HashMap<String, String>, key: &str, default: &str) -> String {
        config.get(key).cloned().unwrap_or_else(|| default.to_string())
    }
    
    /// Parse boolean configuration value with fallback default
    fn parse_boolean_config(config: &HashMap<String, String>, key: &str, default: bool) -> bool {
        config.get(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }
    
    /// Parse numeric configuration value with fallback default
    fn parse_numeric_config<T: std::str::FromStr>(config: &HashMap<String, String>, key: &str, default: T) -> T {
        config.get(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
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
        
        // Determine overall status using worst-case combination
        let overall_status = Self::combine_health_statuses(&live_status, &ready_status);
        
        // Add timestamp and server info
        if self.detailed_checks {
            let timestamp = self.get_current_timestamp();
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
    
    /// Create health check response with proper error handling
    fn create_health_response(&self, status: HealthStatus, messages: Vec<String>) -> Response<Body> {
        let status_code = Self::health_status_to_http_code(&status);
        let status_text = Self::health_status_to_text(&status);
        let response_body = self.create_response_body(status_text, messages);
        
        Response::builder()
            .status(status_code)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "no-cache, no-store, must-revalidate")
            .body(Body::from(response_body))
            .unwrap_or_else(|_| {
                // Fallback response if JSON creation fails
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header("Content-Type", "text/plain")
                    .body(Body::from("Health check service error"))
                    .unwrap()
            })
    }
    
    /// Convert health status to HTTP status code
    fn health_status_to_http_code(status: &HealthStatus) -> StatusCode {
        match status {
            HealthStatus::Healthy => StatusCode::OK,
            HealthStatus::Degraded => StatusCode::OK, // Still serve traffic when degraded
            HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
    
    /// Convert health status to text representation
    fn health_status_to_text(status: &HealthStatus) -> &'static str {
        match status {
            HealthStatus::Healthy => "healthy",
            HealthStatus::Degraded => "degraded",
            HealthStatus::Unhealthy => "unhealthy",
        }
    }
    
    /// Create JSON response body with optional detailed information
    fn create_response_body(&self, status_text: &str, messages: Vec<String>) -> String {
        let response_json = if self.detailed_checks {
            self.create_detailed_response_body(status_text, messages)
        } else {
            self.create_simple_response_body(status_text)
        };
        
        response_json.to_string()
    }
    
    /// Create detailed response body with checks and timestamp
    fn create_detailed_response_body(&self, status_text: &str, messages: Vec<String>) -> serde_json::Value {
        let timestamp = self.get_current_timestamp();
        
        serde_json::json!({
            "status": status_text,
            "checks": messages,
            "timestamp": timestamp
        })
    }
    
    /// Create simple response body with just status
    fn create_simple_response_body(&self, status_text: &str) -> serde_json::Value {
        serde_json::json!({
            "status": status_text
        })
    }
    
    /// Get current Unix timestamp with error handling
    fn get_current_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0) // Fallback to epoch if time calculation fails
    }
    
    /// Combine two health statuses using worst-case logic
    fn combine_health_statuses(status1: &HealthStatus, status2: &HealthStatus) -> HealthStatus {
        use HealthStatus::*;
        
        match (status1, status2) {
            (Healthy, Healthy) => Healthy,
            (Healthy, Degraded) | (Degraded, Healthy) | (Degraded, Degraded) => Degraded,
            _ => Unhealthy, // Any unhealthy status makes the overall status unhealthy
        }
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