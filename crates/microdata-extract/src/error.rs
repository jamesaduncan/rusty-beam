//! Error types for microdata extraction

use thiserror::Error;

/// Result type for microdata extraction operations
pub type Result<T> = std::result::Result<T, MicrodataError>;

/// Errors that can occur during microdata extraction
#[derive(Error, Debug, Clone, PartialEq)]
pub enum MicrodataError {
    /// Failed to parse HTML document
    #[error("Failed to parse HTML: {0}")]
    HtmlParseError(String),

    /// Invalid URL in itemtype or itemid
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// Circular reference detected in itemref
    #[error("Circular reference detected in itemref chain")]
    CircularReference,

    /// Invalid microdata structure
    #[error("Invalid microdata structure: {0}")]
    InvalidStructure(String),

    /// Element referenced by itemref not found
    #[error("Element with id '{0}' referenced by itemref not found")]
    ItemrefNotFound(String),

    /// Invalid property name
    #[error("Invalid property name: {0}")]
    InvalidPropertyName(String),
}