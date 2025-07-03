use dom_query::Document;
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone)]
pub struct PluginConfig {
    pub plugin_path: String,
    pub config: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct HostConfig {
    pub host_root: String,
    pub plugins: Vec<PluginConfig>,
}

pub struct ServerConfig {
    pub server_root: String,
    pub bind_address: String,
    pub bind_port: u16,
    pub hosts: HashMap<String, HostConfig>,
    #[allow(dead_code)] // Reserved for future server-wide plugin support
    pub server_wide_plugins: Vec<PluginConfig>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            server_root: "./files".to_string(),
            bind_address: "127.0.0.1".to_string(),
            bind_port: 3000,
            hosts: HashMap::new(),
            server_wide_plugins: Vec::new(),
        }
    }
}

pub fn load_config_from_html(file_path: &str) -> ServerConfig {
    let mut config = ServerConfig::default();

    match fs::read_to_string(file_path) {
        Ok(content) => {
            let document = Document::from(content);

            // Find elements with the specific itemtype
            let config_elements =
                document.select("[itemscope][itemtype='http://rustybeam.net/ServerConfig']");

            if !config_elements.is_empty() {
                // Find all elements with itemprop attributes
                let props = document.select("[itemprop]");
                for i in 0..props.length() {
                    let prop = props.get(i).unwrap();
                    let prop_name = match prop.attr("itemprop") {
                        Some(name) => name,
                        None => continue,
                    };

                    let prop_value = prop.text().trim().to_string();

                    match &*prop_name {
                        "serverRoot" => config.server_root = prop_value,
                        "bindAddress" => config.bind_address = prop_value,
                        "bindPort" => {
                            if let Ok(port) = prop_value.parse::<u16>() {
                                config.bind_port = port;
                            }
                        }
                        _ => {} // Ignore unknown properties
                    }
                }
            }

            // Load host configurations 
            let host_name_elements = document.select("[itemprop='hostName']");
            let host_root_elements = document.select("[itemprop='hostRoot']");
            
            // Pair up host names and roots - they should be in the same order
            let min_length = std::cmp::min(host_name_elements.length(), host_root_elements.length());
            for i in 0..min_length {
                let host_name_element = host_name_elements.get(i).unwrap();
                let host_root_element = host_root_elements.get(i).unwrap();
                
                let host_name = host_name_element.text().trim().to_string();
                let host_root = host_root_element.text().trim().to_string();
                
                // Load plugins for this host by finding plugin elements in the same tbody
                let mut plugins = Vec::new();
                
                // Find plugin elements - look for plugin-path items that have the same parent tbody
                let plugin_path_elements = document.select("[itemprop='plugin-path']");
                for j in 0..plugin_path_elements.length() {
                    let plugin_element = plugin_path_elements.get(j).unwrap();
                    let plugin_path = plugin_element.text().trim().to_string();
                    
                    if !plugin_path.is_empty() {
                        let mut plugin_config = HashMap::new();
                        
                        // Look for the authFile configuration in the same context
                        let auth_file_elements = document.select("[itemprop='authFile']");
                        for k in 0..auth_file_elements.length() {
                            let auth_file_element = auth_file_elements.get(k).unwrap();
                            let auth_file_path = auth_file_element.text().trim().to_string();
                            if !auth_file_path.is_empty() {
                                plugin_config.insert("authFile".to_string(), auth_file_path);
                            }
                        }
                        
                        plugins.push(PluginConfig {
                            plugin_path,
                            config: plugin_config,
                        });
                    }
                }
                
                // Only add non-empty host configs
                if !host_name.is_empty() && !host_root.is_empty() {
                    let host_config = HostConfig {
                        host_root: host_root.clone(),
                        plugins,
                    };
                    config.hosts.insert(host_name, host_config);
                }
            }

            println!("Loaded configuration from {}", file_path);
            println!("  Configured hosts: {}", config.hosts.len());
            for (name, host_config) in &config.hosts {
                println!("    Host: {} -> {}", name, host_config.host_root);
            }
        }
        Err(e) => {
            eprintln!("Failed to read config file {}: {}", file_path, e);
            eprintln!("Using default configuration");
        }
    }
    config
}