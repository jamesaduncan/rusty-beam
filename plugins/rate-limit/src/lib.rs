use rusty_beam_plugin_api::{Plugin, PluginRequest, PluginContext, create_plugin};
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
    
    /// Extract client IP address from request
    fn extract_client_ip(&self, request: &PluginRequest) -> IpAddr {
        // Check X-Forwarded-For header first (for proxy scenarios)
        if let Some(xff) = request.http_request.headers().get("x-forwarded-for") {
            if let Ok(xff_str) = xff.to_str() {
                if let Some(ip_str) = xff_str.split(',').next() {
                    if let Ok(ip) = ip_str.trim().parse::<IpAddr>() {
                        return ip;
                    }
                }
            }
        }
        
        // Check X-Real-IP header
        if let Some(real_ip) = request.http_request.headers().get("x-real-ip") {
            if let Ok(ip_str) = real_ip.to_str() {
                if let Ok(ip) = ip_str.parse::<IpAddr>() {
                    return ip;
                }
            }
        }
        
        // Fallback to connection remote address (may not be available in all setups)
        // For now, use a default IP
        "127.0.0.1".parse().unwrap()
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
    
    /// Create rate limit exceeded response
    fn create_rate_limit_response(&self, retry_after: Option<Duration>) -> Response<Body> {
        let mut response = Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header("Content-Type", "application/json");
        
        if let Some(duration) = retry_after {
            response = response.header("Retry-After", duration.as_secs().to_string());
        }
        
        let body = serde_json::json!({
            "error": "Rate limit exceeded",
            "message": "Too many requests. Please try again later.",
            "retry_after_seconds": retry_after.map(|d| d.as_secs()).unwrap_or(60)
        });
        
        response.body(Body::from(body.to_string())).unwrap()
    }
}

#[async_trait]
impl Plugin for RateLimitPlugin {
    async fn handle_request(&self, request: &mut PluginRequest, context: &PluginContext) -> Option<Response<Body>> {
        let key = self.extract_key(request);
        let (is_limited, retry_after) = self.check_rate_limit(&key);
        
        if is_limited {
            context.log_verbose(&format!("[RateLimit] Request blocked for key: {} (retry after: {:?})", key, retry_after));
            Some(self.create_rate_limit_response(retry_after))
        } else {
            // Add rate limit info to metadata
            request.metadata.insert("rate_limit_key".to_string(), key);
            None
        }
    }
    
    async fn handle_response(&self, request: &PluginRequest, response: &mut Response<Body>, _context: &PluginContext) {
        // Add rate limit headers to response
        if let Some(key) = request.metadata.get("rate_limit_key") {
            let buckets = self.buckets.lock().unwrap();
            if let Some(bucket) = buckets.get(key) {
                let remaining = bucket.tokens.floor() as u64;
                let limit = self.burst_capacity as u64;
                
                response.headers_mut().insert("X-RateLimit-Limit", limit.to_string().parse().unwrap());
                response.headers_mut().insert("X-RateLimit-Remaining", remaining.to_string().parse().unwrap());
                response.headers_mut().insert("X-RateLimit-Reset", (bucket.last_refill.elapsed().as_secs() + 60).to_string().parse().unwrap());
            }
        }
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

// Export the plugin creation function
create_plugin!(RateLimitPlugin);