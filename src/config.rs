//! Configuration management for Rusty Beam
//!
//! This module handles loading and parsing server configuration from HTML files
//! using microdata attributes. It supports:
//!
//! - Server-level configuration (bind address, daemon settings)
//! - Host-specific configuration (document root, plugin pipelines)
//! - Plugin configuration with nested plugin support
//! - Security validation for plugin URLs
//!
//! The configuration format uses HTML microdata schemas for structured,
//! human-readable configuration that can be validated and documented.

use crate::log_error;
use microdata_extract::MicrodataExtractor;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

// Microdata schema URLs
const SCHEMA_SERVER_CONFIG: &str = "https://rustybeam.net/schema/ServerConfig";
const SCHEMA_HOST_CONFIG: &str = "https://rustybeam.net/schema/HostConfig";
const SCHEMA_CREDENTIAL: &str = "https://rustybeam.net/schema/Credential";

// Plugin URL schemes
const PLUGIN_SCHEME_PIPELINE: &str = "pipeline://nested";

// Default configuration values
const DEFAULT_SERVER_ROOT: &str = "./files";
const DEFAULT_BIND_ADDRESS: &str = "0.0.0.0";
const DEFAULT_BIND_PORT: u16 = 3000;
const DEFAULT_PID_FILE: &str = "/tmp/rusty-beam.pid";
const DEFAULT_STDOUT_FILE: &str = "/tmp/rusty-beam.stdout";
const DEFAULT_STDERR_FILE: &str = "/tmp/rusty-beam.stderr";
const DEFAULT_UMASK: u32 = 0o027;
const DEFAULT_ENCRYPTION: &str = "plaintext";

// Plugin configuration property names
const COMMON_PLUGIN_PROPERTIES: &[&str] = &["realm", "authfile", "log_file"];

/// Configuration for a single plugin in the processing pipeline
#[derive(Debug, Clone, serde::Serialize)]
pub struct PluginConfig {
    /// URL to plugin library (file://, http://, https://, or special schemes)
    pub library: String,
    /// Human-readable plugin type (inferred from library name)
    #[allow(dead_code)] // Reserved for future use
    pub plugin_type: Option<String>,
    /// Plugin-specific configuration parameters
    pub config: HashMap<String, String>,
    /// Nested plugins for pipeline-type plugins
    pub nested_plugins: Vec<PluginConfig>,
}


/// User credential information for authentication
#[derive(Debug, Clone)]
pub struct User {
    /// Username for authentication
    #[allow(dead_code)] // Used by authorization plugin through FFI
    pub username: String,
    /// Password (encrypted or plaintext based on encryption field)
    #[allow(dead_code)] // Password is read directly from HTML by basic-auth plugin
    pub password: String,
    /// List of roles assigned to this user
    #[allow(dead_code)] // Used by authorization plugin through FFI
    pub roles: Vec<String>,
    /// Encryption method for the password ("plaintext", "bcrypt", etc.)
    #[allow(dead_code)] // Reserved for future password encryption support
    pub encryption: String,
}

/// Authentication configuration containing user credentials
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// List of users for authentication
    #[allow(dead_code)] // Legacy field - auth is now handled by plugins
    pub users: Vec<User>,
}

/// Configuration for a specific virtual host
#[derive(Debug, Clone)]
pub struct HostConfig {
    /// Document root directory for this host
    pub host_root: String,
    /// Plugin pipeline for processing requests to this host
    pub plugins: Vec<PluginConfig>,
    /// Custom Server header value for this host
    pub server_header: Option<String>,
}

/// Main server configuration loaded from HTML microdata
pub struct ServerConfig {
    /// Default document root directory
    pub server_root: String,
    /// IP address to bind the server to
    pub bind_address: String,
    /// Port number to bind the server to
    pub bind_port: u16,
    /// Virtual host configurations keyed by hostname
    pub hosts: HashMap<String, HostConfig>,
    /// Server-wide plugins (reserved for future use)
    #[allow(dead_code)] // Reserved for future server-wide plugin support
    pub server_wide_plugins: Vec<PluginConfig>,
    
    // Daemon configuration options
    /// Path to PID file for daemon mode
    pub daemon_pid_file: Option<String>,
    /// User to run daemon as
    pub daemon_user: Option<String>,
    /// Group to run daemon as
    pub daemon_group: Option<String>,
    /// File creation mask for daemon
    pub daemon_umask: Option<u32>,
    /// Path to redirect stdout in daemon mode
    pub daemon_stdout: Option<String>,
    /// Path to redirect stderr in daemon mode
    pub daemon_stderr: Option<String>,
    /// Whether to change ownership of PID file
    pub daemon_chown_pid_file: Option<bool>,
    /// Working directory for daemon
    pub daemon_working_directory: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            server_root: DEFAULT_SERVER_ROOT.to_string(),
            bind_address: DEFAULT_BIND_ADDRESS.to_string(),
            bind_port: DEFAULT_BIND_PORT,
            hosts: HashMap::new(),
            server_wide_plugins: Vec::new(),
            
            // Sensible daemon defaults
            daemon_pid_file: Some(DEFAULT_PID_FILE.to_string()),
            daemon_user: None,
            daemon_group: None,
            daemon_umask: Some(DEFAULT_UMASK),
            daemon_stdout: Some(DEFAULT_STDOUT_FILE.to_string()),
            daemon_stderr: Some(DEFAULT_STDERR_FILE.to_string()),
            daemon_chown_pid_file: Some(true),
            daemon_working_directory: None, // Will be set to config file directory
        }
    }
}

/// Reads and validates the configuration file
fn read_config_file(file_path: &str) -> Result<String, std::io::Error> {
    match fs::read_to_string(file_path) {
        Ok(content) => Ok(content),
        Err(e) => {
            log_error!("Failed to read config file {}: {}", file_path, e);
            log_error!("Using default configuration");
            Err(e)
        }
    }
}

/// Parses microdata from HTML content
fn parse_microdata(
    content: &str, 
    file_path: &str
) -> Result<Vec<microdata_extract::MicrodataItem>, Box<dyn std::error::Error>> {
    let extractor = MicrodataExtractor::new();
    match extractor.extract(content) {
        Ok(items) => Ok(items),
        Err(e) => {
            log_error!("Failed to parse microdata from {}: {}", file_path, e);
            log_error!("Using default configuration");
            Err(Box::new(e))
        }
    }
}

/// Parses an optional string property from microdata item
fn parse_optional_string(item: &microdata_extract::MicrodataItem, property: &str) -> Option<String> {
    item.get_property(property)
}

/// Parses an optional boolean property from microdata item
/// 
/// Accepts "true" (case-insensitive) as true, everything else as false
fn parse_optional_bool(item: &microdata_extract::MicrodataItem, property: &str) -> Option<bool> {
    item.get_property(property).map(|s| s.to_lowercase() == "true")
}

/// Parses an optional octal umask from microdata item
/// 
/// Accepts both "0o027" and "027" formats
fn parse_optional_umask(item: &microdata_extract::MicrodataItem, property: &str) -> Option<u32> {
    item.get_property(property)
        .and_then(|s| u32::from_str_radix(&s.trim_start_matches("0o"), 8).ok())
}

/// Loads server configuration from an HTML file using microdata
/// 
/// Falls back to default configuration on any errors
pub fn load_config_from_html(file_path: &str) -> ServerConfig {
    let content = match read_config_file(file_path) {
        Ok(content) => content,
        Err(_) => return ServerConfig::default(),
    };

    let items = match parse_microdata(&content, file_path) {
        Ok(items) => items,
        Err(_) => return ServerConfig::default(),
    };

    let mut config = ServerConfig::default();
    
    // Find ServerConfig items
    for item in &items {
        if item.item_type() == Some(SCHEMA_SERVER_CONFIG) {
            if let Some(server_root) = item.get_property("serverRoot") {
                config.server_root = server_root;
            }
            if let Some(bind_address) = item.get_property("bindAddress") {
                config.bind_address = bind_address;
            }
            if let Some(bind_port) = item.get_property("bindPort") {
                match bind_port.parse::<u16>() {
                    Ok(port) => config.bind_port = port,
                    Err(e) => { log_error!("Invalid bind port '{}': {}", bind_port, e); }
                }
            }
            
            // Parse daemon configuration options
            config.daemon_pid_file = parse_optional_string(item, "daemonPidFile");
            config.daemon_user = parse_optional_string(item, "daemonUser");
            config.daemon_group = parse_optional_string(item, "daemonGroup");
            config.daemon_umask = parse_optional_umask(item, "daemonUmask");
            config.daemon_stdout = parse_optional_string(item, "daemonStdout");
            config.daemon_stderr = parse_optional_string(item, "daemonStderr");
            config.daemon_chown_pid_file = parse_optional_bool(item, "daemonChownPidFile");
            config.daemon_working_directory = parse_optional_string(item, "daemonWorkingDirectory");
        }
    }

    // Load host configurations from all items
    for item in &items {
        if item.item_type() == Some(SCHEMA_HOST_CONFIG) {
            // Process HostConfig item
            // Get all hostname values (cardinality 1..n)
            let hostnames = item.get_property_values("hostname");
            let host_root = item.get_property("hostRoot").unwrap_or_default();
            let server_header = item.get_property("serverHeader");

            if hostnames.is_empty() {
                log_error!("HostConfig missing required hostname property");
                continue;
            }

            if host_root.is_empty() {
                log_error!("HostConfig missing required hostRoot property");
                continue;
            }

            // Configure host for multiple hostnames

            // Parse plugin pipeline from the new format
            let plugins = parse_plugin_pipeline(item);

            // Create HostConfig once
            let host_config = HostConfig {
                host_root,
                plugins,
                server_header,
            };

            // Insert the same HostConfig for each hostname
            for hostname in hostnames {
                // Add host configuration
                // Clone the HostConfig for each hostname
                config.hosts.insert(hostname, host_config.clone());
            }
        }
    }

    config
}

/// Parses plugin pipeline from host configuration microdata
/// 
/// Processes nested plugin structures and filters out duplicates to build
/// a clean pipeline of top-level plugins with their nested children
fn parse_plugin_pipeline(host_item: &microdata_extract::MicrodataItem) -> Vec<PluginConfig> {
    let mut plugins = Vec::new();
    let mut nested_plugin_refs = std::collections::HashSet::new();

    // Extract plugin configurations from microdata
    let plugin_props = host_item.get_properties("plugin");

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
    for (_i, prop) in plugin_props.iter().enumerate() {
        if let Some(plugin_item) = prop.as_item() {
            // Create fingerprint for this plugin
            let fingerprint = format!("{:?}", plugin_item.properties());

            // Skip if this plugin is nested within another
            if nested_plugin_refs.contains(&fingerprint) {
                // Skip nested plugin (will be processed by parent)
                continue;
            }

            // Process top-level plugin configuration

            if let Some(plugin_config) = parse_plugin_config(plugin_item) {
                // Plugin configuration parsed successfully
                plugins.push(plugin_config);
            } else {
                // Plugin configuration rejected or incomplete
            }
        }
    }

    plugins
}

/// Parses a plugin configuration from microdata
fn parse_plugin_config(plugin_item: &microdata_extract::MicrodataItem) -> Option<PluginConfig> {
    let library = get_direct_property(plugin_item, "library");
    let nested_plugins = parse_nested_plugins(plugin_item);
    
    // Validate plugin has either a library or nested plugins
    if !is_valid_plugin_configuration(&library, &nested_plugins) {
        return None;
    }
    
    // Security validation for library URLs
    if let Some(ref lib) = library {
        if !lib.is_empty() && !is_secure_plugin_url(lib) {
            log_error!("Security warning: Rejecting unsafe plugin URL: {}", lib);
            return None;
        }
    }
    
    let config = extract_plugin_properties(plugin_item);
    let plugin_type = library.as_ref().map(|lib| infer_plugin_type(lib));
    let final_library = library.unwrap_or_else(|| PLUGIN_SCHEME_PIPELINE.to_string());
    
    Some(PluginConfig {
        library: final_library,
        plugin_type,
        config,
        nested_plugins,
    })
}

/// Validates that a plugin configuration is valid
fn is_valid_plugin_configuration(library: &Option<String>, nested_plugins: &[PluginConfig]) -> bool {
    match library {
        Some(lib) => !lib.is_empty() || !nested_plugins.is_empty(),
        None => !nested_plugins.is_empty(),
    }
}

/// Extracts configuration properties from a plugin item
fn extract_plugin_properties(plugin_item: &microdata_extract::MicrodataItem) -> HashMap<String, String> {
    let mut config = HashMap::new();
    
    // Extract commonly used plugin configuration properties
    for &property_name in COMMON_PLUGIN_PROPERTIES {
        if let Some(value) = get_direct_property(plugin_item, property_name) {
            if !value.is_empty() {
                config.insert(property_name.to_string(), value);
            }
        }
    }
    
    // Extract any additional properties (excluding core plugin properties)
    for property in plugin_item.properties() {
        let key = property.name();
        if !is_core_plugin_property(key) {
            let value = property.value_as_string();
            if !value.is_empty() {
                config.insert(key.to_string(), value);
            }
        }
    }
    
    config
}

/// Checks if a property is a core plugin property that shouldn't be included in config
fn is_core_plugin_property(property_name: &str) -> bool {
    matches!(property_name, "library" | "realm" | "authfile" | "log_file")
}

/// Extracts a property value that belongs directly to this item, not nested items
/// 
/// This works around a microdata-extract library limitation where nested item properties
/// are incorrectly included in the parent's property list.
fn get_direct_property(
    item: &microdata_extract::MicrodataItem,
    property_name: &str,
) -> Option<String> {
    let properties = item.get_properties(property_name);
    if properties.is_empty() {
        return None;
    }
    
    let first_value = properties.first()?.value_as_string();
    
    // Check if this property value belongs to a nested item
    if is_property_from_nested_item(item, property_name, &first_value) {
        return None;
    }
    
    Some(first_value)
}

/// Checks if a property value originates from a nested item rather than the current item
fn is_property_from_nested_item(
    item: &microdata_extract::MicrodataItem,
    property_name: &str,
    property_value: &str,
) -> bool {
    let nested_items = item.get_nested_items("plugin");
    
    for nested in nested_items {
        if let Some(nested_value) = nested.get_property(property_name) {
            if nested_value == property_value {
                return true; // Property belongs to nested item
            }
        }
    }
    
    false
}

/// Recursively parses nested plugin configurations
/// 
/// Processes plugin items that are children of other plugins,
/// supporting arbitrarily deep nesting
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

/// Validates plugin URLs for security compliance
/// 
/// Ensures plugin URLs follow security policies:
/// - Local files: Allow .so/.dll/.dylib and .wasm
/// - Remote URLs: Only allow .wasm (sandboxed execution)
/// - Reject all other schemes and extensions
fn is_secure_plugin_url(url: &str) -> bool {
    let Ok(parsed_url) = url::Url::parse(url) else {
        return false; // Invalid URL format
    };
    
    let scheme = parsed_url.scheme();
    let path = parsed_url.path();
    
    match scheme {
        "file" => is_allowed_local_plugin(path),
        "http" | "https" => is_allowed_remote_plugin(path),
        _ => false, // Unknown/unsupported scheme
    }
}

/// Checks if a local file path has an allowed plugin extension
fn is_allowed_local_plugin(path: &str) -> bool {
    path.ends_with(".so") || path.ends_with(".dll") || path.ends_with(".dylib") || path.ends_with(".wasm")
}

/// Checks if a remote URL path has an allowed plugin extension (WASM only)
fn is_allowed_remote_plugin(path: &str) -> bool {
    path.ends_with(".wasm") // Only WASM allowed from remote sources for security
}

/// Infers a human-readable plugin type from the library filename
/// 
/// Converts library names like "libbasic-auth.so" to "Basic Auth"
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

/// Loads authentication configuration from HTML file using microdata
/// 
/// Returns None if file doesn't exist or cannot be parsed
#[allow(dead_code)] // Used by authorization plugins for loading auth configurations
pub fn load_auth_config_from_html(file_path: &str) -> Option<AuthConfig> {
    if !Path::new(file_path).exists() {
        return None;
    }

    let content = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => {
            log_error!("Failed to read auth config file {}: {}", file_path, e);
            return None;
        }
    };

    let items = match MicrodataExtractor::new().extract(&content) {
        Ok(items) => items,
        Err(e) => {
            log_error!(
                "Failed to parse microdata from auth config file {}: {}",
                file_path,
                e
            );
            return None;
        }
    };

    let users = extract_users_from_items(&items);
    
    Some(AuthConfig { users })
}

/// Extracts user credentials from microdata items
fn extract_users_from_items(items: &[microdata_extract::MicrodataItem]) -> Vec<User> {
    let mut users = Vec::new();

    for item in items {
        if item.item_type() == Some(SCHEMA_CREDENTIAL) {
            if let Some(user) = create_user_from_item(item) {
                users.push(user);
            }
        }
    }

    users
}

/// Creates a User from a microdata item if valid
fn create_user_from_item(item: &microdata_extract::MicrodataItem) -> Option<User> {
    let username = item.get_property("username").unwrap_or_default();
    let password = item.get_property("password").unwrap_or_default();
    let encryption = item
        .get_property("encryption")
        .unwrap_or_else(|| DEFAULT_ENCRYPTION.to_string());
    let roles = item.get_property_values("role");

    if username.is_empty() {
        return None;
    }

    Some(User {
        username,
        password,
        roles,
        encryption,
    })
}
