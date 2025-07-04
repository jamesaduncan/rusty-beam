use microdata_extract::MicrodataExtractor;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PluginConfig {
    pub plugin_path: String,
    pub plugin_type: Option<String>,
    pub config: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Permission {
    Allow,
    #[allow(dead_code)] // Used by authorization plugins and rules parsing
    Deny,
}

#[derive(Debug, Clone)]
pub struct AuthorizationRule {
    #[allow(dead_code)] // Used by legacy system and file-authz plugin
    pub username: String,
    #[allow(dead_code)] // Used by legacy system and file-authz plugin
    pub resource: String,
    #[allow(dead_code)] // Used by legacy system and file-authz plugin
    pub methods: Vec<String>,
    #[allow(dead_code)] // Used by legacy system and file-authz plugin
    pub permission: Permission,
}

#[derive(Debug, Clone)]
pub struct User {
    pub username: String,
    #[allow(dead_code)] // Used in authentication plugins and tests
    pub password: String,
    #[allow(dead_code)] // Used in authorization engine and plugin interfaces
    pub roles: Vec<String>,
    #[allow(dead_code)] // Used in authentication plugins and config parsing
    pub encryption: String,
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    #[allow(dead_code)] // Used in auth.rs get_user method and config loading
    pub users: Vec<User>,
    #[allow(dead_code)] // Used by legacy system and file-authz plugin
    pub authorization_rules: Vec<AuthorizationRule>,
}

#[derive(Debug, Clone)]
pub struct HostConfig {
    pub host_root: String,
    pub plugins: Vec<PluginConfig>,
    pub server_header: Option<String>,
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
            let extractor = MicrodataExtractor::new();
            match extractor.extract(&content) {
                Ok(items) => {
                    // Find ServerConfig items
                    for item in &items {
                        if item.item_type() == Some("http://rustybeam.net/ServerConfig") {
                            if let Some(server_root) = item.get_property("serverRoot") {
                                config.server_root = server_root;
                            }
                            if let Some(bind_address) = item.get_property("bindAddress") {
                                config.bind_address = bind_address;
                            }
                            if let Some(bind_port) = item.get_property("bindPort") {
                                if let Ok(port) = bind_port.parse::<u16>() {
                                    config.bind_port = port;
                                }
                            }
                        }
                    }

                    // Load host configurations from nested items
                    for item in &items {
                        if item.item_type() == Some("http://rustybeam.net/ServerConfig") {
                            // Get nested host items
                            let host_items = item.get_nested_items("host");
                            for host_item in host_items {
                                let host_name = host_item.get_property("hostName").unwrap_or_default();
                                let host_root = host_item.get_property("hostRoot").unwrap_or_default();
                                let server_header = host_item.get_property("serverHeader");
                                
                                if !host_name.is_empty() && !host_root.is_empty() {
                                    let mut plugins = Vec::new();
                                    
                                    // Get plugin configurations from nested plugin items
                                    let plugin_items = host_item.get_nested_items("plugin");
                                    for plugin_item in plugin_items {
                                        if let Some(plugin_path) = plugin_item.get_property("plugin-path") {
                                            if !plugin_path.is_empty() {
                                                let mut plugin_config = HashMap::new();
                                                
                                                if let Some(auth_file) = plugin_item.get_property("authFile") {
                                                    if !auth_file.is_empty() {
                                                        plugin_config.insert("authFile".to_string(), auth_file);
                                                    }
                                                }
                                                
                                                if let Some(log_file) = plugin_item.get_property("log_file") {
                                                    if !log_file.is_empty() {
                                                        plugin_config.insert("log_file".to_string(), log_file);
                                                    }
                                                }
                                                
                                                if let Some(realm) = plugin_item.get_property("realm") {
                                                    if !realm.is_empty() {
                                                        plugin_config.insert("realm".to_string(), realm);
                                                    }
                                                }
                                                
                                                let plugin_type = plugin_item.get_property("plugin-type");
                                                
                                                plugins.push(PluginConfig {
                                                    plugin_path,
                                                    plugin_type,
                                                    config: plugin_config,
                                                });
                                            }
                                        }
                                    }
                                    
                                    let host_config = HostConfig {
                                        host_root,
                                        plugins,
                                        server_header,
                                    };
                                    config.hosts.insert(host_name, host_config);
                                }
                            }
                        }
                    }

                    
                    // Configuration loaded
                }
                Err(e) => {
                    eprintln!("Failed to parse microdata from {}: {}", file_path, e);
                    eprintln!("Using default configuration");
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to read config file {}: {}", file_path, e);
            eprintln!("Using default configuration");
        }
    }
    config
}

#[allow(dead_code)] // Used by authorization plugins for loading auth configurations
pub fn load_auth_config_from_html(file_path: &str) -> Option<AuthConfig> {
    if !Path::new(file_path).exists() {
        return None;
    }

    match fs::read_to_string(file_path) {
        Ok(content) => {
            let extractor = MicrodataExtractor::new();
            match extractor.extract(&content) {
                Ok(items) => {
                    let mut users = Vec::new();
                    let mut authorization_rules = Vec::new();
                    
                    // Load users using microdata extraction
                    for item in &items {
                        if item.item_type() == Some("http://rustybeam.net/User") {
                            let username = item.get_property("username").unwrap_or_default();
                            let password = item.get_property("password").unwrap_or_default();
                            let encryption = item.get_property("encryption").unwrap_or_else(|| "plaintext".to_string());
                            
                            // Get roles (multiple values for the same property)
                            let roles = item.get_property_values("role");
                            
                            if !username.is_empty() {
                                users.push(User {
                                    username,
                                    password,
                                    roles,
                                    encryption,
                                });
                            }
                        }
                    }
            
                    // Load authorization rules using microdata extraction
                    for item in &items {
                        if item.item_type() == Some("http://rustybeam.net/Authorization") {
                            let username = item.get_property("username").unwrap_or_default();
                            let resource = item.get_property("resource").unwrap_or_default();
                            let permission_str = item.get_property("permission").unwrap_or_else(|| "deny".to_string());
                            
                            let permission = match permission_str.to_lowercase().as_str() {
                                "allow" => Permission::Allow,
                                _ => Permission::Deny,
                            };
                            
                            // Get methods (multiple values for the same property)
                            let methods = item.get_property_values("method");
                            
                            if !username.is_empty() && !resource.is_empty() && !methods.is_empty() {
                                authorization_rules.push(AuthorizationRule {
                                    username,
                                    resource,
                                    methods,
                                    permission,
                                });
                            }
                        }
                    }
            
                    
                    Some(AuthConfig {
                        users,
                        authorization_rules,
                    })
                }
                Err(e) => {
                    eprintln!("Failed to parse microdata from auth config file {}: {}", file_path, e);
                    None
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to read auth config file {}: {}", file_path, e);
            None
        }
    }
}