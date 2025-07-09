use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

#[tokio::test]
async fn test_websocket_upgrade() {
    // Test that the WebSocket plugin correctly handles the upgrade request
    let url = Url::parse("ws://localhost:3000/test.html").unwrap();
    
    match connect_async(url).await {
        Ok((_ws_stream, response)) => {
            assert_eq!(response.status(), 101, "Expected WebSocket upgrade status");
            assert_eq!(
                response.headers().get("upgrade").unwrap(),
                "websocket",
                "Expected upgrade header"
            );
        }
        Err(e) => panic!("WebSocket connection failed: {}", e),
    }
}

#[tokio::test]
async fn test_websocket_subscription() {
    // Test subscribing to document updates
    let url = Url::parse("ws://localhost:3000/test.html").unwrap();
    let (mut ws_stream, _) = connect_async(url)
        .await
        .expect("Failed to connect to WebSocket");

    // Send subscription message (following das-ws.js protocol)
    let subscribe_msg = Message::Text("{\"action\": \"subscribe\", \"selector\": \"body\"}".to_string());
    ws_stream
        .send(subscribe_msg)
        .await
        .expect("Failed to send subscription");

    // Should receive acknowledgment
    if let Some(Ok(msg)) = ws_stream.next().await {
        match msg {
            Message::Text(text) => {
                assert!(text.contains("subscribed"), "Expected subscription acknowledgment");
            }
            _ => panic!("Expected text message"),
        }
    }
}

#[tokio::test]
async fn test_stream_item_format() {
    // Test that StreamItem messages follow the correct format
    let url = Url::parse("ws://localhost:3000/test.html").unwrap();
    let (mut ws_stream, _) = connect_async(url)
        .await
        .expect("Failed to connect to WebSocket");

    // Subscribe to updates
    let subscribe_msg = Message::Text("{\"action\": \"subscribe\", \"selector\": \"#content\"}".to_string());
    ws_stream.send(subscribe_msg).await.unwrap();
    
    // Skip acknowledgment
    ws_stream.next().await;

    // Trigger an update (this would be done through selector-handler in real scenario)
    // For now, we'll test that the message format is correct when received
    
    // Simulate receiving a StreamItem
    let _expected_stream_item = r#"<div itemscope itemtype="http://rustybeam.net/StreamItem">
        <span itemprop="method">PUT</span>
        <span itemprop="url">/test.html</span>
        <span itemprop="selector">#content</span>
        <div itemprop="content">
            <p>Updated content</p>
        </div>
    </div>"#;
    
    // In real test, we'd trigger an update and verify the format
    // For now, this test documents the expected format
}

#[tokio::test]
async fn test_multiple_clients() {
    // Test that multiple clients can subscribe to the same document
    let url = Url::parse("ws://localhost:3000/test.html").unwrap();
    
    // Connect two clients
    let (mut ws_stream1, _) = connect_async(url.clone())
        .await
        .expect("Failed to connect first client");
    let (mut ws_stream2, _) = connect_async(url)
        .await
        .expect("Failed to connect second client");

    // Both subscribe to same selector
    let subscribe_msg = Message::Text("{\"action\": \"subscribe\", \"selector\": \".updates\"}".to_string());
    ws_stream1.send(subscribe_msg.clone()).await.unwrap();
    ws_stream2.send(subscribe_msg).await.unwrap();

    // Both should receive acknowledgments
    ws_stream1.next().await;
    ws_stream2.next().await;

    // When an update occurs, both should receive it
    // (In real scenario, this would be triggered by selector-handler)
}

#[tokio::test]
async fn test_unsubscribe() {
    // Test unsubscribing from updates
    let url = Url::parse("ws://localhost:3000/test.html").unwrap();
    let (mut ws_stream, _) = connect_async(url)
        .await
        .expect("Failed to connect to WebSocket");

    // Subscribe first
    let subscribe_msg = Message::Text("{\"action\": \"subscribe\", \"selector\": \"#content\"}".to_string());
    ws_stream.send(subscribe_msg).await.unwrap();
    ws_stream.next().await; // Skip acknowledgment

    // Unsubscribe
    let unsubscribe_msg = Message::Text("{\"action\": \"unsubscribe\", \"selector\": \"#content\"}".to_string());
    ws_stream.send(unsubscribe_msg).await.unwrap();

    // Should receive unsubscribe acknowledgment
    if let Some(Ok(Message::Text(text))) = ws_stream.next().await {
        assert!(text.contains("unsubscribed"), "Expected unsubscribe acknowledgment");
    }
}

#[tokio::test]
async fn test_authorization() {
    // Test that WebSocket respects authorization rules
    let url = Url::parse("ws://localhost:3000/protected.html").unwrap();
    
    // Try to connect without authorization
    match connect_async(url).await {
        Ok(_) => panic!("Expected connection to fail without authorization"),
        Err(e) => {
            // Should fail with 401 or 403
            assert!(e.to_string().contains("401") || e.to_string().contains("403"),
                    "Expected authorization error");
        }
    }
}

#[tokio::test]
async fn test_selector_handler_integration() {
    // Test that updates via selector-handler are broadcast to WebSocket clients
    let url = Url::parse("ws://localhost:3000/test.html").unwrap();
    let (mut ws_stream, _) = connect_async(url)
        .await
        .expect("Failed to connect to WebSocket");

    // Subscribe to a specific selector
    let subscribe_msg = Message::Text("{\"action\": \"subscribe\", \"selector\": \"#dynamic-content\"}".to_string());
    ws_stream.send(subscribe_msg).await.unwrap();
    ws_stream.next().await; // Skip acknowledgment

    // In a real scenario, another HTTP client would make a request with:
    // Range: selector=#dynamic-content
    // This would trigger an update that should be broadcast to our WebSocket client

    // The WebSocket client should receive a StreamItem with the update
    // For now, this test documents the expected integration
}

#[tokio::test]
async fn test_reconnection() {
    // Test that clients can reconnect and resume subscriptions
    let url = Url::parse("ws://localhost:3000/test.html").unwrap();
    
    // First connection
    let (mut ws_stream, _) = connect_async(url.clone())
        .await
        .expect("Failed to connect");
    
    // Subscribe
    let subscribe_msg = Message::Text("{\"action\": \"subscribe\", \"selector\": \"body\"}".to_string());
    ws_stream.send(subscribe_msg.clone()).await.unwrap();
    ws_stream.next().await;
    
    // Close connection
    ws_stream.close(None).await.unwrap();
    
    // Reconnect
    let (mut ws_stream2, _) = connect_async(url)
        .await
        .expect("Failed to reconnect");
    
    // Should be able to subscribe again
    ws_stream2.send(subscribe_msg).await.unwrap();
    if let Some(Ok(Message::Text(text))) = ws_stream2.next().await {
        assert!(text.contains("subscribed"), "Expected subscription acknowledgment after reconnection");
    }
}

#[tokio::test]
async fn test_invalid_messages() {
    // Test that invalid messages are handled gracefully
    let url = Url::parse("ws://localhost:3000/test.html").unwrap();
    let (mut ws_stream, _) = connect_async(url)
        .await
        .expect("Failed to connect to WebSocket");

    // Send invalid JSON
    let invalid_msg = Message::Text("not valid json".to_string());
    ws_stream.send(invalid_msg).await.unwrap();

    // Should receive error response
    if let Some(Ok(Message::Text(text))) = ws_stream.next().await {
        assert!(text.contains("error"), "Expected error response for invalid message");
    }

    // Send message with missing required fields
    let incomplete_msg = Message::Text("{\"action\": \"subscribe\"}".to_string());
    ws_stream.send(incomplete_msg).await.unwrap();

    // Should receive error response
    if let Some(Ok(Message::Text(text))) = ws_stream.next().await {
        assert!(text.contains("error"), "Expected error response for incomplete message");
    }
}

#[tokio::test]
async fn test_binary_messages() {
    // Test that binary messages are rejected (das-ws.js uses text only)
    let url = Url::parse("ws://localhost:3000/test.html").unwrap();
    let (mut ws_stream, _) = connect_async(url)
        .await
        .expect("Failed to connect to WebSocket");

    // Send binary message
    let binary_msg = Message::Binary(vec![1, 2, 3, 4]);
    ws_stream.send(binary_msg).await.unwrap();

    // Should receive error or close connection
    if let Some(Ok(msg)) = ws_stream.next().await {
        match msg {
            Message::Text(text) => assert!(text.contains("error"), "Expected error for binary message"),
            Message::Close(_) => {}, // Also acceptable
            _ => panic!("Unexpected message type"),
        }
    }
}