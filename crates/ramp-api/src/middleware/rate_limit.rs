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
    config: RateLimitConfig,
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
                result.reset_after_seconds.to_string().parse().unwrap_or(
                    axum::http::HeaderValue::from_static("60")
                ),
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
            result.reset_after_seconds.to_string().parse().unwrap_or(
                axum::http::HeaderValue::from_static("60")
            ),
        );
        response
            .headers_mut()
            .insert("X-RateLimit-Limit", limit.to_string().parse().unwrap_or(
                axum::http::HeaderValue::from_static("0")
            ));
        response
            .headers_mut()
            .insert("X-RateLimit-Remaining", "0".parse().unwrap_or(
                axum::http::HeaderValue::from_static("0")
            ));
        response.headers_mut().insert(
            "X-RateLimit-Reset",
            result.reset_after_seconds.to_string().parse().unwrap_or(
                axum::http::HeaderValue::from_static("0")
            ),
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
    response
        .headers_mut()
        .insert("X-RateLimit-Limit", limit.to_string().parse().unwrap_or(
            axum::http::HeaderValue::from_static("0")
        ));
    response.headers_mut().insert(
        "X-RateLimit-Remaining",
        result.remaining.to_string().parse().unwrap_or(
            axum::http::HeaderValue::from_static("0")
        ),
    );
    response.headers_mut().insert(
        "X-RateLimit-Reset",
        result.reset_after_seconds.to_string().parse().unwrap_or(
            axum::http::HeaderValue::from_static("0")
        ),
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
}
