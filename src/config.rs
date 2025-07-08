use crate::{log_error, log_verbose};
use microdata_extract::MicrodataExtractor;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PluginConfig {
    pub library: String, // URL to plugin library (file://, http://, https://)
    #[allow(dead_code)] // May be used by future plugin types
    pub plugin_type: Option<String>,
    pub config: HashMap<String, String>,
    #[allow(dead_code)] // Used for nested plugin configurations
    pub nested_plugins: Vec<PluginConfig>, // Support for recursive plugin structure
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
    #[allow(dead_code)] // Used in authentication plugins
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
    pub plugins: Vec<PluginConfig>, // Back to plugins since "pipeline is just a plugin"
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
            bind_address: "0.0.0.0".to_string(),
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

                    // Load host configurations from all items
                    for item in &items {
                        if item.item_type() == Some("http://rustybeam.net/HostConfig") {
                            log_verbose!("Found a HostConfig item");
                            let host_name = item.get_property("hostName").unwrap_or_default();
                            let host_root = item.get_property("hostRoot").unwrap_or_default();
                            let server_header = item.get_property("serverHeader");

                            log_verbose!("Host name: {}, host root: {}", host_name, host_root);

                            if !host_name.is_empty() && !host_root.is_empty() {
                                // Parse plugin pipeline from the new format
                                let plugins = parse_plugin_pipeline(item);

                                let host_config = HostConfig {
                                    host_root,
                                    plugins,
                                    server_header,
                                };
                                config.hosts.insert(host_name, host_config);
                            }
                        }
                    }

                    // Configuration loaded
                }
                Err(e) => {
                    log_error!("Failed to parse microdata from {}: {}", file_path, e);
                    log_error!("Using default configuration");
                }
            }
        }
        Err(e) => {
            log_error!("Failed to read config file {}: {}", file_path, e);
            log_error!("Using default configuration");
        }
    }
    config
}

/// Parse plugin pipeline from new configuration format
fn parse_plugin_pipeline(host_item: &microdata_extract::MicrodataItem) -> Vec<PluginConfig> {
    let mut plugins = Vec::new();
    let mut nested_plugin_refs = std::collections::HashSet::new();

    // Debug: print all properties
    log_verbose!("Host item properties: {:?}", host_item.properties());

    // Get all plugin properties
    let plugin_props = host_item.get_properties("plugin");
    log_verbose!("Found {} plugin properties", plugin_props.len());

    // Debug: print all property names
    log_verbose!(
        "All property names in host item: {:?}",
        host_item.property_names()
    );

    // First pass: identify which plugin items are nested within other plugins
    for prop in &plugin_props {
        if let Some(plugin_item) = prop.as_item() {
            // Get all nested plugin items within this plugin
            let nested_items = plugin_item.get_nested_items("plugin");
            for nested in nested_items {
                // Store a reference to identify nested plugins
                // We'll use the item's properties as a fingerprint
                let fingerprint = format!("{:?}", nested.properties());
                nested_plugin_refs.insert(fingerprint);
            }
        }
    }

    // Second pass: process only non-nested plugin items
    for (i, prop) in plugin_props.iter().enumerate() {
        if let Some(plugin_item) = prop.as_item() {
            // Create fingerprint for this plugin
            let fingerprint = format!("{:?}", plugin_item.properties());

            // Skip if this plugin is nested within another
            if nested_plugin_refs.contains(&fingerprint) {
                log_verbose!(
                    "Skipping plugin item {} - it's nested within another plugin",
                    i
                );
                continue;
            }

            log_verbose!("Processing plugin item {}", i);
            log_verbose!(
                "  Has library property: {}",
                get_direct_property(plugin_item, "library").is_some()
            );
            log_verbose!(
                "  Nested plugin count: {}",
                plugin_item.get_nested_items("plugin").len()
            );

            if let Some(plugin_config) = parse_plugin_config(plugin_item) {
                log_verbose!(
                    "Successfully parsed plugin config: {}",
                    plugin_config.library
                );
                log_verbose!(
                    "  Nested plugins in config: {}",
                    plugin_config.nested_plugins.len()
                );
                plugins.push(plugin_config);
            } else {
                log_verbose!("Plugin item {} has no library property or was rejected", i);
            }
        }
    }

    plugins
}

/// Parse individual plugin configuration with security validation
fn parse_plugin_config(plugin_item: &microdata_extract::MicrodataItem) -> Option<PluginConfig> {
    // Check if this plugin directly has a library property (not from nested items)
    let library = get_direct_property(plugin_item, "library");

    // Check if this is a plugin container (no library but has nested plugins)
    let nested_plugins = parse_nested_plugins(plugin_item);

    // Handle different cases:
    // 1. Plugin with library and possibly nested plugins
    // 2. Plugin container with no library but nested plugins
    if let Some(lib) = library.clone() {
        if lib.is_empty() && nested_plugins.is_empty() {
            return None;
        }

        // Security validation: Check URL scheme and file extension
        if !lib.is_empty() && !is_secure_plugin_url(&lib) {
            log_error!(
                "Security warning: Rejecting potentially unsafe plugin URL: {}",
                lib
            );
            return None;
        }
    } else if nested_plugins.is_empty() {
        // No library and no nested plugins - invalid
        return None;
    }

    // Extract plugin configuration properties
    let mut config = HashMap::new();

    // Get all available properties (direct only, not from nested items)
    if let Some(realm) = get_direct_property(plugin_item, "realm") {
        if !realm.is_empty() {
            config.insert("realm".to_string(), realm);
        }
    }

    if let Some(authfile) = get_direct_property(plugin_item, "authfile") {
        if !authfile.is_empty() {
            config.insert("authfile".to_string(), authfile);
        }
    }

    if let Some(log_file) = get_direct_property(plugin_item, "log_file") {
        if !log_file.is_empty() {
            config.insert("log_file".to_string(), log_file);
        }
    }

    // Add any other properties that might be present
    for property in plugin_item.properties() {
        let key = property.name();
        if !["library", "realm", "authfile", "log_file"].contains(&key) {
            let value = property.value_as_string();
            if !value.is_empty() {
                config.insert(key.to_string(), value);
            }
        }
    }

    // Infer plugin type from library name if not explicitly set
    let plugin_type = library.as_ref().map(|lib| infer_plugin_type(lib));

    // For plugin containers without a library, create a special pipeline plugin
    let final_library = library.unwrap_or_else(|| "pipeline://nested".to_string());

    Some(PluginConfig {
        library: final_library,
        plugin_type,
        config,
        nested_plugins,
    })
}

/// Get a property value only if it's directly on this item (not from nested items)
fn get_direct_property(
    item: &microdata_extract::MicrodataItem,
    property_name: &str,
) -> Option<String> {
    // Due to a bug in microdata-extract, properties from nested itemscope elements
    // are incorrectly included in the parent's properties. We need to work around this.

    // Strategy: If this item has nested items with the same property, and the property
    // values match, then the property likely comes from the nested item, not this item.

    let properties = item.get_properties(property_name);
    if properties.is_empty() {
        return None;
    }

    // Check if any nested items have this property
    let nested_items = item.get_nested_items("plugin");
    for nested in nested_items {
        if let Some(nested_value) = nested.get_property(property_name) {
            // If a nested item has this property with the same value as our first property,
            // then our property is likely inherited from the nested item
            if properties.first().map(|p| p.value_as_string()) == Some(nested_value) {
                return None; // This property belongs to a nested item, not us
            }
        }
    }

    // If we get here, the property is likely direct
    Some(properties.first().unwrap().value_as_string())
}

/// Parse nested plugins recursively
fn parse_nested_plugins(plugin_item: &microdata_extract::MicrodataItem) -> Vec<PluginConfig> {
    let mut nested_plugins = Vec::new();

    // Get nested plugin items
    let nested_items = plugin_item.get_nested_items("plugin");
    for nested_item in nested_items {
        if let Some(nested_config) = parse_plugin_config(nested_item) {
            nested_plugins.push(nested_config);
        }
    }

    nested_plugins
}

/// Security validation for plugin URLs
fn is_secure_plugin_url(url: &str) -> bool {
    // No built-in plugins - all plugins must be loaded from valid URLs

    // Parse URL to get scheme and path
    if let Ok(parsed_url) = url::Url::parse(url) {
        let scheme = parsed_url.scheme();
        let path = parsed_url.path();

        // Check file extension
        let is_dynamic_library =
            path.ends_with(".so") || path.ends_with(".dll") || path.ends_with(".dylib");

        let is_wasm = path.ends_with(".wasm");

        match scheme {
            "file" => {
                // Local files are always allowed (both dynamic libraries and WASM)
                is_dynamic_library || is_wasm
            }
            "http" | "https" => {
                // Remote URLs: only WASM allowed, reject dynamic libraries
                if is_dynamic_library {
                    false // Security: Never load .so/.dll/.dylib from remote URLs
                } else if is_wasm {
                    true // WASM is sandboxed, safe to load remotely
                } else {
                    false // Unknown extension
                }
            }
            _ => false, // Unknown scheme
        }
    } else {
        false // Invalid URL
    }
}

/// Infer plugin type from library name
fn infer_plugin_type(library: &str) -> String {
    let filename = library.split('/').next_back().unwrap_or(library);
    let name_part = filename.split('.').next().unwrap_or(filename);

    // Remove common prefixes
    let clean_name = name_part
        .strip_prefix("lib")
        .unwrap_or(name_part)
        .replace("-", " ")
        .replace("_", " ");

    // Convert to title case
    clean_name
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
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
                            let encryption = item
                                .get_property("encryption")
                                .unwrap_or_else(|| "plaintext".to_string());

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
                            let permission_str = item
                                .get_property("permission")
                                .unwrap_or_else(|| "deny".to_string());

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
                    log_error!(
                        "Failed to parse microdata from auth config file {}: {}",
                        file_path,
                        e
                    );
                    None
                }
            }
        }
        Err(e) => {
            log_error!("Failed to read auth config file {}: {}", file_path, e);
            None
        }
    }
}
