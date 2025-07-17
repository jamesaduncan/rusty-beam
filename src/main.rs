//! Rusty Beam - Plugin-based HTTP server
//!
//! This server uses a plugin architecture for CSS selector-based HTML manipulation
//! via HTTP Range headers, with support for authentication, authorization, and logging.

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
            // Load dynamic library
            load_dynamic_plugin(library_path, v2_config)
        }
        Some("wasm") => {
            log_verbose!("WASM plugins are not supported: {}", library_url);
            None
        }
        _ => {
            log_verbose!("Warning: Unknown plugin: {}", library_url);
            None
        }
    }
}


/// Load a plugin from a dynamic library (.so/.dll/.dylib)
/// 
/// # Safety
/// 
/// This function performs several unsafe operations:
/// 
/// 1. **Dynamic Library Loading**: Uses `libloading` to load arbitrary shared libraries.
///    The library must be trusted as it can execute arbitrary code.
/// 
/// 2. **FFI Function Call**: Calls an external C function `create_plugin` which must:
///    - Accept a null-terminated C string containing JSON configuration
///    - Return a valid pointer to a `Box<Box<dyn Plugin>>` or null
///    - Not panic or unwind across the FFI boundary
/// 
/// 3. **Pointer Casting**: The returned void pointer is cast to `Box<Box<dyn Plugin>>`.
///    The plugin must ensure this pointer is valid and properly aligned.
/// 
/// 4. **Memory Management**: The plugin transfers ownership of the boxed plugin to Rust.
///    The plugin must not free or access this memory after returning.
/// 
/// The plugin library must follow these conventions for safe operation.
fn load_dynamic_plugin(
    library_path: &str,
    config: HashMap<String, String>,
) -> Option<Box<dyn rusty_beam_plugin_api::Plugin>> {
    use libloading::{Library, Symbol};

    // Use a closure to handle errors uniformly
    let load_plugin_inner = || -> Option<Box<dyn rusty_beam_plugin_api::Plugin>> {
        unsafe {
            // Load the dynamic library
            // SAFETY: The library path must point to a valid plugin library
            let lib = Library::new(library_path).ok()?;

            // Look for the plugin creation function
            // Convention: extern "C" fn create_plugin(config: *const c_char) -> *mut c_void
            let create_fn: Symbol<
                unsafe extern "C" fn(*const std::os::raw::c_char) -> *mut std::ffi::c_void,
            > = lib.get(b"create_plugin").ok()?;

            // Serialize config to JSON for passing to plugin
            let config_json = serde_json::to_string(&config).ok()?;
            let config_cstr = std::ffi::CString::new(config_json).ok()?;

            // Call the plugin creation function
            // SAFETY: The create_fn must follow the documented conventions
            let plugin_ptr = create_fn(config_cstr.as_ptr());
            if plugin_ptr.is_null() {
                return None;
            }

            // Cast the void pointer back to Box<Box<dyn Plugin>> and unwrap one level
            // SAFETY: The plugin must return a valid Box<Box<dyn Plugin>> pointer
            let plugin_box = Box::from_raw(plugin_ptr as *mut Box<dyn rusty_beam_plugin_api::Plugin>);
            let plugin = *plugin_box;
            let wrapper = DynamicPluginWrapper {
                _library: lib,
                plugin,
            };
            Some(Box::new(wrapper))
        }
    };

    // Execute and log any failures
    match load_plugin_inner() {
        Some(plugin) => Some(plugin),
        None => {
            log_verbose!("Failed to load plugin from {}", library_path);
            None
        }
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

    log_verbose!("Creating host pipelines for {} hosts", config.hosts.len());

    for (host_name, host_config) in &config.hosts {
        let mut pipeline: Vec<Arc<dyn rusty_beam_plugin_api::Plugin>> = Vec::new();

        log_verbose!(
            "Loading {} plugins for host: {}",
            host_config.plugins.len(),
            host_name
        );

        // Load plugins in order from config
        for plugin_config in &host_config.plugins {
            log_verbose!("Attempting to load plugin: {}", plugin_config.library);
            if let Some(plugin) = load_plugin(plugin_config) {
                log_verbose!("Successfully loaded plugin: {}", plugin_config.library);
                pipeline.push(Arc::from(plugin));
            } else {
                log_verbose!("Failed to load plugin: {}", plugin_config.library);
            }
        }

        log_verbose!("Host {} has {} loaded plugins", host_name, pipeline.len());
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

    log_verbose!(
        "Processing request for host: {}, path: {}",
        host_name,
        raw_path
    );

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
            log_verbose!("No pipeline found for host: {}", host_name);
            let response = create_error_response(StatusCode::NOT_FOUND, "Host not found");
            return Ok(PipelineResult {
                response,
                upgrade_handler: None,
            });
        }
    };

    log_verbose!(
        "Found pipeline with {} plugins for host: {}",
        pipeline.len(),
        host_name
    );

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
    
    for (i, plugin) in pipeline.iter().enumerate() {
        log_verbose!("Executing plugin {} ({})", i + 1, plugin.name());

        if let Some(plugin_response) = plugin
            .handle_request(&mut plugin_request, &plugin_context)
            .await
        {
            log_verbose!("Plugin {} returned a response", plugin.name());
            
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

    log_verbose!("No plugin handled the request, returning 404");

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
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    let mut verbose = false;
    let mut config_path = None;

    // Simple argument parsing
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

    // Validate config file exists and make path absolute
    let config_path = match std::fs::canonicalize(&config_path) {
        Ok(absolute_path) => absolute_path.to_string_lossy().to_string(),
        Err(e) => {
            eprintln!("Error: Configuration file '{}' not found or inaccessible: {}", config_path, e);
            std::process::exit(1);
        }
    };

    // Load config early to get daemon settings (before daemonization)
    let config_for_daemon = load_config_from_html(&config_path);
    
    // Daemonize early if needed (before tokio runtime)
    if !verbose {
        // Show PID before daemonizing so user knows the process ID
        println!("PID: {}", std::process::id());
        
        // Get working directory for daemon - use config setting or default to current dir
        let working_dir = if let Some(ref work_dir) = config_for_daemon.daemon_working_directory {
            std::path::PathBuf::from(work_dir)
        } else {
            // Default to current working directory to preserve relative paths in config
            std::env::current_dir()
                .expect("Failed to get current working directory")
        };
        
        // Create daemon configuration with settings from config file
        let mut daemon = Daemonize::new()
            .working_directory(&working_dir)
            .umask(config_for_daemon.daemon_umask.unwrap_or(0o027));
            
        // Set PID file if configured
        if let Some(ref pid_file) = config_for_daemon.daemon_pid_file {
            daemon = daemon.pid_file(pid_file);
        }
        
        // Set chown PID file if configured
        if let Some(chown_pid) = config_for_daemon.daemon_chown_pid_file {
            daemon = daemon.chown_pid_file(chown_pid);
        }
        
        // Set user if configured
        if let Some(ref user) = config_for_daemon.daemon_user {
            daemon = daemon.user(user.as_str());
        }
        
        // Set group if configured
        if let Some(ref group) = config_for_daemon.daemon_group {
            // Try to parse as number first, then as name
            if let Ok(gid) = group.parse::<u32>() {
                daemon = daemon.group(gid);
            } else {
                daemon = daemon.group(group.as_str());
            }
        }
        
        // Set stdout redirect if configured
        if let Some(ref stdout_path) = config_for_daemon.daemon_stdout {
            if let Ok(stdout_file) = std::fs::File::create(stdout_path) {
                daemon = daemon.stdout(stdout_file);
            } else {
                eprintln!("Warning: Could not create stdout file: {}", stdout_path);
            }
        }
        
        // Set stderr redirect if configured  
        if let Some(ref stderr_path) = config_for_daemon.daemon_stderr {
            if let Ok(stderr_file) = std::fs::File::create(stderr_path) {
                daemon = daemon.stderr(stderr_file);
            } else {
                eprintln!("Warning: Could not create stderr file: {}", stderr_path);
            }
        }
        
        // Daemonize the process
        match daemon.start() {
            Ok(_) => {
                // We're now running as a daemon
                // The original process has exited, so any further output
                // goes to /dev/null unless redirected
            },
            Err(e) => {
                eprintln!("Failed to daemonize: {}", e);
                std::process::exit(1);
            }
        }
    }

    // Initialize logging after daemonization
    logging::init_logging(verbose);

    log_verbose!("Starting Rusty Beam with plugin architecture...");

    // Create tokio runtime after daemonization
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("Failed to create tokio runtime: {}", e);
            std::process::exit(1);
        }
    };

    // Run the async main function
    runtime.block_on(async_main(config_path, verbose));
}

async fn async_main(config_path: String, verbose: bool) {
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
            eprintln!(
                "Failed to start server on {}:{}",
                config.bind_address, config.bind_port
            );
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
        
        // Print startup information (daemonization already happened in main())
        if verbose {
            // In verbose mode, show startup information
            println!("PID: {}", std::process::id());
            println!(
                "Rusty Beam server running on http://{}:{}",
                config.bind_address,
                config.bind_port
            );
            println!("Send SIGHUP to reload configuration");
        }
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
            log_verbose!("Signal handler task ended");
        }
    }

    // Cleanup signal handler
    handle.close();
}
