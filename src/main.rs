mod config;
mod handlers;
mod utils;

use config::{load_config_from_html, ServerConfig};
use handlers::*;
use utils::canonicalize_file_path;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Result, Server, StatusCode};
use std::convert::Infallible;
use std::path::Path;
use std::sync::LazyLock;

static CONFIG: LazyLock<ServerConfig> = LazyLock::new(|| load_config_from_html("config.html"));


async fn handle_request(req: Request<Body>) -> Result<Response<Body>> {
    let path = req.uri().path();

    // Determine server root based on Host header
    let server_root = {
        let host_header = req.headers().get("host")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");
        
        // Remove port from host header if present (e.g., "localhost:3000" -> "localhost")
        let host_name = host_header.split(':').next().unwrap_or("");
        
        // Look up host configuration
        if let Some(host_config) = CONFIG.hosts.get(host_name) {
            println!("Using host-specific root for '{}': {}", host_name, host_config.host_root);
            host_config.host_root.clone()
        } else {
            // Fall back to default server root
            println!("Using default server root for unknown host '{}'", host_name);
            CONFIG.server_root.clone()
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

    // Clone the headers to avoid borrowing req
    let headers = req.headers().clone();

    // Check if Range header with selector is present
    let range_header = headers.get("range");

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
            let mut response = Response::new(Body::from(""));
            response
                .headers_mut()
                .insert("Allow", "GET, PUT, POST, DELETE, OPTIONS".parse().unwrap());
            response
                .headers_mut()
                .insert("Accept-Ranges", "selector".parse().unwrap());
            Ok(response)
        }
        (Method::GET, None) => handle_get(&canonicalized).await,
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
            let mut response = Response::new(Body::from("Method not allowed"));
            *response.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
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
