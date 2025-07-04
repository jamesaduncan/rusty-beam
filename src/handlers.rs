use hyper::{Body, Request, Response, Result, StatusCode};
use std::io::prelude::*;
use std::path::Path;
use tokio::fs as async_fs;

// Add standard HTTP headers to response builder
fn add_standard_headers(builder: hyper::http::response::Builder, server_header: &str) -> hyper::http::response::Builder {
    builder
        .header(hyper::header::SERVER, server_header)
}

pub async fn handle_head(file_path: &str, server_header: &str) -> Result<Response<Body>> {
    // HEAD should return same headers as GET but without body
    match async_fs::metadata(file_path).await {
        Ok(metadata) => {
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
            let response = add_standard_headers(Response::builder(), server_header)
                .header("Content-Type", content_type)
                .header("Content-Length", metadata.len().to_string())
                .body(Body::empty())
                .unwrap();
            Ok(response)
        }
        Err(_) => {
            let response = add_standard_headers(Response::builder(), server_header)
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "text/plain")
                .body(Body::empty())
                .unwrap();
            Ok(response)
        }
    }
}

pub async fn handle_get(file_path: &str, server_header: &str) -> Result<Response<Body>> {
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
            let response = add_standard_headers(Response::builder(), server_header)
                .header("Content-Type", content_type)
                .body(Body::from(contents))
                .unwrap();
            Ok(response)
        }
        Err(_) => {
            let response = add_standard_headers(Response::builder(), server_header)
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "text/plain")
                .body(Body::from("File not found"))
                .unwrap();
            Ok(response)
        }
    }
}

pub async fn handle_get_with_selector(
    _req: Request<Body>,
    file_path: &str,
    selector: &str,
    server_header: &str,
) -> Result<Response<Body>> {
    // Read the HTML file
    let html_content = async_fs::read_to_string(file_path)
        .await
        .unwrap_or_else(|_| "Failed to read file".to_string());

    let document = dom_query::Document::from(html_content);
    // let's just make sure the selector is valid first.
    let element = document.try_select(selector);
    if element.is_none() {
        return Ok(add_standard_headers(Response::builder(), server_header)
            .status(StatusCode::NOT_FOUND)
            .header("Content-Type", "text/plain")
            .body(Body::from("No elements matched the selector"))
            .unwrap());
    }

    let final_element = document.select(selector);
    let html_output = final_element.html().to_string();
    let trimmed_output = html_output.trim_end().to_string();
    Ok(add_standard_headers(Response::builder(), server_header)
        .header("Content-Type", "text/html")
        .body(Body::from(trimmed_output))
        .unwrap())
}

pub async fn handle_put(req: Request<Body>, file_path: &str, server_header: &str) -> Result<Response<Body>> {
    let body_bytes = hyper::body::to_bytes(req.into_body()).await?;

    // Check if file exists before writing to determine correct status code
    let file_existed = async_fs::metadata(file_path).await.is_ok();

    // Create directory if it doesn't exist
    if let Some(parent) = Path::new(file_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    match async_fs::write(file_path, body_bytes).await {
        Ok(_) => {
            // RFC 7231: 201 for new resources, 200 for updates
            let status = if file_existed { 
                StatusCode::OK 
            } else { 
                StatusCode::CREATED 
            };
            let response = add_standard_headers(Response::builder(), server_header)
                .status(status)
                .header("Content-Type", "text/plain")
                .body(Body::from("File uploaded successfully"))
                .unwrap();
            Ok(response)
        }
        Err(e) => {
            let response = add_standard_headers(Response::builder(), server_header)
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "text/plain")
                .body(Body::from(format!("Failed to write file: {}", e)))
                .unwrap();
            Ok(response)
        }
    }
}

pub async fn handle_put_with_selector(
    req: Request<Body>,
    file_path: &str,
    selector: &str,
    server_header: &str,
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
                "Invalid request body", server_header,
            ));
        }
    };

    // now lets go about having a look at the contents of the request Body
    let new_content = match String::from_utf8(body_bytes.to_vec()) {
        Ok(content) => content,
        Err(_) => {
            return Ok(error_response(
                StatusCode::BAD_REQUEST,
                "Invalid UTF-8 in request body", server_header,
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
            return Ok(add_standard_headers(Response::builder(), server_header)
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "text/plain")
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
            let response = add_standard_headers(Response::builder(), server_header)
                .status(StatusCode::OK)
                .header("Content-Type", "text/html")
                .body(Body::from(final_content_string))
                .unwrap();
            Ok(response)
        }
        Err(e) => {
            let response = add_standard_headers(Response::builder(), server_header)
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "text/plain")
                .body(Body::from(format!("Failed to write file: {}", e)))
                .unwrap();
            Ok(response)
        }
    }
}

pub async fn handle_post(req: Request<Body>, file_path: &str, server_header: &str) -> Result<Response<Body>> {
    let body_bytes = hyper::body::to_bytes(req.into_body()).await?;

    // For POST, we'll append to the file or create it if it doesn't exist
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
    {
        Ok(mut file) => match file.write_all(&body_bytes) {
            Ok(_) => {
                let response = add_standard_headers(Response::builder(), server_header)
                    .status(StatusCode::OK)
                    .header("Content-Type", "text/plain")
                    .body(Body::from("Content appended successfully"))
                    .unwrap();
                Ok(response)
            }
            Err(e) => {
                let response = add_standard_headers(Response::builder(), server_header)
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header("Content-Type", "text/plain")
                    .body(Body::from(format!("Failed to append to file: {}", e)))
                    .unwrap();
                Ok(response)
            }
        },
        Err(e) => {
            let response = add_standard_headers(Response::builder(), server_header)
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "text/plain")
                .body(Body::from(format!("Failed to open file: {}", e)))
                .unwrap();
            Ok(response)
        }
    }
}

pub async fn handle_post_with_selector(
    req: Request<Body>,
    file_path: &str,
    selector: &str,
    server_header: &str,
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
                "Invalid request body", server_header,
            ));
        }
    };

    // now lets go about having a look at the contents of the request Body
    let new_content = match String::from_utf8(body_bytes.to_vec()) {
        Ok(content) => content,
        Err(_) => {
            return Ok(error_response(
                StatusCode::BAD_REQUEST,
                "Invalid UTF-8 in request body", server_header,
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
            return Ok(add_standard_headers(Response::builder(), server_header)
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "text/plain")
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
            let response = add_standard_headers(Response::builder(), server_header)
                .status(StatusCode::OK)
                .header("Content-Type", "text/html")
                .body(Body::from(final_content_string))
                .unwrap();
            Ok(response)
        }
        Err(e) => {
            let response = add_standard_headers(Response::builder(), server_header)
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "text/plain")
                .body(Body::from(format!("Failed to write file: {}", e)))
                .unwrap();
            Ok(response)
        }
    }
}

pub async fn handle_delete(file_path: &str, server_header: &str) -> Result<Response<Body>> {
    match async_fs::remove_file(file_path).await {
        Ok(_) => {
            let response = add_standard_headers(Response::builder(), server_header)
                .status(StatusCode::OK)
                .header("Content-Type", "text/plain")
                .body(Body::from("File deleted successfully"))
                .unwrap();
            Ok(response)
        }
        Err(e) => {
            let response = add_standard_headers(Response::builder(), server_header)
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "text/plain")
                .body(Body::from(format!("Failed to delete file: {}", e)))
                .unwrap();
            Ok(response)
        }
    }
}

pub async fn handle_delete_with_selector(
    _req: Request<Body>,
    file_path: &str,
    selector: &str,
    server_header: &str,
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
            return Ok(add_standard_headers(Response::builder(), server_header)
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "text/plain")
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
            let response = add_standard_headers(Response::builder(), server_header)
                .status(StatusCode::NO_CONTENT)
                .header("Content-Type", "text/plain")
                .body(Body::from(""))
                .unwrap();
            Ok(response)
        }
        Err(e) => {
            let response = add_standard_headers(Response::builder(), server_header)
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "text/plain")
                .body(Body::from(format!("Failed to write file: {}", e)))
                .unwrap();
            Ok(response)
        }
    }
}

pub fn error_response(status: StatusCode, message: &str, server_header: &str) -> Response<Body> {
    add_standard_headers(Response::builder(), server_header)
        .status(status)
        .header("Content-Type", "text/plain")
        .body(Body::from(message.to_string()))
        .unwrap()
}