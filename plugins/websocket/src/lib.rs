//! WebSocket Plugin for Rusty Beam
//!
//! This plugin enables real-time bidirectional communication through WebSocket connections.
//! It automatically broadcasts HTML updates to all connected clients when content changes
//! through PUT, POST, or DELETE operations.
//!
//! ## Features
//! - Automatic WebSocket upgrade handling
//! - URL-based subscription (clients automatically subscribe to the document they connect to)
//! - Real-time broadcasting of content updates
//! - Connection health monitoring with ping/pong support
//! - Efficient connection management using DashMap

use async_trait::async_trait;
use dashmap::DashMap;
use hyper::{Body, Response, StatusCode};
use rusty_beam_plugin_api::{create_plugin, Plugin, PluginContext, PluginRequest, PluginResponse, UpgradeHandler};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_tungstenite::{
    tungstenite::{
        handshake::derive_accept_key,
        Message as WsMessage,
        protocol::Role,
    },
    WebSocketStream,
};
use uuid::Uuid;
use futures_util::{SinkExt, StreamExt};

// Plugin configuration
const DEFAULT_PLUGIN_NAME: &str = "WebSocket Plugin";
const INDEX_FILE_NAME: &str = "index.html";
const CONNECTION_CHANNEL_SIZE: usize = 256;

// WebSocket keep-alive configuration
// Note: Server-side ping disabled due to runtime context constraints
// Clients should implement their own keep-alive mechanism

// WebSocket protocol constants
const WEBSOCKET_VERSION: &str = "13";
const WEBSOCKET_PROTOCOL: &str = "websocket";
const UPGRADE_VALUE: &str = "Upgrade";

// HTTP header names
const CONNECTION_HEADER: &str = "connection";
const UPGRADE_HEADER: &str = "upgrade";
const WS_CONNECTION_HEADER: &str = "Connection";
const WS_UPGRADE_HEADER: &str = "Upgrade";
const WS_VERSION_HEADER: &str = "Sec-WebSocket-Version";
const WS_KEY_HEADER: &str = "Sec-WebSocket-Key";
const WS_ACCEPT_HEADER: &str = "Sec-WebSocket-Accept";

// StreamItem schema for broadcast messages
const STREAM_ITEM_SCHEMA: &str = "http://rustybeam.net/StreamItem";

// HTTP methods that trigger broadcasts to WebSocket clients
const BROADCAST_METHODS: &[&str] = &["PUT", "POST", "DELETE"];

/// WebSocket plugin that manages real-time connections and broadcasts updates
#[derive(Debug, Clone)]
pub struct WebSocketPlugin {
    /// Thread-safe map of active WebSocket connections
    connections: Arc<DashMap<String, ConnectionState>>,
}

/// State information for each WebSocket connection
#[derive(Debug)]
struct ConnectionState {
    /// The normalized document URL this connection is subscribed to
    url: String,
    /// Channel sender for broadcasting messages to this connection
    tx: broadcast::Sender<WsMessage>,
}


impl Default for WebSocketPlugin {
    fn default() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
        }
    }
}

impl WebSocketPlugin {
    /// Creates a new WebSocket plugin with the given configuration
    pub fn new(_config: HashMap<String, String>) -> Self {
        // Configuration options reserved for future use:
        // - max_connections: Limit concurrent WebSocket connections
        // - ping_interval: Interval between keep-alive pings
        // - message_size_limit: Maximum WebSocket message size
        // - broadcast_buffer_size: Channel buffer size for broadcasts
        
        Self::default()
    }
    
    /// Normalizes URL paths for consistent subscription matching
    /// 
    /// This ensures that `/path/`, `/path`, and `/path/index.html` all
    /// refer to the same document for WebSocket subscription purposes.
    fn normalize_url(url: &str) -> String {
        let mut normalized = url.to_string();
        
        if normalized.ends_with('/') {
            // Directory with trailing slash: append index.html
            normalized.push_str(INDEX_FILE_NAME);
        } else if !normalized.contains('.') && !normalized.contains('?') && !normalized.contains('#') {
            // Directory without trailing slash: append /index.html
            normalized.push('/');
            normalized.push_str(INDEX_FILE_NAME);
        }
        // URLs with extensions are left unchanged
        
        normalized
    }

    /// Handles WebSocket upgrade requests according to RFC 6455
    async fn handle_websocket_upgrade(
        &self, 
        request: &PluginRequest, 
        _context: &PluginContext
    ) -> Option<PluginResponse> {
        let headers = request.http_request.headers();
        
        // Extract required WebSocket headers
        let connection = headers.get(CONNECTION_HEADER)?.to_str().ok()?;
        let upgrade = headers.get(UPGRADE_HEADER)?.to_str().ok()?;
        let ws_key = headers.get(WS_KEY_HEADER)?.to_str().ok()?;
        let ws_version = headers.get(WS_VERSION_HEADER)?.to_str().ok()?;

        // Validate WebSocket upgrade requirements
        if !connection.to_lowercase().contains(UPGRADE_HEADER) 
            || upgrade.to_lowercase() != WEBSOCKET_PROTOCOL
            || ws_version != WEBSOCKET_VERSION {
            return None;
        }

        // Generate WebSocket accept key per RFC 6455
        let accept_key = derive_accept_key(ws_key.as_bytes());

        // Build the 101 Switching Protocols response
        let response = Response::builder()
            .status(StatusCode::SWITCHING_PROTOCOLS)
            .header(WS_CONNECTION_HEADER, UPGRADE_VALUE)
            .header(WS_UPGRADE_HEADER, WEBSOCKET_PROTOCOL)
            .header(WS_ACCEPT_HEADER, accept_key)
            .body(Body::empty())
            .ok()?;

        let connection_id = Uuid::new_v4().to_string();
        let url = request.http_request.uri().to_string();
        
        // Create upgrade handler to manage the WebSocket connection
        let plugin = self.clone();
        let upgrade_handler: UpgradeHandler = Box::new(move |upgraded| {
            let connection_id = connection_id.clone();
            let url = url.clone();
            
            Box::pin(async move {
                // Convert the upgraded HTTP connection to a WebSocket stream
                let ws_stream = WebSocketStream::from_raw_socket(
                    upgraded,
                    Role::Server,
                    None,
                ).await;
                
                // Handle the WebSocket lifecycle
                plugin.handle_websocket_connection(connection_id, url, ws_stream).await;
                
                Ok(())
            })
        });

        Some(PluginResponse {
            response,
            upgrade: Some(upgrade_handler),
        })
    }

    /// Manages the lifecycle of a WebSocket connection
    async fn handle_websocket_connection(
        &self, 
        connection_id: String, 
        url: String,
        mut ws_stream: WebSocketStream<hyper::upgrade::Upgraded>
    ) {
        let normalized_url = Self::normalize_url(&url);
        println!("[WebSocket] Connection established: {} for {}", connection_id, normalized_url);
        
        // Create broadcast channel for sending messages to this connection
        let (tx, mut rx) = broadcast::channel::<WsMessage>(CONNECTION_CHANNEL_SIZE);
        
        // Register the connection with automatic URL subscription
        let connection = ConnectionState {
            url: normalized_url,
            tx: tx.clone(),
        };
        
        self.connections.insert(connection_id.clone(), connection);
        
        // Message handling loop
        // Note: Ping mechanism requires careful runtime context handling in plugin environment
        // Currently disabled to ensure stability - clients should implement their own keep-alive
        loop {
            tokio::select! {
                // Process incoming WebSocket messages
                msg = ws_stream.next() => {
                    match msg {
                        Some(msg) => {
                            if !self.handle_websocket_message(&connection_id, &mut ws_stream, msg).await {
                                break;
                            }
                        }
                        None => break, // Connection closed by client
                    }
                }
                
                // Forward broadcast messages to this connection
                msg = rx.recv() => {
                    match msg {
                        Ok(msg) => {
                            if ws_stream.send(msg).await.is_err() {
                                println!("[WebSocket] Failed to send message to {}, closing connection", connection_id);
                                break;
                            }
                        }
                        Err(_) => {
                            println!("[WebSocket] Broadcast channel closed for {}", connection_id);
                            break;
                        }
                    }
                }
            }
        }
        
        // Connection cleanup
        self.connections.remove(&connection_id);
        println!("[WebSocket] Connection cleaned up: {}", connection_id);
    }

    /// Processes individual WebSocket messages
    /// 
    /// Returns true if the connection should continue, false if it should close
    async fn handle_websocket_message(
        &self,
        connection_id: &str,
        ws_stream: &mut WebSocketStream<hyper::upgrade::Upgraded>,
        msg: Result<WsMessage, tokio_tungstenite::tungstenite::Error>
    ) -> bool {
        match msg {
            Ok(WsMessage::Text(_text)) => {
                // Text messages from clients are currently ignored
                // Clients are automatically subscribed to their connection URL
            }
            Ok(WsMessage::Close(_)) => {
                println!("[WebSocket] Connection closed: {}", connection_id);
                return false;
            }
            Ok(WsMessage::Ping(data)) => {
                // Respond to ping with pong
                if ws_stream.send(WsMessage::Pong(data)).await.is_err() {
                    eprintln!("[WebSocket] Failed to send pong to {}", connection_id);
                    return false;
                }
            }
            Ok(WsMessage::Pong(_)) => {
                // Pong received - connection is healthy
                // Future: implement timeout monitoring
            }
            Err(e) => {
                eprintln!("[WebSocket] Error for {}: {}", connection_id, e);
                return false;
            }
            _ => {} // Binary messages ignored
        }
        true
    }

    
    /// Broadcasts content updates to all connections subscribed to a URL
    async fn broadcast_update(&self, url: &str, selector: &str, content: &str, method: &str) {
        let normalized_url = Self::normalize_url(url);
        
        // Find all connections subscribed to this URL
        for connection in self.connections.iter() {
            if connection.url == normalized_url {
                // Format update as StreamItem microdata
                let stream_item = format!(
                    r#"<div itemscope itemtype="{}">
    <span itemprop="method">{}</span>
    <span itemprop="url">{}</span>
    <span itemprop="selector">{}</span>
    <div itemprop="content">{}</div>
</div>"#,
                    STREAM_ITEM_SCHEMA, method, url, selector, content
                );
                
                // Send to connection (ignoring send errors for disconnected clients)
                let _ = connection.tx.send(WsMessage::Text(stream_item));
            }
        }
    }
}

#[async_trait]
impl Plugin for WebSocketPlugin {
    fn name(&self) -> &str {
        DEFAULT_PLUGIN_NAME
    }
    
    async fn handle_request(
        &self,
        request: &mut PluginRequest,
        context: &PluginContext,
    ) -> Option<PluginResponse> {
        // Only handle WebSocket upgrade requests
        self.handle_websocket_upgrade(request, context).await
    }

    async fn handle_response(
        &self,
        request: &PluginRequest,
        _response: &mut Response<Body>,
        _context: &PluginContext,
    ) {
        // Intercept responses from selector-handler to broadcast updates
        if let Some(applied_selector) = request.get_metadata("applied_selector") {
            let method = request.http_request.method().to_string();
            
            // Only broadcast mutations (PUT, POST, DELETE)
            if BROADCAST_METHODS.contains(&method.as_str()) {
                let url = request.path.clone();
                
                // Determine what content to broadcast
                let content_to_broadcast = if let Some(posted_content) = request.get_metadata("posted_content") {
                    // For POST/PUT: broadcast the new content
                    posted_content
                } else if let Some(selected_content) = request.get_metadata("selected_content") {
                    // For DELETE: broadcast the removed content
                    selected_content
                } else {
                    return; // No content to broadcast
                };
                
                // Send update to all WebSocket clients watching this URL
                self.broadcast_update(&url, applied_selector, &content_to_broadcast, &method).await;
            }
        }
    }
}

// Export the plugin
create_plugin!(WebSocketPlugin);