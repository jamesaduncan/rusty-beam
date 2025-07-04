//! Configuration loader that supports URL-based configs

use super::config::{ServerConfig, load_config_from_html_string};
use super::loader::{load_config_from_url, PluginLoadError};

/// Load server configuration from a URL
pub async fn load_server_config_from_url(url: &str) -> Result<ServerConfig, String> {
    // Fetch the config content
    let html_content = load_config_from_url(url)
        .await
        .map_err(|e| format!("Failed to load config from URL: {}", e))?;
    
    // Parse the HTML config
    load_config_from_html_string(&html_content)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_file_url_loading() {
        // This would test loading from file:// URLs
        // For now, we'll test the error case
        let result = load_server_config_from_url("file:///nonexistent/config.html").await;
        assert!(result.is_err());
    }
}