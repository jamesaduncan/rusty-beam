// Compile-time version from Cargo.toml
#[allow(dead_code)] // Available for future use (logging, debugging, etc.)
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_SERVER_HEADER: &str = concat!("rusty-beam/", env!("CARGO_PKG_VERSION"));