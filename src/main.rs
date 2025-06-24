use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Result, Server, StatusCode};
use std::convert::Infallible;
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

    let canonicalized = match canonicalize_file_path(&file_path, &canonical_root).await {
        Ok(path) => path,
        Err(err) => {
            return Ok(err);
        }
    };

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
                Some(s.strip_prefix("selector=").unwrap())
            } else {
                None
            }
        })
    });
    // Store the method to avoid borrowing req in the match statement
    let method = req.method().clone();

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
            handle_get_with_selector(req, &canonicalized, selector).await
        }
        (Method::PUT, None) => handle_put(req, &canonicalized).await,
        (Method::PUT, Some(selector)) if is_html_file => {
            // Handle PUT request with CSS selector for HTML files
            handle_put_with_selector(req, &canonicalized, selector).await
        }
        (Method::POST, None) => handle_post(req, &canonicalized).await,
        (Method::POST, Some(selector)) => {
            handle_post_with_selector(req, &canonicalized, selector).await
        }
        (Method::DELETE, None) => handle_delete(&canonicalized).await,
        (Method::DELETE, Some(selector)) => {
            handle_delete_with_selector(req, &canonicalized, selector).await
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
        final_element.replace_with_html(new_content);
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
    Ok(Response::builder()
        .header("Content-Type", "text/html")
        .body(Body::from(final_element.html().to_string()))
        .unwrap())
}

async fn canonicalize_file_path(
    file_path: &str,
    canonical_root: &Path,
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
            return Err(response);
        }
    } else {
        let mut response = Response::new(Body::from("File not found"));
        *response.status_mut() = StatusCode::NOT_FOUND;
        return Err(response);
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
    // Create the server root directory if it doesn't exist
    std::fs::create_dir_all(SERVER_ROOT).expect("Failed to create server root directory");

    let make_svc =
        make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle_request)) });

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
