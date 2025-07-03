//! Microdata item representation

use crate::{MicrodataError, MicrodataProperty, Result};
use dom_query::Selection;
use std::collections::HashMap;
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
        
        // TODO: Add itemref support later - for now focus on basic functionality

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

// TODO: Add CSS identifier escaping for itemref support