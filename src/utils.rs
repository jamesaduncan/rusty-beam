use hyper::{Body, Method, Response, StatusCode};
use std::path::Path;

pub async fn canonicalize_file_path(
    file_path: &str,
    canonical_root: &Path,
    method: &Method,
) -> std::result::Result<String, Response<Body>> {
    if Path::new(file_path).exists() {
        if let Ok(canonical_path) = std::fs::canonicalize(file_path) {
            if !canonical_path.starts_with(canonical_root) {
                let mut response = Response::new(Body::from("Access denied"));
                *response.status_mut() = StatusCode::FORBIDDEN;
                Err(response)
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
        if method == Method::PUT || method == Method::POST {
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
            Err(response)
        }
    }
}