//! Example demonstrating itemref cross-reference support

use microdata_extract::MicrodataExtractor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example HTML with itemref - properties are defined outside the item
    let html = r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>Product with Shared License</title>
    </head>
    <body>
        <!-- Two products sharing the same license information -->
        <div itemscope itemtype="https://schema.org/Product" itemref="shared-license manufacturer">
            <h1 itemprop="name">Professional Widget</h1>
            <p itemprop="description">A high-quality widget for professionals</p>
            <span itemprop="price">$99.99</span>
        </div>
        
        <div itemscope itemtype="https://schema.org/Product" itemref="shared-license manufacturer">
            <h1 itemprop="name">Enterprise Widget</h1>
            <p itemprop="description">An enterprise-grade widget with advanced features</p>
            <span itemprop="price">$199.99</span>
        </div>
        
        <!-- Shared properties referenced by itemref -->
        <div id="shared-license">
            <p>License: <span itemprop="license">MIT License</span></p>
            <p>Terms: <span itemprop="termsOfService">https://example.com/terms</span></p>
        </div>
        
        <div id="manufacturer">
            <p>Made by: <span itemprop="manufacturer">Acme Corp</span></p>
        </div>
        
        <!-- Another example: Person with address defined elsewhere -->
        <div itemscope itemtype="https://schema.org/Person" itemref="home-address">
            <span itemprop="name">Jane Smith</span>
            <span itemprop="email">jane@example.com</span>
        </div>
        
        <!-- Address referenced by itemref -->
        <div id="home-address">
            <div itemprop="address" itemscope itemtype="https://schema.org/PostalAddress">
                <span itemprop="streetAddress">789 Oak Street</span>
                <span itemprop="addressLocality">Springfield</span>
                <span itemprop="addressRegion">IL</span>
                <span itemprop="postalCode">62701</span>
            </div>
        </div>
    </body>
    </html>
    "#;

    // Create extractor
    let extractor = MicrodataExtractor::new();
    
    // Extract microdata
    let items = extractor.extract(html)?;
    
    println!("Found {} microdata items\n", items.len());
    
    // Process products
    let products: Vec<_> = items.iter()
        .filter(|item| item.item_type() == Some("https://schema.org/Product"))
        .collect();
    
    println!("=== Products ({}) ===", products.len());
    for product in products {
        println!("Product: {}", product.get_property("name").unwrap_or_default());
        println!("  Price: {}", product.get_property("price").unwrap_or_default());
        println!("  License: {}", product.get_property("license").unwrap_or_default());
        println!("  Manufacturer: {}", product.get_property("manufacturer").unwrap_or_default());
        println!("  Terms: {}", product.get_property("termsOfService").unwrap_or_default());
        println!();
    }
    
    // Process person
    let people: Vec<_> = items.iter()
        .filter(|item| item.item_type() == Some("https://schema.org/Person"))
        .collect();
    
    println!("=== People ({}) ===", people.len());
    for person in people {
        println!("Person: {}", person.get_property("name").unwrap_or_default());
        println!("  Email: {}", person.get_property("email").unwrap_or_default());
        
        // Check for nested address
        let addresses = person.get_nested_items("address");
        if let Some(address) = addresses.first() {
            println!("  Address:");
            println!("    Street: {}", address.get_property("streetAddress").unwrap_or_default());
            println!("    City: {}", address.get_property("addressLocality").unwrap_or_default());
            println!("    State: {}", address.get_property("addressRegion").unwrap_or_default());
            println!("    ZIP: {}", address.get_property("postalCode").unwrap_or_default());
        }
    }
    
    Ok(())
}