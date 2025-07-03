//! Microdata property representation

use crate::{MicrodataValue, Result};
use dom_query::Selection;

/// Represents a microdata property (name-value pair)
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MicrodataProperty {
    /// Property name (from itemprop attribute)
    pub name: String,
    /// Property value
    pub value: MicrodataValue,
}

impl MicrodataProperty {
    /// Create a new microdata property
    pub fn new(name: String, value: MicrodataValue) -> Self {
        Self { name, value }
    }

    /// Extract a property from an HTML element with itemprop
    pub fn from_element(element: &Selection) -> Result<Vec<Self>> {
        let mut properties = Vec::new();

        if let Some(itemprop) = element.attr("itemprop") {
            // itemprop can contain multiple space-separated property names
            let property_names: Vec<&str> = itemprop
                .split_whitespace()
                .filter(|name| !name.is_empty())
                .collect();

            if property_names.is_empty() {
                return Ok(properties);
            }

            // Extract the value once for all property names
            let value = MicrodataValue::extract_from_element(element)?;

            // Create a property for each name
            for name in property_names {
                properties.push(MicrodataProperty::new(name.to_string(), value.clone()));
            }
        }

        Ok(properties)
    }

    /// Get the property name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the property value
    pub fn value(&self) -> &MicrodataValue {
        &self.value
    }

    /// Get the property value as a string
    pub fn value_as_string(&self) -> String {
        self.value.as_string()
    }

    /// Check if this property contains a nested item
    pub fn is_item_property(&self) -> bool {
        self.value.is_item()
    }

    /// Get the nested item if this property contains one
    pub fn as_item(&self) -> Option<&crate::MicrodataItem> {
        self.value.as_item()
    }
}