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
        let name = config.get("name").cloned().unwrap_or_else(|| "compression".to_string());
        
        let enabled_algorithms = config.get("algorithms")
            .map(|s| s.split(',').map(|s| s.trim()).filter_map(|alg| {
                match alg.to_lowercase().as_str() {
                    "gzip" => Some(CompressionAlgorithm::Gzip),
                    "deflate" => Some(CompressionAlgorithm::Deflate),
                    "brotli" | "br" => Some(CompressionAlgorithm::Brotli),
                    _ => None,
                }
            }).collect())
            .unwrap_or_else(|| vec![CompressionAlgorithm::Gzip, CompressionAlgorithm::Deflate, CompressionAlgorithm::Brotli]);
        
        let min_size = config.get("min_size")
            .and_then(|v| v.parse().ok())
            .unwrap_or(1024); // Don't compress responses smaller than 1KB
        
        let max_size = config.get("max_size")
            .and_then(|v| v.parse().ok())
            .unwrap_or(10 * 1024 * 1024); // Don't compress responses larger than 10MB
        
        let compressible_types = config.get("compressible_types")
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_else(|| vec![
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
            ]);
        
        let compression_level = config.get("compression_level")
            .and_then(|v| v.parse().ok())
            .unwrap_or(6); // Default compression level
        
        Self {
            name,
            enabled_algorithms,
            min_size,
            max_size,
            compressible_types,
            compression_level,
        }
    }
    
    /// Parse Accept-Encoding header and return preferred compression algorithm
    fn get_preferred_encoding(&self, accept_encoding: &str) -> Option<CompressionAlgorithm> {
        // Parse Accept-Encoding header and find the best match
        let encodings: Vec<&str> = accept_encoding
            .split(',')
            .map(|s| s.trim().split(';').next().unwrap_or("").trim())
            .collect();
        
        // Priority order: brotli > gzip > deflate
        for algorithm in &[CompressionAlgorithm::Brotli, CompressionAlgorithm::Gzip, CompressionAlgorithm::Deflate] {
            if self.enabled_algorithms.contains(algorithm) {
                let encoding_name = match algorithm {
                    CompressionAlgorithm::Brotli => "br",
                    CompressionAlgorithm::Gzip => "gzip",
                    CompressionAlgorithm::Deflate => "deflate",
                };
                
                if encodings.contains(&encoding_name) || encodings.contains(&"*") {
                    return Some(algorithm.clone());
                }
            }
        }
        
        None
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
    
    /// Get encoding name for algorithm
    fn get_encoding_name(&self, algorithm: &CompressionAlgorithm) -> &'static str {
        match algorithm {
            CompressionAlgorithm::Gzip => "gzip",
            CompressionAlgorithm::Deflate => "deflate",
            CompressionAlgorithm::Brotli => "br",
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
        // Check Accept-Encoding header
        let accept_encoding = request.http_request.headers()
            .get("accept-encoding")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        
        let preferred_encoding = match self.get_preferred_encoding(accept_encoding) {
            Some(enc) => enc,
            None => return, // No supported encoding
        };
        
        // Get response body
        let body = std::mem::replace(response.body_mut(), Body::empty());
        
        // Convert body to bytes (this is a simplified approach)
        // In a real implementation, you'd want to handle streaming bodies
        let body_bytes = match hyper::body::to_bytes(body).await {
            Ok(bytes) => bytes,
            Err(_) => return, // Cannot get body bytes
        };
        
        // Check if we should compress
        if !self.should_compress(response, body_bytes.len()) {
            // Restore original body
            *response.body_mut() = Body::from(body_bytes);
            return;
        }
        
        // Compress the data
        let compressed_data = match self.compress_data(&body_bytes, &preferred_encoding) {
            Ok(data) => data,
            Err(_) => {
                // Compression failed, restore original body
                *response.body_mut() = Body::from(body_bytes);
                return;
            }
        };
        
        // Update response headers
        response.headers_mut().insert(
            CONTENT_ENCODING,
            HeaderValue::from_str(self.get_encoding_name(&preferred_encoding)).unwrap()
        );
        
        response.headers_mut().insert(
            CONTENT_LENGTH,
            HeaderValue::from_str(&compressed_data.len().to_string()).unwrap()
        );
        
        // Add Vary header to indicate that response varies by Accept-Encoding
        response.headers_mut().insert(
            "Vary",
            HeaderValue::from_static("Accept-Encoding")
        );
        
        // Calculate stats before moving data
        let compressed_len = compressed_data.len();
        let compression_ratio = (1.0 - (compressed_len as f64 / body_bytes.len() as f64)) * 100.0;
        
        // Replace body with compressed data
        *response.body_mut() = Body::from(compressed_data);
        
        // Log compression stats
        context.log_verbose(&format!("[Compression] Compressed {} bytes to {} bytes using {} ({:.1}% reduction)",
                 body_bytes.len(),
                 compressed_len,
                 self.get_encoding_name(&preferred_encoding),
                 compression_ratio
        ));
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

// Export the plugin creation function
create_plugin!(CompressionPlugin);