//! Integration tests for microdata extraction

use microdata_extract::MicrodataExtractor;

#[test]
fn test_schema_org_person() {
    let html = r#"
    <div itemscope itemtype="https://schema.org/Person">
        <span itemprop="name">Jane Doe</span>
        <span itemprop="jobTitle">Professor</span>
        <div itemprop="address" itemscope itemtype="https://schema.org/PostalAddress">
            <span itemprop="streetAddress">20 W 34th St</span>
            <span itemprop="addressLocality">New York</span>
            <span itemprop="addressRegion">NY</span>
            <span itemprop="postalCode">10001</span>
            <span itemprop="addressCountry">US</span>
        </div>
        <span itemprop="telephone">(212) 736-3100</span>
        <a itemprop="email" href="mailto:jane.doe@xyz.edu">jane.doe@xyz.edu</a>
    </div>
    "#;

    let extractor = MicrodataExtractor::new();
    let items = extractor.extract(html).unwrap();

    assert_eq!(items.len(), 1);
    let person = &items[0];
    assert_eq!(person.item_type(), Some("https://schema.org/Person"));
    assert_eq!(person.get_property("name"), Some("Jane Doe".to_string()));
    assert_eq!(person.get_property("jobTitle"), Some("Professor".to_string()));
    assert_eq!(person.get_property("telephone"), Some("(212) 736-3100".to_string()));
    
    // Check nested address
    let addresses = person.get_nested_items("address");
    assert_eq!(addresses.len(), 1);
    let address = &addresses[0];
    assert_eq!(address.item_type(), Some("https://schema.org/PostalAddress"));
    assert_eq!(address.get_property("streetAddress"), Some("20 W 34th St".to_string()));
    assert_eq!(address.get_property("addressLocality"), Some("New York".to_string()));
}

#[test]
fn test_schema_org_product() {
    let html = r#"
    <div itemscope itemtype="https://schema.org/Product">
        <span itemprop="name">Executive Anvil</span>
        <img itemprop="image" src="anvil_executive.jpg" alt="Executive Anvil logo" />
        <span itemprop="description">Sleeker than ACME's Classic Anvil, the
            Executive Anvil is perfect for the business traveler
            looking for something to drop from a height.
        </span>
        Product #: <span itemprop="mpn">925872</span><br />
        <span itemprop="brand" itemscope itemtype="https://schema.org/Brand">
            <span itemprop="name">ACME</span>
        </span>
        <span itemprop="offers" itemscope itemtype="https://schema.org/Offer">
            <span itemprop="price">$119.99</span>
            <span itemprop="priceCurrency">USD</span>
            <span itemprop="availability">https://schema.org/InStock</span>
            Valid until: <time itemprop="priceValidUntil" datetime="2024-11-05">5 November!</time>
        </span>
    </div>
    "#;

    let extractor = MicrodataExtractor::new();
    let items = extractor.extract(html).unwrap();

    assert_eq!(items.len(), 1);
    let product = &items[0];
    assert_eq!(product.item_type(), Some("https://schema.org/Product"));
    assert_eq!(product.get_property("name"), Some("Executive Anvil".to_string()));
    assert_eq!(product.get_property("mpn"), Some("925872".to_string()));

    // Check nested brand
    let brands = product.get_nested_items("brand");
    assert_eq!(brands.len(), 1);
    assert_eq!(brands[0].get_property("name"), Some("ACME".to_string()));

    // Check nested offer
    let offers = product.get_nested_items("offers");
    assert_eq!(offers.len(), 1);
    let offer = &offers[0];
    assert_eq!(offer.get_property("price"), Some("$119.99".to_string()));
    assert_eq!(offer.get_property("priceCurrency"), Some("USD".to_string()));
}

#[test]
fn test_itemref_cross_reference() {
    let html = r#"
    <div itemscope itemtype="https://schema.org/Person" itemref="address">
        <span itemprop="name">John Doe</span>
    </div>
    <div id="address">
        <span itemprop="streetAddress">123 Main St</span>
        <span itemprop="addressLocality">Anytown</span>
    </div>
    "#;

    let extractor = MicrodataExtractor::new();
    let items = extractor.extract(html).unwrap();

    assert_eq!(items.len(), 1);
    let person = &items[0];
    assert_eq!(person.get_property("name"), Some("John Doe".to_string()));
    assert_eq!(person.get_property("streetAddress"), Some("123 Main St".to_string()));
    assert_eq!(person.get_property("addressLocality"), Some("Anytown".to_string()));
}

#[test]
fn test_multiple_itemprop_values() {
    let html = r#"
    <div itemscope itemtype="https://schema.org/Person">
        <span itemprop="name familyName">Smith</span>
        <span itemprop="givenName">John</span>
    </div>
    "#;

    let extractor = MicrodataExtractor::new();
    let items = extractor.extract(html).unwrap();

    assert_eq!(items.len(), 1);
    let person = &items[0];
    assert_eq!(person.get_property("name"), Some("Smith".to_string()));
    assert_eq!(person.get_property("familyName"), Some("Smith".to_string()));
    assert_eq!(person.get_property("givenName"), Some("John".to_string()));
}

#[test]
fn test_various_element_types() {
    let html = r#"
    <div itemscope itemtype="https://schema.org/Thing">
        <meta itemprop="description" content="A thing with various properties">
        <a itemprop="url" href="https://example.com">Example</a>
        <img itemprop="image" src="image.jpg" alt="Image">
        <time itemprop="dateCreated" datetime="2024-01-15">January 15, 2024</time>
        <data itemprop="price" value="29.99">$29.99</data>
        <meter itemprop="rating" value="4.5" min="0" max="5">4.5 out of 5</meter>
    </div>
    "#;

    let extractor = MicrodataExtractor::new();
    let items = extractor.extract(html).unwrap();

    assert_eq!(items.len(), 1);
    let item = &items[0];
    assert_eq!(item.get_property("description"), Some("A thing with various properties".to_string()));
    assert_eq!(item.get_property("url"), Some("https://example.com/".to_string()));
    assert_eq!(item.get_property("image"), Some("image.jpg".to_string()));
    assert_eq!(item.get_property("dateCreated"), Some("2024-01-15".to_string()));
    assert_eq!(item.get_property("price"), Some("29.99".to_string()));
    assert_eq!(item.get_property("rating"), Some("4.5".to_string()));
}

#[test]
fn test_rusty_beam_config_format() {
    // Test the specific format used by rusty-beam for configuration
    let html = r#"
    <table>
        <tbody>
            <tr itemscope itemtype="http://rustybeam.net/User">
                <td itemprop="username">admin</td>
                <td itemprop="password">admin123</td>
                <td>
                    <ul>
                        <li itemprop="role">administrators</li>
                        <li itemprop="role">user</li>
                    </ul>
                </td>
                <td itemprop="encryption">plaintext</td>
            </tr>
        </tbody>
    </table>
    "#;

    let extractor = MicrodataExtractor::new();
    let items = extractor.extract(html).unwrap();

    assert_eq!(items.len(), 1);
    let user = &items[0];
    assert_eq!(user.item_type(), Some("http://rustybeam.net/User"));
    assert_eq!(user.get_property("username"), Some("admin".to_string()));
    assert_eq!(user.get_property("password"), Some("admin123".to_string()));
    assert_eq!(user.get_property("encryption"), Some("plaintext".to_string()));

    let roles = user.get_property_values("role");
    assert_eq!(roles.len(), 2);
    assert!(roles.contains(&"administrators".to_string()));
    assert!(roles.contains(&"user".to_string()));
}

#[test]
fn test_complex_authorization_config() {
    let html = r#"
    <table>
        <tbody>
            <tr itemscope itemtype="http://rustybeam.net/Authorization">
                <td itemprop="username">admin</td>
                <td itemprop="resource">/*</td>
                <td>
                    <ul>
                        <li itemprop="method">GET</li>
                        <li itemprop="method">PUT</li>
                        <li itemprop="method">POST</li>
                        <li itemprop="method">DELETE</li>
                    </ul>
                </td>
                <td itemprop="permission">allow</td>
            </tr>
        </tbody>
    </table>
    "#;

    let extractor = MicrodataExtractor::new();
    let items = extractor.extract(html).unwrap();

    assert_eq!(items.len(), 1);
    let auth_rule = &items[0];
    assert_eq!(auth_rule.item_type(), Some("http://rustybeam.net/Authorization"));
    assert_eq!(auth_rule.get_property("username"), Some("admin".to_string()));
    assert_eq!(auth_rule.get_property("resource"), Some("/*".to_string()));
    assert_eq!(auth_rule.get_property("permission"), Some("allow".to_string()));

    let methods = auth_rule.get_property_values("method");
    assert_eq!(methods.len(), 4);
    assert!(methods.contains(&"GET".to_string()));
    assert!(methods.contains(&"PUT".to_string()));
    assert!(methods.contains(&"POST".to_string()));
    assert!(methods.contains(&"DELETE".to_string()));
}

#[test]
fn test_error_handling() {
    let html = r#"
    <div itemscope itemtype="not-a-valid-url">
        <span itemprop="name">Test</span>
    </div>
    "#;

    let extractor = MicrodataExtractor::with_settings(true, false); // validate URLs, don't ignore errors
    let result = extractor.extract(html);
    assert!(result.is_err());

    let extractor = MicrodataExtractor::with_settings(false, true); // don't validate URLs, ignore errors
    let items = extractor.extract(html).unwrap();
    assert_eq!(items.len(), 1);
}

#[test]
fn test_to_hashmap() {
    let html = r#"
    <div itemscope itemtype="https://schema.org/Person">
        <span itemprop="name">John Doe</span>
        <span itemprop="email">john@work.com</span>
        <span itemprop="email">john@personal.com</span>
    </div>
    "#;

    let extractor = MicrodataExtractor::new();
    let items = extractor.extract(html).unwrap();
    let person = &items[0];
    
    let map = person.to_hashmap();
    assert_eq!(map.get("name"), Some(&vec!["John Doe".to_string()]));
    assert_eq!(map.get("email"), Some(&vec!["john@work.com".to_string(), "john@personal.com".to_string()]));
}