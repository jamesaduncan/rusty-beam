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
use plugins::{PluginManager, AuthResult, AuthzResult, PluginRegistry, UserInfo};
use auth::AuthorizedUser;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Result, Server, StatusCode};
use std::convert::Infallible;
use std::path::Path;
use std::sync::LazyLock;

static CONFIG: LazyLock<ServerConfig> = LazyLock::new(|| load_config_from_html("config/config.html"));
static PLUGIN_MANAGER: LazyLock<PluginManager> = LazyLock::new(|| {
    let mut manager = PluginManager::new();
    
    // Load plugins from configuration dynamically
    for (host_name, host_config) in &CONFIG.hosts {
        for plugin_config in &host_config.plugins {
            match PluginRegistry::create_plugin(&plugin_config.plugin_path, &plugin_config.config) {
                Ok(plugin) => {
                    manager.add_host_plugin(host_name.clone(), plugin);
                }
                Err(e) => {
                    eprintln!("Failed to load plugin '{}' for host {}: {}", plugin_config.plugin_path, host_name, e);
                }
            }
        }
        
        // Load authorization plugins
        if host_config.auth_config.is_some() {
            // Create a file-authz plugin for this host
            let mut authz_config = std::collections::HashMap::new();
            
            // Try to find the authorization file path
            if let Some(authz_file) = host_config.plugins.iter()
                .find_map(|p| p.config.get("authFile"))
                .cloned() {
                authz_config.insert("authFile".to_string(), authz_file);
            }
            
            match PluginRegistry::create_authz_plugin("file-authz", &authz_config) {
                Ok(plugin) => {
                    manager.add_host_authz_plugin(host_name.clone(), plugin);
                    println!("Loaded file-authz plugin for host: {}", host_name);
                }
                Err(e) => {
                    eprintln!("Failed to load authorization plugin for host {}: {}", host_name, e);
                }
            }
        }
    }
    
    manager
});


async fn handle_request(req: Request<Body>) -> Result<Response<Body>> {
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
        let host_header = req.headers().get(hyper::header::HOST)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");
        
        // Remove port from host header if present (e.g., "localhost:3000" -> "localhost")
        let host_name = host_header.split(':').next().unwrap_or("");
        
        // Look up host configuration
        if let Some(host_config) = CONFIG.hosts.get(host_name) {
            println!("Using host-specific root for '{}': {}", host_name, host_config.host_root);
            (host_config.host_root.clone(), host_name.to_string())
        } else {
            // Fall back to default server root
            println!("Using default server root for unknown host '{}'", host_name);
            (CONFIG.server_root.clone(), host_name.to_string())
        }
    };
    let file_path = format!("{}{}", server_root, path);
    println!("Received request for: {}", file_path);

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

    println!("Handling request for: {}", canonicalized);

    // Check authentication
    let authorized_user = match PLUGIN_MANAGER.authenticate_request(&req, &host_name, &path).await {
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
    
    // Convert AuthorizedUser to UserInfo for plugin compatibility
    let user_info = UserInfo {
        username: authorized_user.username.clone(),
        roles: authorized_user.roles.clone(),
    };
    
    match PLUGIN_MANAGER.authorize_request(&user_info, &resource_path, method_str, &host_name).await {
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
    match (method, selector_opt) {
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
    }
}

#[tokio::main]
async fn main() {
    // Config will be loaded lazily when first accessed
    println!("Configuration loaded:");
    println!("  Server root: {}", CONFIG.server_root);
    println!(
        "  Bind address: {}:{}",
        CONFIG.bind_address, CONFIG.bind_port
    );
    
    // Initialize plugin manager
    let _plugin_manager = &*PLUGIN_MANAGER;

    let make_svc =
        make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle_request)) });

    let addr = format!("{}:{}", CONFIG.bind_address, CONFIG.bind_port)
        .parse::<std::net::SocketAddr>()
        .expect("Invalid address format");

    // Attempt to bind to the address gracefully
    let server = match Server::try_bind(&addr) {
        Ok(builder) => builder.serve(make_svc),
        Err(e) => {
            eprintln!("Failed to start server on {}:{}", CONFIG.bind_address, CONFIG.bind_port);
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

    println!(
        "Server running on http://{}:{}",
        CONFIG.bind_address, CONFIG.bind_port
    );
    println!("Serving files from: {}", CONFIG.server_root);
    println!("  GET    /path/to/file   - Download file or list directory");
    println!("  PUT    /path/to/file   - Upload/overwrite file");
    println!("  POST   /path/to/file   - Append to file");
    println!("  DELETE /path/to/file   - Delete file or directory");

    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
        std::process::exit(1);
    }
}
