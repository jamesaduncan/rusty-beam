use dom_query::Document;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Result, Server, StatusCode};
use std::collections::HashMap;
use std::convert::Infallible;
use std::fs;
use std::io::prelude::*;
use std::path::Path;
use std::sync::LazyLock;
use tokio::fs as async_fs;

#[derive(Debug, Clone)]
struct HostConfig {
    host_root: String,
}

struct ServerConfig {
    server_root: String,
    bind_address: String,
    bind_port: u16,
    hosts: HashMap<String, HostConfig>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            server_root: "./files".to_string(),
            bind_address: "127.0.0.1".to_string(),
            bind_port: 3000,
            hosts: HashMap::new(),
        }
    }
}
static CONFIG: LazyLock<ServerConfig> = LazyLock::new(|| load_config_from_html("config.html"));

fn load_config_from_html(file_path: &str) -> ServerConfig {
    let mut config = ServerConfig::default();

    match fs::read_to_string(file_path) {
        Ok(content) => {
            let document = Document::from(content);

            // Find elements with the specific itemtype
            let config_elements =
                document.select("[itemscope][itemtype='http://rustybeam.net/ServerConfig']");

            if !config_elements.is_empty() {
                // Find all elements with itemprop attributes
                let props = document.select("[itemprop]");
                for i in 0..props.length() {
                    let prop = props.get(i).unwrap();
                    let prop_name = match prop.attr("itemprop") {
                        Some(name) => name,
                        None => continue,
                    };

                    let prop_value = prop.text().trim().to_string();

                    match &*prop_name {
                        "serverRoot" => config.server_root = prop_value,
                        "bindAddress" => config.bind_address = prop_value,
                        "bindPort" => {
                            if let Ok(port) = prop_value.parse::<u16>() {
                                config.bind_port = port;
                            }
                        }
                        _ => {} // Ignore unknown properties
                    }
                }
            }

            // Load host configurations 
            // Find all elements with itemprop="hostName" and their corresponding hostRoot elements
            let host_name_elements = document.select("[itemprop='hostName']");
            let host_root_elements = document.select("[itemprop='hostRoot']");
            
            // Pair up host names and roots - they should be in the same order
            let min_length = std::cmp::min(host_name_elements.length(), host_root_elements.length());
            for i in 0..min_length {
                let host_name_element = host_name_elements.get(i).unwrap();
                let host_root_element = host_root_elements.get(i).unwrap();
                
                let host_name = host_name_element.text().trim().to_string();
                let host_root = host_root_element.text().trim().to_string();
                
                // Only add non-empty host configs
                if !host_name.is_empty() && !host_root.is_empty() {
                    let host_config = HostConfig {
                        host_root: host_root.clone(),
                    };
                    config.hosts.insert(host_name, host_config);
                }
            }

            println!("Loaded configuration from {}", file_path);
            println!("  Configured hosts: {}", config.hosts.len());
            for (name, host_config) in &config.hosts {
                println!("    Host: {} -> {}", name, host_config.host_root);
            }
        }
        Err(e) => {
            eprintln!("Failed to read config file {}: {}", file_path, e);
            eprintln!("Using default configuration");
        }
    }
    config
}

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
    let result = match (method, selector_opt) {
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
    };

    return result;
}

async fn handle_delete_with_selector(
    _req: Request<Body>,
    file_path: &str,
    selector: &str,
) -> Result<Response<Body>> {
    // Read the HTML file
    let html_content = async_fs::read_to_string(file_path)
        .await
        .unwrap_or_else(|_| "Failed to read file".to_string());

    let final_content_string: String;
    let final_content: Vec<u8>;
    {
        let document = dom_query::Document::from(html_content);

        // let's just make sure the selector is valid first.
        let element = document.try_select(selector);
        if element.is_none() {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("No elements matched the selector"))
                .unwrap());
        }

        document.select(selector).first().remove();
        final_content_string = document.html().to_string();
        final_content = final_content_string.clone().into_bytes();
    }

    // Write the modified HTML back to the file
    match async_fs::write(file_path, final_content).await {
        Ok(_) => {
            let response = Response::builder()
                .status(StatusCode::NO_CONTENT)
                .body(Body::from(""))
                .unwrap();
            Ok(response)
        }
        Err(e) => {
            let mut response = Response::new(Body::from(format!("Failed to write file: {}", e)));
            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            Ok(response)
        }
    }
}

async fn handle_post_with_selector(
    req: Request<Body>,
    file_path: &str,
    selector: &str,
) -> Result<Response<Body>> {
    // Read the HTML file
    let html_content = async_fs::read_to_string(file_path)
        .await
        .unwrap_or_else(|_| "Failed to read file".to_string());

    /* do this early so we don't need to worry about threading */
    let body_bytes = match hyper::body::to_bytes(req.into_body()).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return Ok(error_response(
                StatusCode::BAD_REQUEST,
                "Invalid request body",
            ));
        }
    };

    // now lets go about having a look at the contents of the request Body
    let new_content = match String::from_utf8(body_bytes.to_vec()) {
        Ok(content) => content,
        Err(_) => {
            return Ok(error_response(
                StatusCode::BAD_REQUEST,
                "Invalid UTF-8 in request body",
            ));
        }
    };

    let final_content_string: String;
    let final_content: Vec<u8>;
    {
        let document = dom_query::Document::from(html_content);

        // let's just make sure the selector is valid first.
        let element = document.try_select(selector);
        if element.is_none() {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("No elements matched the selector"))
                .unwrap());
        }

        let final_element = document.select(selector).first();
        final_element.append_html(new_content);
        final_content_string = document.html().to_string();
        final_content = final_content_string.clone().into_bytes();
    }

    // Write the modified HTML back to the file
    match async_fs::write(file_path, final_content).await {
        Ok(_) => {
            let response = Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(final_content_string))
                .unwrap();
            Ok(response)
        }
        Err(e) => {
            let mut response = Response::new(Body::from(format!("Failed to write file: {}", e)));
            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            Ok(response)
        }
    }
}

async fn handle_put_with_selector(
    req: Request<Body>,
    file_path: &str,
    selector: &str,
) -> Result<Response<Body>> {
    // Read the HTML file
    let html_content = async_fs::read_to_string(file_path)
        .await
        .unwrap_or_else(|_| "Failed to read file".to_string());

    /* do this early so we don't need to worry about threading */
    let body_bytes = match hyper::body::to_bytes(req.into_body()).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return Ok(error_response(
                StatusCode::BAD_REQUEST,
                "Invalid request body",
            ));
        }
    };

    // now lets go about having a look at the contents of the request Body
    let new_content = match String::from_utf8(body_bytes.to_vec()) {
        Ok(content) => content,
        Err(_) => {
            return Ok(error_response(
                StatusCode::BAD_REQUEST,
                "Invalid UTF-8 in request body",
            ));
        }
    };

    let final_content_string: String;
    let final_content: Vec<u8>;
    {
        let document = dom_query::Document::from(html_content);

        // let's just make sure the selector is valid first.
        let element = document.try_select(selector);
        if element.is_none() {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("No elements matched the selector"))
                .unwrap());
        }

        let final_element = document.select(selector).first();
        
        // Fix for td/tr/other table elements bug:
        // dom_query's replace_with_html strips "invalid" HTML like standalone <td> tags
        // Use a simpler workaround: replace the element using string manipulation
        if new_content.trim().starts_with("<td") || new_content.trim().starts_with("<tr") || 
           new_content.trim().starts_with("<th") || new_content.trim().starts_with("<tbody") ||
           new_content.trim().starts_with("<thead") || new_content.trim().starts_with("<tfoot") {
            
            // Create a temporary unique marker
            let marker = format!("__RUSTY_BEAM_REPLACE_MARKER_{}__", std::process::id());
            final_element.replace_with_html(marker.clone());
            
            // Get the document HTML and replace the marker with our content
            let document_html = document.html().to_string();
            let modified_html = document_html.replace(&marker, &new_content);
            
            // Parse the modified HTML and return it
            let new_doc = dom_query::Document::from(modified_html);
            final_content_string = new_doc.html().to_string().trim_end().to_string();
        } else {
            final_element.replace_with_html(new_content);
            final_content_string = document.html().to_string().trim_end().to_string();
        }
        
        final_content = final_content_string.clone().into_bytes();
    }

    // Write the modified HTML back to the file
    match async_fs::write(file_path, final_content).await {
        Ok(_) => {
            let response = Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(final_content_string))
                .unwrap();
            Ok(response)
        }
        Err(e) => {
            let mut response = Response::new(Body::from(format!("Failed to write file: {}", e)));
            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            Ok(response)
        }
    }
}

fn error_response(status: StatusCode, message: &str) -> Response<Body> {
    Response::builder()
        .status(status)
        .header("content-type", "text/plain")
        .body(Body::from(message.to_string()))
        .unwrap()
}

async fn handle_get_with_selector(
    _req: Request<Body>,
    file_path: &str,
    selector: &str,
) -> Result<Response<Body>> {
    // Read the HTML file
    let html_content = async_fs::read_to_string(file_path)
        .await
        .unwrap_or_else(|_| "Failed to read file".to_string());

    let document = dom_query::Document::from(html_content);
    // let's just make sure the selector is valid first.
    let element = document.try_select(selector);
    if element.is_none() {
        return Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("No elements matched the selector"))
            .unwrap());
    }

    let final_element = document.select(selector);
    let html_output = final_element.html().to_string();
    let trimmed_output = html_output.trim_end().to_string();
    Ok(Response::builder()
        .header("Content-Type", "text/html")
        .body(Body::from(trimmed_output))
        .unwrap())
}

async fn canonicalize_file_path(
    file_path: &str,
    canonical_root: &Path,
    method: &Method,
) -> std::result::Result<String, Response<Body>> {
    if Path::new(file_path).exists() {
        if let Ok(canonical_path) = std::fs::canonicalize(file_path) {
            if !canonical_path.starts_with(canonical_root) {
                let mut response = Response::new(Body::from("Access denied"));
                *response.status_mut() = StatusCode::FORBIDDEN;
                return Err(response);
            } else if file_path.ends_with('/') || canonical_path.is_dir() {
                let index_file_path = format!("{}/index.html", file_path.trim_end_matches('/'));
                Ok(index_file_path)
            } else {
                Ok(canonical_path.to_string_lossy().to_string())
            }
        } else {
            let mut response = Response::new(Body::from("Invalid file path"));
            *response.status_mut() = StatusCode::FORBIDDEN;
            Err(response)
        }
    } else {
        // For PUT and POST requests, allow creating new files
        if method == &Method::PUT || method == &Method::POST {
            // Check if the parent directory exists and is within our root
            let path = Path::new(file_path);
            if let Some(parent) = path.parent() {
                if parent.exists() {
                    if let Ok(canonical_parent) = std::fs::canonicalize(parent) {
                        if !canonical_parent.starts_with(canonical_root) {
                            let mut response = Response::new(Body::from("Access denied"));
                            *response.status_mut() = StatusCode::FORBIDDEN;
                            return Err(response);
                        }
                    }
                }
            }
            // Return the original path for PUT/POST operations
            Ok(file_path.to_string())
        } else {
            let mut response = Response::new(Body::from("File not found"));
            *response.status_mut() = StatusCode::NOT_FOUND;
            return Err(response);
        }
    }
}

async fn handle_get(file_path: &str) -> Result<Response<Body>> {
    // Try to read the file early, return if successful
    match async_fs::read(file_path).await {
        Ok(contents) => {
            let content_type = match Path::new(file_path)
                .extension()
                .and_then(|ext| ext.to_str())
            {
                Some("html") => "text/html",
                Some("css") => "text/css",
                Some("js") => "application/javascript",
                Some("json") => "application/json",
                Some("png") => "image/png",
                Some("jpg") | Some("jpeg") => "image/jpeg",
                Some("txt") => "text/plain",
                _ => "application/octet-stream",
            };
            let response = Response::builder()
                .header("Content-Type", content_type)
                .body(Body::from(contents))
                .unwrap();
            return Ok(response);
        }
        Err(_) => {
            let mut response = Response::new(Body::from("File not found"));
            *response.status_mut() = StatusCode::NOT_FOUND;
            Ok(response)
        }
    }
}

async fn handle_put(req: Request<Body>, file_path: &str) -> Result<Response<Body>> {
    let body_bytes = hyper::body::to_bytes(req.into_body()).await?;

    // Create directory if it doesn't exist
    if let Some(parent) = Path::new(file_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    match async_fs::write(file_path, body_bytes).await {
        Ok(_) => {
            let response = Response::builder()
                .status(StatusCode::CREATED)
                .body(Body::from("File uploaded successfully"))
                .unwrap();
            Ok(response)
        }
        Err(e) => {
            let mut response = Response::new(Body::from(format!("Failed to write file: {}", e)));
            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            Ok(response)
        }
    }
}

async fn handle_post(req: Request<Body>, file_path: &str) -> Result<Response<Body>> {
    let body_bytes = hyper::body::to_bytes(req.into_body()).await?;

    // For POST, we'll append to the file or create it if it doesn't exist
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
    {
        Ok(mut file) => match file.write_all(&body_bytes) {
            Ok(_) => {
                let response = Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::from("Content appended successfully"))
                    .unwrap();
                Ok(response)
            }
            Err(e) => {
                let mut response =
                    Response::new(Body::from(format!("Failed to append to file: {}", e)));
                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                Ok(response)
            }
        },
        Err(e) => {
            let mut response = Response::new(Body::from(format!("Failed to open file: {}", e)));
            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            Ok(response)
        }
    }
}

async fn handle_delete(file_path: &str) -> Result<Response<Body>> {
    match async_fs::remove_file(file_path).await {
        Ok(_) => {
            let response = Response::builder()
                .status(StatusCode::OK)
                .body(Body::from("File deleted successfully"))
                .unwrap();
            Ok(response)
        }
        Err(e) => {
            let mut response = Response::new(Body::from(format!("Failed to delete file: {}", e)));
            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
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
