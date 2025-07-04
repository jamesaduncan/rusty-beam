//! Plugin loader abstraction supporting multiple plugin types and sources
//! 
//! This module provides a unified interface for loading plugins from:
//! - Local C-style shared libraries (.so, .dylib, .dll)
//! - Local or remote WebAssembly modules (.wasm)
//! - HTTP(S) URLs for remote plugin loading

use super::plugin::{Plugin, PluginRequest, PluginContext};
use async_trait::async_trait;
use hyper::{Body, Response};
use std::collections::HashMap;
use url::Url;

/// Errors that can occur during plugin loading
#[derive(Debug, Clone)]
pub enum PluginLoadError {
    InvalidUrl(String),
    UnsupportedScheme(String),
    NetworkError(String),
    WasmError(String),
    DynamicLibraryError(String),
    PluginInitError(String),
}

impl std::fmt::Display for PluginLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginLoadError::InvalidUrl(msg) => write!(f, "Invalid URL: {}", msg),
            PluginLoadError::UnsupportedScheme(scheme) => write!(f, "Unsupported URL scheme: {}", scheme),
            PluginLoadError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            PluginLoadError::WasmError(msg) => write!(f, "WASM error: {}", msg),
            PluginLoadError::DynamicLibraryError(msg) => write!(f, "Dynamic library error: {}", msg),
            PluginLoadError::PluginInitError(msg) => write!(f, "Plugin initialization error: {}", msg),
        }
    }
}

impl std::error::Error for PluginLoadError {}

/// Plugin source type determined from URL
#[derive(Debug, Clone)]
pub enum PluginSource {
    /// Local C-style shared library
    LocalDynamicLibrary(String),
    /// Local WebAssembly module
    LocalWasm(String),
    /// Remote WebAssembly module
    RemoteWasm(Url),
}

/// Main plugin loader that handles all plugin types
pub struct PluginLoader {
    /// Cache for loaded remote plugins
    remote_cache: HashMap<String, Vec<u8>>,
}

impl PluginLoader {
    pub fn new() -> Self {
        Self {
            remote_cache: HashMap::new(),
        }
    }
    
    /// Load a plugin from a URL string
    pub async fn load_plugin(
        &mut self,
        url_str: &str,
        config: HashMap<String, String>,
    ) -> Result<Box<dyn Plugin>, PluginLoadError> {
        // Parse the URL
        let url = Url::parse(url_str)
            .map_err(|e| PluginLoadError::InvalidUrl(e.to_string()))?;
        
        // Determine plugin source type
        let source = self.determine_source(&url)?;
        
        // Load based on source type
        match source {
            PluginSource::LocalDynamicLibrary(path) => {
                self.load_dynamic_library(&path, config).await
            }
            PluginSource::LocalWasm(path) => {
                self.load_local_wasm(&path, config).await
            }
            PluginSource::RemoteWasm(url) => {
                self.load_remote_wasm(&url, config).await
            }
        }
    }
    
    /// Determine the plugin source type from URL
    fn determine_source(&self, url: &Url) -> Result<PluginSource, PluginLoadError> {
        match url.scheme() {
            "file" => {
                let path = url.path();
                if path.ends_with(".wasm") {
                    Ok(PluginSource::LocalWasm(path.to_string()))
                } else if path.ends_with(".so") || path.ends_with(".dylib") || path.ends_with(".dll") {
                    Ok(PluginSource::LocalDynamicLibrary(path.to_string()))
                } else {
                    // Try to determine from file
                    // For now, default to dynamic library
                    Ok(PluginSource::LocalDynamicLibrary(path.to_string()))
                }
            }
            "http" | "https" => {
                // Remote plugins must be WASM for security
                Ok(PluginSource::RemoteWasm(url.clone()))
            }
            scheme => Err(PluginLoadError::UnsupportedScheme(scheme.to_string())),
        }
    }
    
    /// Load a C-style dynamic library plugin
    async fn load_dynamic_library(
        &mut self,
        path: &str,
        config: HashMap<String, String>,
    ) -> Result<Box<dyn Plugin>, PluginLoadError> {
        // Use the dynamic plugin wrapper
        let plugin = crate::v2::dynamic::DynamicPluginWrapper::new(path, config)?;
        Ok(Box::new(plugin))
    }
    
    /// Load a local WASM plugin
    async fn load_local_wasm(
        &mut self,
        path: &str,
        config: HashMap<String, String>,
    ) -> Result<Box<dyn Plugin>, PluginLoadError> {
        // Read the WASM file
        let wasm_bytes = tokio::fs::read(path).await
            .map_err(|e| PluginLoadError::WasmError(format!("Failed to read WASM file: {}", e)))?;
        
        self.load_wasm_bytes(&wasm_bytes, config).await
    }
    
    /// Load a remote WASM plugin
    async fn load_remote_wasm(
        &mut self,
        url: &Url,
        config: HashMap<String, String>,
    ) -> Result<Box<dyn Plugin>, PluginLoadError> {
        let url_str = url.to_string();
        
        // Check cache first
        if let Some(cached_bytes) = self.remote_cache.get(&url_str) {
            return self.load_wasm_bytes(cached_bytes, config).await;
        }
        
        // Fetch from network
        let client = hyper::Client::new();
        let uri = url.as_str().parse()
            .map_err(|e| PluginLoadError::NetworkError(format!("Invalid URI: {}", e)))?;
        let response = client.get(uri)
            .await
            .map_err(|e| PluginLoadError::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(PluginLoadError::NetworkError(
                format!("HTTP {}: Failed to fetch plugin", response.status())
            ));
        }
        
        let bytes = hyper::body::to_bytes(response.into_body())
            .await
            .map_err(|e| PluginLoadError::NetworkError(e.to_string()))?;
        
        let wasm_bytes = bytes.to_vec();
        
        // Cache for future use
        self.remote_cache.insert(url_str, wasm_bytes.clone());
        
        self.load_wasm_bytes(&wasm_bytes, config).await
    }
    
    /// Load WASM bytes into a plugin
    async fn load_wasm_bytes(
        &mut self,
        wasm_bytes: &[u8],
        config: HashMap<String, String>,
    ) -> Result<Box<dyn Plugin>, PluginLoadError> {
        // Extract plugin name from config or use default
        let name = config.get("name")
            .cloned()
            .unwrap_or_else(|| "wasm-plugin".to_string());
        
        // Create WASM plugin wrapper
        let plugin = crate::v2::wasm::WasmPluginWrapper::new(wasm_bytes, config, name).await?;
        
        Ok(Box::new(plugin))
    }
}

/// Plugin ABI for C-style plugins
/// This defines the functions that must be exported by dynamic library plugins
#[repr(C)]
pub struct CPluginABI {
    /// Initialize the plugin with configuration
    pub init: unsafe extern "C" fn(config: *const u8, config_len: usize) -> *mut std::ffi::c_void,
    
    /// Handle a request
    pub handle_request: unsafe extern "C" fn(
        plugin: *mut std::ffi::c_void,
        request_data: *const u8,
        request_len: usize,
        response_data: *mut u8,
        response_len: *mut usize,
    ) -> i32,
    
    /// Handle a response
    pub handle_response: unsafe extern "C" fn(
        plugin: *mut std::ffi::c_void,
        request_data: *const u8,
        request_len: usize,
        response_data: *mut u8,
        response_len: usize,
    ),
    
    /// Get plugin name
    pub get_name: unsafe extern "C" fn(plugin: *mut std::ffi::c_void) -> *const u8,
    
    /// Destroy the plugin
    pub destroy: unsafe extern "C" fn(plugin: *mut std::ffi::c_void),
}

/// WASM plugin interface
/// This will be the trait that WASM modules must implement
pub trait WasmPlugin {
    /// Initialize with configuration
    fn init(config: &[u8]) -> Result<Box<Self>, String>;
    
    /// Handle request
    fn handle_request(&mut self, request: &[u8]) -> Option<Vec<u8>>;
    
    /// Handle response  
    fn handle_response(&mut self, request: &[u8], response: &mut [u8]);
    
    /// Get plugin name
    fn name(&self) -> &str;
}

/// Load configuration from a URL
pub async fn load_config_from_url(url_str: &str) -> Result<String, PluginLoadError> {
    let url = Url::parse(url_str)
        .map_err(|e| PluginLoadError::InvalidUrl(e.to_string()))?;
    
    match url.scheme() {
        "file" => {
            // Load from local file
            let path = url.path();
            tokio::fs::read_to_string(path)
                .await
                .map_err(|e| PluginLoadError::NetworkError(format!("Failed to read file: {}", e)))
        }
        "http" | "https" => {
            // Fetch from network
            let client = hyper::Client::new();
            let uri = url.as_str().parse()
                .map_err(|e| PluginLoadError::NetworkError(format!("Invalid URI: {}", e)))?;
            let response = client.get(uri)
                .await
                .map_err(|e| PluginLoadError::NetworkError(e.to_string()))?;
            
            if !response.status().is_success() {
                return Err(PluginLoadError::NetworkError(
                    format!("HTTP {}: Failed to fetch config", response.status())
                ));
            }
            
            let bytes = hyper::body::to_bytes(response.into_body())
                .await
                .map_err(|e| PluginLoadError::NetworkError(e.to_string()))?;
            
            String::from_utf8(bytes.to_vec())
                .map_err(|e| PluginLoadError::NetworkError(format!("Invalid UTF-8: {}", e)))
        }
        scheme => Err(PluginLoadError::UnsupportedScheme(scheme.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_plugin_source_determination() {
        let loader = PluginLoader::new();
        
        // Test file URLs
        let file_so = Url::parse("file:///path/to/plugin.so").unwrap();
        match loader.determine_source(&file_so).unwrap() {
            PluginSource::LocalDynamicLibrary(path) => assert_eq!(path, "/path/to/plugin.so"),
            _ => panic!("Expected LocalDynamicLibrary"),
        }
        
        let file_wasm = Url::parse("file:///path/to/plugin.wasm").unwrap();
        match loader.determine_source(&file_wasm).unwrap() {
            PluginSource::LocalWasm(path) => assert_eq!(path, "/path/to/plugin.wasm"),
            _ => panic!("Expected LocalWasm"),
        }
        
        // Test HTTP URLs (should always be WASM)
        let http_url = Url::parse("http://example.com/plugin.so").unwrap();
        match loader.determine_source(&http_url).unwrap() {
            PluginSource::RemoteWasm(url) => assert_eq!(url.as_str(), "http://example.com/plugin.so"),
            _ => panic!("Expected RemoteWasm"),
        }
        
        // Test unsupported scheme
        let ftp_url = Url::parse("ftp://example.com/plugin.so").unwrap();
        assert!(loader.determine_source(&ftp_url).is_err());
    }
    
    #[test]
    fn test_error_display() {
        let err = PluginLoadError::InvalidUrl("bad url".to_string());
        assert_eq!(err.to_string(), "Invalid URL: bad url");
        
        let err = PluginLoadError::UnsupportedScheme("ftp".to_string());
        assert_eq!(err.to_string(), "Unsupported URL scheme: ftp");
    }
}