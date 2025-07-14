use async_trait::async_trait;
use dom_query::Document;
use http::{Method, StatusCode};
use hyper::{Body, Response};
use rusty_beam_plugin_api::{
    create_plugin, Plugin, PluginContext, PluginRequest, PluginResponse,
};
use std::collections::HashMap;

#[derive(Debug)]
pub struct HtmlPrettifierPlugin {
    _config: HashMap<String, String>,
}

impl HtmlPrettifierPlugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        Self { _config: config }
    }

    fn should_prettify(&self, method: &Method) -> bool {
        matches!(
            method,
            &Method::PUT | &Method::POST | &Method::DELETE
        )
    }

    fn is_html_content_type(&self, response: &Response<Body>) -> bool {
        if let Some(content_type) = response.headers().get("content-type") {
            if let Ok(content_type_str) = content_type.to_str() {
                return content_type_str.contains("text/html");
            }
        }
        false
    }

    fn prettify_html(&self, html: &str) -> Result<String, Box<dyn std::error::Error>> {
        // First check for DOCTYPE
        let mut result = String::new();
        let trimmed = html.trim_start();
        
        if trimmed.starts_with("<!DOCTYPE") || trimmed.starts_with("<!doctype") {
            if let Some(doctype_end) = html.find('>') {
                result.push_str(&html[..=doctype_end]);
                result.push('\n');
            }
        }
        
        // Parse the HTML document
        let doc = Document::from(html);
        
        // Select the root html element
        let html_elements = doc.select("html");
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
    
    fn prettify_element_html(&self, html: &str, depth: usize) -> String {
        let mut output = String::new();
        let indent = "  ".repeat(depth);
        
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
        let elements = if doc.select("body").length() > 0 {
            doc.select("body > *")
        } else {
            doc.select("*")
        };
        
        for element in elements.iter() {
            output.push_str(&self.process_element(&element, depth));
        }
        
        output
    }
    
    fn process_element(&self, element: &dom_query::Selection, depth: usize) -> String {
        let mut output = String::new();
        let indent = "  ".repeat(depth);
        
        // Get the tag name from the HTML
        let html = element.html();
        let tag_name = self.extract_tag_name(&html).unwrap_or_else(|| "div".to_string());
        
        // Self-closing tags
        if matches!(tag_name.as_str(), "area" | "base" | "br" | "col" | "embed" | 
                   "hr" | "img" | "input" | "link" | "meta" | "param" | 
                   "source" | "track" | "wbr") {
            // Extract just the tag with attributes
            if let Some(end) = html.find('>') {
                output.push_str(&indent);
                output.push_str(&html[..end]);
                output.push_str(" />");
                output.push('\n');
            }
            return output;
        }
        
        // Start tag with attributes
        output.push_str(&indent);
        if let Some(tag_end) = html.find('>') {
            output.push_str(&html[..=tag_end]);
        } else {
            output.push('<');
            output.push_str(&tag_name);
            output.push('>');
        }
        
        // Get inner content
        let inner_html = element.inner_html();
        let inner_trimmed = inner_html.trim();
        
        // Check if content has nested elements
        let has_elements = inner_html.contains('<') && inner_html.contains('>');
        
        if has_elements {
            output.push('\n');
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
        output.push_str("</");
        output.push_str(&tag_name);
        output.push('>');
        output.push('\n');
        
        output
    }
    
    fn prettify_mixed_content(&self, content: &str, depth: usize) -> String {
        let doc = Document::from(content);
        let mut output = String::new();
        
        // Get all elements in the content
        let elements = doc.select("*");
        
        if elements.length() > 0 {
            for element in elements.iter() {
                output.push_str(&self.process_element(&element, depth));
            }
        } else {
            // Just text
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                let indent = "  ".repeat(depth);
                output.push_str(&indent);
                output.push_str(trimmed);
                output.push('\n');
            }
        }
        
        output
    }
    
    fn extract_tag_name(&self, html: &str) -> Option<String> {
        let trimmed = html.trim_start();
        if trimmed.starts_with('<') {
            let tag_start = 1;
            if let Some(tag_end) = trimmed[tag_start..].find(|c: char| c.is_whitespace() || c == '>' || c == '/') {
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
                    "content-length",
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
        "html-prettifier"
    }
}

create_plugin!(HtmlPrettifierPlugin);