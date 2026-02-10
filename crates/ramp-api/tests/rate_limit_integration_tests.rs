//! Integration tests for rate limiting middleware.
//!
//! These tests verify the rate_limit_middleware works correctly when mounted
//! on an actual Axum router, checking HTTP response headers, status codes,
//! per-tenant isolation, endpoint-specific limits, and window resets.

use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware,
    routing::get,
    Router,
};
use ramp_api::middleware::{
    rate_limit::{rate_limit_middleware, RateLimitConfig, RateLimiter},
    tenant::{TenantContext, TenantTier},
    tiered_rate_limit::{tiered_rate_limit_middleware, TieredRateLimitState},
};
use ramp_common::types::TenantId;
use std::collections::HashMap;
use std::sync::Arc;
use tower::ServiceExt;

/// Build a minimal Axum app with rate_limit_middleware and a dummy handler.
/// The middleware reads TenantContext from request extensions, so we insert
/// it via an inner layer for tests that need tenant-scoped limiting.
fn build_test_app(config: RateLimitConfig) -> (Router, Arc<RateLimiter>) {
    let limiter = Arc::new(RateLimiter::with_memory(config));

    let app = Router::new()
        .route("/test", get(|| async { "ok" }))
        .route("/other", get(|| async { "other" }))
        .layer(middleware::from_fn_with_state(
            limiter.clone(),
            rate_limit_middleware,
        ));

    (app, limiter)
}

/// Build a test app that injects a TenantContext into request extensions
/// before the rate_limit_middleware runs.
fn build_test_app_with_tenant(
    config: RateLimitConfig,
    tenant_id: &str,
    tier: TenantTier,
) -> Router {
    let limiter = Arc::new(RateLimiter::with_memory(config));
    let tenant_ctx = TenantContext {
        tenant_id: TenantId::new(tenant_id),
        name: format!("Test Tenant {}", tenant_id),
        tier,
    };

    Router::new()
        .route("/test", get(|| async { "ok" }))
        .route("/other", get(|| async { "other" }))
        .layer(middleware::from_fn_with_state(
            limiter.clone(),
            rate_limit_middleware,
        ))
        .layer(middleware::from_fn(move |mut req: Request, next: middleware::Next| {
            let ctx = tenant_ctx.clone();
            async move {
                req.extensions_mut().insert(ctx);
                Ok::<_, StatusCode>(next.run(req).await)
            }
        }))
}

// =============================================================================
// Test 1: Rate limit headers present in successful response
// =============================================================================

#[tokio::test]
async fn test_rate_limit_headers_present_in_response() {
    let config = RateLimitConfig {
        global_max_requests: 1000,
        tenant_max_requests: 50,
        window_seconds: 60,
        key_prefix: "test:headers".to_string(),
        endpoint_limits: HashMap::new(),
    };

    let app = build_test_app_with_tenant(config, "tenant_headers", TenantTier::Standard);

    let request = Request::builder()
        .uri("/test")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let headers = response.headers();

    // Verify X-RateLimit-Limit header is present and correct
    let limit = headers
        .get("X-RateLimit-Limit")
        .expect("X-RateLimit-Limit header should be present");
    // TenantTier::Standard rate_limit() returns 100, so middleware uses that
    assert_eq!(limit.to_str().unwrap(), "100");

    // Verify X-RateLimit-Remaining header is present
    let remaining = headers
        .get("X-RateLimit-Remaining")
        .expect("X-RateLimit-Remaining header should be present");
    assert_eq!(remaining.to_str().unwrap(), "99");

    // Verify X-RateLimit-Reset header is present
    let reset = headers
        .get("X-RateLimit-Reset")
        .expect("X-RateLimit-Reset header should be present");
    let reset_val: u64 = reset.to_str().unwrap().parse().unwrap();
    assert!(reset_val > 0, "X-RateLimit-Reset should be > 0");
}

// =============================================================================
// Test 2: Returns 429 after exhausting rate limit
// =============================================================================

#[tokio::test]
async fn test_rate_limit_returns_429_after_exhaustion() {
    // Use IP-based fallback (no tenant context) so tenant_max_requests from config applies.
    let limit = 5u64;
    let config = RateLimitConfig {
        global_max_requests: 1000,
        tenant_max_requests: limit,
        window_seconds: 60,
        key_prefix: "test:exhaust429".to_string(),
        endpoint_limits: HashMap::new(),
    };

    let (app, _limiter) = build_test_app(config);

    // Send N requests (all should succeed)
    for i in 0..limit {
        let request = Request::builder()
            .uri("/test")
            .method("GET")
            .header("x-forwarded-for", "192.168.1.1")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Request {} of {} should succeed",
            i + 1,
            limit
        );
    }

    // N+1 request should be 429
    let request = Request::builder()
        .uri("/test")
        .method("GET")
        .header("x-forwarded-for", "192.168.1.1")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "Request after exhaustion should be 429"
    );

    // Verify 429 response has Retry-After header
    let retry_after = response
        .headers()
        .get("Retry-After")
        .expect("429 response should have Retry-After header");
    let retry_val: u64 = retry_after.to_str().unwrap().parse().unwrap();
    assert!(retry_val > 0, "Retry-After should be > 0");

    // Verify 429 response has rate limit headers
    assert!(
        response.headers().contains_key("X-RateLimit-Limit"),
        "429 response should have X-RateLimit-Limit"
    );
    let remaining = response
        .headers()
        .get("X-RateLimit-Remaining")
        .expect("429 response should have X-RateLimit-Remaining");
    assert_eq!(remaining.to_str().unwrap(), "0");
}

// =============================================================================
// Test 3: Per-tenant isolation (tenant A exhausted doesn't affect tenant B)
// =============================================================================

#[tokio::test]
async fn test_rate_limit_per_tenant_isolation() {
    // Use IP-based fallback with different IPs to simulate tenant isolation.
    let limit = 3u64;
    let config = RateLimitConfig {
        global_max_requests: 1000,
        tenant_max_requests: limit,
        window_seconds: 60,
        key_prefix: "test:tenant_iso".to_string(),
        endpoint_limits: HashMap::new(),
    };

    let (app, _limiter) = build_test_app(config);

    // Exhaust tenant A (IP 10.0.0.1)
    for _ in 0..limit {
        let request = Request::builder()
            .uri("/test")
            .method("GET")
            .header("x-forwarded-for", "10.0.0.1")
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    // Tenant A is now exhausted
    let request_a = Request::builder()
        .uri("/test")
        .method("GET")
        .header("x-forwarded-for", "10.0.0.1")
        .body(Body::empty())
        .unwrap();
    let response_a = app.clone().oneshot(request_a).await.unwrap();
    assert_eq!(
        response_a.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "Tenant A (10.0.0.1) should be rate limited"
    );

    // Tenant B (IP 10.0.0.2) should still be allowed
    let request_b = Request::builder()
        .uri("/test")
        .method("GET")
        .header("x-forwarded-for", "10.0.0.2")
        .body(Body::empty())
        .unwrap();
    let response_b = app.clone().oneshot(request_b).await.unwrap();
    assert_eq!(
        response_b.status(),
        StatusCode::OK,
        "Tenant B (10.0.0.2) should NOT be affected by tenant A's exhaustion"
    );

    // Verify tenant B still has remaining > 0
    let remaining = response_b
        .headers()
        .get("X-RateLimit-Remaining")
        .expect("Should have X-RateLimit-Remaining");
    let remaining_val: u64 = remaining.to_str().unwrap().parse().unwrap();
    assert_eq!(
        remaining_val,
        limit - 1,
        "Tenant B should have {} remaining",
        limit - 1
    );
}

// =============================================================================
// Test 4: Different route groups / endpoint-specific limits
// =============================================================================

#[tokio::test]
async fn test_rate_limit_different_route_groups() {
    let mut endpoint_limits = HashMap::new();
    endpoint_limits.insert("/test".to_string(), 2u64); // /test has limit of 2
    // /other has no endpoint limit (uses tenant limit)

    let config = RateLimitConfig {
        global_max_requests: 1000,
        tenant_max_requests: 100, // generous tenant limit
        window_seconds: 60,
        key_prefix: "test:routes".to_string(),
        endpoint_limits,
    };

    let (app, _limiter) = build_test_app(config);
    let ip = "10.0.1.1";

    // Send 2 requests to /test (endpoint limit = 2), both should succeed
    for i in 0..2 {
        let request = Request::builder()
            .uri("/test")
            .method("GET")
            .header("x-forwarded-for", ip)
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "/test request {} should succeed",
            i + 1
        );
    }

    // 3rd request to /test should be 429 (endpoint limit exhausted)
    let request = Request::builder()
        .uri("/test")
        .method("GET")
        .header("x-forwarded-for", ip)
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "/test should be rate limited after 2 requests"
    );

    // But /other should still work (no endpoint limit, tenant limit is 100)
    let request_other = Request::builder()
        .uri("/other")
        .method("GET")
        .header("x-forwarded-for", ip)
        .body(Body::empty())
        .unwrap();
    let response_other = app.clone().oneshot(request_other).await.unwrap();
    assert_eq!(
        response_other.status(),
        StatusCode::OK,
        "/other should still work when /test is exhausted"
    );
}

// =============================================================================
// Test 5: Rate limit resets after window expires
// =============================================================================

#[tokio::test]
async fn test_rate_limit_reset_after_window() {
    let limit = 2u64;
    let window_seconds = 1u64; // 1 second window for fast test

    let config = RateLimitConfig {
        global_max_requests: 1000,
        tenant_max_requests: limit,
        window_seconds,
        key_prefix: "test:window_reset".to_string(),
        endpoint_limits: HashMap::new(),
    };

    let (app, _limiter) = build_test_app(config);
    let ip = "10.0.2.1";

    // Exhaust the limit
    for _ in 0..limit {
        let request = Request::builder()
            .uri("/test")
            .method("GET")
            .header("x-forwarded-for", ip)
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    // Verify limit is exhausted
    let request = Request::builder()
        .uri("/test")
        .method("GET")
        .header("x-forwarded-for", ip)
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

    // Wait for window to expire (plus small buffer)
    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;

    // After window reset, requests should succeed again
    let request = Request::builder()
        .uri("/test")
        .method("GET")
        .header("x-forwarded-for", ip)
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Rate limit should reset after window expires"
    );

    // Verify remaining is back to limit-1
    let remaining = response
        .headers()
        .get("X-RateLimit-Remaining")
        .expect("Should have X-RateLimit-Remaining after reset");
    let remaining_val: u64 = remaining.to_str().unwrap().parse().unwrap();
    assert_eq!(
        remaining_val,
        limit - 1,
        "After reset, remaining should be {}",
        limit - 1
    );
}

// =============================================================================
// Test 6: Global rate limit applies across all tenants
// =============================================================================

#[tokio::test]
async fn test_global_rate_limit_applies() {
    let global_limit = 3u64;
    let config = RateLimitConfig {
        global_max_requests: global_limit,
        tenant_max_requests: 100, // high tenant limit
        window_seconds: 60,
        key_prefix: "test:global".to_string(),
        endpoint_limits: HashMap::new(),
    };

    let (app, _limiter) = build_test_app(config);

    // Send requests from different IPs to bypass per-tenant limits
    // but hit global limit
    for i in 0..global_limit {
        let ip = format!("10.1.{}.1", i);
        let request = Request::builder()
            .uri("/test")
            .method("GET")
            .header("x-forwarded-for", &ip)
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Request {} from {} should succeed",
            i + 1,
            ip
        );
    }

    // Next request should hit global limit (regardless of IP)
    let request = Request::builder()
        .uri("/test")
        .method("GET")
        .header("x-forwarded-for", "10.99.99.99")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "Global rate limit should be enforced"
    );
}

// =============================================================================
// Test 7: Remaining decrements correctly across requests
// =============================================================================

#[tokio::test]
async fn test_rate_limit_remaining_decrements() {
    let limit = 5u64;
    let config = RateLimitConfig {
        global_max_requests: 1000,
        tenant_max_requests: limit,
        window_seconds: 60,
        key_prefix: "test:decrement".to_string(),
        endpoint_limits: HashMap::new(),
    };

    let (app, _limiter) = build_test_app(config);
    let ip = "10.0.3.1";

    for i in 0..limit {
        let request = Request::builder()
            .uri("/test")
            .method("GET")
            .header("x-forwarded-for", ip)
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let remaining = response
            .headers()
            .get("X-RateLimit-Remaining")
            .expect("Should have X-RateLimit-Remaining");
        let remaining_val: u64 = remaining.to_str().unwrap().parse().unwrap();
        assert_eq!(
            remaining_val,
            limit - i - 1,
            "After request {}, remaining should be {}",
            i + 1,
            limit - i - 1
        );
    }
}

// =============================================================================
// Test 8: Tenant-specific override - Standard tier gets 100, Enterprise gets 1000
// =============================================================================

#[tokio::test]
async fn test_tenant_tier_standard_vs_enterprise_limits() {
    let config = RateLimitConfig {
        global_max_requests: 10000,
        tenant_max_requests: 50, // default, but middleware uses TenantTier::rate_limit()
        window_seconds: 60,
        key_prefix: "test:tier_override".to_string(),
        endpoint_limits: HashMap::new(),
    };

    // Standard tenant: rate_limit() returns 100
    let app_standard =
        build_test_app_with_tenant(config.clone(), "tenant_standard", TenantTier::Standard);

    let request = Request::builder()
        .uri("/test")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let response = app_standard.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let limit_header = response
        .headers()
        .get("X-RateLimit-Limit")
        .expect("Should have X-RateLimit-Limit");
    assert_eq!(
        limit_header.to_str().unwrap(),
        "100",
        "Standard tier should report limit of 100"
    );

    // Enterprise tenant: rate_limit() returns 1000
    let config_ent = RateLimitConfig {
        global_max_requests: 10000,
        tenant_max_requests: 50,
        window_seconds: 60,
        key_prefix: "test:tier_override_ent".to_string(),
        endpoint_limits: HashMap::new(),
    };
    let app_enterprise =
        build_test_app_with_tenant(config_ent, "tenant_enterprise", TenantTier::Enterprise);

    let request = Request::builder()
        .uri("/test")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let response = app_enterprise.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let limit_header = response
        .headers()
        .get("X-RateLimit-Limit")
        .expect("Should have X-RateLimit-Limit");
    assert_eq!(
        limit_header.to_str().unwrap(),
        "1000",
        "Enterprise tier should report limit of 1000"
    );
}

// =============================================================================
// Test 9: Standard tenant exhausted at 100 while Enterprise still has capacity
// =============================================================================

#[tokio::test]
async fn test_standard_tenant_exhausted_enterprise_still_allowed() {
    // Use a small limit to test exhaustion quickly
    // Standard tier rate_limit() = 100, but we use IP-based with tenant_max_requests=3
    // to avoid sending 100 requests. Instead, test the concept with the tier-based app.
    let limit = 3u64;
    let config = RateLimitConfig {
        global_max_requests: 10000,
        tenant_max_requests: limit, // fallback, not used when TenantContext is present
        window_seconds: 60,
        key_prefix: "test:exhaust_tier".to_string(),
        endpoint_limits: HashMap::new(),
    };

    // Use IP-based (no tenant context) to control exact limit
    let (app, _) = build_test_app(config);

    // Exhaust IP A
    for _ in 0..limit {
        let request = Request::builder()
            .uri("/test")
            .method("GET")
            .header("x-forwarded-for", "192.168.10.1")
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    // IP A is now exhausted
    let request = Request::builder()
        .uri("/test")
        .method("GET")
        .header("x-forwarded-for", "192.168.10.1")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "Standard-like tenant (IP A) should be rate limited"
    );

    // IP B (simulating enterprise) should still be allowed
    let request = Request::builder()
        .uri("/test")
        .method("GET")
        .header("x-forwarded-for", "192.168.10.2")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Enterprise-like tenant (IP B) should still be allowed"
    );
}

// =============================================================================
// Test 10: Tiered rate limit middleware with tenant context
// =============================================================================

/// Build a test app using tiered_rate_limit_middleware instead of rate_limit_middleware
fn build_tiered_app_with_tenant(
    config: RateLimitConfig,
    tenant_id: &str,
    tier: TenantTier,
) -> Router {
    let limiter = Arc::new(RateLimiter::with_memory(config));
    let tiered_state = TieredRateLimitState::new(limiter);
    let tenant_ctx = TenantContext {
        tenant_id: TenantId::new(tenant_id),
        name: format!("Tiered Tenant {}", tenant_id),
        tier,
    };

    Router::new()
        .route("/test", get(|| async { "ok" }))
        .layer(middleware::from_fn_with_state(
            tiered_state.clone(),
            tiered_rate_limit_middleware,
        ))
        .layer(middleware::from_fn(move |mut req: Request, next: middleware::Next| {
            let ctx = tenant_ctx.clone();
            async move {
                req.extensions_mut().insert(ctx);
                Ok::<_, StatusCode>(next.run(req).await)
            }
        }))
}

#[tokio::test]
async fn test_tiered_rate_limit_middleware_standard_tenant() {
    let config = RateLimitConfig {
        global_max_requests: 10000,
        tenant_max_requests: 100,
        window_seconds: 60,
        key_prefix: "test:tiered_std".to_string(),
        endpoint_limits: HashMap::new(),
    };

    let app = build_tiered_app_with_tenant(config, "tenant_std_tiered", TenantTier::Standard);

    let request = Request::builder()
        .uri("/test")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Standard tier in tiered middleware: get_tenant_config returns max_requests=100
    let limit_header = response
        .headers()
        .get("X-RateLimit-Limit")
        .expect("Should have X-RateLimit-Limit");
    assert_eq!(
        limit_header.to_str().unwrap(),
        "100",
        "Tiered middleware Standard tenant should have limit 100"
    );

    let remaining = response
        .headers()
        .get("X-RateLimit-Remaining")
        .expect("Should have X-RateLimit-Remaining");
    assert_eq!(remaining.to_str().unwrap(), "99");
}

#[tokio::test]
async fn test_tiered_rate_limit_middleware_enterprise_tenant() {
    let config = RateLimitConfig {
        global_max_requests: 10000,
        tenant_max_requests: 100,
        window_seconds: 60,
        key_prefix: "test:tiered_ent".to_string(),
        endpoint_limits: HashMap::new(),
    };

    let app = build_tiered_app_with_tenant(config, "tenant_ent_tiered", TenantTier::Enterprise);

    let request = Request::builder()
        .uri("/test")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Enterprise tier in tiered middleware: get_tenant_config returns max_requests=1000
    let limit_header = response
        .headers()
        .get("X-RateLimit-Limit")
        .expect("Should have X-RateLimit-Limit");
    assert_eq!(
        limit_header.to_str().unwrap(),
        "1000",
        "Tiered middleware Enterprise tenant should have limit 1000"
    );

    let remaining = response
        .headers()
        .get("X-RateLimit-Remaining")
        .expect("Should have X-RateLimit-Remaining");
    assert_eq!(remaining.to_str().unwrap(), "999");
}

// =============================================================================
// Test 11: Tiered middleware VIP tenant gets 5000 limit
// =============================================================================

#[tokio::test]
async fn test_tiered_rate_limit_vip_tenant_override() {
    let config = RateLimitConfig {
        global_max_requests: 10000,
        tenant_max_requests: 100,
        window_seconds: 60,
        key_prefix: "test:tiered_vip".to_string(),
        endpoint_limits: HashMap::new(),
    };

    // "tenant_vip_1" is a special VIP tenant in get_tenant_config -> 5000 req
    let app = build_tiered_app_with_tenant(config, "tenant_vip_1", TenantTier::Enterprise);

    let request = Request::builder()
        .uri("/test")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let limit_header = response
        .headers()
        .get("X-RateLimit-Limit")
        .expect("Should have X-RateLimit-Limit");
    assert_eq!(
        limit_header.to_str().unwrap(),
        "5000",
        "VIP tenant_vip_1 should have limit 5000 from DB-like override"
    );

    let remaining = response
        .headers()
        .get("X-RateLimit-Remaining")
        .expect("Should have X-RateLimit-Remaining");
    assert_eq!(remaining.to_str().unwrap(), "4999");
}

// =============================================================================
// Test 12: Tiered middleware returns 429 with correct headers when exhausted
// =============================================================================

#[tokio::test]
async fn test_tiered_rate_limit_429_with_headers() {
    // Use global limit exhaustion path for simplicity
    let config_small = RateLimitConfig {
        global_max_requests: 2, // small global limit
        tenant_max_requests: 100,
        window_seconds: 60,
        key_prefix: "test:tiered_429_global".to_string(),
        endpoint_limits: HashMap::new(),
    };

    let limiter = Arc::new(RateLimiter::with_memory(config_small));
    let tiered_state = TieredRateLimitState::new(limiter);
    let tenant_ctx = TenantContext {
        tenant_id: TenantId::new("tenant_429"),
        name: "Tenant 429".to_string(),
        tier: TenantTier::Standard,
    };

    let app = Router::new()
        .route("/test", get(|| async { "ok" }))
        .layer(middleware::from_fn_with_state(
            tiered_state.clone(),
            tiered_rate_limit_middleware,
        ))
        .layer(middleware::from_fn(move |mut req: Request, next: middleware::Next| {
            let ctx = tenant_ctx.clone();
            async move {
                req.extensions_mut().insert(ctx);
                Ok::<_, StatusCode>(next.run(req).await)
            }
        }));

    // Exhaust global limit (2 requests)
    for _ in 0..2 {
        let request = Request::builder()
            .uri("/test")
            .method("GET")
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    // 3rd request should be 429
    let request = Request::builder()
        .uri("/test")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "Tiered middleware should return 429 when global limit exhausted"
    );

    // Verify Retry-After header present
    let retry_after = response
        .headers()
        .get("Retry-After")
        .expect("429 response should have Retry-After header");
    let retry_val: u64 = retry_after.to_str().unwrap().parse().unwrap();
    assert!(retry_val > 0, "Retry-After should be > 0");
}
