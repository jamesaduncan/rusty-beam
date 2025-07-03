//! Basic usage example for microdata extraction

use microdata_extract::{MicrodataExtractor, MicrodataValue};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example HTML with Schema.org Person markup
    let html = r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>John Doe's Profile</title>
    </head>
    <body>
        <div itemscope itemtype="https://schema.org/Person">
            <h1 itemprop="name">John Doe</h1>
            <p itemprop="description">John is a software engineer with expertise in Rust programming.</p>
            
            <h2>Contact Information</h2>
            <div itemprop="address" itemscope itemtype="https://schema.org/PostalAddress">
                <span itemprop="streetAddress">123 Tech Street</span><br>
                <span itemprop="addressLocality">San Francisco</span>,
                <span itemprop="addressRegion">CA</span>
                <span itemprop="postalCode">94105</span><br>
                <span itemprop="addressCountry">United States</span>
            </div>
            
            <p>Email: <a itemprop="email" href="mailto:john@example.com">john@example.com</a></p>
            <p>Phone: <span itemprop="telephone">+1 (555) 123-4567</span></p>
            <p>Website: <a itemprop="url" href="https://johndoe.dev">johndoe.dev</a></p>
            
            <h2>Work</h2>
            <div itemprop="worksFor" itemscope itemtype="https://schema.org/Organization">
                <span itemprop="name">Acme Corporation</span>
                <span itemprop="url">https://acme.com</span>
            </div>
            
            <p>Job Title: <span itemprop="jobTitle">Senior Software Engineer</span></p>
            
            <h2>Skills</h2>
            <ul>
                <li itemprop="knowsAbout">Rust Programming</li>
                <li itemprop="knowsAbout">Web Development</li>
                <li itemprop="knowsAbout">System Architecture</li>
            </ul>
        </div>
    </body>
    </html>
    "#;

    // Create extractor
    let extractor = MicrodataExtractor::new();
    
    // Extract microdata
    let items = extractor.extract(html)?;
    
    println!("Found {} microdata items", items.len());
    println!();
    
    // Process each item
    for (index, item) in items.iter().enumerate() {
        println!("=== Item {} ===", index + 1);
        
        if let Some(item_type) = item.item_type() {
            println!("Type: {}", item_type);
        }
        
        if let Some(item_id) = item.item_id() {
            println!("ID: {}", item_id);
        }
        
        println!("Properties:");
        
        // Get all property names and display them
        for name in item.property_names() {
            let values = item.get_property_values(name);
            
            // Check if any of the properties are nested items
            let nested_items = item.get_nested_items(name);
            
            if !nested_items.is_empty() {
                println!("  {}: [Nested Items]", name);
                for (nested_index, nested_item) in nested_items.iter().enumerate() {
                    println!("    Item {}:", nested_index + 1);
                    if let Some(nested_type) = nested_item.item_type() {
                        println!("      Type: {}", nested_type);
                    }
                    for nested_prop_name in nested_item.property_names() {
                        let nested_values = nested_item.get_property_values(nested_prop_name);
                        println!("      {}: {:?}", nested_prop_name, nested_values);
                    }
                }
            } else {
                println!("  {}: {:?}", name, values);
            }
        }
        
        println!();
    }
    
    // Demonstrate specific property access
    if let Some(person) = items.first() {
        println!("=== Specific Property Access ===");
        println!("Name: {}", person.get_property("name").unwrap_or_else(|| "Unknown".to_string()));
        println!("Email: {}", person.get_property("email").unwrap_or_else(|| "Not provided".to_string()));
        println!("Job Title: {}", person.get_property("jobTitle").unwrap_or_else(|| "Not specified".to_string()));
        
        // Get multiple values for a property
        let skills = person.get_property_values("knowsAbout");
        println!("Skills: {}", skills.join(", "));
        
        // Access nested items
        let addresses = person.get_nested_items("address");
        if let Some(address) = addresses.first() {
            println!("Lives in: {}, {}", 
                address.get_property("addressLocality").unwrap_or_else(|| "Unknown".to_string()),
                address.get_property("addressRegion").unwrap_or_else(|| "Unknown".to_string())
            );
        }
        
        let employers = person.get_nested_items("worksFor");
        if let Some(employer) = employers.first() {
            println!("Works for: {}", employer.get_property("name").unwrap_or_else(|| "Unknown".to_string()));
        }
    }
    
    // Demonstrate HashMap conversion for easy access
    println!("\n=== HashMap Representation ===");
    if let Some(person) = items.first() {
        let map = person.to_hashmap();
        for (key, values) in map {
            println!("{}: {:?}", key, values);
        }
    }
    
    Ok(())
}