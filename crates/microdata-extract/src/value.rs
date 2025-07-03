//! Value extraction from HTML elements according to microdata specification

use crate::Result;
use dom_query::Selection;
use url::Url;

/// Represents a microdata property value
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MicrodataValue {
    /// String value (most common)
    Text(String),
    /// URL value (from href, src, etc.)
    Url(Url),
    /// Nested item value
    Item(crate::MicrodataItem),
    /// Date/time value (as ISO 8601 string)
    DateTime(String),
    /// Numeric value
    Number(f64),
    /// Boolean value
    Boolean(bool),
}

impl MicrodataValue {
    /// Extract value from an HTML element according to microdata rules
    pub fn extract_from_element(element: &Selection) -> Result<Self> {
        // Get tag name from the element's HTML and parse it
        let html = element.html();
        let tag_name = if let Some(start) = html.find('<') {
            if let Some(end) = html[start + 1..].find(|c: char| c.is_whitespace() || c == '>') {
                html[start + 1..start + 1 + end].to_lowercase()
            } else {
                "unknown".to_string()
            }
        } else {
            "unknown".to_string()
        };

        // If element has itemscope, it's a nested item
        if element.has_attr("itemscope") {
            return Ok(MicrodataValue::Item(
                crate::MicrodataItem::from_element(element)?,
            ));
        }

        // Value extraction rules based on element type (per microdata spec)
        match tag_name.as_str() {
            // meta element: use content attribute
            "meta" => {
                if let Some(content) = element.attr("content") {
                    Ok(MicrodataValue::Text(content.to_string()))
                } else {
                    Ok(MicrodataValue::Text(String::new()))
                }
            }

            // Audio, embed, iframe, img, source, track, video: use src
            "audio" | "embed" | "iframe" | "img" | "source" | "track" | "video" => {
                if let Some(src) = element.attr("src") {
                    Self::parse_url_value(src)
                } else {
                    Ok(MicrodataValue::Text(String::new()))
                }
            }

            // a, area, link: use href
            "a" | "area" | "link" => {
                if let Some(href) = element.attr("href") {
                    Self::parse_url_value(href)
                } else {
                    Ok(MicrodataValue::Text(String::new()))
                }
            }

            // object: use data attribute
            "object" => {
                if let Some(data) = element.attr("data") {
                    Self::parse_url_value(data)
                } else {
                    Ok(MicrodataValue::Text(String::new()))
                }
            }

            // data: use value attribute
            "data" => {
                if let Some(value) = element.attr("value") {
                    Ok(MicrodataValue::Text(value.to_string()))
                } else {
                    Ok(MicrodataValue::Text(element.text().to_string()))
                }
            }

            // meter: use value attribute
            "meter" => {
                if let Some(value) = element.attr("value") {
                    Self::parse_numeric_value(value)
                } else {
                    Ok(MicrodataValue::Text(element.text().to_string()))
                }
            }

            // time: use datetime attribute if present, otherwise text content
            "time" => {
                if let Some(datetime) = element.attr("datetime") {
                    Ok(MicrodataValue::DateTime(datetime.to_string()))
                } else {
                    Ok(MicrodataValue::Text(element.text().to_string()))
                }
            }

            // input: depends on type
            "input" => {
                let input_type = element.attr("type").unwrap_or_else(|| "text".into());
                match input_type.as_ref() {
                    "checkbox" | "radio" => {
                        if element.has_attr("checked") {
                            if let Some(value) = element.attr("value") {
                                Ok(MicrodataValue::Text(value.to_string()))
                            } else {
                                Ok(MicrodataValue::Boolean(true))
                            }
                        } else {
                            Ok(MicrodataValue::Boolean(false))
                        }
                    }
                    _ => {
                        if let Some(value) = element.attr("value") {
                            Ok(MicrodataValue::Text(value.to_string()))
                        } else {
                            Ok(MicrodataValue::Text(String::new()))
                        }
                    }
                }
            }

            // select: use selected option's value
            "select" => {
                let selected_option = element.select("option[selected]").first();
                if selected_option.length() > 0 {
                    if let Some(value) = selected_option.attr("value") {
                        Ok(MicrodataValue::Text(value.to_string()))
                    } else {
                        Ok(MicrodataValue::Text(selected_option.text().to_string()))
                    }
                } else {
                    Ok(MicrodataValue::Text(String::new()))
                }
            }

            // All other elements: use text content
            _ => Ok(MicrodataValue::Text(element.text().trim().to_string())),
        }
    }

    /// Parse a URL value, falling back to text if invalid
    fn parse_url_value<T: AsRef<str>>(url_str: T) -> Result<MicrodataValue> {
        match Url::parse(url_str.as_ref()) {
            Ok(url) => Ok(MicrodataValue::Url(url)),
            Err(_) => {
                // If it's not a valid absolute URL, treat as text
                // This handles relative URLs and other non-URL strings
                Ok(MicrodataValue::Text(url_str.as_ref().to_string()))
            }
        }
    }

    /// Parse a numeric value, falling back to text if invalid
    fn parse_numeric_value<T: AsRef<str>>(value_str: T) -> Result<MicrodataValue> {
        match value_str.as_ref().parse::<f64>() {
            Ok(num) => Ok(MicrodataValue::Number(num)),
            Err(_) => Ok(MicrodataValue::Text(value_str.as_ref().to_string())),
        }
    }

    /// Get the value as a string representation
    pub fn as_string(&self) -> String {
        match self {
            MicrodataValue::Text(s) => s.clone(),
            MicrodataValue::Url(url) => url.to_string(),
            MicrodataValue::Item(_) => "[Item]".to_string(),
            MicrodataValue::DateTime(dt) => dt.clone(),
            MicrodataValue::Number(n) => n.to_string(),
            MicrodataValue::Boolean(b) => b.to_string(),
        }
    }

    /// Check if this value is a nested item
    pub fn is_item(&self) -> bool {
        matches!(self, MicrodataValue::Item(_))
    }

    /// Get the nested item if this value is an item
    pub fn as_item(&self) -> Option<&crate::MicrodataItem> {
        match self {
            MicrodataValue::Item(item) => Some(item),
            _ => None,
        }
    }
}