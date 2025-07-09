use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, PluginResponse, create_plugin};
use async_trait::async_trait;
use hyper::{Body, Response, StatusCode, header::LOCATION};
use std::collections::HashMap;
use regex::Regex;
use dom_query::{Document, Selection};
use std::path::Path;

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
    rules_file: Option<String>,
    config: HashMap<String, String>,
}

impl RedirectPlugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        let name = config.get("name").cloned().unwrap_or_else(|| "redirect".to_string());
        let rules_file = config.get("rulesfile").cloned();
        
        Self { name, rules_file, config }
    }
    
    /// Load redirect rules from HTML file or config
    fn load_redirect_rules(&self) -> Vec<RedirectRule> {
        // First, try to load from rules file if specified
        if let Some(rules_file) = &self.rules_file {
            if let Some(rules) = self.load_rules_from_file(rules_file) {
                return rules;
            }
        }
        
        // If no rules file or loading failed, check if we have the config file path
        if let Some(config_file) = self.config.get("config_file") {
            if let Some(rules) = self.load_rules_from_file(config_file) {
                return rules;
            }
        }
        
        // Fall back to default rules
        self.get_default_rules()
    }
    
    /// Load redirect rules from a specific HTML file
    fn load_rules_from_file(&self, rules_file: &str) -> Option<Vec<RedirectRule>> {
        // Handle file:// URLs
        let file_path = if rules_file.starts_with("file://") {
            &rules_file[7..]
        } else {
            rules_file
        };
        
        // Check if file exists
        if !Path::new(file_path).exists() {
            return None;
        }
        
        // Read the HTML file
        let html_content = match std::fs::read_to_string(file_path) {
            Ok(content) => content,
            Err(_) => return None,
        };
        
        // Parse the HTML
        let document = Document::from(html_content.as_str());
        let items = document.select("[itemscope]");
        
        let mut rules = Vec::new();
        
        // Load redirect rules
        for item in items.iter() {
            if let Some(item_type) = item.attr("itemtype") {
                if item_type == "http://rustybeam.net/RedirectRule".into() {
                    let from = self.get_microdata_property(&item, "from").unwrap_or_default();
                    let to = self.get_microdata_property(&item, "to").unwrap_or_default();
                    let status = self.get_microdata_property(&item, "status")
                        .and_then(|s| s.parse::<u16>().ok())
                        .unwrap_or(302);
                    let conditions = self.get_microdata_property(&item, "conditions")
                        .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                        .unwrap_or_else(Vec::new);
                    
                    if !from.is_empty() && !to.is_empty() {
                        if let Ok(pattern) = Regex::new(&from) {
                            rules.push(RedirectRule {
                                pattern,
                                replacement: to,
                                status_code: status,
                                conditions,
                            });
                        }
                    }
                }
            }
        }
        
        if rules.is_empty() {
            None
        } else {
            Some(rules)
        }
    }
    
    /// Get a microdata property value from an element
    fn get_microdata_property(&self, element: &Selection, property: &str) -> Option<String> {
        // Find elements with the specified itemprop
        let prop_elements = element.select(&format!("[itemprop='{}']" , property));
        if prop_elements.length() > 0 {
            Some(prop_elements.text().trim().to_string())
        } else {
            None
        }
    }
    
    /// Get default redirect rules
    fn get_default_rules(&self) -> Vec<RedirectRule> {
        let mut rules = Vec::new();
        
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
        
        rules
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
        let rules = self.load_redirect_rules();
        for rule in &rules {
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