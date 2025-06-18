use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Result, Server, StatusCode};
use std::convert::Infallible;
use std::fs;
use std::io::prelude::*;
use std::path::Path;
use tokio::fs as async_fs;

const SERVER_ROOT: &str = "./files"; // Directory to serve files from

async fn handle_request(req: Request<Body>) -> Result<Response<Body>> {
    let path = req.uri().path();
    let file_path = format!("{}{}", SERVER_ROOT, path);
    
    // Ensure we don't serve files outside our root directory
    let canonical_root = std::fs::canonicalize(SERVER_ROOT).unwrap_or_else(|_| {
        std::fs::create_dir_all(SERVER_ROOT).expect("Failed to create server root directory");
        std::fs::canonicalize(SERVER_ROOT).expect("Failed to canonicalize server root")
    });
    
    match req.method() {
        &Method::GET => handle_get(&file_path, &canonical_root).await,
        &Method::PUT => handle_put(req, &file_path, &canonical_root).await,
        &Method::POST => handle_post(req, &file_path, &canonical_root).await,
        &Method::DELETE => handle_delete(&file_path, &canonical_root).await,
        _ => {
            let mut response = Response::new(Body::from("Method not allowed"));
            *response.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
            Ok(response)
        }
    }
}

async fn handle_get(file_path: &str, canonical_root: &Path) -> Result<Response<Body>> {
    // Security check: ensure path is within server root
    if Path::new(file_path).exists() {
        if let Ok(canonical_path) = std::fs::canonicalize(file_path) {
            if !canonical_path.starts_with(canonical_root) {
                let mut response = Response::new(Body::from("Access denied"));
                *response.status_mut() = StatusCode::FORBIDDEN;
                return Ok(response);
            }
        }
    }

    if file_path.ends_with('/') || Path::new(file_path).is_dir() {
        // List directory contents
        match fs::read_dir(file_path) {
            Ok(entries) => {
                let mut html = String::from("<html><body><h2>Directory Listing</h2><ul>");
                for entry in entries {
                    if let Ok(entry) = entry {
                        let file_name = entry.file_name();
                        let name = file_name.to_string_lossy();
                        let path = entry.path();
                        let display_name = if path.is_dir() {
                            format!("{}/", name)
                        } else {
                            name.to_string()
                        };
                        html.push_str(&format!(
                            "<li><a href=\"{}{}\">{}</a></li>", 
                            file_path.trim_start_matches(SERVER_ROOT),
                            if file_path.ends_with('/') { "" } else { "/" },
                            display_name
                        ));
                    }
                }
                html.push_str("</ul></body></html>");
                
                let response = Response::builder()
                    .header("Content-Type", "text/html")
                    .body(Body::from(html))
                    .unwrap();
                Ok(response)
            }
            Err(_) => {
                let mut response = Response::new(Body::from("Directory not found"));
                *response.status_mut() = StatusCode::NOT_FOUND;
                Ok(response)
            }
        }
    } else {
        // Serve file
        match async_fs::read(file_path).await {
            Ok(contents) => {
                let content_type = match Path::new(file_path).extension().and_then(|ext| ext.to_str()) {
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
                Ok(response)
            }
            Err(_) => {
                let mut response = Response::new(Body::from("File not found"));
                *response.status_mut() = StatusCode::NOT_FOUND;
                Ok(response)
            }
        }
    }
}

async fn handle_put(req: Request<Body>, file_path: &str, canonical_root: &Path) -> Result<Response<Body>> {
    // Security check
    let parent_path = Path::new(file_path).parent().unwrap_or_else(|| Path::new("."));
    if parent_path.exists() {
        if let Ok(canonical_path) = std::fs::canonicalize(parent_path) {
            if !canonical_path.starts_with(canonical_root) {
                let mut response = Response::new(Body::from("Access denied"));
                *response.status_mut() = StatusCode::FORBIDDEN;
                return Ok(response);
            }
        }
    }

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

async fn handle_post(req: Request<Body>, file_path: &str, canonical_root: &Path) -> Result<Response<Body>> {
    // Security check
    let parent_path = Path::new(file_path).parent().unwrap_or_else(|| Path::new("."));
    if parent_path.exists() {
        if let Ok(canonical_path) = std::fs::canonicalize(parent_path) {
            if !canonical_path.starts_with(canonical_root) {
                let mut response = Response::new(Body::from("Access denied"));
                *response.status_mut() = StatusCode::FORBIDDEN;
                return Ok(response);
            }
        }
    }

    let body_bytes = hyper::body::to_bytes(req.into_body()).await?;
    
    // For POST, we'll append to the file or create it if it doesn't exist
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
    {
        Ok(mut file) => {
            match file.write_all(&body_bytes) {
                Ok(_) => {
                    let response = Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::from("Content appended successfully"))
                        .unwrap();
                    Ok(response)
                }
                Err(e) => {
                    let mut response = Response::new(Body::from(format!("Failed to append to file: {}", e)));
                    *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    Ok(response)
                }
            }
        }
        Err(e) => {
            let mut response = Response::new(Body::from(format!("Failed to open file: {}", e)));
            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            Ok(response)
        }
    }
}

async fn handle_delete(file_path: &str, canonical_root: &Path) -> Result<Response<Body>> {
    // Security check
    if let Ok(canonical_path) = std::fs::canonicalize(file_path) {
        if !canonical_path.starts_with(canonical_root) {
            let mut response = Response::new(Body::from("Access denied"));
            *response.status_mut() = StatusCode::FORBIDDEN;
            return Ok(response);
        }
    } else {
        let mut response = Response::new(Body::from("File not found"));
        *response.status_mut() = StatusCode::NOT_FOUND;
        return Ok(response);
    }

    if Path::new(file_path).is_dir() {
        match std::fs::remove_dir_all(file_path) {
            Ok(_) => {
                let response = Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::from("Directory deleted successfully"))
                    .unwrap();
                Ok(response)
            }
            Err(e) => {
                let mut response = Response::new(Body::from(format!("Failed to delete directory: {}", e)));
                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                Ok(response)
            }
        }
    } else {
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
}

#[tokio::main]
async fn main() {
    // Create the server root directory if it doesn't exist
    std::fs::create_dir_all(SERVER_ROOT).expect("Failed to create server root directory");

    let make_svc = make_service_fn(|_conn| async {
        Ok::<_, Infallible>(service_fn(handle_request))
    });

    let addr = ([127, 0, 0, 1], 3000).into();
    let server = Server::bind(&addr).serve(make_svc);

    println!("Server running on http://127.0.0.1:3000");
    println!("Serving files from: {}", SERVER_ROOT);
    println!("Usage:");
    println!("  GET    /path/to/file   - Download file or list directory");
    println!("  PUT    /path/to/file   - Upload/overwrite file");
    println!("  POST   /path/to/file   - Append to file");
    println!("  DELETE /path/to/file   - Delete file or directory");

    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }
}
