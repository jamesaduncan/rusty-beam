use super::plugin::PluginContext;
use super::loader;
use microdata_extract::{MicrodataExtractor, MicrodataItem};
use std::collections::HashMap;

/// Server-wide configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub server_root: String,
    pub bind_address: String,
    pub bind_port: u16,
    pub hosts: HashMap<String, HostConfig>,
}

/// Host-specific configuration
#[derive(Debug, Clone)]
pub struct HostConfig {
    pub host_name: String,
    pub host_root: String,
    pub pipeline_config: PipelineConfig,
    pub host_config: HashMap<String, String>, // Host-level configuration values
}

/// Pipeline configuration that describes the plugin chain
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub items: Vec<PipelineItem>,
}

/// A single item in the pipeline - either a plugin or nested pipeline
#[derive(Debug, Clone)]
pub enum PipelineItem {
    Plugin {
        library_path: String,
        config: HashMap<String, String>,
    },
    Pipeline {
        items: Vec<PipelineItem>,
    },
}

/// Parse configuration from HTML file using microdata
pub fn load_config_from_html(config_path: &str) -> Result<ServerConfig, String> {
    // Read the HTML file
    let html_content = std::fs::read_to_string(config_path)
        .map_err(|e| format!("Failed to read config file {}: {}", config_path, e))?;
    
    load_config_from_html_string(&html_content)
}

/// Parse configuration from HTML string using microdata
pub fn load_config_from_html_string(html_content: &str) -> Result<ServerConfig, String> {
    // Extract microdata
    let extractor = MicrodataExtractor::new();
    let items = extractor.extract(html_content)
        .map_err(|e| format!("Failed to parse microdata: {}", e))?;
    
    // Find ServerConfig item
    let server_item = items.iter()
        .find(|item| item.item_type == Some("http://rustybeam.net/ServerConfig".to_string()))
        .ok_or("No ServerConfig found in configuration")?;
    
    // Parse server configuration
    let server_root = server_item.get_property("serverRoot")
        .unwrap_or("./examples/files".to_string());
    let bind_address = server_item.get_property("bindAddress")
        .unwrap_or("127.0.0.1".to_string());
    let bind_port = server_item.get_property("bindPort")
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    
    // Parse host configurations
    let mut hosts = HashMap::new();
    
    // Find all HostConfig items
    for item in &items {
        if item.item_type == Some("http://rustybeam.net/HostConfig".to_string()) {
            let host_config = parse_host_config(item)?;
            hosts.insert(host_config.host_name.clone(), host_config);
        }
    }
    
    Ok(ServerConfig {
        server_root,
        bind_address,
        bind_port,
        hosts,
    })
}

/// Parse a single host configuration from microdata
fn parse_host_config(item: &MicrodataItem) -> Result<HostConfig, String> {
    let host_name = item.get_property("hostName")
        .ok_or("Host configuration missing hostName")?;
    let host_root = item.get_property("hostRoot")
        .unwrap_or("./examples/files".to_string());
    
    // Parse the pipeline configuration
    let pipeline_config = parse_pipeline_config(item)?;
    
    // Extract any additional host-level configuration
    let mut host_config = HashMap::new();
    
    // Look for any additional properties that aren't structural
    for property in &item.properties {
        match property.name.as_str() {
            "hostName" | "hostRoot" | "pipeline" => continue, // Skip structural properties
            _ => {
                host_config.insert(property.name.clone(), property.value.as_string());
            }
        }
    }
    
    Ok(HostConfig {
        host_name,
        host_root,
        pipeline_config,
        host_config,
    })
}

/// Parse pipeline configuration from microdata
fn parse_pipeline_config(host_item: &MicrodataItem) -> Result<PipelineConfig, String> {
    // Look for pipeline property
    let pipeline_html = host_item.get_property("pipeline")
        .ok_or("Host configuration missing pipeline")?;
    
    // Re-parse the pipeline HTML to extract nested structure
    let extractor = MicrodataExtractor::new();
    let pipeline_items = extractor.extract(&pipeline_html)
        .map_err(|e| format!("Failed to parse pipeline microdata: {}", e))?;
    
    // Find the Pipeline item
    let pipeline_item = pipeline_items.iter()
        .find(|item| item.item_type == Some("http://rustybeam.net/Pipeline".to_string()))
        .ok_or("No Pipeline found in pipeline configuration")?;
    
    let items = parse_pipeline_items(pipeline_item)?;
    
    Ok(PipelineConfig { items })
}

/// Parse individual pipeline items (plugins or nested pipelines)
fn parse_pipeline_items(pipeline_item: &MicrodataItem) -> Result<Vec<PipelineItem>, String> {
    let mut items = Vec::new();
    
    // Look for plugin properties
    let plugin_properties: Vec<_> = pipeline_item.properties.iter()
        .filter(|p| p.name == "plugin")
        .collect();
    
    for plugin_property in plugin_properties {
        let plugin_html = plugin_property.value.as_string();
        // Parse each plugin
        let extractor = MicrodataExtractor::new();
        let plugin_items = extractor.extract(&plugin_html)
            .map_err(|e| format!("Failed to parse plugin microdata: {}", e))?;
        
        for plugin_item in &plugin_items {
            match plugin_item.item_type.as_deref() {
                Some("http://rustybeam.net/Plugin") => {
                    // Parse plugin configuration
                    let library_path = plugin_item.get_property("library")
                        .ok_or("Plugin missing library path")?;
                    
                    // Extract plugin-specific configuration
                    let mut config = HashMap::new();
                    for property in &plugin_item.properties {
                        if property.name != "library" {
                            config.insert(property.name.clone(), property.value.as_string());
                        }
                    }
                    
                    items.push(PipelineItem::Plugin {
                        library_path,
                        config,
                    });
                }
                Some("http://rustybeam.net/Pipeline") => {
                    // Parse nested pipeline
                    let nested_items = parse_pipeline_items(plugin_item)?;
                    items.push(PipelineItem::Pipeline {
                        items: nested_items,
                    });
                }
                _ => {
                    return Err(format!("Unknown pipeline item type: {:?}", plugin_item.item_type));
                }
            }
        }
    }
    
    Ok(items)
}

/// Create a plugin context from configuration
pub fn create_plugin_context(
    server_config: &ServerConfig,
    host_config: &HostConfig,
    plugin_config: HashMap<String, String>,
    host_name: String,
    request_id: String,
) -> PluginContext {
    // Create server-level config map
    let mut server_config_map = HashMap::new();
    server_config_map.insert("serverRoot".to_string(), server_config.server_root.clone());
    server_config_map.insert("bindAddress".to_string(), server_config.bind_address.clone());
    server_config_map.insert("bindPort".to_string(), server_config.bind_port.to_string());
    
    PluginContext {
        plugin_config,
        host_config: host_config.host_config.clone(),
        server_config: server_config_map,
        host_name,
        request_id,
        shared_state: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;
    
    #[test]
    fn test_load_simple_config() {
        let config_html = r#"
<!DOCTYPE html>
<html>
<body>
    <table itemscope itemtype="http://rustybeam.net/ServerConfig">
        <tr>
            <td>Server Root</td>
            <td itemprop="serverRoot">./test/files</td>
        </tr>
        <tr>
            <td>Address</td>
            <td itemprop="bindAddress">0.0.0.0</td>
        </tr>
        <tr>
            <td>Port</td>
            <td itemprop="bindPort">8080</td>
        </tr>
    </table>
    
    <table itemprop="host" itemscope itemtype="http://rustybeam.net/HostConfig">
        <tr>
            <td>Host Name</td>
            <td itemprop="hostName">test.local</td>
        </tr>
        <tr>
            <td>Host Root</td>
            <td itemprop="hostRoot">./test/host</td>
        </tr>
        <tr>
            <td>Pipeline</td>
            <td itemprop="pipeline">
                <ol itemscope itemtype="http://rustybeam.net/Pipeline">
                    <li itemprop="plugin" itemscope itemtype="http://rustybeam.net/Plugin">
                        <span itemprop="library">./plugins/test-plugin</span>
                    </li>
                </ol>
            </td>
        </tr>
    </table>
</body>
</html>
        "#;
        
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(config_html.as_bytes()).unwrap();
        
        let config = load_config_from_html(temp_file.path().to_str().unwrap()).unwrap();
        
        assert_eq!(config.server_root, "./test/files");
        assert_eq!(config.bind_address, "0.0.0.0");
        assert_eq!(config.bind_port, 8080);
        
        assert!(config.hosts.contains_key("test.local"));
        let host_config = &config.hosts["test.local"];
        assert_eq!(host_config.host_name, "test.local");
        assert_eq!(host_config.host_root, "./test/host");
        assert_eq!(host_config.pipeline_config.items.len(), 1);
        
        match &host_config.pipeline_config.items[0] {
            PipelineItem::Plugin { library_path, config: _ } => {
                assert_eq!(library_path, "./plugins/test-plugin");
            }
            _ => panic!("Expected plugin item"),
        }
    }
    
    #[test]
    fn test_create_plugin_context() {
        let server_config = ServerConfig {
            server_root: "./test".to_string(),
            bind_address: "127.0.0.1".to_string(),
            bind_port: 3000,
            hosts: HashMap::new(),
        };
        
        let mut host_config_map = HashMap::new();
        host_config_map.insert("hostSetting".to_string(), "hostValue".to_string());
        
        let host_config = HostConfig {
            host_name: "test.local".to_string(),
            host_root: "./test/host".to_string(),
            pipeline_config: PipelineConfig { items: Vec::new() },
            host_config: host_config_map,
        };
        
        let mut plugin_config = HashMap::new();
        plugin_config.insert("pluginSetting".to_string(), "pluginValue".to_string());
        
        let context = create_plugin_context(
            &server_config,
            &host_config,
            plugin_config,
            "test.local".to_string(),
            "req123".to_string(),
        );
        
        // Test configuration hierarchy
        assert_eq!(context.get_config("pluginSetting"), Some("pluginValue")); // Plugin level
        assert_eq!(context.get_config("hostSetting"), Some("hostValue"));     // Host level
        assert_eq!(context.get_config("serverRoot"), Some("./test"));         // Server level
        assert_eq!(context.get_config("nonexistent"), None);                  // Not found
    }
}