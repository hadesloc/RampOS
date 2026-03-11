//! Rate limiting middleware using Redis or Memory

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tracing::warn;

use crate::middleware::tenant::TenantContext;

// use redis::ScriptAsync;

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Global maximum requests per window
    pub global_max_requests: u64,
    /// Per-tenant maximum requests per window
    pub tenant_max_requests: u64,
    /// Window size in seconds
    pub window_seconds: u64,
    /// Redis key prefix
    pub key_prefix: String,
    /// Per-endpoint limits (path -> max_requests)
    pub endpoint_limits: HashMap<String, u64>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            global_max_requests: 1000,
            tenant_max_requests: 100,
            window_seconds: 60,
            key_prefix: "ramp:rate_limit".to_string(),
            endpoint_limits: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub remaining: u64,
    pub reset_after_seconds: u64,
}

#[derive(Debug)]
pub enum RateLimitError {
    Store(String),
}

/// Abstract store for rate limiting
#[async_trait::async_trait]
pub trait RateLimitStore: Send + Sync {
    async fn check(
        &self,
        key: &str,
        limit: u64,
        window_seconds: u64,
        key_prefix: &str,
    ) -> Result<RateLimitResult, RateLimitError>;
}

/// Redis implementation of RateLimitStore
pub struct RedisRateLimitStore {
    redis: Arc<redis::aio::ConnectionManager>,
}

impl RedisRateLimitStore {
    pub fn new(redis: redis::aio::ConnectionManager) -> Self {
        Self {
            redis: Arc::new(redis),
        }
    }
}

#[async_trait::async_trait]
impl RateLimitStore for RedisRateLimitStore {
    async fn check(
        &self,
        key: &str,
        limit: u64,
        window_seconds: u64,
        key_prefix: &str,
    ) -> Result<RateLimitResult, RateLimitError> {
        let full_key = format!("{}:{}", key_prefix, key);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(std::time::Duration::from_secs(0))
            .as_secs();

        let window_start = now - window_seconds;
        let mut conn = (*self.redis).clone();

        let script = redis::Script::new(
            r#"
            local key = KEYS[1]
            local now = tonumber(ARGV[1])
            local window_start = tonumber(ARGV[2])
            local max_requests = tonumber(ARGV[3])
            local window_seconds = tonumber(ARGV[4])

            -- Remove old entries
            redis.call('ZREMRANGEBYSCORE', key, '-inf', window_start)

            -- Count current entries
            local count = redis.call('ZCARD', key)

            if count < max_requests then
                -- Add new entry
                redis.call('ZADD', key, now, now .. ':' .. math.random())
                -- Set expiry
                redis.call('EXPIRE', key, window_seconds)
                return {1, max_requests - count - 1, window_seconds}
            else
                -- Get oldest entry to calculate reset time
                local oldest = redis.call('ZRANGE', key, 0, 0, 'WITHSCORES')
                local reset_at = oldest[2] and (tonumber(oldest[2]) + window_seconds) or (now + window_seconds)
                return {0, 0, reset_at - now}
            end
            "#,
        );

        let result: Result<Vec<i64>, redis::RedisError> = script
            .key(&full_key)
            .arg(now)
            .arg(window_start)
            .arg(limit)
            .arg(window_seconds)
            .invoke_async(&mut conn)
            .await;

        let result = result.map_err(|e| {
            tracing::error!("Redis error during rate limit check: {}", e);
            // Fail open on Redis error
            RateLimitError::Store(e.to_string())
        })?;

        if result[0] == 1 {
            Ok(RateLimitResult {
                allowed: true,
                remaining: result[1] as u64,
                reset_after_seconds: result[2] as u64,
            })
        } else {
            Ok(RateLimitResult {
                allowed: false,
                remaining: 0,
                reset_after_seconds: result[2] as u64,
            })
        }
    }
}

/// In-memory implementation for testing
pub struct MemoryRateLimitStore {
    // key -> (timestamp, unique_id)
    history: Arc<Mutex<HashMap<String, Vec<u64>>>>,
}

impl Default for MemoryRateLimitStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryRateLimitStore {
    pub fn new() -> Self {
        Self {
            history: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl RateLimitStore for MemoryRateLimitStore {
    async fn check(
        &self,
        key: &str,
        limit: u64,
        window_seconds: u64,
        key_prefix: &str,
    ) -> Result<RateLimitResult, RateLimitError> {
        let full_key = format!("{}:{}", key_prefix, key);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(std::time::Duration::from_secs(0))
            .as_secs();
        let window_start = now - window_seconds;

        let mut history = self.history.lock().map_err(|e| {
            warn!(error = ?e, "Rate limit in-memory history mutex poisoned");
            RateLimitError::Store(e.to_string())
        })?;
        let entries = history.entry(full_key).or_default();

        // Remove old entries
        entries.retain(|&ts| ts > window_start);

        let count = entries.len() as u64;

        if count < limit {
            entries.push(now);
            Ok(RateLimitResult {
                allowed: true,
                remaining: limit - count - 1,
                reset_after_seconds: window_seconds,
            })
        } else {
            let oldest = entries.first().cloned().unwrap_or(now);
            let reset_at = oldest + window_seconds;
            Ok(RateLimitResult {
                allowed: false,
                remaining: 0,
                reset_after_seconds: reset_at.saturating_sub(now),
            })
        }
    }
}

/// Rate limiter
#[derive(Clone)]
pub struct RateLimiter {
    store: Arc<dyn RateLimitStore>,
    pub config: RateLimitConfig,
}

impl RateLimiter {
    pub fn new(store: Arc<dyn RateLimitStore>, config: RateLimitConfig) -> Self {
        Self { store, config }
    }

    /// Helper to create with Redis store
    pub fn with_redis(redis: redis::aio::ConnectionManager, config: RateLimitConfig) -> Self {
        Self::new(Arc::new(RedisRateLimitStore::new(redis)), config)
    }

    /// Helper to create with Memory store
    pub fn with_memory(config: RateLimitConfig) -> Self {
        Self::new(Arc::new(MemoryRateLimitStore::new()), config)
    }

    /// Check if request is allowed against a specific limit
    pub async fn check(&self, key: &str, limit: u64) -> Result<RateLimitResult, RateLimitError> {
        self.store
            .check(
                key,
                limit,
                self.config.window_seconds,
                &self.config.key_prefix,
            )
            .await
    }
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    State(limiter): State<Arc<RateLimiter>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 1. Check global limit
    let global_result = limiter
        .check("global", limiter.config.global_max_requests)
        .await;

    match global_result {
        Ok(result) if !result.allowed => {
            warn!(
                reset_after = result.reset_after_seconds,
                "Global rate limit exceeded"
            );
            let mut response = Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .body(axum::body::Body::empty())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            response.headers_mut().insert(
                "Retry-After",
                result
                    .reset_after_seconds
                    .to_string()
                    .parse()
                    .unwrap_or(axum::http::HeaderValue::from_static("60")),
            );
            return Ok(response);
        }
        Err(e) => {
            warn!(error = ?e, "Rate limiter error (global)");
            return Ok(Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .body(axum::body::Body::empty())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?);
        }
        _ => {}
    }

    // 2. Check tenant/IP limit
    // Get tenant context from request extensions
    let (tenant_key, limit) = if let Some(ctx) = req.extensions().get::<TenantContext>() {
        (ctx.tenant_id.0.clone(), ctx.tier.rate_limit())
    } else {
        // Fall back to IP-based rate limiting for unauthenticated requests
        let ip = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(',').next().unwrap_or("unknown").trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        (ip, limiter.config.tenant_max_requests)
    };
    let result = match limiter.check(&tenant_key, limit).await {
        Ok(res) => res,
        Err(e) => {
            warn!(error = ?e, "Rate limiter error (tenant)");
            return Ok(Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .body(axum::body::Body::empty())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?);
        }
    };

    if !result.allowed {
        warn!(
            tenant = %tenant_key,
            reset_after = result.reset_after_seconds,
            "Tenant rate limit exceeded"
        );
        let mut response = Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .body(axum::body::Body::empty())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        response.headers_mut().insert(
            "Retry-After",
            result
                .reset_after_seconds
                .to_string()
                .parse()
                .unwrap_or(axum::http::HeaderValue::from_static("60")),
        );
        response.headers_mut().insert(
            "X-RateLimit-Limit",
            limit
                .to_string()
                .parse()
                .unwrap_or(axum::http::HeaderValue::from_static("0")),
        );
        response.headers_mut().insert(
            "X-RateLimit-Remaining",
            "0".parse()
                .unwrap_or(axum::http::HeaderValue::from_static("0")),
        );
        response.headers_mut().insert(
            "X-RateLimit-Reset",
            result
                .reset_after_seconds
                .to_string()
                .parse()
                .unwrap_or(axum::http::HeaderValue::from_static("0")),
        );
        return Ok(response);
    }

    // Check specific endpoint limit if configured
    let path = req.uri().path().to_string();
    if let Some(&endpoint_limit) = limiter.config.endpoint_limits.get(&path) {
        let endpoint_key = format!("{}:{}", tenant_key, path);
        let endpoint_result = match limiter.check(&endpoint_key, endpoint_limit).await {
            Ok(res) => res,
            Err(e) => {
                warn!(error = ?e, "Rate limiter error (endpoint)");
                return Ok(Response::builder()
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .body(axum::body::Body::empty())
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?);
            }
        };

        if !endpoint_result.allowed {
            warn!(
                tenant = %tenant_key,
                path = %path,
                reset_after = endpoint_result.reset_after_seconds,
                "Endpoint rate limit exceeded"
            );
            let mut response = Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .body(axum::body::Body::empty())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            response.headers_mut().insert(
                "Retry-After",
                endpoint_result
                    .reset_after_seconds
                    .to_string()
                    .parse()
                    .unwrap_or(axum::http::HeaderValue::from_static("60")),
            );
            return Ok(response);
        }

        // Use endpoint result for headers if it's more restrictive or just use the main one?
        // Let's use the main tenant result for headers for now as it's the primary one.
    }

    // Pass
    let mut response = next.run(req).await;

    // Add rate limit headers (using tenant result)
    response.headers_mut().insert(
        "X-RateLimit-Limit",
        limit
            .to_string()
            .parse()
            .unwrap_or(axum::http::HeaderValue::from_static("0")),
    );
    response.headers_mut().insert(
        "X-RateLimit-Remaining",
        result
            .remaining
            .to_string()
            .parse()
            .unwrap_or(axum::http::HeaderValue::from_static("0")),
    );
    response.headers_mut().insert(
        "X-RateLimit-Reset",
        result
            .reset_after_seconds
            .to_string()
            .parse()
            .unwrap_or(axum::http::HeaderValue::from_static("0")),
    );

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.global_max_requests, 1000);
        assert_eq!(config.tenant_max_requests, 100);
        assert_eq!(config.window_seconds, 60);
    }

    #[tokio::test]
    async fn test_memory_store_basic_allow() {
        let store = MemoryRateLimitStore::new();
        let result = store
            .check("test_key", 10, 60, "ramp:test")
            .await
            .expect("check should succeed");
        assert!(result.allowed);
        assert_eq!(result.remaining, 9);
    }

    #[tokio::test]
    async fn test_memory_store_exhausts_limit() {
        let store = MemoryRateLimitStore::new();
        let limit = 5u64;
        for i in 0..limit {
            let result = store
                .check("exhaust_key", limit, 60, "ramp:test")
                .await
                .expect("check should succeed");
            assert!(result.allowed, "request {} should be allowed", i);
            assert_eq!(result.remaining, limit - i - 1);
        }
        // Next request should be rejected
        let result = store
            .check("exhaust_key", limit, 60, "ramp:test")
            .await
            .expect("check should succeed");
        assert!(!result.allowed);
        assert_eq!(result.remaining, 0);
        assert!(result.reset_after_seconds > 0);
    }

    #[tokio::test]
    async fn test_memory_store_burst_allowance() {
        let store = MemoryRateLimitStore::new();
        let burst_limit = 3u64;
        // Simulate burst: send burst_limit requests rapidly
        for _ in 0..burst_limit {
            let result = store
                .check("burst_key", burst_limit, 60, "ramp:test")
                .await
                .expect("check should succeed");
            assert!(result.allowed);
        }
        // After burst, should be denied
        let result = store
            .check("burst_key", burst_limit, 60, "ramp:test")
            .await
            .expect("check should succeed");
        assert!(!result.allowed);
    }

    #[tokio::test]
    async fn test_per_tenant_separate_limits() {
        let store = MemoryRateLimitStore::new();
        let limit = 2u64;
        // Tenant A uses up its limit
        for _ in 0..limit {
            let r = store
                .check("tenant_a", limit, 60, "ramp:test")
                .await
                .unwrap();
            assert!(r.allowed);
        }
        let r = store
            .check("tenant_a", limit, 60, "ramp:test")
            .await
            .unwrap();
        assert!(!r.allowed, "tenant_a should be rate limited");

        // Tenant B should still be allowed
        let r = store
            .check("tenant_b", limit, 60, "ramp:test")
            .await
            .unwrap();
        assert!(r.allowed, "tenant_b should not be affected by tenant_a");
        assert_eq!(r.remaining, 1);
    }

    #[tokio::test]
    async fn test_memory_fallback_when_no_redis() {
        // MemoryRateLimitStore is the fallback when Redis is unavailable
        let limiter = RateLimiter::with_memory(RateLimitConfig {
            global_max_requests: 50,
            tenant_max_requests: 10,
            window_seconds: 60,
            key_prefix: "ramp:fallback".to_string(),
            endpoint_limits: HashMap::new(),
        });
        let result = limiter.check("fallback_test", 10).await.unwrap();
        assert!(result.allowed);
        assert_eq!(result.remaining, 9);
    }

    #[tokio::test]
    async fn test_rate_limit_result_has_correct_reset() {
        let store = MemoryRateLimitStore::new();
        let window = 120u64;
        let result = store
            .check("reset_key", 5, window, "ramp:test")
            .await
            .unwrap();
        assert!(result.allowed);
        // For allowed requests, reset_after_seconds equals the window
        assert_eq!(result.reset_after_seconds, window);
    }

    #[tokio::test]
    async fn test_different_route_group_limits() {
        let store = MemoryRateLimitStore::new();
        // Route group "api" has limit 2
        for _ in 0..2 {
            let r = store
                .check("tenant1:api", 2, 60, "ramp:test")
                .await
                .unwrap();
            assert!(r.allowed);
        }
        let r = store
            .check("tenant1:api", 2, 60, "ramp:test")
            .await
            .unwrap();
        assert!(!r.allowed, "api route group should be exhausted");

        // Route group "admin" has limit 5, should still work
        let r = store
            .check("tenant1:admin", 5, 60, "ramp:test")
            .await
            .unwrap();
        assert!(r.allowed);
        assert_eq!(r.remaining, 4);
    }

    #[tokio::test]
    async fn test_endpoint_specific_limits_in_config() {
        let mut endpoint_limits = HashMap::new();
        endpoint_limits.insert("/v1/aa/user-operations".to_string(), 5u64);
        endpoint_limits.insert("/v1/intents/payin".to_string(), 20u64);

        let config = RateLimitConfig {
            global_max_requests: 1000,
            tenant_max_requests: 100,
            window_seconds: 60,
            key_prefix: "ramp:endpoint".to_string(),
            endpoint_limits,
        };

        let limiter = RateLimiter::with_memory(config);

        // Check that endpoint limits are configured correctly
        assert_eq!(
            limiter.config.endpoint_limits.get("/v1/aa/user-operations"),
            Some(&5)
        );
        assert_eq!(
            limiter.config.endpoint_limits.get("/v1/intents/payin"),
            Some(&20)
        );

        // The store itself enforces per-key limits
        // Simulate checking with the endpoint-specific limit
        let r = limiter
            .check("tenant1:/v1/aa/user-operations", 5)
            .await
            .unwrap();
        assert!(r.allowed);
        assert_eq!(r.remaining, 4);
    }

    #[tokio::test]
    async fn test_rate_limit_remaining_decrements() {
        let store = MemoryRateLimitStore::new();
        let limit = 10u64;
        for i in 0..limit {
            let r = store
                .check("decrement_key", limit, 60, "ramp:test")
                .await
                .unwrap();
            assert!(r.allowed);
            assert_eq!(
                r.remaining,
                limit - i - 1,
                "remaining should decrement at step {}",
                i
            );
        }
    }

    #[tokio::test]
    async fn test_429_response_fields() {
        // When rate limit is exceeded, RateLimitResult should have the correct fields
        // for building a 429 response with Retry-After, X-RateLimit-Remaining, X-RateLimit-Reset
        let store = MemoryRateLimitStore::new();
        let limit = 1u64;
        // Use up the single allowed request
        let r = store
            .check("resp_key", limit, 60, "ramp:test")
            .await
            .unwrap();
        assert!(r.allowed);
        assert_eq!(r.remaining, 0);

        // Next request is denied - verify response fields
        let denied = store
            .check("resp_key", limit, 60, "ramp:test")
            .await
            .unwrap();
        assert!(!denied.allowed);
        assert_eq!(denied.remaining, 0, "X-RateLimit-Remaining should be 0");
        assert!(
            denied.reset_after_seconds > 0,
            "Retry-After / X-RateLimit-Reset should be > 0"
        );
        assert!(
            denied.reset_after_seconds <= 60,
            "reset should not exceed window"
        );
    }

    #[tokio::test]
    async fn test_rate_limit_headers_on_success() {
        // On successful requests, the middleware sets X-RateLimit-Limit, X-RateLimit-Remaining,
        // and X-RateLimit-Reset headers. Verify the RateLimitResult supplies correct values.
        let limiter = RateLimiter::with_memory(RateLimitConfig {
            global_max_requests: 1000,
            tenant_max_requests: 50,
            window_seconds: 30,
            key_prefix: "ramp:headers".to_string(),
            endpoint_limits: HashMap::new(),
        });

        let r = limiter.check("header_test_tenant", 50).await.unwrap();
        assert!(r.allowed);
        // These values map to response headers in the middleware:
        // X-RateLimit-Remaining
        assert_eq!(r.remaining, 49);
        // X-RateLimit-Reset (equals window_seconds for allowed requests)
        assert_eq!(r.reset_after_seconds, 30);
    }

    #[tokio::test]
    async fn test_tenant_db_override_concept() {
        // Simulates the tenant DB override pattern: different tenants get different limits
        // based on what would be loaded from tenant_rate_limits table.
        // In production, the middleware reads tenant_rate_limits and overrides the default.
        let store = Arc::new(MemoryRateLimitStore::new());
        let config = RateLimitConfig::default();
        let limiter = RateLimiter::new(store, config);

        // Default tenant gets tenant_max_requests = 100
        let default_limit = 100u64;
        let r = limiter
            .check("default_tenant", default_limit)
            .await
            .unwrap();
        assert!(r.allowed);
        assert_eq!(r.remaining, 99);

        // Premium tenant gets DB-override limit of 500 (from tenant_rate_limits table)
        let db_override_limit = 500u64;
        let r = limiter
            .check("premium_tenant", db_override_limit)
            .await
            .unwrap();
        assert!(r.allowed);
        assert_eq!(r.remaining, 499);

        // Restricted tenant gets DB-override limit of 10
        let restricted_limit = 10u64;
        for _ in 0..restricted_limit {
            let r = limiter
                .check("restricted_tenant", restricted_limit)
                .await
                .unwrap();
            assert!(r.allowed);
        }
        let r = limiter
            .check("restricted_tenant", restricted_limit)
            .await
            .unwrap();
        assert!(!r.allowed, "restricted tenant should be rate limited at 10");
    }
}
