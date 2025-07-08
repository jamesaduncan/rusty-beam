//! Main microdata extraction engine

use crate::{MicrodataDocument, MicrodataError, MicrodataItem, Result};
use dom_query::Document;
use url::Url;

/// Main microdata extractor
#[derive(Debug, Clone, Default)]
pub struct MicrodataExtractor {
    /// Whether to validate URLs in itemtype and itemid
    pub validate_urls: bool,
    /// Whether to ignore extraction errors and continue processing
    pub ignore_errors: bool,
}

impl MicrodataExtractor {
    /// Create a new microdata extractor with default settings
    pub fn new() -> Self {
        Self {
            validate_urls: true,
            ignore_errors: false,
        }
    }

    /// Create a new extractor with custom settings
    pub fn with_settings(validate_urls: bool, ignore_errors: bool) -> Self {
        Self {
            validate_urls,
            ignore_errors,
        }
    }

    /// Extract microdata from HTML string
    pub fn extract(&self, html: &str) -> Result<Vec<MicrodataItem>> {
        let document = Document::from(html);
        self.extract_from_document(&document, None)
    }

    /// Extract microdata from HTML string with base URL
    pub fn extract_with_base_url(&self, html: &str, base_url: &str) -> Result<MicrodataDocument> {
        let document = Document::from(html);
        let base_url = Url::parse(base_url)
            .map_err(|e| MicrodataError::InvalidUrl(format!("Invalid base URL: {}", e)))?;
        
        let items = self.extract_from_document(&document, Some(&base_url))?;
        Ok(MicrodataDocument::new(items, Some(base_url)))
    }

    /// Extract microdata from a DOM document
    pub fn extract_from_document(
        &self,
        document: &Document,
        _base_url: Option<&Url>,
    ) -> Result<Vec<MicrodataItem>> {
        let mut items = Vec::new();
        let mut errors = Vec::new();

        // Find all top-level items (elements with itemscope that are not properties of other items)
        let top_level_items = self.find_top_level_items(document);

        for item_element in top_level_items {
            // Use the new method with document for itemref support
            match MicrodataItem::from_element_with_document(&item_element, document) {
                Ok(item) => {
                    if self.validate_item(&item).is_ok() {
                        items.push(item);
                    } else if !self.ignore_errors {
                        return Err(MicrodataError::InvalidStructure(
                            "Invalid item structure".to_string(),
                        ));
                    }
                }
                Err(e) => {
                    if self.ignore_errors {
                        errors.push(e);
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        // Log errors if ignoring them
        if self.ignore_errors && !errors.is_empty() {
            eprintln!("Microdata extraction encountered {} errors (ignored)", errors.len());
            for error in errors {
                eprintln!("  - {}", error);
            }
        }

        Ok(items)
    }

    /// Find top-level microdata items (not nested within other items)
    fn find_top_level_items<'a>(&self, document: &'a Document) -> Vec<dom_query::Selection<'a>> {
        let mut top_level_items = Vec::new();

        // Find all elements with itemscope that are NOT nested within another itemscope
        // We do this by finding all itemscope elements and then filtering out those that have
        // an itemscope ancestor
        let all_items = document.select("[itemscope]");

        for item in all_items.iter() {
            // For each itemscope element, check if it's a child of another itemscope element
            let item_html = item.html();
            let nested_items = document.select("[itemscope] [itemscope]");
            let mut is_nested = false;
            
            // Check if this item appears in the nested selection
            for nested_item in nested_items.iter() {
                let nested_html = nested_item.html();
                if item_html == nested_html {
                    is_nested = true;
                    break;
                }
            }

            if !is_nested {
                top_level_items.push(item);
            }
        }

        top_level_items
    }

    /// Validate a microdata item
    fn validate_item(&self, item: &MicrodataItem) -> Result<()> {
        // Validate itemtype URLs if enabled
        if self.validate_urls {
            if let Some(item_type) = &item.item_type {
                if Url::parse(item_type).is_err() {
                    return Err(MicrodataError::InvalidUrl(format!(
                        "Invalid itemtype URL: {}",
                        item_type
                    )));
                }
            }

            if let Some(item_id) = &item.item_id {
                if Url::parse(item_id).is_err() {
                    return Err(MicrodataError::InvalidUrl(format!(
                        "Invalid itemid URL: {}",
                        item_id
                    )));
                }
            }
        }

        // Additional validation can be added here
        Ok(())
    }

    /// Extract microdata and return JSON representation
    #[cfg(feature = "serde")]
    pub fn extract_to_json(&self, html: &str) -> Result<String> {
        let items = self.extract(html)?;
        serde_json::to_string_pretty(&items)
            .map_err(|e| MicrodataError::InvalidStructure(format!("JSON serialization failed: {}", e)))
    }

    /// Convenience method to extract items of a specific type
    pub fn extract_items_of_type(&self, html: &str, item_type: &str) -> Result<Vec<MicrodataItem>> {
        let items = self.extract(html)?;
        Ok(items
            .into_iter()
            .filter(|item| item.item_type() == Some(item_type))
            .collect())
    }

    /// Extract the first item of a specific type
    pub fn extract_first_item_of_type(&self, html: &str, item_type: &str) -> Result<Option<MicrodataItem>> {
        let mut items = self.extract_items_of_type(html, item_type)?;
        Ok(items.pop())
    }
}

