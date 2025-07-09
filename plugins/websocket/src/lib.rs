use async_trait::async_trait;
use dashmap::DashMap;
use hyper::{Body, Response, StatusCode};
use rusty_beam_plugin_api::{create_plugin, Plugin, PluginContext, PluginRequest, PluginResponse, UpgradeHandler};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
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

#[derive(Debug, Clone)]
pub struct WebSocketPlugin {
    connections: Arc<DashMap<String, ConnectionState>>,
    // Future use for global subscription management
    #[allow(dead_code)]
    subscriptions: Arc<RwLock<HashMap<String, Vec<String>>>>,
    // Future use for global broadcasting
    #[allow(dead_code)]
    broadcast_tx: broadcast::Sender<BroadcastMessage>,
}

#[derive(Debug)]
struct ConnectionState {
    #[allow(dead_code)]
    id: String,
    url: String,  // The document URL this connection is subscribed to
    tx: broadcast::Sender<WsMessage>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct BroadcastMessage {
    url: String,
    selector: String,
    content: String,
}

impl Default for WebSocketPlugin {
    fn default() -> Self {
        let (broadcast_tx, _) = broadcast::channel(1024);
        Self {
            connections: Arc::new(DashMap::new()),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
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
            normalized.push_str("index.html");
        }
        // If URL is a directory path (no extension), append '/index.html'
        else if !normalized.contains('.') && !normalized.contains('?') && !normalized.contains('#') {
            if !normalized.ends_with('/') {
                normalized.push('/');
            }
            normalized.push_str("index.html");
        }
        
        normalized
    }

    async fn handle_websocket_upgrade(&self, request: &PluginRequest) -> Option<PluginResponse> {
        // Check for WebSocket upgrade headers
        let headers = request.http_request.headers();
        
        let connection = headers.get("connection")?.to_str().ok()?;
        let upgrade = headers.get("upgrade")?.to_str().ok()?;
        let ws_key = headers.get("sec-websocket-key")?.to_str().ok()?;
        let ws_version = headers.get("sec-websocket-version")?.to_str().ok()?;

        // Validate WebSocket headers
        if !connection.to_lowercase().contains("upgrade") 
            || upgrade.to_lowercase() != "websocket"
            || ws_version != "13" {
            return None;
        }

        // Generate accept key
        let accept_key = derive_accept_key(ws_key.as_bytes());

        // Create response for WebSocket handshake
        let response = Response::builder()
            .status(StatusCode::SWITCHING_PROTOCOLS)
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Accept", accept_key)
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
        println!("WebSocket connection established: {} for {}", connection_id, normalized_url);
        
        // Create a channel for this connection
        let (tx, mut rx) = broadcast::channel::<WsMessage>(256);
        
        // Store connection info - automatically subscribed to the normalized URL
        let connection = ConnectionState {
            id: connection_id.clone(),
            url: normalized_url,
            tx: tx.clone(),
        };
        
        self.connections.insert(connection_id.clone(), connection);
        
        // Handle both incoming and outgoing messages
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
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
        }
        
        // Clean up
        self.connections.remove(&connection_id);
        println!("WebSocket connection cleaned up: {}", connection_id);
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
                println!("WebSocket connection closed: {}", connection_id);
                return false;
            }
            Ok(WsMessage::Ping(data)) => {
                let _ = ws_stream.send(WsMessage::Pong(data)).await;
            }
            Err(e) => {
                eprintln!("WebSocket error for {}: {}", connection_id, e);
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
                    r#"<div itemscope itemtype="http://rustybeam.net/StreamItem">
    <span itemprop="method">{}</span>
    <span itemprop="url">{}</span>
    <span itemprop="selector">{}</span>
    <div itemprop="content">{}</div>
</div>"#,
                    method, url, selector, content
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
        "WebSocket Plugin"
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
            if matches!(method.as_str(), "PUT" | "POST" | "DELETE") {
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