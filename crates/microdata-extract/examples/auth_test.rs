use std::fs;
use microdata_extract::MicrodataExtractor;

fn main() {
    // Load the authorization.html file
    let auth_file = "../../examples/guestbook/auth/authorization.html";
    let content = fs::read_to_string(auth_file).expect("Failed to read authorization file");
    
    println!("File content length: {}", content.len());
    
    let extractor = MicrodataExtractor::new();
    match extractor.extract(&content) {
        Ok(items) => {
            println!("Found {} items", items.len());
            
            // Look for users
            for item in &items {
                if item.item_type() == Some("http://rustybeam.net/Credential") {
                    println!("User found:");
                    println!("  Username: {:?}", item.get_property("username"));
                    println!("  Roles: {:?}", item.get_property_values("role"));
                }
            }
            
            // Look for authorization rules
            for item in &items {
                if item.item_type() == Some("http://rustybeam.net/AuthorizationRule") {
                    println!("Authorization rule found:");
                    println!("  Username: {:?}", item.get_property("username"));
                    println!("  Role: {:?}", item.get_property("role"));
                    println!("  Path: {:?}", item.get_property("path"));
                    println!("  Selector: {:?}", item.get_property("selector"));
                    println!("  Action: {:?}", item.get_property("action"));
                    println!("  Methods: {:?}", item.get_property_values("method"));
                }
            }
        }
        Err(e) => {
            println!("Error extracting microdata: {:?}", e);
        }
    }
}