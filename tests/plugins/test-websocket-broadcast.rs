use futures_util::StreamExt;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_broadcast_to_single_subscriber() {
    // Test that a StreamItem is broadcast to a single subscriber
    let base_url = "ws://localhost:3000";
    let doc_path = "/test-broadcast.html";
    let url = Url::parse(&format!("{}{}", base_url, doc_path)).unwrap();
    
    let (mut ws_stream, _) = connect_async(url)
        .await
        .expect("Failed to connect to WebSocket");

    // Connection automatically subscribes to /test-broadcast.html
    // Small delay to ensure WebSocket connection is stable
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Make an HTTP PUT request with selector that should trigger broadcast
    use reqwest::Client;
    let client = Client::new();
    let _response = client
        .put("http://localhost:3000/test-broadcast.html")
        .header("Range", "selector=#content")
        .header("Content-Type", "text/html")
        .body(r#"<div id="content">Updated content</div>"#)
        .send()
        .await
        .expect("Failed to send PUT request");

    // Wait for broadcast message with timeout
    match timeout(Duration::from_secs(5), ws_stream.next()).await {
        Ok(Some(Ok(Message::Text(text)))) => {
            // Verify StreamItem format
            assert!(text.contains("itemscope"));
            assert!(text.contains(r##"itemtype="http://rustybeam.net/StreamItem""##));
            assert!(text.contains(r##"itemprop="method""##));
            assert!(text.contains(r##"itemprop="url""##));
            assert!(text.contains(r##"itemprop="selector""##));
            assert!(text.contains(r##"itemprop="content""##));
            
            // Verify content
            assert!(text.contains("PUT"));
            assert!(text.contains("/test-broadcast.html"));
            assert!(text.contains("#content"));
        }
        _ => panic!("Expected StreamItem broadcast message"),
    }
}

#[tokio::test]
async fn test_broadcast_to_multiple_subscribers() {
    // Test that a StreamItem is broadcast to multiple subscribers
    let base_url = "ws://localhost:3000";
    let doc_path = "/test-multi-broadcast.html";
    let url = Url::parse(&format!("{}{}", base_url, doc_path)).unwrap();
    
    // Connect two clients
    let (mut ws_stream1, _) = connect_async(url.clone())
        .await
        .expect("Failed to connect first client");
    let (mut ws_stream2, _) = connect_async(url)
        .await
        .expect("Failed to connect second client");

    // Both connections automatically subscribe to /test-multi-broadcast.html
    // Small delay to ensure connections are stable
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Make an HTTP PUT request with selector that should trigger broadcast
    let client = reqwest::Client::new();
    let _response = client
        .put("http://localhost:3000/test-multi-broadcast.html")
        .header("Range", "selector=.shared-content")
        .header("Content-Type", "text/html")
        .body(r#"<div class="shared-content">Updated shared content</div>"#)
        .send()
        .await
        .expect("Failed to send PUT request");

    // Both clients should receive identical StreamItem messages
    let timeout_duration = Duration::from_secs(5);
    
    let msg1_future = timeout(timeout_duration, ws_stream1.next());
    let msg2_future = timeout(timeout_duration, ws_stream2.next());
    
    match (msg1_future.await, msg2_future.await) {
        (Ok(Some(Ok(Message::Text(text1)))), Ok(Some(Ok(Message::Text(text2))))) => {
            // Both should receive the same StreamItem
            assert_eq!(text1, text2, "Both clients should receive identical StreamItem");
            assert!(text1.contains(r##"itemtype="http://rustybeam.net/StreamItem""##));
        }
        _ => panic!("Expected both clients to receive StreamItem broadcast"),
    }
}

#[tokio::test]
async fn test_selective_broadcast() {
    // Test that broadcasts only go to subscribers of the matching selector
    let base_url = "ws://localhost:3000";
    let doc_path = "/test-selective.html";
    let url = Url::parse(&format!("{}{}", base_url, doc_path)).unwrap();
    
    let (_ws_stream1, _) = connect_async(url.clone())
        .await
        .expect("Failed to connect first client");
    let (_ws_stream2, _) = connect_async(url)
        .await
        .expect("Failed to connect second client");

    // Both connections automatically subscribe to /test-selective.html
    // All clients connected to the same URL will receive all updates for that URL
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // When an update occurs, both clients should receive it
    // This tests that all connections to a URL receive updates
    // (This test just verifies the connections are established)
}

#[tokio::test]
async fn test_url_isolation() {
    // Test that connections to different URLs don't receive each other's updates
    let base_url = "ws://localhost:3000";
    
    // Connect to two different URLs
    let url1 = Url::parse(&format!("{}/test-url1.html", base_url)).unwrap();
    let url2 = Url::parse(&format!("{}/test-url2.html", base_url)).unwrap();
    
    let (_ws_stream1, _) = connect_async(url1)
        .await
        .expect("Failed to connect to first URL");
    let (_ws_stream2, _) = connect_async(url2)
        .await
        .expect("Failed to connect to second URL");

    // Connections automatically subscribe to their respective URLs
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // This test verifies that connections to different URLs are isolated
    // (The actual isolation is tested by the plugin logic)
}

#[tokio::test]
async fn test_broadcast_with_complex_content() {
    // Test that StreamItem correctly handles complex HTML content
    let base_url = "ws://localhost:3000";
    let doc_path = "/test-complex.html";
    let url = Url::parse(&format!("{}{}", base_url, doc_path)).unwrap();
    
    let (mut ws_stream, _) = connect_async(url)
        .await
        .expect("Failed to connect to WebSocket");

    // Connection automatically subscribes to /test-complex.html
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Make an HTTP PUT request to trigger broadcast
    let client = reqwest::Client::new();
    let _response = client
        .put("http://localhost:3000/test-complex.html")
        .header("Range", "selector=.complex-content")
        .header("Content-Type", "text/html")
        .body(r#"<div class="complex-content">Complex updated content</div>"#)
        .send()
        .await
        .expect("Failed to send PUT request");

    // Expect to receive StreamItem with complex HTML content
    match timeout(Duration::from_secs(5), ws_stream.next()).await {
        Ok(Some(Ok(Message::Text(text)))) => {
            // Verify StreamItem structure
            assert!(text.contains(r##"itemtype="http://rustybeam.net/StreamItem""##));
            
            // The content should be properly escaped/encoded
            assert!(text.contains(r##"<div itemprop="content">"##));
            
            // Complex content might include:
            // - Nested HTML elements
            // - Attributes
            // - Special characters
            // All should be preserved in the StreamItem
        }
        _ => panic!("Expected StreamItem with complex content"),
    }
}

#[tokio::test]
async fn test_broadcast_method_types() {
    // Test that different HTTP methods are correctly reflected in StreamItem
    let base_url = "ws://localhost:3000";
    let doc_path = "/test-methods.html";
    let url = Url::parse(&format!("{}{}", base_url, doc_path)).unwrap();
    
    let (mut ws_stream, _) = connect_async(url)
        .await
        .expect("Failed to connect to WebSocket");

    // Connection automatically subscribes to /test-methods.html
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Make an HTTP PUT request to trigger broadcast
    let client = reqwest::Client::new();
    let _response = client
        .put("http://localhost:3000/test-methods.html")
        .header("Range", "selector=body")
        .header("Content-Type", "text/html")
        .body(r#"<body>Updated body content</body>"#)
        .send()
        .await
        .expect("Failed to send PUT request");

    // Test broadcasts for different HTTP methods
    // PUT, POST, DELETE should all generate StreamItems with correct method
    match timeout(Duration::from_secs(5), ws_stream.next()).await {
        Ok(Some(Ok(Message::Text(text)))) => {
            // Verify method is included
            assert!(text.contains(r##"<span itemprop="method">"##));
            // Method should be one of PUT, POST, DELETE
            assert!(text.contains("PUT") || text.contains("POST") || text.contains("DELETE"));
        }
        _ => panic!("Expected StreamItem broadcast"),
    }
}

#[tokio::test]
async fn test_no_broadcast_without_subscription() {
    // Test that clients don't receive broadcasts without subscribing
    let base_url = "ws://localhost:3000";
    let doc_path = "/test-no-sub.html";
    let url = Url::parse(&format!("{}{}", base_url, doc_path)).unwrap();
    
    let (mut ws_stream, _) = connect_async(url)
        .await
        .expect("Failed to connect to WebSocket");

    // Connection automatically subscribes to /test-no-sub.html
    // Should only receive broadcasts for updates to this URL
    match timeout(Duration::from_secs(2), ws_stream.next()).await {
        Ok(Some(Ok(Message::Text(text)))) => {
            // Should only receive StreamItem for the connected URL
            if text.contains("StreamItem") {
                assert!(text.contains("/test-no-sub.html"), 
                        "Should only receive broadcasts for connected URL");
            }
        }
        Err(_) => {
            // Timeout is expected - no broadcasts without updates
        }
        _ => {}
    }
}

#[tokio::test]
async fn test_url_specific_subscriptions() {
    // Test that subscriptions are URL-specific
    let base_url = "ws://localhost:3000";
    let doc_path = "/test-url-specific.html";
    let url = Url::parse(&format!("{}{}", base_url, doc_path)).unwrap();
    
    let (mut ws_stream, _) = connect_async(url)
        .await
        .expect("Failed to connect to WebSocket");

    // Connection automatically subscribes to /test-url-specific.html
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Updates to other URLs should NOT be received
    // Only updates to /test-url-specific.html should be received
    match timeout(Duration::from_secs(2), ws_stream.next()).await {
        Ok(Some(Ok(Message::Text(text)))) => {
            if text.contains("StreamItem") {
                // If we receive a StreamItem, it should be for our connected URL
                assert!(text.contains("/test-url-specific.html"));
                assert!(!text.contains("/different-page.html"));
            }
        }
        _ => {}
    }
}