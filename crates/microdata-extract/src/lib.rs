//! # Microdata Extract
//!
//! A standards-compliant HTML microdata extraction library for Rust.
//!
//! This library implements the [HTML microdata specification](https://html.spec.whatwg.org/multipage/microdata.html)
//! to extract structured data from HTML documents using itemscope, itemtype, itemprop, and itemref attributes.
//!
//! ## Features
//!
//! - Full HTML microdata specification compliance
//! - Support for nested items and cross-references (itemref)
//! - Proper value extraction based on element types
//! - Clean, type-safe API
//! - Optional serde support for serialization
//!
//! ## Quick Start
//!
//! ```rust
//! use microdata_extract::MicrodataExtractor;
//!
//! let html = r#"
//! <div itemscope itemtype="https://schema.org/Person">
//!   <span itemprop="name">John Doe</span>
//!   <span itemprop="email">john@example.com</span>
//! </div>
//! "#;
//!
//! let extractor = MicrodataExtractor::new();
//! let items = extractor.extract(html).unwrap();
//!
//! assert_eq!(items.len(), 1);
//! assert_eq!(items[0].item_type(), Some("https://schema.org/Person"));
//! assert_eq!(items[0].get_property("name"), Some("John Doe".to_string()));
//! ```

// Removed unused imports
use url::Url;

mod error;
mod extractor;
mod item;
mod property;
mod value;

pub use error::{MicrodataError, Result};
pub use extractor::MicrodataExtractor;
pub use item::MicrodataItem;
pub use property::MicrodataProperty;
pub use value::MicrodataValue;

/// Represents a complete microdata extraction result
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MicrodataDocument {
    /// Top-level items found in the document
    pub items: Vec<MicrodataItem>,
    /// URL of the document (if provided)
    pub document_url: Option<Url>,
}

impl MicrodataDocument {
    /// Create a new microdata document
    pub fn new(items: Vec<MicrodataItem>, document_url: Option<Url>) -> Self {
        Self {
            items,
            document_url,
        }
    }

    /// Get all items of a specific type
    pub fn items_of_type(&self, item_type: &str) -> Vec<&MicrodataItem> {
        self.items
            .iter()
            .filter(|item| item.item_type() == Some(item_type))
            .collect()
    }

    /// Get the first item of a specific type
    pub fn first_item_of_type(&self, item_type: &str) -> Option<&MicrodataItem> {
        self.items_of_type(item_type).into_iter().next()
    }
}