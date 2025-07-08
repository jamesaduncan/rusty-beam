//! Rusty Beam - Plugin-based HTTP server
//! 
//! This server uses a plugin architecture for CSS selector-based HTML manipulation
//! via HTTP Range headers, with support for authentication, authorization, and logging.

// Import modules
mod config;
mod constants;

use async_trait::async_trait;
use config::PluginConfig;
use rusty_beam_plugin_api::{PluginRequest, PluginContext};
use config::{load_config_from_html, ServerConfig};
use constants::DEFAULT_SERVER_HEADER;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Result, Server, StatusCode};
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::env;
use tokio::sync::RwLock;
use uuid::Uuid;
use signal_hook::consts::SIGHUP;
use signal_hook_tokio::Signals;
use futures::stream::StreamExt;

/// Type alias for host pipelines
type HostPipelines = HashMap<String, Vec<Arc<dyn rusty_beam_plugin_api::Plugin>>>;

/// Application State using plugin architecture
#[derive(Clone)]
struct AppState {
    config: Arc<RwLock<ServerConfig>>,
    host_pipelines: Arc<RwLock<HostPipelines>>,
    config_path: String,
}

impl AppState {
    async fn new(config_path: String) -> Self {
        let config = load_config_from_html(&config_path);
        let host_pipelines = create_host_pipelines(&config);
        
        Self {
            config: Arc::new(RwLock::new(config)),
            host_pipelines: Arc::new(RwLock::new(host_pipelines)),
            config_path,
        }
    }
    
    async fn reload(&self) -> std::result::Result<(), String> {
        // Load new configuration
        let new_config = load_config_from_html(&self.config_path);
        let new_pipelines = create_host_pipelines(&new_config);
        
        // Atomically update the shared state
        {
            let mut config_lock = self.config.write().await;
            *config_lock = new_config;
        }
        
        {
            let mut pipelines_lock = self.host_pipelines.write().await;
            *pipelines_lock = new_pipelines;
        }
        
        Ok(())
    }
}

/// Create plugin pipelines for each host based on configuration
/// Dynamically load a plugin from its library path
fn load_plugin(plugin_config: &PluginConfig) -> Option<Box<dyn rusty_beam_plugin_api::Plugin>> {
    use std::ffi::OsStr;
    use std::path::Path;
    
    // Convert plugin config to plugin config map
    let mut v2_config = HashMap::new();
    for (key, value) in &plugin_config.config {
        v2_config.insert(key.clone(), value.clone());
    }
    
    let library_url = &plugin_config.library;
    
    // Handle special pipeline:// URLs for nested plugin containers
    if library_url == "pipeline://nested" {
        // Create a pipeline plugin that executes nested plugins
        return create_pipeline_plugin(plugin_config);
    }
    
    // Handle directory:// URLs for directory-based plugin containers
    if library_url.starts_with("directory://") || library_url.starts_with("file://./plugins/directory.so") {
        // Create a directory plugin that executes nested plugins for matching paths
        return create_directory_plugin(plugin_config);
    }
    
    // All plugins must be loaded from external libraries - no built-ins
    
    // Handle file:// URLs
    let library_path = library_url.strip_prefix("file://").unwrap_or(library_url);
    
    let path = Path::new(library_path);
    let extension = path.extension().and_then(OsStr::to_str);
    
    match extension {
        Some("so") | Some("dll") | Some("dylib") => {
            // Load dynamic library
            load_dynamic_plugin(library_path, v2_config)
        }
        Some("wasm") => {
            // Load WASM plugin
            load_wasm_plugin(library_path, v2_config)
        }
        _ => {
            eprintln!("Warning: Unknown plugin: {}", library_url);
            None
        }
    }
}

/// Create a pipeline plugin that executes nested plugins in sequence
fn create_pipeline_plugin(plugin_config: &PluginConfig) -> Option<Box<dyn rusty_beam_plugin_api::Plugin>> {
    use std::sync::Arc;
    
    // Load all nested plugins
    let mut nested_pipeline = Vec::new();
    for nested_config in &plugin_config.nested_plugins {
        if let Some(plugin) = load_plugin(nested_config) {
            nested_pipeline.push(Arc::from(plugin));
        }
    }
    
    if nested_pipeline.is_empty() {
        eprintln!("Warning: Pipeline plugin has no valid nested plugins");
        return None;
    }
    
    Some(Box::new(PipelinePlugin {
        nested_plugins: nested_pipeline,
    }))
}

/// Create a directory plugin that executes nested plugins only for matching paths
fn create_directory_plugin(plugin_config: &PluginConfig) -> Option<Box<dyn rusty_beam_plugin_api::Plugin>> {
    use std::sync::Arc;
    
    // Get the directory configuration
    let directory = plugin_config.config.get("directory")
        .map(|d| {
            // Handle file:// URLs by extracting just the directory part
            if d.starts_with("file://") {
                // Extract the path after the host root
                // e.g., "file://./examples/localhost/admin" -> "/admin"
                if let Some(last_part) = d.rsplit('/').next() {
                    format!("/{}", last_part)
                } else {
                    d.clone()
                }
            } else {
                d.clone()
            }
        })
        .unwrap_or_else(|| "/".to_string());
    
    eprintln!("Creating directory plugin for path: {}", directory);
    
    // Load all nested plugins
    let mut nested_pipeline = Vec::new();
    for nested_config in &plugin_config.nested_plugins {
        if let Some(plugin) = load_plugin(nested_config) {
            nested_pipeline.push(Arc::from(plugin));
        }
    }
    
    if nested_pipeline.is_empty() {
        eprintln!("Warning: Directory plugin has no valid nested plugins");
        return None;
    }
    
    Some(Box::new(DirectoryPlugin {
        directory,
        nested_plugins: nested_pipeline,
    }))
}

/// A plugin that executes nested plugins in sequence
#[derive(Debug)]
struct PipelinePlugin {
    nested_plugins: Vec<Arc<dyn rusty_beam_plugin_api::Plugin>>,
}

#[async_trait]
impl rusty_beam_plugin_api::Plugin for PipelinePlugin {
    async fn handle_request(&self, request: &mut rusty_beam_plugin_api::PluginRequest, context: &rusty_beam_plugin_api::PluginContext) -> Option<Response<Body>> {
        // Execute nested plugins in order
        for plugin in &self.nested_plugins {
            if let Some(response) = plugin.handle_request(request, context).await {
                return Some(response);
            }
        }
        None
    }
    
    fn name(&self) -> &str {
        "pipeline"
    }
}

/// A plugin that executes nested plugins only if the request path matches a directory
#[derive(Debug)]
struct DirectoryPlugin {
    directory: String,
    nested_plugins: Vec<Arc<dyn rusty_beam_plugin_api::Plugin>>,
}

#[async_trait]
impl rusty_beam_plugin_api::Plugin for DirectoryPlugin {
    async fn handle_request(&self, request: &mut rusty_beam_plugin_api::PluginRequest, context: &rusty_beam_plugin_api::PluginContext) -> Option<Response<Body>> {
        // Check if the request path matches the configured directory
        let normalized_dir = self.directory.trim_end_matches('/');
        let normalized_path = request.path.trim_end_matches('/');
        
        // Check if path matches exactly or starts with directory followed by /
        let matches = normalized_path == normalized_dir || 
                     request.path.starts_with(&format!("{}/", normalized_dir));
        
        if !matches {
            // Path doesn't match, pass through to next plugin
            return None;
        }
        
        // Path matches, execute nested plugins in order
        for plugin in &self.nested_plugins {
            if let Some(response) = plugin.handle_request(request, context).await {
                return Some(response);
            }
        }
        None
    }
    
    fn name(&self) -> &str {
        "directory"
    }
}

/// Load a plugin from a dynamic library (.so/.dll/.dylib)
fn load_dynamic_plugin(library_path: &str, config: HashMap<String, String>) -> Option<Box<dyn rusty_beam_plugin_api::Plugin>> {
    use libloading::{Library, Symbol};
    
    unsafe {
        match Library::new(library_path) {
        Ok(lib) => {
                // Look for the plugin creation function
                // Convention: extern "C" fn create_plugin(config: *const c_char) -> *mut c_void
                let create_fn: Symbol<unsafe extern "C" fn(*const std::os::raw::c_char) -> *mut std::ffi::c_void> = 
                    match lib.get(b"create_plugin") {
                        Ok(func) => func,
                        Err(e) => {
                            eprintln!("Failed to find create_plugin function in {}: {}", library_path, e);
                            return None;
                        }
                    };
                
                // Serialize config to JSON for passing to plugin
                let config_json = match serde_json::to_string(&config) {
                    Ok(json) => json,
                    Err(e) => {
                        eprintln!("Failed to serialize plugin config: {}", e);
                        return None;
                    }
                };
                
                let config_cstr = match std::ffi::CString::new(config_json) {
                    Ok(cstr) => cstr,
                    Err(e) => {
                        eprintln!("Failed to create CString from config: {}", e);
                        return None;
                    }
                };
                
                let plugin_ptr = create_fn(config_cstr.as_ptr());
                if plugin_ptr.is_null() {
                    eprintln!("Plugin creation function returned null for {}", library_path);
                    return None;
                }
                
                // Cast the void pointer back to Box<Box<dyn Plugin>> and unwrap one level
                let plugin_box = Box::from_raw(plugin_ptr as *mut Box<dyn rusty_beam_plugin_api::Plugin>);
                let plugin = *plugin_box;
                let wrapper = DynamicPluginWrapper {
                    _library: lib,
                    plugin,
                };
                Some(Box::new(wrapper))
        }
        Err(e) => {
            eprintln!("Failed to load dynamic library {}: {}", library_path, e);
            None
        }
        }
    }
}

/// Dynamic library plugin wrapper that keeps the library loaded
struct DynamicPluginWrapper {
    _library: libloading::Library,  // Keep library alive
    plugin: Box<dyn rusty_beam_plugin_api::Plugin>,
}

impl std::fmt::Debug for DynamicPluginWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DynamicPlugin({})", self.plugin.name())
    }
}

#[async_trait]
impl rusty_beam_plugin_api::Plugin for DynamicPluginWrapper {
    async fn handle_request(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<Response<Body>> {
        self.plugin.handle_request(request, context).await
    }
    
    async fn handle_response(&self, request: &PluginRequest, response: &mut Response<Body>, context: &PluginContext) {
        self.plugin.handle_response(request, response, context).await
    }
    
    fn name(&self) -> &str {
        self.plugin.name()
    }
}

/// WASM Plugin wrapper
#[derive(Debug)]
struct WasmPlugin {
    name: String,
}

impl WasmPlugin {
    fn new(_instance: wasmtime::Instance, _store: wasmtime::Store<()>, _plugin_id: i32, config: HashMap<String, String>) -> Self {
        Self {
            name: config.get("name").cloned().unwrap_or_else(|| "wasm-plugin".to_string()),
        }
    }
}

#[async_trait]
impl rusty_beam_plugin_api::Plugin for WasmPlugin {
    async fn handle_request(&self, _request: &mut rusty_beam_plugin_api::PluginRequest, _context: &rusty_beam_plugin_api::PluginContext) -> Option<Response<Body>> {
        // TODO: Implement WASM plugin request handling
        None
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

/// Load a plugin from a WASM module
fn load_wasm_plugin(library_path: &str, config: HashMap<String, String>) -> Option<Box<dyn rusty_beam_plugin_api::Plugin>> {
    use wasmtime::{Engine, Module, Store, Linker};
    
    // Read WASM file
    let wasm_bytes = match std::fs::read(library_path) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Failed to read WASM file {}: {}", library_path, e);
            return None;
        }
    };
    
    // Create WASM engine and module
    let engine = Engine::default();
    let module = match Module::new(&engine, &wasm_bytes) {
        Ok(module) => module,
        Err(e) => {
            eprintln!("Failed to create WASM module from {}: {}", library_path, e);
            return None;
        }
    };
    
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    
    // Add WASI support if needed  
    // Note: WASI linker setup would be more complex in practice
    
    // Instantiate the module
    let instance = match linker.instantiate(&mut store, &module) {
        Ok(instance) => instance,
        Err(e) => {
            eprintln!("Failed to instantiate WASM module {}: {}", library_path, e);
            return None;
        }
    };
    
    // Look for plugin creation function
    let create_plugin = match instance.get_typed_func::<(), i32>(&mut store, "create_plugin") {
        Ok(func) => func,
        Err(e) => {
            eprintln!("Failed to find create_plugin function in WASM module {}: {}", library_path, e);
            return None;
        }
    };
    
    // Call the plugin creation function
    match create_plugin.call(&mut store, ()) {
        Ok(plugin_id) => {
            // Create a WASM plugin wrapper
            Some(Box::new(WasmPlugin::new(instance, store, plugin_id, config)))
        }
        Err(e) => {
            eprintln!("Failed to create plugin from WASM module {}: {}", library_path, e);
            None
        }
    }
}

fn create_host_pipelines(config: &ServerConfig) -> HostPipelines {
    let mut host_pipelines = HashMap::new();
    
    eprintln!("Creating host pipelines for {} hosts", config.hosts.len());
    
    for (host_name, host_config) in &config.hosts {
        let mut pipeline: Vec<Arc<dyn rusty_beam_plugin_api::Plugin>> = Vec::new();
        
        eprintln!("Loading {} plugins for host: {}", host_config.plugins.len(), host_name);
        
        // Load plugins in order from config
        for plugin_config in &host_config.plugins {
            eprintln!("Attempting to load plugin: {}", plugin_config.library);
            if let Some(plugin) = load_plugin(plugin_config) {
                eprintln!("Successfully loaded plugin: {}", plugin_config.library);
                pipeline.push(Arc::from(plugin));
            } else {
                eprintln!("Failed to load plugin: {}", plugin_config.library);
            }
        }
        
        eprintln!("Host {} has {} loaded plugins", host_name, pipeline.len());
        host_pipelines.insert(host_name.clone(), pipeline);
    }
    
    host_pipelines
}

/// Process request through plugin pipeline
async fn process_request_through_pipeline(
    req: Request<Body>, 
    app_state: AppState
) -> Result<Response<Body>> {
    use std::collections::HashMap;
    
    let raw_path = req.uri().path();
    let host_name = req.headers()
        .get(hyper::header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost")
        .split(':')
        .next()
        .unwrap_or("localhost")
        .to_lowercase();
    
    eprintln!("Processing request for host: {}, path: {}", host_name, raw_path);
    
    // Decode percent-encoded URI path (RFC 3986)
    let path = match urlencoding::decode(raw_path) {
        Ok(decoded) => decoded.into_owned(),
        Err(_) => {
            let response = Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Content-Type", "text/plain")
                .header(hyper::header::SERVER, DEFAULT_SERVER_HEADER)
                .header(hyper::header::DATE, httpdate::fmt_http_date(std::time::SystemTime::now()))
                .body(Body::from("Invalid URI encoding"))
                .unwrap();
            return Ok(response);
        }
    };

    // Get the plugin pipeline for this host
    let pipeline = {
        let host_pipelines = app_state.host_pipelines.read().await;
        host_pipelines.get(&host_name).cloned()
    };
    
    let pipeline = match pipeline {
        Some(p) => p,
        None => {
            eprintln!("No pipeline found for host: {}", host_name);
            let response = Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "text/plain")
                .header(hyper::header::SERVER, DEFAULT_SERVER_HEADER)
                .header(hyper::header::DATE, httpdate::fmt_http_date(std::time::SystemTime::now()))
                .body(Body::from("Host not found"))
                .unwrap();
            return Ok(response);
        }
    };
    
    eprintln!("Found pipeline with {} plugins for host: {}", pipeline.len(), host_name);

    // Check for unsupported methods
    match req.method() {
        &hyper::Method::GET | &hyper::Method::HEAD | &hyper::Method::POST | 
        &hyper::Method::PUT | &hyper::Method::DELETE | &hyper::Method::OPTIONS => {
            // Supported methods, continue
        }
        _ => {
            // Unsupported method, return 405
            let response = Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header("Allow", "GET, HEAD, POST, PUT, DELETE, OPTIONS")
                .header(hyper::header::SERVER, DEFAULT_SERVER_HEADER)
                .header(hyper::header::DATE, httpdate::fmt_http_date(std::time::SystemTime::now()))
                .body(Body::from("Method not allowed"))
                .unwrap();
            return Ok(response);
        }
    }
    
    // Create a PluginRequest
    let mut plugin_request = PluginRequest::new(req, path.clone());
    
    // Get host configuration
    let (host_config_map, server_config_map) = {
        let config = app_state.config.read().await;
        let host_config = config.hosts.get(&host_name)
            .map(|hc| {
                let mut map = HashMap::new();
                map.insert("hostRoot".to_string(), hc.host_root.clone());
                if let Some(server_header) = &hc.server_header {
                    map.insert("serverHeader".to_string(), server_header.clone());
                }
                map
            })
            .unwrap_or_default();
        
        let mut server_map = HashMap::new();
        server_map.insert("serverRoot".to_string(), config.server_root.clone());
        server_map.insert("bindAddress".to_string(), config.bind_address.clone());
        server_map.insert("bindPort".to_string(), config.bind_port.to_string());
        
        (host_config, server_map)
    };
    
    // Create a plugin context with runtime handle
    let plugin_context = PluginContext {
        plugin_config: HashMap::new(),
        host_config: host_config_map,
        server_config: server_config_map,
        host_name: host_name.clone(),
        request_id: Uuid::new_v4().to_string(),
        runtime_handle: Some(tokio::runtime::Handle::current()),
    };
    
    // Execute the plugin pipeline
    for (i, plugin) in pipeline.iter().enumerate() {
        eprintln!("Executing plugin {} ({})", i + 1, plugin.name());
        
        if let Some(mut response) = plugin.handle_request(&mut plugin_request, &plugin_context).await {
            eprintln!("Plugin {} returned a response", plugin.name());
            // Add standard headers if not present
            if !response.headers().contains_key(hyper::header::SERVER) {
                response.headers_mut().insert(
                    hyper::header::SERVER,
                    hyper::header::HeaderValue::from_static(DEFAULT_SERVER_HEADER)
                );
            }
            if !response.headers().contains_key(hyper::header::DATE) {
                let now = httpdate::fmt_http_date(std::time::SystemTime::now());
                if let Ok(date_value) = hyper::header::HeaderValue::from_str(&now) {
                    response.headers_mut().insert(hyper::header::DATE, date_value);
                }
            }
            return Ok(response);
        }
    }
    
    eprintln!("No plugin handled the request, returning 404");
    
    // If no plugin handled the request, return 404
    let response = Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("Content-Type", "text/plain")
        .header(hyper::header::SERVER, DEFAULT_SERVER_HEADER)
        .header(hyper::header::DATE, httpdate::fmt_http_date(std::time::SystemTime::now()))
        .body(Body::from("File not found"))
        .unwrap();
    Ok(response)
}

/// Handle incoming requests using plugin architecture
async fn handle_request(req: Request<Body>, app_state: AppState) -> Result<Response<Body>> {
    // Decode percent-encoded URI path (RFC 3986)
    let raw_path = req.uri().path();
    let path = match urlencoding::decode(raw_path) {
        Ok(decoded) => decoded.into_owned(),
        Err(_) => {
            let response = Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(hyper::header::SERVER, DEFAULT_SERVER_HEADER)
                .header("Content-Type", "text/plain")
                .body(Body::from("Invalid URI encoding"))
                .unwrap();
            return Ok(response);
        }
    };
    
    // Update request path
    let mut req = req;
    let uri = req.uri().clone();
    let query = uri.query().map(|q| format!("?{}", q)).unwrap_or_default();
    let mut parts = uri.into_parts();
    parts.path_and_query = Some(
        format!("{}{}", path, query).parse().unwrap()
    );
    *req.uri_mut() = hyper::Uri::from_parts(parts).unwrap();
    
    process_request_through_pipeline(req, app_state).await
}

#[tokio::main]
async fn main() {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 2 {
        eprintln!("Usage: {} <config-file>", args[0]);
        eprintln!("Example: {} config/config.html", args[0]);
        std::process::exit(1);
    }
    
    let config_path = args[1].clone();
    
    // Validate config file exists
    if !std::path::Path::new(&config_path).exists() {
        eprintln!("Error: Configuration file '{}' not found", config_path);
        std::process::exit(1);
    }
    
    println!("Starting Rusty Beam with plugin architecture...");
    
    // Initialize application state
    let app_state = AppState::new(config_path).await;
    
    // Set up signal handling for configuration reload
    let signals = Signals::new([SIGHUP]).expect("Failed to register signal handler");
    let handle = signals.handle();
    let app_state_for_signals = app_state.clone();
    
    let signals_task = tokio::spawn(async move {
        let mut signals = signals;
        while let Some(signal) = signals.next().await {
            match signal {
                SIGHUP => {
                    println!("Received SIGHUP, reloading configuration...");
                    if let Err(e) = app_state_for_signals.reload().await {
                        eprintln!("Failed to reload configuration: {}", e);
                    } else {
                        println!("Configuration reloaded successfully");
                    }
                }
                _ => unreachable!(),
            }
        }
    });
    
    // Create service with app state
    let app_state_for_service = app_state.clone();
    let make_svc = make_service_fn(move |_conn| {
        let app_state = app_state_for_service.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                let app_state = app_state.clone();
                handle_request(req, app_state)
            }))
        }
    });
    
    // Get bind address from current config
    let addr = {
        let config = app_state.config.read().await;
        format!("{}:{}", config.bind_address, config.bind_port)
            .parse::<std::net::SocketAddr>()
            .expect("Invalid address format")
    };
    
    // Attempt to bind to the address gracefully
    let server = match Server::try_bind(&addr) {
        Ok(builder) => builder.serve(make_svc),
        Err(e) => {
            let config = app_state.config.read().await;
            eprintln!("Failed to start server on {}:{}", config.bind_address, config.bind_port);
            eprintln!("Error: {}", e);
            
            // Provide helpful error message for common issues
            if e.to_string().contains("Address already in use") {
                eprintln!("It looks like another server is already running on this port.");
                eprintln!("Please either:");
                eprintln!("  - Stop the other server");
                eprintln!("  - Change the port in config.html");
                eprintln!("  - Use a different bind address");
            } else if e.to_string().contains("Permission denied") {
                eprintln!("Permission denied. You may need to:");
                eprintln!("  - Use a port number above 1024");
                eprintln!("  - Run with appropriate permissions for privileged ports");
            }
            
            std::process::exit(1);
        }
    };
    
    {
        let config = app_state.config.read().await;
        println!(
            "Rusty Beam server running on http://{}:{}",
            config.bind_address, config.bind_port
        );
        println!("PID: {}", std::process::id());
        println!("Send SIGHUP to reload configuration");
    }
    
    // Run server and signal handler concurrently
    tokio::select! {
        result = server => {
            if let Err(e) = result {
                eprintln!("Server error: {}", e);
                std::process::exit(1);
            }
        }
        _ = signals_task => {
            eprintln!("Signal handler task ended");
        }
    }
    
    // Cleanup signal handler
    handle.close();
}