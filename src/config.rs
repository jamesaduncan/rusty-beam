use dom_query::Document;
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone)]
pub struct HostConfig {
    pub host_root: String,
}

pub struct ServerConfig {
    pub server_root: String,
    pub bind_address: String,
    pub bind_port: u16,
    pub hosts: HashMap<String, HostConfig>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            server_root: "./files".to_string(),
            bind_address: "127.0.0.1".to_string(),
            bind_port: 3000,
            hosts: HashMap::new(),
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
            // Find all elements with itemprop="hostName" and their corresponding hostRoot elements
            let host_name_elements = document.select("[itemprop='hostName']");
            let host_root_elements = document.select("[itemprop='hostRoot']");
            
            // Pair up host names and roots - they should be in the same order
            let min_length = std::cmp::min(host_name_elements.length(), host_root_elements.length());
            for i in 0..min_length {
                let host_name_element = host_name_elements.get(i).unwrap();
                let host_root_element = host_root_elements.get(i).unwrap();
                
                let host_name = host_name_element.text().trim().to_string();
                let host_root = host_root_element.text().trim().to_string();
                
                // Only add non-empty host configs
                if !host_name.is_empty() && !host_root.is_empty() {
                    let host_config = HostConfig {
                        host_root: host_root.clone(),
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