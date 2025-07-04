//! WebAssembly plugin support using wasmtime
//! 
//! This module provides the runtime for executing WASM plugins safely

use super::plugin::{Plugin, PluginRequest, PluginContext};
use super::loader::PluginLoadError;
use async_trait::async_trait;
use hyper::{Body, Response};
use std::collections::HashMap;
use wasmtime::*;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};

/// WASM plugin wrapper that implements the Plugin trait
#[derive(Debug)]
pub struct WasmPluginWrapper {
    store: Store<WasmState>,
    instance: Instance,
    name: String,
}

/// State stored in the WASM store
struct WasmState {
    wasi: WasiCtx,
    memory: Option<Memory>,
}

impl WasmPluginWrapper {
    /// Create a new WASM plugin from bytes
    pub async fn new(
        wasm_bytes: &[u8],
        config: HashMap<String, String>,
        name: String,
    ) -> Result<Self, PluginLoadError> {
        // Create engine with default config
        let engine = Engine::default();
        
        // Compile the module
        let module = Module::new(&engine, wasm_bytes)
            .map_err(|e| PluginLoadError::WasmError(format!("Failed to compile WASM: {}", e)))?;
        
        // Create WASI context
        let wasi = WasiCtxBuilder::new()
            .inherit_stdio()
            .build();
        
        let mut store = Store::new(&engine, WasmState {
            wasi,
            memory: None,
        });
        
        // Create a linker and add WASI
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::add_to_linker(&mut linker, |state: &mut WasmState| &mut state.wasi)
            .map_err(|e| PluginLoadError::WasmError(format!("Failed to link WASI: {}", e)))?;
        
        // Add our plugin API functions
        Self::add_plugin_api(&mut linker)?;
        
        // Instantiate the module
        let instance = linker.instantiate(&mut store, &module)
            .map_err(|e| PluginLoadError::WasmError(format!("Failed to instantiate WASM: {}", e)))?;
        
        // Get memory export
        if let Some(memory) = instance.get_memory(&mut store, "memory") {
            store.data_mut().memory = Some(memory);
        }
        
        // Call init function if it exists
        if let Some(init_func) = instance.get_typed_func::<(i32, i32), i32>(&mut store, "plugin_init").ok() {
            // Serialize config to JSON
            let config_json = serde_json::to_string(&config)
                .map_err(|e| PluginLoadError::PluginInitError(format!("Failed to serialize config: {}", e)))?;
            
            let config_bytes = config_json.as_bytes();
            
            // Write config to WASM memory
            let config_ptr = Self::write_bytes_to_memory(&mut store, config_bytes)?;
            
            // Call init
            let result = init_func.call(&mut store, (config_ptr as i32, config_bytes.len() as i32))
                .map_err(|e| PluginLoadError::PluginInitError(format!("Init failed: {}", e)))?;
            
            if result != 0 {
                return Err(PluginLoadError::PluginInitError("Plugin init returned error".to_string()));
            }
        }
        
        Ok(WasmPluginWrapper {
            store,
            instance,
            name,
        })
    }
    
    /// Add plugin API functions to the linker
    fn add_plugin_api(linker: &mut Linker<WasmState>) -> Result<(), PluginLoadError> {
        // Add logging function
        linker.func_wrap("env", "plugin_log", |caller: Caller<'_, WasmState>, ptr: i32, len: i32| {
            if let Some(memory) = caller.data().memory {
                let data = memory.data(&caller);
                let start = ptr as usize;
                let end = start + len as usize;
                
                if end <= data.len() {
                    if let Ok(message) = std::str::from_utf8(&data[start..end]) {
                        println!("[WASM Plugin] {}", message);
                    }
                }
            }
        }).map_err(|e| PluginLoadError::WasmError(format!("Failed to add plugin_log: {}", e)))?;
        
        Ok(())
    }
    
    /// Write bytes to WASM memory and return pointer
    fn write_bytes_to_memory(store: &mut Store<WasmState>, bytes: &[u8]) -> Result<usize, PluginLoadError> {
        let memory = store.data().memory
            .ok_or_else(|| PluginLoadError::WasmError("No memory export found".to_string()))?;
        
        // Get allocate function
        // For now, we'll use a simple approach - real implementation would call malloc
        let data_len = memory.data(&mut *store).len();
        let ptr = data_len - bytes.len(); // Write at end of memory
        
        memory.write(store, ptr, bytes)
            .map_err(|e| PluginLoadError::WasmError(format!("Failed to write to memory: {}", e)))?;
        
        Ok(ptr)
    }
    
    /// Read bytes from WASM memory
    #[allow(dead_code)]
    fn read_bytes_from_memory(store: &Store<WasmState>, ptr: usize, len: usize) -> Result<Vec<u8>, PluginLoadError> {
        let memory = store.data().memory
            .ok_or_else(|| PluginLoadError::WasmError("No memory export found".to_string()))?;
        
        let data = memory.data(store);
        
        if ptr + len > data.len() {
            return Err(PluginLoadError::WasmError("Memory access out of bounds".to_string()));
        }
        
        Ok(data[ptr..ptr + len].to_vec())
    }
}

#[async_trait]
impl Plugin for WasmPluginWrapper {
    async fn handle_request(&self, request: &mut PluginRequest, _context: &PluginContext) -> Option<Response<Body>> {
        // This is a simplified implementation
        // A real implementation would:
        // 1. Serialize the request and context to a format the WASM module understands
        // 2. Call the WASM handle_request function
        // 3. Deserialize the response if any
        
        // For now, just log that we're handling a request
        println!("[WASM Plugin {}] Handling request: {} {}", 
                 self.name, 
                 request.http_request.method(), 
                 request.path);
        
        None
    }
    
    async fn handle_response(&self, request: &PluginRequest, response: &mut Response<Body>, _context: &PluginContext) {
        println!("[WASM Plugin {}] Handling response for: {} -> {}", 
                 self.name,
                 request.path,
                 response.status());
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

/// Example WASM plugin interface (what WASM modules should export)
/// This would be documented for plugin developers
#[allow(dead_code)]
pub mod wasm_plugin_interface {
    /// Initialize the plugin with JSON configuration
    /// Returns 0 on success, non-zero on error
    pub extern "C" fn plugin_init(_config_ptr: i32, _config_len: i32) -> i32 {
        // Plugin implementation
        0
    }
    
    /// Handle a request
    /// Returns 1 if response was generated, 0 otherwise
    pub extern "C" fn plugin_handle_request(
        _request_ptr: i32,
        _request_len: i32,
        _response_ptr: *mut i32,
        _response_len: *mut i32,
    ) -> i32 {
        // Plugin implementation
        0
    }
    
    /// Handle a response
    pub extern "C" fn plugin_handle_response(
        _request_ptr: i32,
        _request_len: i32,
        _response_ptr: i32,
        _response_len: i32,
    ) {
        // Plugin implementation
    }
    
    /// Get plugin name
    /// Returns pointer to null-terminated string
    pub extern "C" fn plugin_get_name() -> i32 {
        // Plugin implementation
        0
    }
    
    /// Allocate memory in WASM
    pub extern "C" fn plugin_alloc(_size: i32) -> i32 {
        // Plugin implementation
        0
    }
    
    /// Free memory in WASM
    pub extern "C" fn plugin_free(_ptr: i32) {
        // Plugin implementation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Note: Testing WASM requires actual WASM modules
    // For now, we'll test the basic structure
    
    #[test]
    fn test_wasm_state_creation() {
        let engine = Engine::default();
        let wasi = WasiCtxBuilder::new().build();
        
        let store = Store::new(&engine, WasmState {
            wasi,
            memory: None,
        });
        
        assert!(store.data().memory.is_none());
    }
}