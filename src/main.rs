//! # Rusty Beam - Plugin-based HTTP Server
//!
//! Rusty Beam is a high-performance HTTP server built with a plugin architecture that enables
//! CSS selector-based HTML manipulation via HTTP Range headers. The server supports real-time
//! WebSocket connections, authentication, authorization, and comprehensive logging.
//!
//! ## Architecture
//!
//! The server uses a dynamic plugin system where each request passes through a pipeline of
//! plugins that can modify, intercept, or respond to HTTP requests. Core plugins include:
//!
//! - **Authentication & Authorization**: OAuth2, Basic Auth, and role-based access control
//! - **Content Processing**: CSS selector-based HTML manipulation, file serving
//! - **Real-time Communication**: WebSocket support for live updates
//! - **Observability**: Access logging, error handling, health checks
//! - **Performance**: Rate limiting, compression, caching
//!
//! ## Plugin Pipeline
//!
//! Each host can have its own plugin pipeline configured via HTML microdata. Plugins are loaded
//! dynamically from shared libraries and execute in a defined order to process requests.
//!
//! ## Configuration
//!
//! The server is configured through HTML files using microdata attributes, allowing for
//! structured configuration that's both human-readable and machine-parseable.
//!
//! ## Deployment
//!
//! Supports daemon mode with configurable process management, signal handling for
//! configuration reloads, and graceful shutdown.

// Import modules
mod config;
mod constants;
mod logging;

use async_trait::async_trait;
use config::PluginConfig;
use config::{ServerConfig, load_config_from_html};
use constants::DEFAULT_SERVER_HEADER;

/// Plugin URL scheme constants
const PLUGIN_SCHEME_PIPELINE: &str = "pipeline://nested";
const PLUGIN_SCHEME_DIRECTORY_PREFIX: &str = "directory://";
const PLUGIN_SCHEME_FILE_PREFIX: &str = "file://";
use rusty_beam_plugin_api::{PluginContext, PluginRequest, PluginResponse};

use futures::stream::StreamExt;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Result, Server, StatusCode};
use signal_hook::consts::SIGHUP;
use signal_hook_tokio::Signals;
use std::collections::HashMap;
use std::convert::Infallible;
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use daemonize::Daemonize;

/// Type alias for host pipelines
type HostPipelines = HashMap<String, Vec<Arc<dyn rusty_beam_plugin_api::Plugin>>>;

/// Command line arguments
struct Args {
    verbose: bool,
    config_path: String,
}

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
    
    // Serialize nested plugins to JSON for FFI compatibility
    // The plugin FFI boundary only supports HashMap<String, String>, so complex
    // data structures must be serialized. This creates a double-conversion:
    // HTML microdata -> PluginConfig struct -> JSON string -> PluginConfig struct
    // See directory plugin for discussion of future improvements.
    if !plugin_config.nested_plugins.is_empty() {
        if let Ok(nested_json) = serde_json::to_string(&plugin_config.nested_plugins) {
            v2_config.insert("nested_plugins".to_string(), nested_json);
        }
    }

    let library_url = &plugin_config.library;

    // Map special URLs to actual plugin paths
    let library_path = match library_url.as_str() {
        PLUGIN_SCHEME_PIPELINE => "file://./plugins/libpipeline.so",
        url if url.starts_with(PLUGIN_SCHEME_DIRECTORY_PREFIX) => "file://./plugins/libdirectory.so",
        url => url,
    };

    // All plugins must be loaded from external libraries - no built-ins

    // Handle file:// URLs
    let library_path = library_path.strip_prefix(PLUGIN_SCHEME_FILE_PREFIX).unwrap_or(library_path);

    let path = Path::new(library_path);
    let extension = path.extension().and_then(OsStr::to_str);

    match extension {
        Some("so") | Some("dll") | Some("dylib") => {
            load_dynamic_plugin(library_path, v2_config)
        }
        _ => None, // WASM and other formats not supported
    }
}


/// Loads a plugin from a dynamic library
fn load_dynamic_plugin(
    library_path: &str,
    config: HashMap<String, String>,
) -> Option<Box<dyn rusty_beam_plugin_api::Plugin>> {
    match create_plugin_instance(library_path, config) {
        Ok(plugin) => Some(plugin),
        Err(error) => {
            eprintln!("Failed to load plugin {}: {}", library_path, error);
            None
        }
    }
}

/// Creates a plugin instance via FFI
///
/// # Safety
/// Loads external code through FFI. The plugin must follow documented conventions.
fn create_plugin_instance(
    library_path: &str,
    config: HashMap<String, String>,
) -> std::result::Result<Box<dyn rusty_beam_plugin_api::Plugin>, String> {
    use libloading::{Library, Symbol};
    
    unsafe {
        let lib = Library::new(library_path)
            .map_err(|e| format!("Failed to load library: {}", e))?;
            
        let create_fn: Symbol<
            unsafe extern "C" fn(*const std::os::raw::c_char) -> *mut std::ffi::c_void,
        > = lib.get(b"create_plugin")
            .map_err(|_| "Plugin missing create_plugin function")?;
            
        let config_json = serde_json::to_string(&config)
            .map_err(|e| format!("Config serialization failed: {}", e))?;
        let config_cstr = std::ffi::CString::new(config_json)
            .map_err(|_| "Invalid config string")?;
            
        let plugin_ptr = create_fn(config_cstr.as_ptr());
        if plugin_ptr.is_null() {
            return Err("Plugin creation returned null".to_string());
        }
        
        let plugin_box = Box::from_raw(plugin_ptr as *mut Box<dyn rusty_beam_plugin_api::Plugin>);
        let plugin = *plugin_box;
        
        Ok(Box::new(DynamicPluginWrapper {
            _library: lib,
            plugin,
        }))
    }
}

/// Dynamic library plugin wrapper that keeps the library loaded
struct DynamicPluginWrapper {
    _library: libloading::Library, // Keep library alive
    plugin: Box<dyn rusty_beam_plugin_api::Plugin>,
}

impl std::fmt::Debug for DynamicPluginWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DynamicPlugin({})", self.plugin.name())
    }
}

#[async_trait]
impl rusty_beam_plugin_api::Plugin for DynamicPluginWrapper {
    async fn handle_request(
        &self,
        request: &mut PluginRequest,
        context: &PluginContext,
    ) -> Option<PluginResponse> {
        self.plugin.handle_request(request, context).await
    }

    async fn handle_response(
        &self,
        request: &PluginRequest,
        response: &mut Response<Body>,
        context: &PluginContext,
    ) {
        self.plugin
            .handle_response(request, response, context)
            .await
    }

    fn name(&self) -> &str {
        self.plugin.name()
    }
}


/// Create a standardized error response with proper headers
fn create_error_response(status: StatusCode, body: &str) -> Response<Body> {
    create_error_response_with_headers(status, body, vec![])
}

/// Create a standardized error response with proper headers and additional custom headers
fn create_error_response_with_headers(
    status: StatusCode,
    body: &str,
    additional_headers: Vec<(&str, &str)>,
) -> Response<Body> {
    let mut builder = Response::builder()
        .status(status)
        .header("Content-Type", "text/plain")
        .header(hyper::header::SERVER, DEFAULT_SERVER_HEADER)
        .header(
            hyper::header::DATE,
            httpdate::fmt_http_date(std::time::SystemTime::now()),
        );
    
    for (name, value) in additional_headers {
        builder = builder.header(name, value);
    }
    
    builder.body(Body::from(body.to_string())).unwrap()
}

fn create_host_pipelines(config: &ServerConfig) -> HostPipelines {
    let mut host_pipelines = HashMap::new();

    // Create pipelines for each configured host

    for (host_name, host_config) in &config.hosts {
        let mut pipeline: Vec<Arc<dyn rusty_beam_plugin_api::Plugin>> = Vec::new();

        // Load plugins for this host

        // Load plugins in order from config
        for plugin_config in &host_config.plugins {
            // Attempt to load the plugin
            if let Some(plugin) = load_plugin(plugin_config) {
                // Plugin loaded successfully
                pipeline.push(Arc::from(plugin));
            } else {
                eprintln!("Warning: Failed to load plugin: {}", plugin_config.library);
            }
        }

        // Pipeline configured for host
        host_pipelines.insert(host_name.clone(), pipeline);
    }

    host_pipelines
}

/// Result of processing a request through the plugin pipeline
struct PipelineResult {
    response: Response<Body>,
    upgrade_handler: Option<rusty_beam_plugin_api::UpgradeHandler>,
}

/// Process request through plugin pipeline
async fn process_request_through_pipeline(
    req: Request<Body>,
    app_state: AppState,
) -> Result<PipelineResult> {
    use std::collections::HashMap;

    let raw_path = req.uri().path();
    let host_name = req
        .headers()
        .get(hyper::header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost")
        .split(':')
        .next()
        .unwrap_or("localhost")
        .to_lowercase();

    // Process request for the given host and path

    // Decode percent-encoded URI path (RFC 3986)
    let path = match urlencoding::decode(raw_path) {
        Ok(decoded) => decoded.into_owned(),
        Err(_) => {
            let response = create_error_response(StatusCode::BAD_REQUEST, "Invalid URI encoding");
            return Ok(PipelineResult {
                response,
                upgrade_handler: None,
            });
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
            let response = create_error_response(StatusCode::NOT_FOUND, "Host not found");
            return Ok(PipelineResult {
                response,
                upgrade_handler: None,
            });
        }
    };

    // Execute the plugin pipeline

    // Check for unsupported methods
    match req.method() {
        &hyper::Method::GET
        | &hyper::Method::HEAD
        | &hyper::Method::POST
        | &hyper::Method::PUT
        | &hyper::Method::DELETE
        | &hyper::Method::OPTIONS => {
            // Supported methods, continue
        }
        _ => {
            // Unsupported method, return 405
            let response = create_error_response_with_headers(
                StatusCode::METHOD_NOT_ALLOWED,
                "Method not allowed",
                vec![("Allow", "GET, HEAD, POST, PUT, DELETE, OPTIONS")],
            );
            return Ok(PipelineResult {
                response,
                upgrade_handler: None,
            });
        }
    }

    // Create a PluginRequest
    let mut plugin_request = PluginRequest::new(req, path.clone());

    // Get host configuration
    let (host_config_map, server_config_map) = {
        let config = app_state.config.read().await;
        let host_config = config
            .hosts
            .get(&host_name)
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

    // Create server metadata
    let mut server_metadata = HashMap::new();
    server_metadata.insert("config_file_path".to_string(), app_state.config_path.clone());

    // Create a plugin context with runtime handle
    let plugin_context = PluginContext {
        plugin_config: HashMap::new(),
        host_config: host_config_map,
        server_config: server_config_map,
        server_metadata,
        host_name: host_name.clone(),
        request_id: Uuid::new_v4().to_string(),
        runtime_handle: Some(tokio::runtime::Handle::current()),
        verbose: logging::is_verbose(),
    };

    // Execute the plugin pipeline
    let mut final_response = None;
    let mut upgrade_handler = None;
    
    for (_i, plugin) in pipeline.iter().enumerate() {
        // Execute plugin in pipeline

        if let Some(plugin_response) = plugin
            .handle_request(&mut plugin_request, &plugin_context)
            .await
        {
            // Plugin handled the request
            
            let mut response = plugin_response.response;
            upgrade_handler = plugin_response.upgrade;
            
            // Add standard headers if not present
            if !response.headers().contains_key(hyper::header::SERVER) {
                response.headers_mut().insert(
                    hyper::header::SERVER,
                    hyper::header::HeaderValue::from_static(DEFAULT_SERVER_HEADER),
                );
            }
            if !response.headers().contains_key(hyper::header::DATE) {
                let now = httpdate::fmt_http_date(std::time::SystemTime::now());
                if let Ok(date_value) = hyper::header::HeaderValue::from_str(&now) {
                    response
                        .headers_mut()
                        .insert(hyper::header::DATE, date_value);
                }
            }
            final_response = Some(response);
            break;
        }
    }

    // If we have a response, call handle_response on all plugins
    if let Some(mut response) = final_response {
        for plugin in pipeline.iter() {
            plugin
                .handle_response(&plugin_request, &mut response, &plugin_context)
                .await;
        }
        
        return Ok(PipelineResult { 
            response, 
            upgrade_handler 
        });
    }

    // No plugin handled the request

    // If no plugin handled the request, return 404
    let response = create_error_response(StatusCode::NOT_FOUND, "File not found");
    Ok(PipelineResult {
        response,
        upgrade_handler: None,
    })
}

/// Handle incoming requests using plugin architecture
async fn handle_request(req: Request<Body>, app_state: AppState) -> Result<Response<Body>> {
    // Check if this might be an upgrade request before processing
    let mut req = req;
    let is_upgrade = req.method() != &hyper::Method::OPTIONS &&
        req.headers()
            .get(hyper::header::CONNECTION)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.to_lowercase().contains("upgrade"))
            .unwrap_or(false) &&
        req.headers()
            .get("sec-websocket-key")
            .is_some();
    
    // Get the on_upgrade future if this is an upgrade request
    let on_upgrade = if is_upgrade {
        Some(hyper::upgrade::on(&mut req))
    } else {
        None
    };
    
    let pipeline_result = process_request_through_pipeline(req, app_state).await?;
    
    // Handle upgrade if present
    if let Some(upgrade_handler) = pipeline_result.upgrade_handler {
        if let Some(on_upgrade) = on_upgrade {
            // Check if this is an upgrade request
            if pipeline_result.response.status() == StatusCode::SWITCHING_PROTOCOLS {
                // Spawn the upgrade handler
                tokio::spawn(async move {
                    match on_upgrade.await {
                        Ok(upgraded) => {
                            if let Err(e) = upgrade_handler(upgraded).await {
                                eprintln!("Upgrade handler error: {:?}", e);
                            }
                        }
                        Err(e) => eprintln!("Upgrade error: {:?}", e),
                    }
                });
            }
        }
    }
    
    Ok(pipeline_result.response)
}

fn main() {
    let args = parse_command_line();
    let config_path = validate_config_path(&args.config_path);
    let config = load_config_from_html(&config_path);
    
    // Daemonize if not in verbose mode
    if !args.verbose {
        println!("PID: {}", std::process::id());
        daemonize_server(&config);
    }

    // Initialize logging after daemonization
    logging::init_logging(args.verbose);

    println!("Starting Rusty Beam with plugin architecture...");

    // Create tokio runtime after daemonization
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("Failed to create tokio runtime: {}", e);
            std::process::exit(1);
        }
    };

    // Run the async main function
    runtime.block_on(async_main(config_path, args.verbose));
}

async fn async_main(config_path: String, verbose: bool) {
    let app_state = AppState::new(config_path).await;
    
    // Set up signal handling
    let signals_task = setup_signal_handler(app_state.clone());
    
    // Start server
    tokio::select! {
        result = start_http_server(&app_state, verbose) => {
            if let Err(e) = result {
                eprintln!("Server error: {}", e);
                std::process::exit(1);
            }
        }
        _ = signals_task => {
            // Signal handler task ended
        }
    }
}

/// Sets up SIGHUP signal handler for configuration reload
fn setup_signal_handler(app_state: AppState) -> tokio::task::JoinHandle<()> {
    let signals = Signals::new([SIGHUP]).expect("Failed to register signal handler");
    
    tokio::spawn(async move {
        let mut signals = signals;
        while let Some(signal) = signals.next().await {
            if signal == SIGHUP {
                println!("Received SIGHUP, reloading configuration...");
                match app_state.reload().await {
                    Ok(()) => println!("Configuration reloaded successfully"),
                    Err(e) => eprintln!("Failed to reload configuration: {}", e),
                }
            }
        }
    })
}

/// Starts the HTTP server
async fn start_http_server(app_state: &AppState, verbose: bool) -> std::result::Result<(), hyper::Error> {
    let config = app_state.config.read().await;
    let addr = format!("{}:{}", config.bind_address, config.bind_port)
        .parse::<std::net::SocketAddr>()
        .expect("Invalid address format");
    
    let make_svc = make_service_fn(move |_conn| {
        let app_state = app_state.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                let app_state = app_state.clone();
                handle_request(req, app_state)
            }))
        }
    });
    
    let server = match Server::try_bind(&addr) {
        Ok(builder) => builder.serve(make_svc),
        Err(e) => {
            handle_bind_error(e, &config.bind_address, config.bind_port);
        }
    };
    
    if verbose {
        print_startup_info(app_state).await;
    }
    
    server.await
}

/// Handles server bind errors with helpful messages
fn handle_bind_error(error: hyper::Error, bind_address: &str, bind_port: u16) -> ! {
    eprintln!("Failed to start server on {}:{}", bind_address, bind_port);
    eprintln!("Error: {}", error);
    
    let error_msg = error.to_string();
    if error_msg.contains("Address already in use") {
        eprintln!("\nAnother process is using this port. Try:");
        eprintln!("  - Stopping the other server");
        eprintln!("  - Changing the port in your config file");
        eprintln!("  - Using a different bind address");
    } else if error_msg.contains("Permission denied") {
        eprintln!("\nPermission denied. Try:");
        eprintln!("  - Using a port number above 1024");
        eprintln!("  - Running with appropriate permissions");
    }
    
    std::process::exit(1);
}

/// Prints server startup information
async fn print_startup_info(app_state: &AppState) {
    let config = app_state.config.read().await;
    println!("PID: {}", std::process::id());
    println!("Rusty Beam server running on http://{}:{}", config.bind_address, config.bind_port);
    println!("Send SIGHUP to reload configuration");
}

/// Parses command line arguments
fn parse_command_line() -> Args {
    let args: Vec<String> = env::args().collect();
    let mut verbose = false;
    let mut config_path = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-v" | "--verbose" => verbose = true,
            arg if !arg.starts_with('-') => {
                if config_path.is_none() {
                    config_path = Some(arg.to_string());
                }
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                std::process::exit(1);
            }
        }
        i += 1;
    }

    let config_path = match config_path {
        Some(path) => path,
        None => {
            eprintln!("Usage: {} [-v|--verbose] <config-file>", args[0]);
            eprintln!("Example: {} config/config.html", args[0]);
            eprintln!("         {} -v config/config.html", args[0]);
            std::process::exit(1);
        }
    };

    Args { verbose, config_path }
}

/// Validates the config file path and returns the absolute path
fn validate_config_path(config_path: &str) -> String {
    match std::fs::canonicalize(config_path) {
        Ok(absolute_path) => absolute_path.to_string_lossy().to_string(),
        Err(e) => {
            eprintln!("Error: Configuration file '{}' not found or inaccessible: {}", config_path, e);
            std::process::exit(1);
        }
    }
}

/// Configures and starts the server as a daemon
fn daemonize_server(config: &ServerConfig) {
    let working_dir = config.daemon_working_directory.as_ref()
        .map(|dir| std::path::PathBuf::from(dir))
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));
        
    let mut daemon = Daemonize::new()
        .working_directory(&working_dir)
        .umask(config.daemon_umask.unwrap_or(0o027));
        
    if let Some(ref pid_file) = config.daemon_pid_file {
        daemon = daemon.pid_file(pid_file);
    }
    
    if let Some(chown_pid) = config.daemon_chown_pid_file {
        daemon = daemon.chown_pid_file(chown_pid);
    }
    
    if let Some(ref user) = config.daemon_user {
        daemon = daemon.user(user.as_str());
    }
    
    if let Some(ref group) = config.daemon_group {
        daemon = if let Ok(gid) = group.parse::<u32>() {
            daemon.group(gid)
        } else {
            daemon.group(group.as_str())
        };
    }
    
    if let Some(ref stdout_path) = config.daemon_stdout {
        if let Ok(stdout_file) = std::fs::File::create(stdout_path) {
            daemon = daemon.stdout(stdout_file);
        }
    }
    
    if let Some(ref stderr_path) = config.daemon_stderr {
        if let Ok(stderr_file) = std::fs::File::create(stderr_path) {
            daemon = daemon.stderr(stderr_file);
        }
    }
    
    if let Err(e) = daemon.start() {
        eprintln!("Failed to daemonize: {}", e);
        std::process::exit(1);
    }
}
