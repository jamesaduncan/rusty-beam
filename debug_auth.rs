use std::fs;
use microdata_extract::MicrodataExtractor;

fn main() {
    // Test the authorization file that the server is using
    let auth_file = "./examples/guestbook/auth/authorization.html";
    let content = fs::read_to_string(auth_file).expect("Failed to read authorization file");
    
    println!("=== Authorization File Debug ===");
    println!("File exists: {}", std::path::Path::new(auth_file).exists());
    println!("File size: {} bytes", content.len());
    
    let extractor = MicrodataExtractor::new();
    match extractor.extract(&content) {
        Ok(items) => {
            println!("Found {} microdata items", items.len());
            
            // Check for the specific rule that should allow GET /*
            let mut found_wildcard_get = false;
            
            for item in &items {
                if item.item_type() == Some("http://rustybeam.net/AuthorizationRule") {
                    let username = item.get_property("username").unwrap_or_default();
                    let role = item.get_property("role").unwrap_or_default();
                    let path = item.get_property("path").unwrap_or_default();
                    let action = item.get_property("action").unwrap_or_default();
                    let methods = item.get_property_values("method");
                    
                    println!("Rule: user='{}', role='{}', path='{}', action='{}', methods={:?}", 
                             username, role, path, action, methods);
                    
                    // Check if this is the rule that should allow GET /*
                    if username == "*" && path == "/*" && action == "allow" && methods.contains(&"GET".to_string()) {
                        found_wildcard_get = true;
                        println!("  ✓ Found wildcard GET rule that should allow access");
                    }
                }
            }
            
            if !found_wildcard_get {
                println!("  ✗ WARNING: No wildcard GET rule found that allows access to /*");
            }
            
            // Test path matching logic
            println!("\n=== Path Matching Test ===");
            test_path_matches("/", "/*");
            test_path_matches("/", "/");
            test_path_matches("/index.html", "/*");
        }
        Err(e) => {
            println!("Error extracting microdata: {:?}", e);
        }
    }
}

fn test_path_matches(path: &str, pattern: &str) {
    let matches = path_matches(path, pattern);
    println!("Path '{}' matches pattern '{}': {}", path, pattern, matches);
}

fn path_matches(path: &str, pattern: &str) -> bool {
    // Replicate the path matching logic from the authorization plugin
    if path == pattern {
        return true;
    }
    
    if pattern.ends_with("/*") {
        let prefix = &pattern[..pattern.len() - 2];
        if prefix.is_empty() && path == "/" {
            return true;
        }
        if path.starts_with(prefix) {
            return true;
        }
    }
    
    false
}