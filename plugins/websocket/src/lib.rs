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

// Constants
const DEFAULT_PLUGIN_NAME: &str = "WebSocket Plugin";
const INDEX_FILE_NAME: &str = "index.html";
const WEBSOCKET_VERSION: &str = "13";
const WEBSOCKET_PROTOCOL: &str = "websocket";
const UPGRADE_HEADER: &str = "upgrade";
const CONNECTION_HEADER: &str = "connection";
const UPGRADE_VALUE: &str = "Upgrade";
const STREAM_ITEM_SCHEMA: &str = "http://rustybeam.net/StreamItem";

// Buffer sizes
const CONNECTION_CHANNEL_SIZE: usize = 256;

// WebSocket timing
const PING_INTERVAL_SECONDS: u64 = 30;
const PONG_TIMEOUT_SECONDS: u64 = 10;

// HTTP methods that trigger broadcasts
const BROADCAST_METHODS: &[&str] = &["PUT", "POST", "DELETE"];

// WebSocket headers
const WS_CONNECTION_HEADER: &str = "Connection";
const WS_UPGRADE_HEADER: &str = "Upgrade";
const WS_VERSION_HEADER: &str = "Sec-WebSocket-Version";
const WS_KEY_HEADER: &str = "Sec-WebSocket-Key";
const WS_ACCEPT_HEADER: &str = "Sec-WebSocket-Accept";

#[derive(Debug, Clone)]
pub struct WebSocketPlugin {
    connections: Arc<DashMap<String, ConnectionState>>,
}

#[derive(Debug)]
struct ConnectionState {
    url: String,  // The document URL this connection is subscribed to
    tx: broadcast::Sender<WsMessage>,
    last_pong: std::time::Instant,  // Track last pong received for health monitoring
}


impl Default for WebSocketPlugin {
    fn default() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
        }
    }
}

impl WebSocketPlugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        // Future configuration options could include:
        // - max_connections: Maximum concurrent WebSocket connections
        // - ping_interval: Interval for sending ping frames
        // - message_size_limit: Maximum message size
        // - broadcast_buffer_size: Size of broadcast channel buffer
        
        // For now, use defaults
        let _ = config; // Suppress unused warning
        Self::default()
    }
    
    /// Normalize URL paths to treat directory paths and their index.html as the same
    fn normalize_url(url: &str) -> String {
        let mut normalized = url.to_string();
        
        // If URL ends with '/', append 'index.html'
        if normalized.ends_with('/') {
            normalized.push_str(INDEX_FILE_NAME);
        }
        // If URL is a directory path (no extension), append '/index.html'
        else if !normalized.contains('.') && !normalized.contains('?') && !normalized.contains('#') {
            if !normalized.ends_with('/') {
                normalized.push('/');
            }
            normalized.push_str(INDEX_FILE_NAME);
        }
        
        normalized
    }

    async fn handle_websocket_upgrade(&self, request: &PluginRequest) -> Option<PluginResponse> {
        // Check for WebSocket upgrade headers
        let headers = request.http_request.headers();
        
        let connection = headers.get(CONNECTION_HEADER)?.to_str().ok()?;
        let upgrade = headers.get(UPGRADE_HEADER)?.to_str().ok()?;
        let ws_key = headers.get(WS_KEY_HEADER)?.to_str().ok()?;
        let ws_version = headers.get(WS_VERSION_HEADER)?.to_str().ok()?;

        // Validate WebSocket headers
        if !connection.to_lowercase().contains(UPGRADE_HEADER.to_lowercase().as_str()) 
            || upgrade.to_lowercase() != WEBSOCKET_PROTOCOL
            || ws_version != WEBSOCKET_VERSION {
            return None;
        }

        // Generate accept key
        let accept_key = derive_accept_key(ws_key.as_bytes());

        // Create response for WebSocket handshake
        let response = Response::builder()
            .status(StatusCode::SWITCHING_PROTOCOLS)
            .header(WS_CONNECTION_HEADER, UPGRADE_VALUE)
            .header(WS_UPGRADE_HEADER, WEBSOCKET_PROTOCOL)
            .header(WS_ACCEPT_HEADER, accept_key)
            .body(Body::empty())
            .unwrap();

        let connection_id = Uuid::new_v4().to_string();
        let url = request.http_request.uri().to_string();
        
        // Clone self for the upgrade handler
        let plugin = self.clone();
        
        // Create the upgrade handler
        let upgrade_handler: UpgradeHandler = Box::new(move |upgraded| {
            let connection_id = connection_id.clone();
            let url = url.clone();
            
            Box::pin(async move {
                // Create WebSocket from the upgraded connection
                let ws_stream = WebSocketStream::from_raw_socket(
                    upgraded,
                    Role::Server,
                    None,
                ).await;
                
                // Handle the WebSocket connection
                plugin.handle_websocket_connection(connection_id, url, ws_stream).await;
                
                Ok(())
            })
        });

        Some(PluginResponse {
            response,
            upgrade: Some(upgrade_handler),
        })
    }

    async fn handle_websocket_connection(
        &self, 
        connection_id: String, 
        url: String,
        mut ws_stream: WebSocketStream<hyper::upgrade::Upgraded>
    ) {
        // Normalize the URL for consistent matching
        let normalized_url = Self::normalize_url(&url);
        println!("[WebSocket] Connection established: {} for {}", connection_id, normalized_url);
        
        // Create a channel for this connection
        let (tx, mut rx) = broadcast::channel::<WsMessage>(CONNECTION_CHANNEL_SIZE);
        
        // Store connection info - automatically subscribed to the normalized URL
        let connection = ConnectionState {
            url: normalized_url,
            tx: tx.clone(),
            last_pong: std::time::Instant::now(),
        };
        
        self.connections.insert(connection_id.clone(), connection);
        
        // Create ping interval timer
        let mut ping_interval = tokio::time::interval(tokio::time::Duration::from_secs(PING_INTERVAL_SECONDS));
        ping_interval.tick().await; // Skip first immediate tick
        
        // Handle incoming, outgoing messages, and keep-alive pings
        loop {
            tokio::select! {
                // Handle incoming WebSocket messages
                msg = ws_stream.next() => {
                    match msg {
                        Some(msg) => {
                            if !self.handle_websocket_message(&connection_id, &mut ws_stream, msg).await {
                                break;
                            }
                        }
                        None => break,
                    }
                }
                // Handle outgoing broadcast messages
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
                // Send periodic ping to keep connection alive
                _ = ping_interval.tick() => {
                    let ping_data = format!("ping-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()).into_bytes();
                    if ws_stream.send(WsMessage::Ping(ping_data)).await.is_err() {
                        println!("[WebSocket] Failed to send ping to {}, closing connection", connection_id);
                        break;
                    }
                }
            }
        }
        
        // Clean up
        self.connections.remove(&connection_id);
        println!("[WebSocket] Connection cleaned up: {}", connection_id);
    }

    async fn handle_websocket_message(
        &self,
        connection_id: &str,
        ws_stream: &mut WebSocketStream<hyper::upgrade::Upgraded>,
        msg: Result<WsMessage, tokio_tungstenite::tungstenite::Error>
    ) -> bool {
        match msg {
            Ok(WsMessage::Text(_text)) => {
                // Connections are automatically subscribed to their URL
                // No client messages needed
            }
            Ok(WsMessage::Close(_)) => {
                println!("[WebSocket] Connection closed: {}", connection_id);
                return false;
            }
            Ok(WsMessage::Ping(data)) => {
                if ws_stream.send(WsMessage::Pong(data)).await.is_err() {
                    eprintln!("[WebSocket] Failed to send pong to {}", connection_id);
                    return false;
                }
            }
            Ok(WsMessage::Pong(_)) => {
                // Update last pong time for this connection
                if let Some(mut conn) = self.connections.get_mut(connection_id) {
                    conn.last_pong = std::time::Instant::now();
                }
            }
            Err(e) => {
                eprintln!("[WebSocket] Error for {}: {}", connection_id, e);
                return false;
            }
            _ => {}
        }
        true
    }

    
    async fn broadcast_update(&self, url: &str, selector: &str, content: &str, method: &str) {
        // Normalize the URL for consistent matching
        let normalized_url = Self::normalize_url(url);
        
        // Send to all connections subscribed to this normalized URL
        for connection in self.connections.iter() {
            if connection.url == normalized_url {
                // Format as StreamItem - use the original URL in the broadcast
                let stream_item = format!(
                    r#"<div itemscope itemtype="{}">
    <span itemprop="method">{}</span>
    <span itemprop="url">{}</span>
    <span itemprop="selector">{}</span>
    <div itemprop="content">{}</div>
</div>"#,
                    STREAM_ITEM_SCHEMA, method, url, selector, content
                );
                
                let ws_msg = WsMessage::Text(stream_item);
                let _ = connection.tx.send(ws_msg);
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
        _context: &PluginContext,
    ) -> Option<PluginResponse> {
        // Check if this is a WebSocket upgrade request
        if let Some(response) = self.handle_websocket_upgrade(request).await {
            return Some(response);
        }

        // Pass through non-WebSocket requests
        None
    }

    async fn handle_response(
        &self,
        request: &PluginRequest,
        _response: &mut Response<Body>,
        _context: &PluginContext,
    ) {
        // Check if this response is from selector-handler
        // Look for metadata that indicates a selector was applied
        if let Some(applied_selector) = request.get_metadata("applied_selector") {
            // Get the HTTP method from the request
            let method = request.http_request.method().to_string();
            
            // Only broadcast for mutation methods
            if BROADCAST_METHODS.contains(&method.as_str()) {
                // Get the URL path
                let url = request.path.clone();
                
                // Extract the response body content
                // Note: We can't directly read the response body here as it's already consumed
                // Instead, we'll rely on metadata set by selector-handler
                // For POST/PUT operations, broadcast the posted content, not the target element
                let content_to_broadcast = if let Some(posted_content) = request.get_metadata("posted_content") {
                    // Broadcast the actual content that was posted/put
                    posted_content
                } else if let Some(selected_content) = request.get_metadata("selected_content") {
                    // Fallback to selected content (for operations like DELETE)
                    selected_content
                } else {
                    return; // No content to broadcast
                };
                
                // Broadcast the update to all subscribers
                self.broadcast_update(&url, applied_selector, &content_to_broadcast, &method).await;
            }
        }
    }
}

// Export the plugin
create_plugin!(WebSocketPlugin);