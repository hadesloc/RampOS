use crate::middleware::{rate_limit_middleware, RateLimitConfig, RateLimitError, RateLimitResult};
use crate::middleware::{RateLimitStore, RateLimiter};
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::from_fn_with_state,
    routing::get,
    Router,
};
use std::sync::{Arc, Mutex};
use tokio::time::Duration;
use tower::ServiceExt; // for oneshot

struct SequenceFailingStore {
    fail_at: u32,
    calls: Mutex<u32>,
}

impl SequenceFailingStore {
    fn new(fail_at: u32) -> Self {
        Self {
            fail_at,
            calls: Mutex::new(0),
        }
    }
}

#[async_trait::async_trait]
impl RateLimitStore for SequenceFailingStore {
    async fn check(
        &self,
        _key: &str,
        limit: u64,
        _window_seconds: u64,
        _key_prefix: &str,
    ) -> Result<RateLimitResult, RateLimitError> {
        let mut calls = self.calls.lock().unwrap();
        *calls += 1;
        if *calls == self.fail_at {
            return Err(RateLimitError::Store("store error".to_string()));
        }
        Ok(RateLimitResult {
            allowed: true,
            remaining: limit.saturating_sub(1),
            reset_after_seconds: 60,
        })
    }
}

#[tokio::test]
async fn test_rate_limiter_basics() {
    let config = RateLimitConfig::default();
    assert_eq!(config.global_max_requests, 1000);
    assert_eq!(config.tenant_max_requests, 100);
    assert_eq!(config.window_seconds, 60);
}

#[tokio::test]
async fn test_memory_rate_limiter() {
    let mut config = RateLimitConfig::default();
    config.tenant_max_requests = 2; // Set low limit for testing
    config.window_seconds = 1;

    let limiter = RateLimiter::with_memory(config);
    let key = "test_tenant";

    // First request should be allowed
    let res1 = limiter.check(key, 2).await.unwrap();
    assert!(res1.allowed);
    assert_eq!(res1.remaining, 1);

    // Second request should be allowed
    let res2 = limiter.check(key, 2).await.unwrap();
    assert!(res2.allowed);
    assert_eq!(res2.remaining, 0);

    // Third request should be blocked
    let res3 = limiter.check(key, 2).await.unwrap();
    assert!(!res3.allowed);

    // Wait for window to expire
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Should be allowed again
    let res4 = limiter.check(key, 2).await.unwrap();
    assert!(res4.allowed);
}

#[tokio::test]
async fn test_rate_limit_middleware_global_store_error_returns_503() {
    let config = RateLimitConfig::default();
    let limiter = Arc::new(RateLimiter::new(
        Arc::new(SequenceFailingStore::new(1)),
        config,
    ));

    let app = Router::new()
        .route("/", get(|| async { "ok" }))
        .layer(from_fn_with_state(limiter, rate_limit_middleware));

    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn test_rate_limit_middleware_tenant_store_error_returns_503() {
    let config = RateLimitConfig::default();
    let limiter = Arc::new(RateLimiter::new(
        Arc::new(SequenceFailingStore::new(2)),
        config,
    ));

    let app = Router::new()
        .route("/", get(|| async { "ok" }))
        .layer(from_fn_with_state(limiter, rate_limit_middleware));

    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn test_rate_limit_middleware_endpoint_store_error_returns_503() {
    let mut config = RateLimitConfig::default();
    config.endpoint_limits.insert("/".to_string(), 1);

    let limiter = Arc::new(RateLimiter::new(
        Arc::new(SequenceFailingStore::new(3)),
        config,
    ));

    let app = Router::new()
        .route("/", get(|| async { "ok" }))
        .layer(from_fn_with_state(limiter, rate_limit_middleware));

    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}
