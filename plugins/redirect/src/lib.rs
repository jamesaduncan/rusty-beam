//! Redirect Plugin for Rusty Beam
//!
//! This plugin provides URL redirection capabilities with pattern matching support.
//! It can handle both request redirects (immediate redirects) and response redirects
//! (redirects triggered by specific HTTP response codes).
//!
//! ## Features
//! - **Pattern-based redirects**: Uses regex patterns for flexible URL matching
//! - **Request redirects**: Immediate redirects based on incoming request paths
//! - **Response redirects**: Conditional redirects triggered by specific response codes
//! - **Conditional logic**: Support for redirect conditions (https_only, http_only)
//! - **Configurable status codes**: Support for 301, 302, 303, 307, 308 redirects
//! - **Microdata configuration**: Rules defined in HTML files using microdata
//!
//! ## Configuration
//! Redirect rules are loaded from HTML files using microdata with the schema:
//! `http://rustybeam.net/RedirectRule`
//!
//! ## Rule Types
//! - **Request Redirects**: Process incoming requests and redirect immediately
//! - **Response Redirects**: Triggered by specific HTTP response codes (404, 500, etc.)
//!
//! ## Usage
//! Configure redirect rules in your HTML configuration file and include the
//! redirect plugin in your pipeline. The plugin will automatically process
//! matching requests and responses according to the defined rules.

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
    pub response_triggers: Vec<u16>, // Response codes that trigger this redirect
}

impl RedirectRule {
    /// Check if this is a response redirect (has response triggers)
    pub fn is_response_redirect(&self) -> bool {
        !self.response_triggers.is_empty()
    }
    
    /// Check if this is a request redirect (no response triggers)
    pub fn is_request_redirect(&self) -> bool {
        self.response_triggers.is_empty()
    }
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
        let file_path = self.normalize_file_path(rules_file);
        
        if !Path::new(&file_path).exists() {
            return None;
        }
        
        let html_content = std::fs::read_to_string(&file_path).ok()?;
        let document = Document::from(html_content.as_str());
        
        let rules = self.parse_redirect_rules_from_document(&document);
        
        if rules.is_empty() {
            None
        } else {
            Some(rules)
        }
    }
    
    /// Normalize file path by removing file:// prefix if present
    fn normalize_file_path(&self, file_path: &str) -> String {
        if file_path.starts_with("file://") {
            file_path[7..].to_string()
        } else {
            file_path.to_string()
        }
    }
    
    /// Parse redirect rules from HTML document
    fn parse_redirect_rules_from_document(&self, document: &Document) -> Vec<RedirectRule> {
        let items = document.select("[itemscope]");
        let mut rules = Vec::new();
        
        for item in items.iter() {
            if let Some(redirect_rule) = self.parse_single_redirect_rule(&item) {
                rules.push(redirect_rule);
            }
        }
        
        rules
    }
    
    /// Parse a single redirect rule from a microdata item
    fn parse_single_redirect_rule(&self, item: &dom_query::Selection) -> Option<RedirectRule> {
        // Check if this is a redirect rule item
        let item_type = item.attr("itemtype")?;
        if item_type != "http://rustybeam.net/RedirectRule".into() {
            return None;
        }
        
        let from = self.get_microdata_property(item, "from")?;
        let to = self.get_microdata_property(item, "to")?;
        
        if from.is_empty() || to.is_empty() {
            return None;
        }
        
        let pattern = Regex::new(&from).ok()?;
        let status_code = self.parse_status_code(item);
        let conditions = self.parse_conditions(item);
        let response_triggers = self.parse_response_triggers(item);
        
        Some(RedirectRule {
            pattern,
            replacement: to,
            status_code,
            conditions,
            response_triggers,
        })
    }
    
    /// Parse status code from microdata, defaulting to 302
    fn parse_status_code(&self, item: &dom_query::Selection) -> u16 {
        self.get_microdata_property(item, "status")
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(302)
    }
    
    /// Parse conditions from microdata
    fn parse_conditions(&self, item: &dom_query::Selection) -> Vec<String> {
        self.get_microdata_property(item, "conditions")
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_else(Vec::new)
    }
    
    /// Parse response triggers from microdata
    fn parse_response_triggers(&self, item: &dom_query::Selection) -> Vec<u16> {
        self.get_microdata_properties(item, "on")
            .into_iter()
            .filter_map(|s| s.parse::<u16>().ok())
            .collect()
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
    
    /// Get all microdata property values from an element (for properties that can appear multiple times)
    fn get_microdata_properties(&self, element: &Selection, property: &str) -> Vec<String> {
        let prop_elements = element.select(&format!("[itemprop='{}']", property));
        let mut values = Vec::new();
        
        for i in 0..prop_elements.length() {
            if let Some(prop_element) = prop_elements.get(i) {
                let text = prop_element.text().trim().to_string();
                if !text.is_empty() {
                    values.push(text);
                }
            }
        }
        
        values
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
                response_triggers: Vec::new(),
            });
        }
        
        // Example: redirect /api/v1/* to /api/v2/*
        if let Ok(pattern) = Regex::new("^/api/v1/(.*)$") {
            rules.push(RedirectRule {
                pattern,
                replacement: "/api/v2/$1".to_string(),
                status_code: 302,
                conditions: Vec::new(),
                response_triggers: Vec::new(),
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
    
    /// Find matching redirect rule based on type and optional response code
    fn find_matching_redirect_rule(
        &self, 
        path: &str, 
        request: &PluginRequest, 
        context: &PluginContext, 
        response_code: Option<u16>
    ) -> Option<(String, u16)> {
        let rules = self.load_redirect_rules();
        for rule in &rules {
            if self.rule_matches(rule, path, response_code) {
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
    
    /// Check if a rule matches the given criteria
    fn rule_matches(&self, rule: &RedirectRule, path: &str, response_code: Option<u16>) -> bool {
        // Check if pattern matches path
        if !rule.pattern.is_match(path) {
            return false;
        }
        
        match response_code {
            Some(code) => {
                // For response redirects, check if rule has response triggers and matches the code
                rule.is_response_redirect() && rule.response_triggers.contains(&code)
            }
            None => {
                // For request redirects, rule should not have response triggers
                rule.is_request_redirect()
            }
        }
    }
    
    /// Find matching redirect rule for requests
    fn find_request_redirect_rule(&self, path: &str, request: &PluginRequest, context: &PluginContext) -> Option<(String, u16)> {
        self.find_matching_redirect_rule(path, request, context, None)
    }
    
    /// Find matching redirect rule for responses
    fn find_response_redirect_rule(&self, path: &str, response_code: u16, request: &PluginRequest, context: &PluginContext) -> Option<(String, u16)> {
        self.find_matching_redirect_rule(path, request, context, Some(response_code))
    }
    
    /// Create redirect response with proper HTTP status code
    fn create_redirect_response(&self, location: &str, status_code: u16) -> Response<Body> {
        let status = self.map_status_code_to_hyper(status_code);
        
        Response::builder()
            .status(status)
            .header(LOCATION, location)
            .header("Content-Type", "text/plain")
            .body(Body::from(format!("Redirecting to: {}", location)))
            .unwrap_or_else(|_| {
                // Fallback response if header creation fails
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("Failed to create redirect response"))
                    .unwrap()
            })
    }
    
    /// Map redirect status code to Hyper StatusCode
    fn map_status_code_to_hyper(&self, status_code: u16) -> StatusCode {
        match status_code {
            301 => StatusCode::MOVED_PERMANENTLY,
            302 => StatusCode::FOUND,
            303 => StatusCode::SEE_OTHER,
            307 => StatusCode::TEMPORARY_REDIRECT,
            308 => StatusCode::PERMANENT_REDIRECT,
            _ => StatusCode::FOUND, // Default to 302 Found for invalid codes
        }
    }
}

#[async_trait]
impl Plugin for RedirectPlugin {
    async fn handle_request(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<PluginResponse> {
        // Check if the request path matches any request redirect rules
        if let Some((new_location, status_code)) = self.find_request_redirect_rule(&request.path, request, context) {
            // Create redirect response
            let response = self.create_redirect_response(&new_location, status_code);
            
            // Log the redirect
            context.log_verbose(&format!("[Redirect] Request {} -> {} ({})", request.path, new_location, status_code));
            
            return Some(response.into());
        }
        
        // No redirect needed, continue to next plugin
        None
    }
    
    async fn handle_response(&self, request: &PluginRequest, response: &mut Response<Body>, context: &PluginContext) {
        // Check if the response status matches any response redirect rules
        let response_code = response.status().as_u16();
        
        if let Some((new_location, redirect_status_code)) = self.find_response_redirect_rule(&request.path, response_code, request, context) {
            // Create redirect response
            let redirect_response = self.create_redirect_response(&new_location, redirect_status_code);
            
            // Log the redirect
            context.log_verbose(&format!("[Redirect] Response {} {} -> {} ({})", response_code, request.path, new_location, redirect_status_code));
            
            // Replace the response
            *response = redirect_response;
        }
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

// Export the plugin creation function
create_plugin!(RedirectPlugin);