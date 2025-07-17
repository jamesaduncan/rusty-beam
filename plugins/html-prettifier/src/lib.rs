//! HTML Prettifier Plugin for Rusty Beam
//!
//! This plugin automatically formats and prettifies HTML responses to improve readability
//! and maintainability. It processes HTML content after modification operations to ensure
//! well-formatted output.
//!
//! ## Features
//! - **Automatic HTML Formatting**: Prettifies HTML responses with proper indentation
//! - **Method-Specific Processing**: Only processes responses from PUT, POST, DELETE operations
//! - **Content-Type Filtering**: Only processes HTML content types
//! - **DOCTYPE Preservation**: Maintains DOCTYPE declarations in formatted output
//! - **Self-Closing Tag Support**: Properly handles void elements like `<img>`, `<br>`, etc.
//! - **Mixed Content Handling**: Processes both element and text content appropriately
//!
//! ## Operation
//! This plugin operates in the response phase, processing HTML content after it has been
//! modified by other plugins. It formats the HTML with consistent indentation and structure
//! while preserving semantic meaning.
//!
//! ## Pipeline Integration
//! This plugin should be placed late in the pipeline, after content modification plugins
//! like file-handler and selector-handler, but before logging plugins.

use async_trait::async_trait;
use dom_query::Document;
use http::{Method, StatusCode};
use hyper::{Body, Response};
use rusty_beam_plugin_api::{
    create_plugin, Plugin, PluginContext, PluginRequest, PluginResponse,
};
use std::collections::HashMap;

// Default configuration values
const DEFAULT_PLUGIN_NAME: &str = "html-prettifier";
const DEFAULT_INDENT: &str = "  ";

// Content-Type detection
const CONTENT_TYPE_HEADER: &str = "content-type";
const CONTENT_TYPE_HTML: &str = "text/html";
const CONTENT_LENGTH_HEADER: &str = "content-length";

// HTML parsing constants
const DOCTYPE_PREFIX: &str = "<!DOCTYPE";
const DOCTYPE_PREFIX_LOWER: &str = "<!doctype";
const HTML_ELEMENT_SELECTOR: &str = "html";
const BODY_ELEMENT_SELECTOR: &str = "body";
const BODY_CHILDREN_SELECTOR: &str = "body > *";
const ALL_ELEMENTS_SELECTOR: &str = "*";
const DEFAULT_TAG_NAME: &str = "div";

// Self-closing HTML tags (void elements)
const SELF_CLOSING_TAGS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", 
    "link", "meta", "param", "source", "track", "wbr"
];

// HTML syntax characters
const ANGLE_BRACKET_OPEN: char = '<';
const ANGLE_BRACKET_CLOSE: char = '>';
const FORWARD_SLASH: char = '/';
const NEWLINE: char = '\n';
const SPACE: char = ' ';

/// HTML Prettifier Plugin for formatting HTML responses
#[derive(Debug)]
pub struct HtmlPrettifierPlugin {
    _config: HashMap<String, String>,
}

impl HtmlPrettifierPlugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        Self { _config: config }
    }

    /// Determines if the response should be prettified based on HTTP method
    /// 
    /// Only prettifies responses from modification operations (PUT, POST, DELETE)
    /// to avoid unnecessary processing of read-only operations.
    fn should_prettify(&self, method: &Method) -> bool {
        matches!(
            method,
            &Method::PUT | &Method::POST | &Method::DELETE
        )
    }

    /// Checks if the response has HTML content type
    /// 
    /// Only processes responses with HTML content to avoid formatting
    /// non-HTML content like JSON, CSS, or JavaScript.
    fn is_html_content_type(&self, response: &Response<Body>) -> bool {
        if let Some(content_type) = response.headers().get(CONTENT_TYPE_HEADER) {
            if let Ok(content_type_str) = content_type.to_str() {
                return content_type_str.contains(CONTENT_TYPE_HTML);
            }
        }
        false
    }

    fn prettify_html(&self, html: &str) -> Result<String, Box<dyn std::error::Error>> {
        // First check for DOCTYPE
        let mut result = String::new();
        let trimmed = html.trim_start();
        
        if trimmed.starts_with(DOCTYPE_PREFIX) || trimmed.starts_with(DOCTYPE_PREFIX_LOWER) {
            if let Some(doctype_end) = html.find(ANGLE_BRACKET_CLOSE) {
                result.push_str(&html[..=doctype_end]);
                result.push(NEWLINE);
            }
        }
        
        // Parse the HTML document
        let doc = Document::from(html);
        
        // Select the root html element
        let html_elements = doc.select(HTML_ELEMENT_SELECTOR);
        if html_elements.length() > 0 {
            // Get the entire HTML element as a string and prettify it
            let html_content = html_elements.html();
            let prettified = self.prettify_element_html(&html_content, 0);
            result.push_str(&prettified);
        } else {
            // No html tag found, try to prettify what we have
            let prettified = self.prettify_fragment(html, 0);
            result.push_str(&prettified);
        }
        
        Ok(result)
    }
    
    /// Prettifies a single HTML element with proper indentation
    /// 
    /// Processes the HTML element and its children recursively to create
    /// properly formatted output with consistent indentation.
    fn prettify_element_html(&self, html: &str, depth: usize) -> String {
        let mut output = String::new();
        let _indent = DEFAULT_INDENT.repeat(depth);
        
        // Parse as fragment to process this element
        let doc = Document::from(html);
        
        // Process all top-level elements
        let root_selector = doc.select("*");
        if root_selector.length() == 0 {
            return html.to_string();
        }
        
        // Get the first element (which should be our root)
        let first = root_selector.first();
        if first.length() > 0 {
            output.push_str(&self.process_element(&first, depth));
        }
        
        output
    }
    
    fn prettify_fragment(&self, html: &str, depth: usize) -> String {
        let doc = Document::from(html);
        let mut output = String::new();
        
        // Try to select body content or just get everything
        let elements = if doc.select(BODY_ELEMENT_SELECTOR).length() > 0 {
            doc.select(BODY_CHILDREN_SELECTOR)
        } else {
            doc.select(ALL_ELEMENTS_SELECTOR)
        };
        
        for element in elements.iter() {
            output.push_str(&self.process_element(&element, depth));
        }
        
        output
    }
    
    /// Processes a single DOM element and formats it with proper indentation
    /// 
    /// Handles both self-closing and regular elements, applying appropriate
    /// formatting rules for each element type.
    fn process_element(&self, element: &dom_query::Selection, depth: usize) -> String {
        let mut output = String::new();
        let indent = DEFAULT_INDENT.repeat(depth);
        
        // Get the tag name from the HTML
        let html = element.html();
        let tag_name = self.extract_tag_name(&html).unwrap_or_else(|| DEFAULT_TAG_NAME.to_string());
        
        // Self-closing tags
        if SELF_CLOSING_TAGS.contains(&tag_name.as_str()) {
            // Extract just the tag with attributes
            if let Some(end) = html.find(ANGLE_BRACKET_CLOSE) {
                output.push_str(&indent);
                output.push_str(&html[..end]);
                output.push(SPACE);
                output.push(FORWARD_SLASH);
                output.push(ANGLE_BRACKET_CLOSE);
                output.push(NEWLINE);
            }
            return output;
        }
        
        // Start tag with attributes
        output.push_str(&indent);
        if let Some(tag_end) = html.find(ANGLE_BRACKET_CLOSE) {
            output.push_str(&html[..=tag_end]);
        } else {
            output.push(ANGLE_BRACKET_OPEN);
            output.push_str(&tag_name);
            output.push(ANGLE_BRACKET_CLOSE);
        }
        
        // Get inner content
        let inner_html = element.inner_html();
        let inner_trimmed = inner_html.trim();
        
        // Check if content has nested elements
        let has_elements = inner_html.contains(ANGLE_BRACKET_OPEN) && inner_html.contains(ANGLE_BRACKET_CLOSE);
        
        if has_elements {
            output.push(NEWLINE);
            // Process children recursively
            let children = element.children();
            if children.length() > 0 {
                for child in children.iter() {
                    output.push_str(&self.process_element(&child, depth + 1));
                }
            } else {
                // Has HTML but no selectable children, might be text with inline elements
                output.push_str(&self.prettify_mixed_content(&inner_html, depth + 1));
            }
            output.push_str(&indent);
        } else if !inner_trimmed.is_empty() {
            // Just text content
            output.push_str(inner_trimmed);
        }
        
        // Close tag
        output.push(ANGLE_BRACKET_OPEN);
        output.push(FORWARD_SLASH);
        output.push_str(&tag_name);
        output.push(ANGLE_BRACKET_CLOSE);
        output.push(NEWLINE);
        
        output
    }
    
    fn prettify_mixed_content(&self, content: &str, depth: usize) -> String {
        let doc = Document::from(content);
        let mut output = String::new();
        
        // Get all elements in the content
        let elements = doc.select(ALL_ELEMENTS_SELECTOR);
        
        if elements.length() > 0 {
            for element in elements.iter() {
                output.push_str(&self.process_element(&element, depth));
            }
        } else {
            // Just text
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                let indent = DEFAULT_INDENT.repeat(depth);
                output.push_str(&indent);
                output.push_str(trimmed);
                output.push(NEWLINE);
            }
        }
        
        output
    }
    
    /// Extracts the tag name from HTML element string
    /// 
    /// Parses the HTML string to extract the element tag name,
    /// handling both simple tags and tags with attributes.
    fn extract_tag_name(&self, html: &str) -> Option<String> {
        let trimmed = html.trim_start();
        if trimmed.starts_with(ANGLE_BRACKET_OPEN) {
            let tag_start = 1;
            if let Some(tag_end) = trimmed[tag_start..].find(|c: char| c.is_whitespace() || c == ANGLE_BRACKET_CLOSE || c == FORWARD_SLASH) {
                return Some(trimmed[tag_start..tag_start + tag_end].to_lowercase());
            }
        }
        None
    }
}

#[async_trait]
impl Plugin for HtmlPrettifierPlugin {
    async fn handle_request(
        &self,
        _request: &mut PluginRequest,
        _context: &PluginContext,
    ) -> Option<PluginResponse> {
        None
    }

    async fn handle_response(
        &self,
        request: &PluginRequest,
        response: &mut Response<Body>,
        context: &PluginContext,
    ) {
        // Only prettify for PUT, POST, DELETE methods
        if !self.should_prettify(&request.http_request.method()) {
            return;
        }

        // Only prettify HTML responses
        if !self.is_html_content_type(response) {
            return;
        }

        // Skip error responses
        if response.status() != StatusCode::OK {
            return;
        }

        // Extract the body
        let body = std::mem::replace(response.body_mut(), Body::empty());
        
        // Convert body to bytes
        let body_bytes = match hyper::body::to_bytes(body).await {
            Ok(bytes) => bytes,
            Err(e) => {
                context.log_verbose(&format!(
                    "html-prettifier: Failed to read response body: {}",
                    e
                ));
                return;
            }
        };

        // Convert to string
        let html_str = match String::from_utf8(body_bytes.to_vec()) {
            Ok(s) => s,
            Err(e) => {
                context.log_verbose(&format!(
                    "html-prettifier: Response body is not valid UTF-8: {}",
                    e
                ));
                *response.body_mut() = Body::from(body_bytes);
                return;
            }
        };

        // Prettify the HTML
        match self.prettify_html(&html_str) {
            Ok(prettified) => {
                context.log_verbose(&format!(
                    "html-prettifier: Prettified HTML response for {} {} ({}B -> {}B)",
                    request.http_request.method(),
                    request.http_request.uri().path(),
                    html_str.len(),
                    prettified.len()
                ));
                
                // Update content-length header
                response.headers_mut().insert(
                    CONTENT_LENGTH_HEADER,
                    prettified.len().to_string().parse().unwrap(),
                );
                
                *response.body_mut() = Body::from(prettified);
            }
            Err(e) => {
                context.log_verbose(&format!(
                    "html-prettifier: Failed to prettify HTML: {}",
                    e
                ));
                // Restore original body
                *response.body_mut() = Body::from(body_bytes);
            }
        }
    }

    fn name(&self) -> &str {
        DEFAULT_PLUGIN_NAME
    }
}

create_plugin!(HtmlPrettifierPlugin);