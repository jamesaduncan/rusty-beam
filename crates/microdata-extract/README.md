# Microdata Extract

A standards-compliant HTML microdata extraction library for Rust.

[![Crates.io](https://img.shields.io/crates/v/microdata-extract.svg)](https://crates.io/crates/microdata-extract)
[![Documentation](https://docs.rs/microdata-extract/badge.svg)](https://docs.rs/microdata-extract)
[![License](https://img.shields.io/crates/l/microdata-extract.svg)](https://github.com/jamesaduncan/rusty-beam/tree/main/crates/microdata-extract)

## Overview

This library implements the [HTML microdata specification](https://html.spec.whatwg.org/multipage/microdata.html) to extract structured data from HTML documents using `itemscope`, `itemtype`, `itemprop`, and `itemref` attributes.

## Features

- **Standards Compliant**: Fully implements the HTML microdata specification
- **Nested Items**: Support for nested items and complex data structures  
- **Cross-references**: Handle `itemref` attributes for non-hierarchical relationships
- **Type-safe API**: Clean, ergonomic Rust API with proper error handling
- **Value Extraction**: Correct value extraction based on HTML element types
- **Schema.org Ready**: Works seamlessly with Schema.org vocabularies
- **Optional Serde**: Serialization support when the `serde` feature is enabled

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
microdata-extract = "0.1"

# Enable serde support for serialization
microdata-extract = { version = "0.1", features = ["serde"] }
```

## Basic Usage

```rust
use microdata_extract::MicrodataExtractor;

let html = r#"
<div itemscope itemtype="https://schema.org/Person">
  <span itemprop="name">John Doe</span>
  <span itemprop="email">john@example.com</span>
  <div itemprop="address" itemscope itemtype="https://schema.org/PostalAddress">
    <span itemprop="streetAddress">123 Main St</span>
    <span itemprop="addressLocality">Anytown</span>
  </div>
</div>
"#;

let extractor = MicrodataExtractor::new();
let items = extractor.extract(html).unwrap();

let person = &items[0];
assert_eq!(person.item_type(), Some("https://schema.org/Person"));
assert_eq!(person.get_property("name"), Some("John Doe"));
assert_eq!(person.get_property("email"), Some("john@example.com"));

// Access nested items
let addresses = person.get_nested_items("address");
let address = &addresses[0];
assert_eq!(address.get_property("streetAddress"), Some("123 Main St"));
```

## Advanced Features

### Multiple Property Values

```rust
let html = r#"
<div itemscope itemtype="https://schema.org/Person">
  <span itemprop="name">John Doe</span>
  <span itemprop="email">john@work.com</span>
  <span itemprop="email">john@personal.com</span>
</div>
"#;

let items = extractor.extract(html).unwrap();
let person = &items[0];

// Get all email addresses
let emails = person.get_property_values("email");
assert_eq!(emails, vec!["john@work.com", "john@personal.com"]);
```

### Cross-references with itemref

```rust
let html = r#"
<div itemscope itemtype="https://schema.org/Person" itemref="address">
  <span itemprop="name">John Doe</span>
</div>
<div id="address">
  <span itemprop="streetAddress">123 Main St</span>
  <span itemprop="addressLocality">Anytown</span>
</div>
"#;

let items = extractor.extract(html).unwrap();
let person = &items[0];

// Properties from referenced elements are included
assert_eq!(person.get_property("streetAddress"), Some("123 Main St"));
```

### Configuration Options

```rust
let extractor = MicrodataExtractor::with_settings(
    false, // Don't validate URLs
    true   // Ignore extraction errors and continue
);

let items = extractor.extract(html).unwrap();
```

### Extract Specific Item Types

```rust
// Extract only Person items
let people = extractor.extract_items_of_type(html, "https://schema.org/Person").unwrap();

// Extract first Product item
let product = extractor.extract_first_item_of_type(html, "https://schema.org/Product").unwrap();
```

## Value Extraction Rules

The library follows the HTML microdata specification for value extraction:

| Element Type | Value Source |
|-------------|-------------|
| `meta` | `content` attribute |
| `audio`, `embed`, `iframe`, `img`, `source`, `track`, `video` | `src` attribute |
| `a`, `area`, `link` | `href` attribute |
| `object` | `data` attribute |
| `data` | `value` attribute |
| `meter` | `value` attribute (as number) |
| `time` | `datetime` attribute or text content |
| `input` | `value` attribute (type-dependent) |
| `select` | Selected option's value |
| Elements with `itemscope` | Nested item |
| All others | Text content |

## Error Handling

```rust
use microdata_extract::{MicrodataError, MicrodataExtractor};

let result = extractor.extract(invalid_html);
match result {
    Ok(items) => println!("Extracted {} items", items.len()),
    Err(MicrodataError::HtmlParseError(msg)) => eprintln!("HTML parse error: {}", msg),
    Err(MicrodataError::InvalidUrl(url)) => eprintln!("Invalid URL: {}", url),
    Err(MicrodataError::CircularReference) => eprintln!("Circular itemref detected"),
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Schema.org Integration

This library works seamlessly with [Schema.org](https://schema.org/) vocabularies:

```rust
let html = r#"
<div itemscope itemtype="https://schema.org/Product">
  <span itemprop="name">Executive Anvil</span>
  <span itemprop="description">Perfect for the business traveler.</span>
  <div itemprop="offers" itemscope itemtype="https://schema.org/Offer">
    <span itemprop="price">$119.99</span>
    <span itemprop="priceCurrency">USD</span>
  </div>
</div>
"#;

let items = extractor.extract(html).unwrap();
let product = &items[0];

assert_eq!(product.item_type(), Some("https://schema.org/Product"));
assert_eq!(product.get_property("name"), Some("Executive Anvil"));

let offers = product.get_nested_items("offers");
assert_eq!(offers[0].get_property("price"), Some("$119.99"));
```

## Real-world Example

See the [examples directory](examples/) for complete working examples, including:

- [Basic Usage](examples/basic_usage.rs) - Simple extraction and property access
- [Complex Structures](examples/complex_structures.rs) - Nested items and cross-references
- [Configuration Parsing](examples/config_parsing.rs) - Parsing application configuration

## Specification Compliance

This library implements:

- ✅ Item creation with `itemscope`
- ✅ Item typing with `itemtype` 
- ✅ Property assignment with `itemprop`
- ✅ Cross-references with `itemref`
- ✅ Nested item structures
- ✅ Proper value extraction per element type
- ✅ Global identifiers with `itemid`
- ✅ Multiple property values
- ✅ Circular reference detection

## Contributing

Contributions are welcome! Please see the [contributing guidelines](../../CONTRIBUTING.md) for details.

## License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.