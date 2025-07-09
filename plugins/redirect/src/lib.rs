use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, PluginResponse, create_plugin};
use async_trait::async_trait;
use hyper::{Body, Response, StatusCode, header::LOCATION};
use std::collections::HashMap;
use regex::Regex;

/// Redirect rule configuration
#[derive(Debug, Clone)]
pub struct RedirectRule {
    pub pattern: Regex,
    pub replacement: String,
    pub status_code: u16,
    pub conditions: Vec<String>,
}

/// Plugin for URL redirection with pattern matching
#[derive(Debug)]
pub struct RedirectPlugin {
    name: String,
    rules: Vec<RedirectRule>,
}

impl RedirectPlugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        let name = config.get("name").cloned().unwrap_or_else(|| "redirect".to_string());
        let default_status_code = config.get("default_status_code")
            .and_then(|v| v.parse().ok())
            .unwrap_or(302);
        
        let mut rules = Vec::new();
        
        // Parse redirect rules from config
        // Format: redirect_rule_N_pattern, redirect_rule_N_replacement, redirect_rule_N_status
        let mut rule_indices = std::collections::HashSet::new();
        
        for key in config.keys() {
            if key.starts_with("redirect_rule_") && key.ends_with("_pattern") {
                if let Some(index_str) = key.strip_prefix("redirect_rule_").and_then(|s| s.strip_suffix("_pattern")) {
                    if let Ok(index) = index_str.parse::<usize>() {
                        rule_indices.insert(index);
                    }
                }
            }
        }
        
        for index in rule_indices {
            let pattern_key = format!("redirect_rule_{}_pattern", index);
            let replacement_key = format!("redirect_rule_{}_replacement", index);
            let status_key = format!("redirect_rule_{}_status", index);
            let condition_key = format!("redirect_rule_{}_condition", index);
            
            if let (Some(pattern_str), Some(replacement)) = (config.get(&pattern_key), config.get(&replacement_key)) {
                if let Ok(pattern) = Regex::new(pattern_str) {
                    let status_code = config.get(&status_key)
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(default_status_code);
                    
                    let conditions = config.get(&condition_key)
                        .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                        .unwrap_or_else(Vec::new);
                    
                    rules.push(RedirectRule {
                        pattern,
                        replacement: replacement.clone(),
                        status_code,
                        conditions,
                    });
                }
            }
        }
        
        // Add some default rules if none are configured
        if rules.is_empty() {
            // Example: redirect /old-page to /new-page
            if let Ok(pattern) = Regex::new("^/old-page$") {
                rules.push(RedirectRule {
                    pattern,
                    replacement: "/new-page".to_string(),
                    status_code: 301,
                    conditions: Vec::new(),
                });
            }
            
            // Example: redirect /api/v1/* to /api/v2/*
            if let Ok(pattern) = Regex::new("^/api/v1/(.*)$") {
                rules.push(RedirectRule {
                    pattern,
                    replacement: "/api/v2/$1".to_string(),
                    status_code: 302,
                    conditions: Vec::new(),
                });
            }
        }
        
        Self { name, rules }
    }
    
    /// Check if redirect conditions are met
    fn check_conditions(&self, conditions: &[String], request: &PluginRequest, _context: &PluginContext) -> bool {
        for condition in conditions {
            match condition.as_str() {
                "https_only" => {
                    // Check if request is HTTPS
                    if let Some(proto) = request.http_request.headers().get("x-forwarded-proto") {
                        if let Ok(proto_str) = proto.to_str() {
                            if proto_str.to_lowercase() != "https" {
                                return false;
                            }
                        }
                    }
                }
                "http_only" => {
                    // Check if request is HTTP
                    if let Some(proto) = request.http_request.headers().get("x-forwarded-proto") {
                        if let Ok(proto_str) = proto.to_str() {
                            if proto_str.to_lowercase() != "http" {
                                return false;
                            }
                        }
                    }
                }
                _ => {
                    // Custom conditions can be added here
                    // For now, unknown conditions are ignored
                }
            }
        }
        true
    }
    
    /// Find matching redirect rule
    fn find_redirect_rule(&self, path: &str, request: &PluginRequest, context: &PluginContext) -> Option<(String, u16)> {
        for rule in &self.rules {
            if rule.pattern.is_match(path) {
                // Check conditions
                if self.check_conditions(&rule.conditions, request, context) {
                    // Apply regex replacement
                    let new_path = rule.pattern.replace(path, &rule.replacement).to_string();
                    return Some((new_path, rule.status_code));
                }
            }
        }
        None
    }
    
    /// Create redirect response
    fn create_redirect_response(&self, location: &str, status_code: u16) -> Response<Body> {
        let status = match status_code {
            301 => StatusCode::MOVED_PERMANENTLY,
            302 => StatusCode::FOUND,
            303 => StatusCode::SEE_OTHER,
            307 => StatusCode::TEMPORARY_REDIRECT,
            308 => StatusCode::PERMANENT_REDIRECT,
            _ => StatusCode::FOUND,
        };
        
        Response::builder()
            .status(status)
            .header(LOCATION, location)
            .header("Content-Type", "text/plain")
            .body(Body::from(format!("Redirecting to: {}", location)))
            .unwrap()
    }
}

#[async_trait]
impl Plugin for RedirectPlugin {
    async fn handle_request(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<PluginResponse> {
        // Check if the request path matches any redirect rules
        if let Some((new_location, status_code)) = self.find_redirect_rule(&request.path, request, context) {
            // Create redirect response
            let response = self.create_redirect_response(&new_location, status_code);
            
            // Log the redirect
            context.log_verbose(&format!("[Redirect] {} -> {} ({})", request.path, new_location, status_code));
            
            return Some(response.into());
        }
        
        // No redirect needed, continue to next plugin
        None
    }
    
    async fn handle_response(&self, _request: &PluginRequest, _response: &mut Response<Body>, _context: &PluginContext) {
        // Redirect plugin doesn't modify responses
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

// Export the plugin creation function
create_plugin!(RedirectPlugin);