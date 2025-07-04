mod config;
mod handlers;
mod utils;
mod plugins;
mod auth;

#[cfg(test)]
mod auth_integration_tests;

#[cfg(test)]
mod integration_tests;

use config::{load_config_from_html, ServerConfig};
use handlers::*;
use utils::canonicalize_file_path;
use plugins::{PluginManager, AuthResult, AuthzResult, PluginRegistry, UserInfo, AccessLogEntry};
use auth::AuthorizedUser;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Result, Server, StatusCode};
use std::convert::Infallible;
use std::path::Path;
use std::sync::Arc;
use std::env;
use tokio::sync::RwLock;
use signal_hook::consts::SIGHUP;
use signal_hook_tokio::Signals;
use futures::stream::StreamExt;

// Shared application state for hot-reloading
#[derive(Clone)]
struct AppState {
    config: Arc<RwLock<ServerConfig>>,
    plugin_manager: Arc<RwLock<PluginManager>>,
    config_path: String,
}

impl AppState {
    fn new(config_path: String) -> Self {
        let config = load_config_from_html(&config_path);
        let plugin_manager = create_plugin_manager(&config);
        
        Self {
            config: Arc::new(RwLock::new(config)),
            plugin_manager: Arc::new(RwLock::new(plugin_manager)),
            config_path,
        }
    }
    
    async fn reload(&self) -> std::result::Result<(), String> {
        // Reloading configuration silently
        
        // Load new configuration
        let new_config = load_config_from_html(&self.config_path);
        
        // Note: Plugin reloading has limitations with dynamic libraries.
        // For production use, consider restarting the server for plugin changes.
        // Configuration changes (hosts, paths, etc.) will take effect immediately.
        let new_plugin_manager = create_plugin_manager(&new_config);
        
        // Atomically update the shared state
        {
            let mut config_lock = self.config.write().await;
            *config_lock = new_config;
        }
        
        {
            let mut plugin_lock = self.plugin_manager.write().await;
            *plugin_lock = new_plugin_manager;
        }
        
        // Configuration reloaded successfully
        Ok(())
    }
}

fn create_plugin_manager(config: &ServerConfig) -> PluginManager {
    let mut manager = PluginManager::new();
    
    // Load plugins from configuration dynamically
    for (host_name, host_config) in &config.hosts {
        for plugin_config in &host_config.plugins {
            match plugin_config.plugin_type.as_deref() {
                Some(plugin_type) => {
                    // Normalize plugin type: trim whitespace and convert to lowercase
                    let normalized_type = plugin_type.trim().to_lowercase();
                    
                    // Check if it contains "authorization" or "authz"
                    if normalized_type.contains("authorization") || normalized_type.contains("authz") {
                        // Load as authorization plugin
                        match PluginRegistry::create_authz_plugin(&plugin_config.plugin_path, &plugin_config.config) {
                            Ok(plugin) => {
                                manager.add_host_authz_plugin(host_name.clone(), plugin);
                                // Authorization plugin loaded
                            }
                            Err(e) => {
                                eprintln!("Failed to load authorization plugin '{}' for host {}: {}", plugin_config.plugin_path, host_name, e);
                            }
                        }
                    }
                    // Check if it contains "authentication" or "auth" (but not "authz")
                    else if normalized_type.contains("authentication") || (normalized_type.contains("auth") && !normalized_type.contains("authz")) {
                        // Load as authentication plugin
                        match PluginRegistry::create_plugin(&plugin_config.plugin_path, &plugin_config.config) {
                            Ok(plugin) => {
                                manager.add_host_plugin(host_name.clone(), plugin);
                                // Authentication plugin loaded
                            }
                            Err(e) => {
                                eprintln!("Failed to load authentication plugin '{}' for host {}: {}", plugin_config.plugin_path, host_name, e);
                            }
                        }
                    }
                    // Check if it contains "access" and "log"
                    else if normalized_type.contains("access") && normalized_type.contains("log") {
                        // Load as access log plugin
                        match plugins::PluginRegistry::create_access_log_plugin(&plugin_config.plugin_path, &plugin_config.config) {
                            Ok(plugin) => {
                                manager.add_host_access_log_plugin(host_name.clone(), plugin);
                                // Access log plugin loaded successfully
                            }
                            Err(e) => {
                                eprintln!("Failed to load access log plugin '{}' for host {}: {}", plugin_config.plugin_path, host_name, e);
                            }
                        }
                    }
                    // Check if it contains "error" and "log"
                    else if normalized_type.contains("error") && normalized_type.contains("log") {
                        // Load as error log plugin
                        match plugins::PluginRegistry::create_error_log_plugin(&plugin_config.plugin_path, &plugin_config.config) {
                            Ok(plugin) => {
                                manager.add_host_error_log_plugin(host_name.clone(), plugin);
                                // Error log plugin loaded successfully
                            }
                            Err(e) => {
                                eprintln!("Failed to load error log plugin '{}' for host {}: {}", plugin_config.plugin_path, host_name, e);
                            }
                        }
                    }
                    else {
                        eprintln!("Unknown plugin type '{}' for plugin '{}' on host {}. Plugin type should contain 'authentication', 'authorization', 'access log', or 'error log'", plugin_type, plugin_config.plugin_path, host_name);
                    }
                }
                None => {
                    eprintln!("Plugin type not specified for plugin '{}' on host {}. Please add 'plugin-type' property", plugin_config.plugin_path, host_name);
                }
            }
        }
    }
    
    manager
}


async fn handle_request(req: Request<Body>, app_state: AppState) -> Result<Response<Body>> {
    // Store request info for logging before processing
    let _start_time = std::time::Instant::now();
    let remote_addr = "127.0.0.1".to_string(); // In real implementation, extract from connection
    let user_agent = req.headers().get(hyper::header::USER_AGENT)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    let referer = req.headers().get(hyper::header::REFERER)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    let method_str = req.method().as_str().to_string();
    let query_string = req.uri().query().map(|s| s.to_string());
    let path = req.uri().path().to_string();
    let host_name = {
        let host_header = req.headers().get(hyper::header::HOST)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");
        host_header.split(':').next().unwrap_or("").to_string()
    };
    
    // Process the request and capture the result
    let response_result = handle_request_internal(req, app_state.clone()).await;
    
    // Log the request regardless of success or failure
    if let Ok(response) = &response_result {
        let status_code = response.status().as_u16();
        let response_size = 0u64; // Would need to capture actual response size
        
        // Create timestamp in Apache log format
        let timestamp = chrono::Utc::now().format("%d/%b/%Y:%H:%M:%S %z").to_string();
        
        let access_entry = AccessLogEntry {
            remote_addr,
            timestamp,
            method: method_str,
            path,
            query: query_string,
            status_code,
            response_size,
            user_agent,
            referer,
            username: Some("unknown".to_string()), // Would need to extract from auth context
        };
        
        // Log access asynchronously
        let plugin_manager = app_state.plugin_manager.clone();
        let host_name_clone = host_name.clone();
        tokio::spawn(async move {
            plugin_manager.read().await.log_access(&access_entry, &host_name_clone).await;
        });
    }
    
    response_result
}

async fn handle_request_internal(req: Request<Body>, app_state: AppState) -> Result<Response<Body>> {
    
    let raw_path = req.uri().path();
    
    // Decode percent-encoded URI path (RFC 3986)
    let path = match urlencoding::decode(raw_path) {
        Ok(decoded) => decoded.into_owned(),
        Err(_) => {
            // Invalid percent encoding
            let response = Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(hyper::header::SERVER, "rusty-beam/0.1.0")
                .header("Content-Type", "text/plain")
                .body(Body::from("Invalid URI encoding"))
                .unwrap();
            return Ok(response);
        }
    };

    // Determine server root based on Host header
    let (server_root, host_name) = {
        let config = app_state.config.read().await;
        let host_header = req.headers().get(hyper::header::HOST)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");
        
        // Remove port from host header if present (e.g., "localhost:3000" -> "localhost")
        let host_name = host_header.split(':').next().unwrap_or("");
        
        // Look up host configuration
        if let Some(host_config) = config.hosts.get(host_name) {
            // Using host-specific root
            (host_config.host_root.clone(), host_name.to_string())
        } else {
            // Fall back to default server root
            // Using default server root for unknown host
            (config.server_root.clone(), host_name.to_string())
        }
    };
    let file_path = format!("{}{}", server_root, path);
    // Request received

    // Ensure we don't serve files outside our root directory
    let canonical_root = std::fs::canonicalize(&server_root).unwrap_or_else(|_| {
        std::fs::create_dir_all(&server_root).expect("Failed to create server root directory");
        std::fs::canonicalize(&server_root).expect("Failed to canonicalize server root")
    });

    // Store the method to check if it's a PUT request
    let method = req.method().clone();

    let canonicalized = match canonicalize_file_path(&file_path, &canonical_root, &method).await {
        Ok(path) => path,
        Err(err) => {
            return Ok(err);
        }
    };

    // Handling request

    // Check authentication (OPTIONS requests should not require authentication per HTTP spec)
    let authorized_user = if req.method() == hyper::Method::OPTIONS {
        // Bypassing authentication for OPTIONS request
        // Create a dummy authorized user for OPTIONS requests
        AuthorizedUser {
            username: "anonymous".to_string(),
            roles: vec!["anonymous".to_string()],
        }
    } else {
        match app_state.plugin_manager.read().await.authenticate_request(&req, &host_name, &path).await {
        AuthResult::Authorized(user_info) => {
            // Create authorized user for authorization check
            AuthorizedUser {
                username: user_info.username.clone(),
                roles: user_info.roles.clone(),
            }
        }
        AuthResult::Unauthorized => {
            let response = Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header(hyper::header::SERVER, "rusty-beam/0.1.0")
                .header("WWW-Authenticate", "Basic realm=\"Rusty Beam\"")
                .header("Content-Type", "text/plain")
                .body(Body::from("Authentication required"))
                .unwrap();
            return Ok(response);
        }
        AuthResult::Error(err) => {
            eprintln!("Authentication error: {}", err);
            let response = Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header(hyper::header::SERVER, "rusty-beam/0.1.0")
                .header("Content-Type", "text/plain")
                .body(Body::from("Authentication error"))
                .unwrap();
            return Ok(response);
        }
    }
    };

    // Check authorization using plugins
    // Get the resource path for authorization (combine path and selector if present)
    let resource_path = if let Some(range_header) = req.headers().get(hyper::header::RANGE) {
        if let Ok(range_str) = range_header.to_str() {
            if range_str.starts_with("selector=") {
                let selector = range_str.strip_prefix("selector=").unwrap();
                let decoded_selector = urlencoding::decode(selector).unwrap_or_else(|_| selector.into());
                format!("{}#(selector={})", path, decoded_selector)
            } else {
                path.clone()
            }
        } else {
            path.clone()
        }
    } else {
        path.clone()
    };
    
    let method_str = req.method().as_str();
    
    // OPTIONS requests should always be allowed per HTTP specification (RFC 7231)
    if method_str != "OPTIONS" {
        // Convert AuthorizedUser to UserInfo for plugin compatibility
        let user_info = UserInfo {
            username: authorized_user.username.clone(),
            roles: authorized_user.roles.clone(),
        };
        
        match app_state.plugin_manager.read().await.authorize_request(&user_info, &resource_path, method_str, &host_name).await {
            AuthzResult::Authorized => {
                // Continue with request processing
            }
            AuthzResult::Denied => {
            let response = Response::builder()
                .status(StatusCode::FORBIDDEN)
                .header(hyper::header::SERVER, "rusty-beam/0.1.0")
                .header("Content-Type", "text/plain")
                .body(Body::from("Access denied"))
                .unwrap();
            return Ok(response);
        }
        AuthzResult::Error(err) => {
            eprintln!("Authorization error: {}", err);
            let response = Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header(hyper::header::SERVER, "rusty-beam/0.1.0")
                .header("Content-Type", "text/plain")
                .body(Body::from("Authorization error"))
                .unwrap();
            return Ok(response);
        }
    }
    } // Close the OPTIONS check if statement

    // Clone the headers to avoid borrowing req
    let headers = req.headers().clone();

    // Check if Range header with selector is present
    let range_header = headers.get(hyper::header::RANGE);

    // Check if the requested file is HTML before processing selector
    let is_html_file = Path::new(&canonicalized)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("html"))
        .unwrap_or(false);

    // Only process selector if it's an HTML file
    let range_header = if is_html_file { range_header } else { None };
    let selector_opt = range_header.and_then(|header| {
        header.to_str().ok().and_then(|s| {
            if s.starts_with("selector=") {
                let encoded_selector = s.strip_prefix("selector=").unwrap();
                // URL decode the selector
                match urlencoding::decode(encoded_selector) {
                    Ok(decoded) => Some(decoded.into_owned()),
                    Err(_) => Some(encoded_selector.to_string()), // fallback to original if decode fails
                }
            } else {
                None
            }
        })
    });


    // Process the request based on method and if a selector is present
    let response_result = match (method, selector_opt) {
        (Method::OPTIONS, None) => {
            // Handle OPTIONS request
            let response = Response::builder()
                .header(hyper::header::SERVER, "rusty-beam/0.1.0")
                .header("Allow", "GET, HEAD, PUT, POST, DELETE, OPTIONS")
                .header("Accept-Ranges", "selector")
                .body(Body::from(""))
                .unwrap();
            Ok(response)
        }
        (Method::GET, None) => handle_get(&canonicalized).await,
        (Method::HEAD, None) => handle_head(&canonicalized).await,
        (Method::GET, Some(selector)) if is_html_file => {
            // Handle GET request with CSS selector
            handle_get_with_selector(req, &canonicalized, &selector).await
        }
        (Method::PUT, None) => handle_put(req, &canonicalized).await,
        (Method::PUT, Some(selector)) if is_html_file => {
            // Handle PUT request with CSS selector for HTML files
            handle_put_with_selector(req, &canonicalized, &selector).await
        }
        (Method::POST, None) => handle_post(req, &canonicalized).await,
        (Method::POST, Some(selector)) => {
            handle_post_with_selector(req, &canonicalized, &selector).await
        }
        (Method::DELETE, None) => handle_delete(&canonicalized).await,
        (Method::DELETE, Some(selector)) => {
            handle_delete_with_selector(req, &canonicalized, &selector).await
        }
        (_, _) => {
            let response = Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(hyper::header::SERVER, "rusty-beam/0.1.0")
                .header("Allow", "GET, HEAD, PUT, POST, DELETE, OPTIONS")
                .header("Content-Type", "text/plain")
                .body(Body::from("Method not allowed"))
                .unwrap();
            Ok(response)
        }
    };

    response_result
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
    
    // Initialize application state
    let app_state = AppState::new(config_path);
    
    // Display initial configuration
    {
        let _config = app_state.config.read().await;
        // Configuration loaded
    }

    // Set up signal handling for configuration reload
    let signals = Signals::new(&[SIGHUP]).expect("Failed to register signal handler");
    let handle = signals.handle();
    let app_state_for_signals = app_state.clone();
    
    let signals_task = tokio::spawn(async move {
        let mut signals = signals;
        while let Some(signal) = signals.next().await {
            match signal {
                SIGHUP => {
                    // Received SIGHUP signal
                    if let Err(e) = app_state_for_signals.reload().await {
                        eprintln!("Failed to reload configuration: {}", e);
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
        eprintln!(
            "Server running on http://{}:{}",
            config.bind_address, config.bind_port
        );
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
