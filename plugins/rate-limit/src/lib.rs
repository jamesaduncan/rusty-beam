//! Rate Limit Plugin for Rusty Beam
//!
//! This plugin provides token bucket-based rate limiting to protect against abuse
//! and ensure fair usage of server resources. It supports multiple rate limiting
//! strategies and provides detailed feedback to clients.
//!
//! ## Features
//! - **Token Bucket Algorithm**: Smooth rate limiting with burst capacity support
//! - **Multiple Key Strategies**: Rate limit by IP address, authenticated user, or host
//! - **Configurable Limits**: Customizable requests per second and burst capacity
//! - **Automatic Cleanup**: Removes inactive rate limit buckets to prevent memory leaks
//! - **Standard Headers**: Adds X-RateLimit-* headers for client awareness
//! - **Retry-After Support**: Provides precise timing for when clients can retry
//!
//! ## Configuration
//! - `requests_per_second`: Base rate limit (default: 10)
//! - `burst_capacity`: Maximum burst size (default: 2x requests_per_second)
//! - `key_strategy`: "ip", "user", or "host" (default: "ip")
//! - `cleanup_interval`: How often to clean old buckets (default: 300 seconds)
//!
//! ## Rate Limiting Keys
//! - **IP Strategy**: Uses client IP address (supports X-Forwarded-For)
//! - **User Strategy**: Uses authenticated user ID, falls back to IP
//! - **Host Strategy**: Uses Host header for domain-based limiting
//!
//! ## HTTP Headers
//! - **X-RateLimit-Limit**: Maximum requests allowed
//! - **X-RateLimit-Remaining**: Requests remaining in current window
//! - **X-RateLimit-Reset**: Time until limit resets
//! - **Retry-After**: Seconds to wait before retrying (when rate limited)

use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, PluginResponse, create_plugin};
use async_trait::async_trait;
use hyper::{Body, Response, StatusCode};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Token bucket for rate limiting
#[derive(Debug, Clone)]
pub struct TokenBucket {
    pub tokens: f64,
    pub capacity: f64,
    pub refill_rate: f64,
    pub last_refill: Instant,
}

impl TokenBucket {
    pub fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            tokens: capacity,
            capacity,
            refill_rate,
            last_refill: Instant::now(),
        }
    }
    
    /// Refill tokens based on elapsed time
    pub fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        
        // Add tokens based on elapsed time and refill rate
        let tokens_to_add = elapsed * self.refill_rate;
        self.tokens = (self.tokens + tokens_to_add).min(self.capacity);
        self.last_refill = now;
    }
    
    /// Try to consume tokens
    pub fn consume(&mut self, tokens: f64) -> bool {
        self.refill();
        
        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }
    
    /// Get time until next token is available
    pub fn time_until_available(&mut self, tokens: f64) -> Option<Duration> {
        self.refill();
        
        if self.tokens >= tokens {
            None
        } else {
            let tokens_needed = tokens - self.tokens;
            let time_needed = tokens_needed / self.refill_rate;
            Some(Duration::from_secs_f64(time_needed))
        }
    }
}

/// Plugin for token bucket rate limiting
#[derive(Debug)]
pub struct RateLimitPlugin {
    name: String,
    buckets: Arc<Mutex<HashMap<String, TokenBucket>>>,
    requests_per_second: f64,
    burst_capacity: f64,
    key_strategy: String,
    cleanup_interval: Duration,
    last_cleanup: Arc<Mutex<Instant>>,
}

impl RateLimitPlugin {
    pub fn new(config: HashMap<String, String>) -> Self {
        let name = config.get("name").cloned().unwrap_or_else(|| "rate-limit".to_string());
        
        let requests_per_second = config.get("requests_per_second")
            .and_then(|v| v.parse().ok())
            .unwrap_or(10.0);
        
        let burst_capacity = config.get("burst_capacity")
            .and_then(|v| v.parse().ok())
            .unwrap_or(requests_per_second * 2.0);
        
        let key_strategy = config.get("key_strategy")
            .cloned()
            .unwrap_or_else(|| "ip".to_string());
        
        let cleanup_interval = config.get("cleanup_interval")
            .and_then(|v| v.parse().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(300)); // 5 minutes
        
        Self {
            name,
            buckets: Arc::new(Mutex::new(HashMap::new())),
            requests_per_second,
            burst_capacity,
            key_strategy,
            cleanup_interval,
            last_cleanup: Arc::new(Mutex::new(Instant::now())),
        }
    }
    
    /// Extract rate limiting key from request
    fn extract_key(&self, request: &PluginRequest) -> String {
        match self.key_strategy.as_str() {
            "ip" => {
                self.extract_client_ip(request).to_string()
            }
            "user" => {
                // Use authenticated user if available, otherwise fallback to IP
                request.metadata.get("authenticated_user")
                    .cloned()
                    .unwrap_or_else(|| self.extract_client_ip(request).to_string())
            }
            "host" => {
                // Use Host header
                request.http_request.headers()
                    .get("host")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("unknown")
                    .to_string()
            }
            _ => {
                // Default to IP
                self.extract_client_ip(request).to_string()
            }
        }
    }
    
    /// Extract client IP address from request using proxy-aware headers
    fn extract_client_ip(&self, request: &PluginRequest) -> IpAddr {
        // Try X-Forwarded-For header first (most common proxy header)
        if let Some(ip) = self.extract_ip_from_forwarded_for(request) {
            return ip;
        }
        
        // Try X-Real-IP header (alternative proxy header)
        if let Some(ip) = self.extract_ip_from_real_ip_header(request) {
            return ip;
        }
        
        // Fallback to localhost (connection remote address not available in plugin context)
        self.get_fallback_ip()
    }
    
    /// Extract IP from X-Forwarded-For header (supports comma-separated list)
    fn extract_ip_from_forwarded_for(&self, request: &PluginRequest) -> Option<IpAddr> {
        let xff_header = request.http_request.headers().get("x-forwarded-for")?;
        let xff_str = xff_header.to_str().ok()?;
        let first_ip = xff_str.split(',').next()?.trim();
        first_ip.parse::<IpAddr>().ok()
    }
    
    /// Extract IP from X-Real-IP header
    fn extract_ip_from_real_ip_header(&self, request: &PluginRequest) -> Option<IpAddr> {
        let real_ip_header = request.http_request.headers().get("x-real-ip")?;
        let ip_str = real_ip_header.to_str().ok()?;
        ip_str.parse::<IpAddr>().ok()
    }
    
    /// Get fallback IP address when no proxy headers are available
    fn get_fallback_ip(&self) -> IpAddr {
        // Use localhost as fallback since connection remote address
        // is not available in the plugin context
        "127.0.0.1".parse().expect("Hardcoded localhost IP should always parse")
    }
    
    /// Clean up old buckets
    fn cleanup_old_buckets(&self) {
        let mut last_cleanup = self.last_cleanup.lock().unwrap();
        let now = Instant::now();
        
        if now.duration_since(*last_cleanup) > self.cleanup_interval {
            let mut buckets = self.buckets.lock().unwrap();
            let cutoff = now - Duration::from_secs(3600); // Remove buckets older than 1 hour
            
            buckets.retain(|_, bucket| bucket.last_refill > cutoff);
            *last_cleanup = now;
        }
    }
    
    /// Check if request should be rate limited
    fn check_rate_limit(&self, key: &str) -> (bool, Option<Duration>) {
        self.cleanup_old_buckets();
        
        let mut buckets = self.buckets.lock().unwrap();
        let bucket = buckets.entry(key.to_string())
            .or_insert_with(|| TokenBucket::new(self.burst_capacity, self.requests_per_second));
        
        if bucket.consume(1.0) {
            (false, None) // Request allowed
        } else {
            let retry_after = bucket.time_until_available(1.0);
            (true, retry_after) // Request blocked
        }
    }
    
    /// Create rate limit exceeded response with proper error handling
    fn create_rate_limit_response(&self, retry_after: Option<Duration>) -> Response<Body> {
        let retry_seconds = retry_after.map(|d| d.as_secs()).unwrap_or(60);
        
        let mut response_builder = Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header("Content-Type", "application/json");
        
        if let Some(duration) = retry_after {
            response_builder = response_builder.header("Retry-After", duration.as_secs().to_string());
        }
        
        let body = self.create_rate_limit_error_body(retry_seconds);
        
        response_builder.body(Body::from(body))
            .unwrap_or_else(|_| {
                // Fallback response if JSON body creation fails
                Response::builder()
                    .status(StatusCode::TOO_MANY_REQUESTS)
                    .header("Content-Type", "text/plain")
                    .body(Body::from("Rate limit exceeded. Please try again later."))
                    .unwrap()
            })
    }
    
    /// Create JSON error body for rate limit response
    fn create_rate_limit_error_body(&self, retry_seconds: u64) -> String {
        let body = serde_json::json!({
            "error": "Rate limit exceeded",
            "message": "Too many requests. Please try again later.",
            "retry_after_seconds": retry_seconds
        });
        
        body.to_string()
    }
    
    /// Add standard rate limit headers to the response
    fn add_rate_limit_headers_to_response(&self, response: &mut Response<Body>, key: &str) {
        if let Ok(buckets) = self.buckets.lock() {
            if let Some(bucket) = buckets.get(key) {
                let headers = response.headers_mut();
                
                // X-RateLimit-Limit: Maximum requests allowed
                if let Ok(limit_header) = (self.burst_capacity as u64).to_string().parse() {
                    headers.insert("X-RateLimit-Limit", limit_header);
                }
                
                // X-RateLimit-Remaining: Requests remaining in current window
                let remaining = bucket.tokens.floor().max(0.0) as u64;
                if let Ok(remaining_header) = remaining.to_string().parse() {
                    headers.insert("X-RateLimit-Remaining", remaining_header);
                }
                
                // X-RateLimit-Reset: Time until limit resets (approximate)
                let reset_time = self.calculate_reset_time(bucket);
                if let Ok(reset_header) = reset_time.to_string().parse() {
                    headers.insert("X-RateLimit-Reset", reset_header);
                }
            }
        }
    }
    
    /// Calculate approximate time until rate limit resets
    fn calculate_reset_time(&self, bucket: &TokenBucket) -> u64 {
        let elapsed_since_refill = bucket.last_refill.elapsed().as_secs();
        let time_to_full_refill = (self.burst_capacity / self.requests_per_second) as u64;
        
        // Conservative estimate: current time + time to fully refill bucket
        elapsed_since_refill + time_to_full_refill
    }
}

#[async_trait]
impl Plugin for RateLimitPlugin {
    async fn handle_request(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<PluginResponse> {
        let key = self.extract_key(request);
        let (is_limited, retry_after) = self.check_rate_limit(&key);
        
        if is_limited {
            context.log_verbose(&format!("[RateLimit] Request blocked for key: {} (retry after: {:?})", key, retry_after));
            Some(self.create_rate_limit_response(retry_after).into())
        } else {
            // Add rate limit info to metadata
            request.metadata.insert("rate_limit_key".to_string(), key);
            None
        }
    }
    
    async fn handle_response(&self, request: &PluginRequest, response: &mut Response<Body>, _context: &PluginContext) {
        // Add rate limit headers to response if rate limiting was applied
        if let Some(key) = request.metadata.get("rate_limit_key") {
            self.add_rate_limit_headers_to_response(response, key);
        }
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

// Export the plugin creation function
create_plugin!(RateLimitPlugin);