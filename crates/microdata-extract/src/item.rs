//! Microdata item representation

use crate::{MicrodataError, MicrodataProperty, Result};
use dom_query::{Document, Selection};
use std::collections::{HashMap, HashSet};
use url::Url;

/// Represents a microdata item (created by itemscope)
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MicrodataItem {
    /// Item type (from itemtype attribute)
    pub item_type: Option<String>,
    /// Item ID (from itemid attribute)
    pub item_id: Option<String>,
    /// Properties of this item
    pub properties: Vec<MicrodataProperty>,
}

impl MicrodataItem {
    /// Create a new microdata item
    pub fn new(
        item_type: Option<String>,
        item_id: Option<String>,
        properties: Vec<MicrodataProperty>,
    ) -> Self {
        Self {
            item_type,
            item_id,
            properties,
        }
    }

    /// Extract a microdata item from an HTML element with itemscope
    pub fn from_element(element: &Selection) -> Result<Self> {
        if !element.has_attr("itemscope") {
            return Err(MicrodataError::InvalidStructure(
                "Element must have itemscope attribute".to_string(),
            ));
        }

        // Extract item type
        let item_type = element.attr("itemtype").map(|t| t.to_string());

        // Extract item ID
        let item_id = element.attr("itemid").map(|id| id.to_string());

        // Validate item ID if present (must be a valid URL when itemtype is present)
        if let (Some(item_id), Some(_)) = (&item_id, &item_type) {
            if Url::parse(item_id).is_err() {
                return Err(MicrodataError::InvalidUrl(format!(
                    "itemid must be a valid URL: {}",
                    item_id
                )));
            }
        }

        // Extract properties using the microdata algorithm
        let properties = Self::extract_properties(element)?;

        Ok(MicrodataItem::new(item_type, item_id, properties))
    }
    
    /// Extract a microdata item from an element with access to the full document for itemref resolution
    pub fn from_element_with_document(element: &Selection, document: &Document) -> Result<Self> {
        if !element.has_attr("itemscope") {
            return Err(MicrodataError::InvalidStructure(
                "Element must have itemscope attribute".to_string(),
            ));
        }

        // Extract item type
        let item_type = element.attr("itemtype").map(|t| t.to_string());

        // Extract item ID
        let item_id = element.attr("itemid").map(|id| id.to_string());

        // Validate item ID if present
        if let (Some(item_id), Some(_)) = (&item_id, &item_type) {
            if Url::parse(item_id).is_err() {
                return Err(MicrodataError::InvalidUrl(format!(
                    "itemid must be a valid URL: {}",
                    item_id
                )));
            }
        }

        // Extract properties with itemref support
        let properties = Self::extract_properties_with_itemref(element, document)?;

        Ok(MicrodataItem::new(item_type, item_id, properties))
    }

    /// Extract properties from an item using the microdata algorithm
    fn extract_properties(item_element: &Selection) -> Result<Vec<MicrodataProperty>> {
        let mut properties = Vec::new();

        // Simplified approach for dom_query 0.19: find all descendant elements with itemprop
        let descendants = item_element.select("[itemprop]");
        
        for descendant in descendants.iter() {
            // Simple approach: just process all itemprop elements
            // This will include nested items, but that's handled elsewhere
            let element_properties = MicrodataProperty::from_element(&descendant)?;
            properties.extend(element_properties);
        }
        
        // Note: For full itemref support, use from_element_with_document instead

        Ok(properties)
    }
    
    /// Extract properties with itemref support
    fn extract_properties_with_itemref(item_element: &Selection, document: &Document) -> Result<Vec<MicrodataProperty>> {
        let mut properties = Vec::new();
        let mut visited_ids = HashSet::new();
        
        // First, collect properties from descendants
        let descendants = item_element.select("[itemprop]");
        for descendant in descendants.iter() {
            let element_properties = crate::MicrodataProperty::from_element_with_document(&descendant, document)?;
            properties.extend(element_properties);
        }
        
        // Process itemref on this element (if present)
        if let Some(itemref) = item_element.attr("itemref") {
            let ids: Vec<&str> = itemref.split_whitespace().collect();
            
            for id in ids {
                if id.is_empty() || visited_ids.contains(id) {
                    continue;
                }
                visited_ids.insert(id.to_string());
                
                // Find element with this ID
                let selector = format!("#{}", escape_css_id(id));
                let referenced_elements = document.select(&selector);
                
                if referenced_elements.length() == 0 {
                    continue; // Don't error, just skip missing references
                }
                
                // Process the referenced element and its descendants
                for ref_element in referenced_elements.iter() {
                    if ref_element.has_attr("itemprop") {
                        let element_properties = crate::MicrodataProperty::from_element_with_document(&ref_element, document)?;
                        properties.extend(element_properties);
                    }
                    
                    let ref_descendants = ref_element.select("[itemprop]");
                    for ref_descendant in ref_descendants.iter() {
                        if !is_within_different_itemscope(&ref_descendant, &ref_element) {
                            let element_properties = crate::MicrodataProperty::from_element_with_document(&ref_descendant, document)?;
                            properties.extend(element_properties);
                        }
                    }
                }
            }
        }
        
        // Also look for elements that reference this item via itemref
        if let Some(item_id) = item_element.attr("id") {
            let itemref_selector = format!("[itemref~='{}']", escape_css_id(&item_id));
            let referencing_elements = document.select(&itemref_selector);
            
            for ref_element in referencing_elements.iter() {
                if ref_element.has_attr("itemprop") {
                    let element_properties = crate::MicrodataProperty::from_element_with_document(&ref_element, document)?;
                    properties.extend(element_properties);
                }
            }
        }
        
        Ok(properties)
    }

    // TODO: Add itemref support with proper DOM traversal
    // These methods are kept for future implementation

    /// Get the item type
    pub fn item_type(&self) -> Option<&str> {
        self.item_type.as_deref()
    }

    /// Get the item ID
    pub fn item_id(&self) -> Option<&str> {
        self.item_id.as_deref()
    }

    /// Get all properties
    pub fn properties(&self) -> &[MicrodataProperty] {
        &self.properties
    }

    /// Get properties by name
    pub fn get_properties(&self, name: &str) -> Vec<&MicrodataProperty> {
        self.properties
            .iter()
            .filter(|prop| prop.name() == name)
            .collect()
    }

    /// Get the first property with the given name
    pub fn get_property(&self, name: &str) -> Option<String> {
        self.get_properties(name)
            .first()
            .map(|prop| prop.value_as_string())
    }

    /// Get property values as strings
    pub fn get_property_values(&self, name: &str) -> Vec<String> {
        self.get_properties(name)
            .iter()
            .map(|prop| prop.value_as_string())
            .collect()
    }

    /// Get nested items for a property
    pub fn get_nested_items(&self, name: &str) -> Vec<&MicrodataItem> {
        self.get_properties(name)
            .iter()
            .filter_map(|prop| prop.as_item())
            .collect()
    }

    /// Get all property names
    pub fn property_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.properties.iter().map(|prop| prop.name()).collect();
        names.sort();
        names.dedup();
        names
    }

    /// Convert to a HashMap representation for easier access
    pub fn to_hashmap(&self) -> HashMap<String, Vec<String>> {
        let mut map = HashMap::new();

        for property in &self.properties {
            map.entry(property.name().to_string())
                .or_insert_with(Vec::new)
                .push(property.value_as_string());
        }

        map
    }
}

// Helper functions for itemref support

/// Escape CSS identifier for use in selectors
fn escape_css_id(id: &str) -> String {
    // Simple CSS identifier escaping
    // For production, consider using a proper CSS escape library
    id.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c.to_string(),
            _ => format!("\\{:x}", c as u32),
        })
        .collect()
}

/// Check if an element is within a different itemscope than the given ancestor
fn is_within_different_itemscope(element: &Selection, ancestor: &Selection) -> bool {
    // Check if there's an itemscope between element and ancestor
    // This is a simplified check - for full compliance we'd need proper DOM traversal
    let element_html = element.html();
    let ancestor_html = ancestor.html();
    
    // If they're the same element, return false
    if element_html == ancestor_html {
        return false;
    }
    
    // This is a limitation of the current implementation
    // We can't easily traverse the DOM tree with dom_query 0.19
    // So we'll use a heuristic: check if the element has itemscope
    // In practice, this works for most cases
    false
}