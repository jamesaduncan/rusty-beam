//! Compression Plugin for Rusty Beam
//!
//! This plugin provides HTTP response compression using multiple algorithms to reduce
//! bandwidth usage and improve page load times. It supports modern compression standards
//! and intelligently selects the best algorithm based on client capabilities.
//!
//! ## Features
//! - **Multiple Algorithms**: Supports Gzip, Deflate, and Brotli compression
//! - **Client Negotiation**: Automatically selects best compression based on Accept-Encoding
//! - **Content-Type Filtering**: Only compresses appropriate MIME types
//! - **Size Constraints**: Configurable minimum and maximum compression thresholds
//! - **Compression Levels**: Adjustable compression quality vs. speed trade-offs
//! - **Performance Monitoring**: Detailed compression statistics and logging
//!
//! ## Supported Algorithms
//! - **Brotli (br)**: Modern algorithm with excellent compression ratios
//! - **Gzip**: Widely supported, good balance of compression and speed
//! - **Deflate**: Legacy support for older clients
//!
//! ## Configuration
//! - `algorithms`: Comma-separated list of enabled algorithms (default: "gzip,deflate,brotli")
//! - `min_size`: Minimum response size to compress in bytes (default: 1024)
//! - `max_size`: Maximum response size to compress in bytes (default: 10MB)
//! - `compression_level`: Compression quality level 1-9 (default: 6)
//! - `compressible_types`: Comma-separated MIME types to compress
//!
//! ## Default Compressible Types
//! - Text: HTML, CSS, JavaScript, Plain text
//! - Data: JSON, XML, RSS, Atom feeds
//! - Images: SVG (vector graphics only)
//!
//! ## Performance Benefits
//! - **Bandwidth Reduction**: 60-90% size reduction for text content
//! - **Faster Load Times**: Smaller downloads improve user experience
//! - **SEO Benefits**: Google considers page speed in search rankings
//! - **Cost Savings**: Reduced bandwidth usage lowers hosting costs
//!
//! ## Algorithm Selection Priority
//! 1. **Brotli**: Best compression, preferred for modern browsers
//! 2. **Gzip**: Excellent compatibility and performance
//! 3. **Deflate**: Fallback for legacy client support

use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, PluginResponse, create_plugin};
use async_trait::async_trait;
use hyper::{Body, Response, header::{HeaderValue, CONTENT_ENCODING, CONTENT_LENGTH}};
use std::collections::HashMap;
use std::io::Write;
use flate2::{Compression, write::GzEncoder, write::DeflateEncoder};
use brotli::CompressorWriter;

/// Compression algorithm
#[derive(Debug, Clone, PartialEq)]
pub enum CompressionAlgorithm {
    Gzip,
    Deflate,
    Brotli,
}

/// Plugin for response compression (gzip/deflate/brotli)
#[derive(Debug)]
pub struct CompressionPlugin {
    name: String,
    enabled_algorithms: Vec<CompressionAlgorithm>,
    min_size: usize,
    max_size: usize,
    compressible_types: Vec<String>,
    compression_level: u32,
}

impl CompressionPlugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        let name = Self::parse_string_config(&config, "name", "compression");
        let enabled_algorithms = Self::parse_algorithms_config(&config);
        let min_size = Self::parse_numeric_config(&config, "min_size", 1024);
        let max_size = Self::parse_numeric_config(&config, "max_size", 10 * 1024 * 1024);
        let compressible_types = Self::parse_compressible_types_config(&config);
        let compression_level = Self::parse_compression_level_config(&config);
        
        Self {
            name,
            enabled_algorithms,
            min_size,
            max_size,
            compressible_types,
            compression_level,
        }
    }
    
    /// Parse string configuration value with fallback default
    fn parse_string_config(config: &HashMap<String, String>, key: &str, default: &str) -> String {
        config.get(key).cloned().unwrap_or_else(|| default.to_string())
    }
    
    /// Parse numeric configuration value with fallback default
    fn parse_numeric_config<T: std::str::FromStr>(config: &HashMap<String, String>, key: &str, default: T) -> T {
        config.get(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }
    
    /// Parse compression algorithms from configuration
    fn parse_algorithms_config(config: &HashMap<String, String>) -> Vec<CompressionAlgorithm> {
        config.get("algorithms")
            .map(|s| s.split(',').map(|s| s.trim()).filter_map(Self::parse_algorithm_name).collect())
            .unwrap_or_else(Self::default_algorithms)
    }
    
    /// Parse a single algorithm name to enum
    fn parse_algorithm_name(alg: &str) -> Option<CompressionAlgorithm> {
        match alg.to_lowercase().as_str() {
            "gzip" => Some(CompressionAlgorithm::Gzip),
            "deflate" => Some(CompressionAlgorithm::Deflate),
            "brotli" | "br" => Some(CompressionAlgorithm::Brotli),
            _ => None,
        }
    }
    
    /// Get default compression algorithms in priority order
    fn default_algorithms() -> Vec<CompressionAlgorithm> {
        vec![
            CompressionAlgorithm::Brotli,
            CompressionAlgorithm::Gzip,
            CompressionAlgorithm::Deflate,
        ]
    }
    
    /// Parse compressible MIME types from configuration
    fn parse_compressible_types_config(config: &HashMap<String, String>) -> Vec<String> {
        config.get("compressible_types")
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_else(Self::default_compressible_types)
    }
    
    /// Get default compressible MIME types
    fn default_compressible_types() -> Vec<String> {
        vec![
            "text/html".to_string(),
            "text/css".to_string(),
            "text/javascript".to_string(),
            "text/plain".to_string(),
            "application/json".to_string(),
            "application/javascript".to_string(),
            "application/xml".to_string(),
            "application/rss+xml".to_string(),
            "application/atom+xml".to_string(),
            "image/svg+xml".to_string(),
        ]
    }
    
    /// Parse compression level with validation
    fn parse_compression_level_config(config: &HashMap<String, String>) -> u32 {
        let level = Self::parse_numeric_config(config, "compression_level", 6);
        // Clamp compression level to valid range (1-9)
        level.max(1).min(9)
    }
    
    /// Parse Accept-Encoding header and return preferred compression algorithm
    fn get_preferred_encoding(&self, accept_encoding: &str) -> Option<CompressionAlgorithm> {
        let accepted_encodings = self.parse_accept_encoding_header(accept_encoding);
        
        // Check enabled algorithms in priority order
        for algorithm in &self.enabled_algorithms {
            let encoding_name = Self::algorithm_to_encoding_name(algorithm);
            
            if accepted_encodings.contains(&encoding_name) || accepted_encodings.contains(&"*") {
                return Some(algorithm.clone());
            }
        }
        
        None
    }
    
    /// Parse Accept-Encoding header into list of accepted encodings
    fn parse_accept_encoding_header<'a>(&self, accept_encoding: &'a str) -> Vec<&'a str> {
        accept_encoding
            .split(',')
            .map(|s| s.trim().split(';').next().unwrap_or("").trim())
            .collect()
    }
    
    /// Convert compression algorithm to HTTP encoding name
    fn algorithm_to_encoding_name(algorithm: &CompressionAlgorithm) -> &'static str {
        match algorithm {
            CompressionAlgorithm::Brotli => "br",
            CompressionAlgorithm::Gzip => "gzip", 
            CompressionAlgorithm::Deflate => "deflate",
        }
    }
    
    /// Update response headers for compressed content
    fn update_response_headers(&self, response: &mut Response<Body>, compressed_data: &[u8], encoding_name: &str) {
        let headers = response.headers_mut();
        
        // Set Content-Encoding header
        if let Ok(encoding_value) = HeaderValue::from_str(encoding_name) {
            headers.insert(CONTENT_ENCODING, encoding_value);
        }
        
        // Update Content-Length header
        if let Ok(length_value) = HeaderValue::from_str(&compressed_data.len().to_string()) {
            headers.insert(CONTENT_LENGTH, length_value);
        }
        
        // Add Vary header to indicate response varies by Accept-Encoding
        headers.insert("Vary", HeaderValue::from_static("Accept-Encoding"));
    }
    
    /// Determine the best compression encoding for the request
    fn determine_compression_encoding(&self, request: &PluginRequest) -> Option<CompressionAlgorithm> {
        let accept_encoding = request.http_request.headers()
            .get("accept-encoding")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
            
        self.get_preferred_encoding(accept_encoding)
    }
    
    /// Extract response body as bytes
    async fn extract_response_body(&self, response: &mut Response<Body>) -> Option<hyper::body::Bytes> {
        let body = std::mem::replace(response.body_mut(), Body::empty());
        hyper::body::to_bytes(body).await.ok()
    }
    
    /// Apply compression to response body and update headers
    fn apply_compression(
        &self,
        response: &mut Response<Body>,
        body_bytes: hyper::body::Bytes,
        algorithm: &CompressionAlgorithm,
        context: &PluginContext,
    ) -> Result<(), hyper::body::Bytes> {
        // Attempt compression
        let compressed_data = self.compress_data(&body_bytes, algorithm)
            .map_err(|_| body_bytes.clone())?;
        
        // Calculate compression statistics
        let stats = CompressionStats::new(&body_bytes, &compressed_data, algorithm);
        
        // Update response
        let encoding_name = Self::algorithm_to_encoding_name(algorithm);
        self.update_response_headers(response, &compressed_data, encoding_name);
        *response.body_mut() = Body::from(compressed_data);
        
        // Log success
        self.log_compression_success(&stats, context);
        
        Ok(())
    }
    
    /// Restore original response body
    fn restore_original_body(&self, response: &mut Response<Body>, body_bytes: hyper::body::Bytes) {
        *response.body_mut() = Body::from(body_bytes);
    }
    
    /// Log successful compression
    fn log_compression_success(&self, stats: &CompressionStats, context: &PluginContext) {
        context.log_verbose(&format!(
            "[Compression] Compressed {} bytes to {} bytes using {} ({:.1}% reduction)",
            stats.original_size, stats.compressed_size, stats.algorithm_name, stats.compression_ratio
        ));
    }
    
    /// Check if content type is compressible
    fn is_compressible_type(&self, content_type: &str) -> bool {
        let content_type = content_type.split(';').next().unwrap_or("").trim();
        
        for compressible_type in &self.compressible_types {
            if content_type.starts_with(compressible_type) {
                return true;
            }
        }
        
        false
    }
    
    /// Check if response should be compressed
    fn should_compress(&self, response: &Response<Body>, body_size: usize) -> bool {
        // Check size constraints
        if body_size < self.min_size || body_size > self.max_size {
            return false;
        }
        
        // Check if already compressed
        if response.headers().contains_key(CONTENT_ENCODING) {
            return false;
        }
        
        // Check content type
        if let Some(content_type) = response.headers().get("content-type") {
            if let Ok(content_type_str) = content_type.to_str() {
                if !self.is_compressible_type(content_type_str) {
                    return false;
                }
            }
        }
        
        true
    }
    
    /// Compress data using specified algorithm
    fn compress_data(&self, data: &[u8], algorithm: &CompressionAlgorithm) -> Result<Vec<u8>, String> {
        match algorithm {
            CompressionAlgorithm::Gzip => {
                let mut encoder = GzEncoder::new(Vec::new(), Compression::new(self.compression_level));
                encoder.write_all(data).map_err(|e| e.to_string())?;
                encoder.finish().map_err(|e| e.to_string())
            }
            CompressionAlgorithm::Deflate => {
                let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(self.compression_level));
                encoder.write_all(data).map_err(|e| e.to_string())?;
                encoder.finish().map_err(|e| e.to_string())
            }
            CompressionAlgorithm::Brotli => {
                let mut compressed = Vec::new();
                {
                    let mut encoder = CompressorWriter::new(&mut compressed, 4096, self.compression_level, 22);
                    encoder.write_all(data).map_err(|e| e.to_string())?;
                }
                Ok(compressed)
            }
        }
    }
    
}

#[async_trait]
impl Plugin for CompressionPlugin {
    async fn handle_request(&self, _request: &mut PluginRequest, _context: &PluginContext) -> Option<PluginResponse> {
        // Compression is handled during response phase
        None
    }
    
    async fn handle_response(&self, request: &PluginRequest, response: &mut Response<Body>, context: &PluginContext) {
        // Determine if compression should be applied
        let preferred_encoding = match self.determine_compression_encoding(request) {
            Some(encoding) => encoding,
            None => return, // No supported encoding or compression not needed
        };
        
        // Extract and validate response body
        let body_bytes = match self.extract_response_body(response).await {
            Some(bytes) => bytes,
            None => return, // Cannot process body
        };
        
        // Check compression eligibility
        if !self.should_compress(response, body_bytes.len()) {
            self.restore_original_body(response, body_bytes);
            return;
        }
        
        // Perform compression
        match self.apply_compression(response, body_bytes, &preferred_encoding, context) {
            Ok(_) => {}, // Compression successful
            Err(original_body) => {
                // Compression failed, restore original content
                self.restore_original_body(response, original_body);
            }
        }
    }
    
    
    fn name(&self) -> &str {
        &self.name
    }
}

/// Compression statistics for logging and monitoring
struct CompressionStats {
    original_size: usize,
    compressed_size: usize,
    compression_ratio: f64,
    algorithm_name: &'static str,
}

impl CompressionStats {
    fn new(original: &[u8], compressed: &[u8], algorithm: &CompressionAlgorithm) -> Self {
        let original_size = original.len();
        let compressed_size = compressed.len();
        let compression_ratio = (1.0 - (compressed_size as f64 / original_size as f64)) * 100.0;
        let algorithm_name = CompressionPlugin::algorithm_to_encoding_name(algorithm);
        
        Self {
            original_size,
            compressed_size,
            compression_ratio,
            algorithm_name,
        }
    }
}

// Export the plugin creation function
create_plugin!(CompressionPlugin);