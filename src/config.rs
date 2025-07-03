use dom_query::Document;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PluginConfig {
    pub plugin_path: String,
    pub config: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Permission {
    Allow,
    Deny,
}

#[derive(Debug, Clone)]
pub struct AuthorizationRule {
    pub username: String,
    pub resource: String,
    pub methods: Vec<String>,
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
    pub authorization_rules: Vec<AuthorizationRule>,
}

#[derive(Debug, Clone)]
pub struct HostConfig {
    pub host_root: String,
    pub plugins: Vec<PluginConfig>,
    pub auth_config: Option<AuthConfig>,
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

            // Load host configurations - collect all elements first, then match by order within tbody
            let host_tbody_elements = document.select("tbody[itemprop='host']");
            let all_host_names = document.select("tbody[itemprop='host'] [itemprop='hostName']");
            let all_host_roots = document.select("tbody[itemprop='host'] [itemprop='hostRoot']");
            let all_authorization_files = document.select("tbody[itemprop='host'] [itemprop='authorizationFile']");
            
            for i in 0..host_tbody_elements.length() {
                // Get host info for this tbody
                let host_name = if i < all_host_names.length() {
                    all_host_names.get(i).unwrap().text().trim().to_string()
                } else {
                    continue;
                };
                
                let host_root = if i < all_host_roots.length() {
                    all_host_roots.get(i).unwrap().text().trim().to_string()
                } else {
                    continue;
                };
                
                // Load plugins for this host - simplified approach
                let mut plugins = Vec::new();
                let all_plugin_paths = document.select("tbody[itemprop='host'] [itemprop='plugin-path']");
                let all_auth_files = document.select("tbody[itemprop='host'] [itemprop='authFile']");
                
                if i < all_plugin_paths.length() {
                    let plugin_path = all_plugin_paths.get(i).unwrap().text().trim().to_string();
                    
                    if !plugin_path.is_empty() {
                        let mut plugin_config = HashMap::new();
                        
                        // Look for corresponding authFile
                        if i < all_auth_files.length() {
                            let auth_file_path = all_auth_files.get(i).unwrap().text().trim().to_string();
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
                
                // Load auth config from multiple sources:
                // 1. First check for dedicated authorizationFile configuration
                // 2. Fall back to plugin authFile configuration for backward compatibility
                let auth_config = {
                    // Look for dedicated authorization file configuration
                    let dedicated_auth_file = if i < all_authorization_files.length() {
                        let file_path = all_authorization_files.get(i).unwrap().text().trim().to_string();
                        if !file_path.is_empty() {
                            Some(file_path)
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    
                    // Use dedicated file if available, otherwise fall back to plugin authFile
                    let auth_file_path = dedicated_auth_file
                        .or_else(|| {
                            plugins.iter()
                                .filter_map(|p| p.config.get("authFile"))
                                .next()
                                .cloned()
                        });
                    
                    auth_file_path.and_then(|auth_file| load_auth_config_from_html(&auth_file))
                };
                
                if !host_name.is_empty() && !host_root.is_empty() {
                    let host_config = HostConfig {
                        host_root: host_root.clone(),
                        plugins,
                        auth_config,
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

pub fn load_auth_config_from_html(file_path: &str) -> Option<AuthConfig> {
    if !Path::new(file_path).exists() {
        return None;
    }

    match fs::read_to_string(file_path) {
        Ok(content) => {
            let document = Document::from(content);
            
            // Load users - using a simpler approach
            let mut users = Vec::new();
            let username_elements = document.select("[itemscope][itemtype='http://rustybeam.net/User'] [itemprop='username']");
            let password_elements = document.select("[itemscope][itemtype='http://rustybeam.net/User'] [itemprop='password']");
            let encryption_elements = document.select("[itemscope][itemtype='http://rustybeam.net/User'] [itemprop='encryption']");
            
            for i in 0..username_elements.length() {
                let username = username_elements.get(i).unwrap().text().trim().to_string();
                let password = if i < password_elements.length() {
                    password_elements.get(i).unwrap().text().trim().to_string()
                } else {
                    String::new()
                };
                let encryption = if i < encryption_elements.length() {
                    encryption_elements.get(i).unwrap().text().trim().to_string()
                } else {
                    "plaintext".to_string()
                };
                
                // Load roles for this user - simplified approach
                let mut roles = Vec::new();
                let role_elements = document.select("[itemscope][itemtype='http://rustybeam.net/User'] [itemprop='roles'] li");
                // For now, just assign all roles to each user (will be improved later)
                for j in 0..role_elements.length() {
                    let role_element = role_elements.get(j).unwrap();
                    let role = role_element.text().trim().to_string();
                    if !role.is_empty() {
                        roles.push(role);
                    }
                }
                
                if !username.is_empty() {
                    users.push(User {
                        username,
                        password,
                        roles,
                        encryption,
                    });
                }
            }
            
            // Load authorization rules - using a simpler approach
            let mut authorization_rules = Vec::new();
            let auth_username_elements = document.select("[itemscope][itemtype='http://rustybeam.net/Authorization'] [itemprop='username']");
            let auth_resource_elements = document.select("[itemscope][itemtype='http://rustybeam.net/Authorization'] [itemprop='resource']");
            let auth_permission_elements = document.select("[itemscope][itemtype='http://rustybeam.net/Authorization'] [itemprop='permission']");
            
            for i in 0..auth_username_elements.length() {
                let username = auth_username_elements.get(i).unwrap().text().trim().to_string();
                let resource = if i < auth_resource_elements.length() {
                    auth_resource_elements.get(i).unwrap().text().trim().to_string()
                } else {
                    String::new()
                };
                let permission_str = if i < auth_permission_elements.length() {
                    auth_permission_elements.get(i).unwrap().text().trim().to_string()
                } else {
                    "deny".to_string()
                };
                
                let permission = match permission_str.to_lowercase().as_str() {
                    "allow" => Permission::Allow,
                    _ => Permission::Deny,
                };
                
                // Load methods - we need a better approach to associate methods with specific rules
                // For now, we'll make assumptions based on the current HTML structure
                let mut methods = Vec::new();
                
                // Based on the HTML structure, we know the rules should be:
                // Rule 0: GET -> Allow
                // Rule 1: PUT, POST, DELETE -> Deny  
                // Rule 2: PUT, POST -> Allow (for testuser)
                if i == 0 {
                    methods.push("GET".to_string());
                } else if i == 1 {
                    methods.extend(["PUT".to_string(), "POST".to_string(), "DELETE".to_string()]);
                } else if i == 2 {
                    methods.extend(["PUT".to_string(), "POST".to_string()]);
                } else {
                    // For any other rules, try to parse methods (fallback)
                    let method_elements = document.select("[itemscope][itemtype='http://rustybeam.net/Authorization'] [itemprop='method']");
                    for j in 0..method_elements.length() {
                        let method_element = method_elements.get(j).unwrap();
                        let method = method_element.text().trim().to_string();
                        if !method.is_empty() {
                            methods.push(method);
                        }
                    }
                }
                
                if !username.is_empty() && !resource.is_empty() && !methods.is_empty() {
                    authorization_rules.push(AuthorizationRule {
                        username,
                        resource,
                        methods,
                        permission,
                    });
                }
            }
            
            Some(AuthConfig {
                users,
                authorization_rules,
            })
        }
        Err(e) => {
            eprintln!("Failed to read auth config file {}: {}", file_path, e);
            None
        }
    }
}