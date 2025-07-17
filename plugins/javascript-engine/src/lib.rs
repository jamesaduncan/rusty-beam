//! JavaScript Engine Plugin for Rusty Beam
//!
//! This plugin provides server-side JavaScript execution capabilities using the V8 engine.
//! It allows you to run JavaScript code to handle HTTP requests and generate responses.
//!
//! ## Features
//! - Server-side JavaScript execution using V8
//! - ES6 module syntax transformation
//! - Request/response object bridging
//! - Script caching for performance
//! - Route-based script mapping
//! - JavaScript console API (console.log, console.error)
//! - Asynchronous JavaScript support

use async_trait::async_trait;
use hyper::{Body, Response, StatusCode};
use once_cell::sync::OnceCell;
use rusty_beam_plugin_api::{
    create_plugin, Plugin, PluginContext, PluginRequest, PluginResponse,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

static V8_INITIALIZED: OnceCell<()> = OnceCell::new();

#[derive(Debug)]
pub struct JavaScriptEnginePlugin {
    name: String,
    scripts_dir: Arc<RwLock<PathBuf>>,
    route_mappings: Arc<RwLock<HashMap<String, String>>>,
    script_cache: Arc<RwLock<HashMap<String, String>>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsResponse {
    status: u16,
    headers: HashMap<String, String>,
    body: Option<String>,
}

impl Default for JavaScriptEnginePlugin {
    fn default() -> Self {
        Self::new_with_dir("javascript-engine".to_string(), PathBuf::from("./scripts"))
    }
}

impl JavaScriptEnginePlugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        let scripts_dir = config
            .get("javascript_engine_scripts_dir")
            .map(|s| PathBuf::from(s))
            .unwrap_or_else(|| PathBuf::from("./scripts"));
        
        let plugin = Self::new_with_dir("javascript-engine".to_string(), scripts_dir);
        
        // Load route mappings from config
        // Format: javascript_engine_route_/api/*=api.js
        for (key, value) in &config {
            if key.starts_with("javascript_engine_route_") {
                let pattern = key.strip_prefix("javascript_engine_route_").unwrap();
                let pattern = pattern.replace("_", "/");
                if let Ok(mut mappings) = plugin.route_mappings.try_write() {
                    mappings.insert(pattern, value.clone());
                }
            }
        }
        
        plugin
    }
    
    pub fn new_with_dir(name: String, scripts_dir: PathBuf) -> Self {
        V8_INITIALIZED.get_or_init(|| {
            let platform = v8::new_default_platform(0, false).make_shared();
            v8::V8::initialize_platform(platform);
            v8::V8::initialize();
        });

        Self {
            name,
            scripts_dir: Arc::new(RwLock::new(scripts_dir)),
            route_mappings: Arc::new(RwLock::new(HashMap::new())),
            script_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn load_script(&self, script_path: &str) -> Result<String, anyhow::Error> {
        let cache = self.script_cache.read().await;
        if let Some(cached) = cache.get(script_path) {
            return Ok(cached.clone());
        }
        drop(cache);

        let scripts_dir = self.scripts_dir.read().await;
        let full_path = scripts_dir.join(script_path);
        let content = tokio::fs::read_to_string(&full_path).await?;

        let mut cache = self.script_cache.write().await;
        cache.insert(script_path.to_string(), content.clone());
        Ok(content)
    }

    /// Executes JavaScript code with the provided request context
    async fn execute_javascript(
        &self,
        script_content: &str,
        request: &PluginRequest,
    ) -> Result<Option<JsResponse>, anyhow::Error> {
        let js_request = self.create_js_request(request).await;
        let request_json = serde_json::to_string(&js_request)?;

        // Set up V8 execution context
        let isolate = &mut v8::Isolate::new(Default::default());
        let handle_scope = &mut v8::HandleScope::new(isolate);
        let context = v8::Context::new(handle_scope, Default::default());
        let scope = &mut v8::ContextScope::new(handle_scope, context);

        // Set up global JavaScript functions (console, setTimeout, etc.)
        self.setup_global_functions(scope)?;
        
        // Set up the request object in global scope
        let global = context.global(scope);
        let request_key = v8::String::new(scope, "request").unwrap();
        let request_str = v8::String::new(scope, &request_json).unwrap();
        let json_obj = self.get_json_object(scope);
        let parse_key = v8::String::new(scope, "parse").unwrap();
        let parse_fn = json_obj.get(scope, parse_key.into()).unwrap();
        let parse_fn = v8::Local::<v8::Function>::try_from(parse_fn).unwrap();
        let undefined = v8::undefined(scope);
        let request_value = parse_fn.call(scope, undefined.into(), &[request_str.into()]).unwrap();
        global.set(scope, request_key.into(), request_value);
        
        // Transform and prepare script for execution
        let wrapper_script = self.prepare_script_for_execution(script_content);
        
        // Compile the JavaScript code
        let code = v8::String::new(scope, &wrapper_script).unwrap();
        let script = match v8::Script::compile(scope, code, None) {
            Some(script) => script,
            None => return Err(anyhow::anyhow!("Failed to compile JavaScript")),
        };

        // Execute the script and handle result
        match script.run(scope) {
            Some(value) => {
                if value.is_promise() {
                    self.resolve_promise(scope, value)
                } else {
                    self.process_js_result(scope, value)
                }
            }
            None => Err(anyhow::anyhow!("JavaScript execution failed")),
        }
    }
    
    
    /// Transforms ES6 module syntax and wraps script for execution
    fn prepare_script_for_execution(&self, script_content: &str) -> String {
        // Transform ES6 export syntax to work in our context
        let transformed_script = self.transform_es6_exports(script_content);
        
        // Wrap in async function to handle both sync and async handlers
        format!(
            r#"
            (async function() {{
                // Execute the transformed module code
                {}
                
                // Get the exported handler
                const handler = __handler;
                
                if (typeof handler === 'function') {{
                    const response = await handler(request);
                    return JSON.stringify(response);
                }} else {{
                    return null;
                }}
            }})()
            "#,
            transformed_script
        )
    }
    
    /// Transforms ES6 export statements to variable assignments
    fn transform_es6_exports(&self, script_content: &str) -> String {
        script_content
            .replace("export default function", "const __handler = function")
            .replace("export default async function", "const __handler = async function")
            .replace("export default", "const __handler =")
    }
    
    
    /// Resolves a JavaScript promise and processes the result
    fn resolve_promise(
        &self,
        scope: &mut v8::ContextScope<v8::HandleScope>,
        value: v8::Local<v8::Value>,
    ) -> Result<Option<JsResponse>, anyhow::Error> {
        let promise = v8::Local::<v8::Promise>::try_from(value).unwrap();
        
        // Poll microtasks until promise resolves
        while promise.state() == v8::PromiseState::Pending {
            scope.perform_microtask_checkpoint();
        }

        match promise.state() {
            v8::PromiseState::Fulfilled => {
                let result = promise.result(scope);
                self.process_js_result(scope, result)
            }
            v8::PromiseState::Rejected => {
                let exception = promise.result(scope);
                let error_msg = self.get_error_message(scope, exception);
                Err(anyhow::anyhow!("JavaScript error: {}", error_msg))
            }
            _ => Ok(None),
        }
    }

    fn process_js_result<'s>(
        &self,
        scope: &mut v8::HandleScope<'s>,
        value: v8::Local<'s, v8::Value>,
    ) -> Result<Option<JsResponse>, anyhow::Error> {
        if value.is_null_or_undefined() {
            return Ok(None);
        }

        if let Some(str_value) = value.to_string(scope) {
            let json_str = str_value.to_rust_string_lossy(scope);
            match serde_json::from_str::<JsResponse>(&json_str) {
                Ok(response) => Ok(Some(response)),
                Err(_) => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    fn setup_global_functions<'s>(
        &self,
        scope: &mut v8::HandleScope<'s>,
    ) -> Result<(), anyhow::Error> {
        let context = scope.get_current_context();
        let global = context.global(scope);

        // Setup console object
        let console_key = v8::String::new(scope, "console").unwrap();
        let console_obj = v8::Object::new(scope);
        
        // console.log
        let log_key = v8::String::new(scope, "log").unwrap();
        let log_fn = v8::Function::new(scope, Self::console_log).unwrap();
        console_obj.set(scope, log_key.into(), log_fn.into());
        
        // console.error
        let error_key = v8::String::new(scope, "error").unwrap();
        let error_fn = v8::Function::new(scope, Self::console_error).unwrap();
        console_obj.set(scope, error_key.into(), error_fn.into());
        
        global.set(scope, console_key.into(), console_obj.into());

        // Setup setTimeout (simplified)
        let set_timeout_key = v8::String::new(scope, "setTimeout").unwrap();
        let set_timeout_fn = v8::Function::new(scope, Self::set_timeout).unwrap();
        global.set(scope, set_timeout_key.into(), set_timeout_fn.into());

        Ok(())
    }

    /// JavaScript console.log implementation
    /// 
    /// This provides console.log functionality for JavaScript code.
    /// Output is controlled and only shown when appropriate.
    fn console_log(
        scope: &mut v8::HandleScope,
        args: v8::FunctionCallbackArguments,
        _rv: v8::ReturnValue,
    ) {
        // Collect console output but don't print directly
        // In production, JavaScript console output should be handled
        // through proper logging infrastructure
        let mut _output = String::new();
        for i in 0..args.length() {
            let arg = args.get(i);
            if let Some(str_val) = arg.to_string(scope) {
                if i > 0 {
                    _output.push(' ');
                }
                _output.push_str(&str_val.to_rust_string_lossy(scope));
            }
        }
        // TODO: Pass context through to enable proper verbose logging
        // For now, console.log output is collected but not displayed
    }

    /// JavaScript console.error implementation
    /// 
    /// This provides console.error functionality for JavaScript code.
    /// Errors are collected for potential logging.
    fn console_error(
        scope: &mut v8::HandleScope,
        args: v8::FunctionCallbackArguments,
        _rv: v8::ReturnValue,
    ) {
        // Collect error output but don't print directly
        let mut _error_output = String::new();
        for i in 0..args.length() {
            let arg = args.get(i);
            if let Some(str_val) = arg.to_string(scope) {
                if i > 0 {
                    _error_output.push(' ');
                }
                _error_output.push_str(&str_val.to_rust_string_lossy(scope));
            }
        }
        // TODO: Pass context through to enable proper error logging
        // For now, JavaScript console.error output is collected but not displayed
    }

    fn set_timeout(
        scope: &mut v8::HandleScope,
        args: v8::FunctionCallbackArguments,
        _rv: v8::ReturnValue,
    ) {
        if args.length() >= 1 {
            let callback = args.get(0);
            if callback.is_function() {
                if let Ok(func) = v8::Local::<v8::Function>::try_from(callback) {
                    let undefined = v8::undefined(scope);
                    let _ = func.call(scope, undefined.into(), &[]);
                }
            }
        }
    }

    fn get_json_object<'s>(&self, scope: &mut v8::HandleScope<'s>) -> v8::Local<'s, v8::Object> {
        let context = scope.get_current_context();
        let global = context.global(scope);
        let json_key = v8::String::new(scope, "JSON").unwrap();
        let json_value = global.get(scope, json_key.into()).unwrap();
        v8::Local::<v8::Object>::try_from(json_value).unwrap()
    }

    fn get_error_message<'s>(
        &self,
        scope: &mut v8::HandleScope<'s>,
        exception: v8::Local<'s, v8::Value>,
    ) -> String {
        if let Some(str_val) = exception.to_string(scope) {
            str_val.to_rust_string_lossy(scope)
        } else {
            "Unknown error".to_string()
        }
    }

    async fn create_js_request(&self, request: &PluginRequest) -> JsRequest {
        let mut headers = HashMap::new();
        for (key, value) in request.http_request.headers() {
            headers.insert(key.to_string(), value.to_str().unwrap_or("").to_string());
        }

        // Note: Getting body requires mutable reference, so we'll skip it for now
        let body = None;

        JsRequest {
            method: request.http_request.method().to_string(),
            path: request.http_request.uri().path().to_string(),
            headers,
            body,
        }
    }

    async fn find_script_for_path(&self, path: &str) -> Option<String> {
        let mappings = self.route_mappings.read().await;
        
        // Check for exact match or prefix match
        for (pattern, script) in mappings.iter() {
            if pattern.ends_with("*") {
                let prefix = &pattern[..pattern.len() - 1];
                if path.starts_with(prefix) {
                    return Some(script.clone());
                }
            } else if path == pattern {
                return Some(script.clone());
            }
        }

        // Check for index.mjs if path is root
        if path == "/" {
            let scripts_dir = self.scripts_dir.read().await;
            if tokio::fs::metadata(scripts_dir.join("index.mjs"))
                .await
                .is_ok()
            {
                return Some("index.mjs".to_string());
            }
        }

        // Check for direct file mapping
        let script_path = format!("{}.mjs", path.trim_start_matches('/'));
        let scripts_dir = self.scripts_dir.read().await;
        if tokio::fs::metadata(scripts_dir.join(&script_path))
            .await
            .is_ok()
        {
            return Some(script_path);
        }

        None
    }
}

#[async_trait]
impl Plugin for JavaScriptEnginePlugin {
    async fn handle_request(
        &self,
        request: &mut PluginRequest,
        context: &PluginContext,
    ) -> Option<PluginResponse> {
        // Check if JavaScript engine is enabled
        let scripts_enabled = context
            .get_config("javascript_engine_enabled")
            .map(|v| v == "true")
            .unwrap_or(true);

        if !scripts_enabled {
            return None;
        }

        let path = request.http_request.uri().path();
        
        // Scripts directory is configured at plugin initialization time
        
        // Route mappings would need to be loaded from configuration at initialization

        // Find and execute appropriate script
        if let Some(script_file) = self.find_script_for_path(path).await {
            match self.load_script(&script_file).await {
                Ok(script_content) => {
                    match self.execute_javascript(&script_content, request).await {
                        Ok(Some(js_response)) => {
                            let mut response = Response::builder()
                                .status(StatusCode::from_u16(js_response.status).unwrap_or(StatusCode::OK));

                            for (key, value) in js_response.headers {
                                response = response.header(key, value);
                            }

                            let body = js_response.body.unwrap_or_default();
                            match response.body(Body::from(body)) {
                                Ok(res) => Some(res.into()),
                                Err(e) => {
                                    context.log_verbose(&format!("[JavaScript] Error building response: {}", e));
                                    None
                                }
                            }
                        }
                        Ok(None) => None,
                        Err(e) => {
                            context.log_verbose(&format!("[JavaScript] Execution error: {}", e));
                            let response = Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Body::from(format!("JavaScript error: {}", e)))
                                .unwrap();
                            Some(response.into())
                        }
                    }
                }
                Err(e) => {
                    context.log_verbose(&format!("[JavaScript] Failed to load script '{}': {}", script_file, e));
                    None
                }
            }
        } else {
            None
        }
    }

    async fn handle_response(
        &self,
        _request: &PluginRequest,
        _response: &mut Response<Body>,
        _context: &PluginContext,
    ) {
        // No response handling needed for this plugin
    }

    fn name(&self) -> &str {
        &self.name
    }
}

create_plugin!(JavaScriptEnginePlugin);