//! E2E integration tests for rate limiting with a real HTTP server.
//!
//! These tests spin up an actual Axum server on a random port and send
//! real HTTP requests via reqwest. This validates the full middleware stack
//! including TCP connection handling, header serialization, and concurrent access.

use axum::{
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
use tokio::net::TcpListener;

/// Start a real HTTP server on a random port, returning the base URL.
async fn start_server(app: Router) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give the server a moment to start accepting connections
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    base_url
}

/// Build a minimal rate-limited app (IP-based fallback, no tenant context).
fn build_ip_based_app(config: RateLimitConfig) -> Router {
    let limiter = Arc::new(RateLimiter::with_memory(config));

    Router::new()
        .route("/test", get(|| async { "ok" }))
        .route("/other", get(|| async { "other" }))
        .layer(middleware::from_fn_with_state(
            limiter.clone(),
            rate_limit_middleware,
        ))
}

/// Build a rate-limited app that injects TenantContext before rate limiting.
fn build_tenant_app(config: RateLimitConfig, tenant_id: &str, tier: TenantTier) -> Router {
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
        .layer(middleware::from_fn(
            move |mut req: Request, next: middleware::Next| {
                let ctx = tenant_ctx.clone();
                async move {
                    req.extensions_mut().insert(ctx);
                    Ok::<_, StatusCode>(next.run(req).await)
                }
            },
        ))
}

/// Build a tiered rate-limited app with tenant context.
fn build_tiered_tenant_app(config: RateLimitConfig, tenant_id: &str, tier: TenantTier) -> Router {
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
            tiered_state,
            tiered_rate_limit_middleware,
        ))
        .layer(middleware::from_fn(
            move |mut req: Request, next: middleware::Next| {
                let ctx = tenant_ctx.clone();
                async move {
                    req.extensions_mut().insert(ctx);
                    Ok::<_, StatusCode>(next.run(req).await)
                }
            },
        ))
}

// =============================================================================
// Test 1: Rate limit returns 429 after exceeding limit
// =============================================================================

#[tokio::test]
async fn test_rate_limit_returns_429_after_exceeding_limit() {
    let limit = 5u64;
    let config = RateLimitConfig {
        global_max_requests: 1000,
        tenant_max_requests: limit,
        window_seconds: 60,
        key_prefix: "e2e:429".to_string(),
        endpoint_limits: HashMap::new(),
    };

    let app = build_ip_based_app(config);
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    // Send `limit` requests -- all should succeed
    for i in 0..limit {
        let resp = client
            .get(format!("{}/test", base_url))
            .header("x-forwarded-for", "192.168.1.1")
            .send()
            .await
            .unwrap();
        assert_eq!(
            resp.status().as_u16(),
            200,
            "Request {} of {} should succeed",
            i + 1,
            limit
        );
    }

    // Next request should be 429 Too Many Requests
    let resp = client
        .get(format!("{}/test", base_url))
        .header("x-forwarded-for", "192.168.1.1")
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status().as_u16(),
        429,
        "Request after exhaustion should be 429"
    );
}

// =============================================================================
// Test 2: Rate limit headers present on success and 429
// =============================================================================

#[tokio::test]
async fn test_rate_limit_headers_present() {
    let config = RateLimitConfig {
        global_max_requests: 1000,
        tenant_max_requests: 50,
        window_seconds: 60,
        key_prefix: "e2e:headers".to_string(),
        endpoint_limits: HashMap::new(),
    };

    let app = build_tenant_app(config, "tenant_e2e_headers", TenantTier::Standard);
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/test", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);

    // Verify X-RateLimit-Limit header (Standard tier rate_limit() = 100)
    let limit_val = resp
        .headers()
        .get("X-RateLimit-Limit")
        .expect("X-RateLimit-Limit header should be present")
        .to_str()
        .unwrap();
    assert_eq!(limit_val, "100", "Standard tier should report limit of 100");

    // Verify X-RateLimit-Remaining header (should be 99 after first request)
    let remaining_val = resp
        .headers()
        .get("X-RateLimit-Remaining")
        .expect("X-RateLimit-Remaining header should be present")
        .to_str()
        .unwrap();
    assert_eq!(remaining_val, "99");

    // Verify X-RateLimit-Reset header
    let reset_val: u64 = resp
        .headers()
        .get("X-RateLimit-Reset")
        .expect("X-RateLimit-Reset header should be present")
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!(reset_val > 0, "X-RateLimit-Reset should be > 0");

    // Send a second request to verify remaining decrements
    let resp2 = client
        .get(format!("{}/test", base_url))
        .send()
        .await
        .unwrap();
    let remaining2 = resp2
        .headers()
        .get("X-RateLimit-Remaining")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(remaining2, "98", "Remaining should decrement to 98");
}

// =============================================================================
// Test 3: Per-tenant isolation
// =============================================================================

#[tokio::test]
async fn test_rate_limit_per_tenant_isolation() {
    let limit = 3u64;
    let config = RateLimitConfig {
        global_max_requests: 1000,
        tenant_max_requests: limit,
        window_seconds: 60,
        key_prefix: "e2e:tenant_iso".to_string(),
        endpoint_limits: HashMap::new(),
    };

    let app = build_ip_based_app(config);
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    // Exhaust Tenant A (IP 10.0.0.1)
    for _ in 0..limit {
        let resp = client
            .get(format!("{}/test", base_url))
            .header("x-forwarded-for", "10.0.0.1")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status().as_u16(), 200);
    }

    // Tenant A is now exhausted
    let resp_a = client
        .get(format!("{}/test", base_url))
        .header("x-forwarded-for", "10.0.0.1")
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp_a.status().as_u16(),
        429,
        "Tenant A should be rate limited"
    );

    // Tenant B (IP 10.0.0.2) should still be allowed
    let resp_b = client
        .get(format!("{}/test", base_url))
        .header("x-forwarded-for", "10.0.0.2")
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp_b.status().as_u16(),
        200,
        "Tenant B should NOT be affected by Tenant A"
    );

    let remaining = resp_b
        .headers()
        .get("X-RateLimit-Remaining")
        .unwrap()
        .to_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();
    assert_eq!(remaining, limit - 1, "Tenant B should have full quota minus 1");
}

// =============================================================================
// Test 4: Sliding window reset
// =============================================================================

#[tokio::test]
async fn test_rate_limit_sliding_window_reset() {
    let limit = 2u64;
    let window_seconds = 1u64;

    let config = RateLimitConfig {
        global_max_requests: 1000,
        tenant_max_requests: limit,
        window_seconds,
        key_prefix: "e2e:window_reset".to_string(),
        endpoint_limits: HashMap::new(),
    };

    let app = build_ip_based_app(config);
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();
    let ip = "10.0.2.1";

    // Exhaust the limit
    for _ in 0..limit {
        let resp = client
            .get(format!("{}/test", base_url))
            .header("x-forwarded-for", ip)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status().as_u16(), 200);
    }

    // Verify exhausted
    let resp = client
        .get(format!("{}/test", base_url))
        .header("x-forwarded-for", ip)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 429);

    // Wait for window to expire
    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;

    // After reset, requests should succeed again
    let resp = client
        .get(format!("{}/test", base_url))
        .header("x-forwarded-for", ip)
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status().as_u16(),
        200,
        "Rate limit should reset after window expires"
    );

    let remaining = resp
        .headers()
        .get("X-RateLimit-Remaining")
        .unwrap()
        .to_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();
    assert_eq!(remaining, limit - 1, "After reset, remaining should be limit - 1");
}

// =============================================================================
// Test 5: VIP tier gets higher limit than Standard
// =============================================================================

#[tokio::test]
async fn test_rate_limit_vip_tier_higher_limit() {
    // Standard tenant
    let config_std = RateLimitConfig {
        global_max_requests: 10000,
        tenant_max_requests: 100,
        window_seconds: 60,
        key_prefix: "e2e:vip_std".to_string(),
        endpoint_limits: HashMap::new(),
    };
    let app_std = build_tiered_tenant_app(config_std, "tenant_regular", TenantTier::Standard);
    let base_std = start_server(app_std).await;

    // VIP tenant (tenant_vip_1 gets 5000 from tiered middleware DB-like override)
    let config_vip = RateLimitConfig {
        global_max_requests: 10000,
        tenant_max_requests: 100,
        window_seconds: 60,
        key_prefix: "e2e:vip_vip".to_string(),
        endpoint_limits: HashMap::new(),
    };
    let app_vip = build_tiered_tenant_app(config_vip, "tenant_vip_1", TenantTier::Enterprise);
    let base_vip = start_server(app_vip).await;

    let client = reqwest::Client::new();

    // Check Standard tenant's limit
    let resp_std = client
        .get(format!("{}/test", base_std))
        .send()
        .await
        .unwrap();
    assert_eq!(resp_std.status().as_u16(), 200);
    let std_limit: u64 = resp_std
        .headers()
        .get("X-RateLimit-Limit")
        .unwrap()
        .to_str()
        .unwrap()
        .parse()
        .unwrap();

    // Check VIP tenant's limit
    let resp_vip = client
        .get(format!("{}/test", base_vip))
        .send()
        .await
        .unwrap();
    assert_eq!(resp_vip.status().as_u16(), 200);
    let vip_limit: u64 = resp_vip
        .headers()
        .get("X-RateLimit-Limit")
        .unwrap()
        .to_str()
        .unwrap()
        .parse()
        .unwrap();

    assert!(
        vip_limit > std_limit,
        "VIP limit ({}) should be higher than Standard limit ({})",
        vip_limit,
        std_limit
    );
    assert_eq!(std_limit, 100, "Standard should have limit 100");
    assert_eq!(vip_limit, 5000, "VIP tenant_vip_1 should have limit 5000");
}

// =============================================================================
// Test 6: Concurrent requests are counted correctly
// =============================================================================

#[tokio::test]
async fn test_rate_limit_concurrent_requests() {
    let limit = 10u64;
    let config = RateLimitConfig {
        global_max_requests: 1000,
        tenant_max_requests: limit,
        window_seconds: 60,
        key_prefix: "e2e:concurrent".to_string(),
        endpoint_limits: HashMap::new(),
    };

    let app = build_ip_based_app(config);
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();
    let ip = "10.0.5.1";

    // Send `limit` requests concurrently
    let mut handles = Vec::new();
    for _ in 0..limit {
        let c = client.clone();
        let url = format!("{}/test", base_url);
        let ip_val = ip.to_string();
        handles.push(tokio::spawn(async move {
            c.get(&url)
                .header("x-forwarded-for", ip_val)
                .send()
                .await
                .unwrap()
                .status()
                .as_u16()
        }));
    }

    let mut ok_count = 0u64;
    for h in handles {
        let status = h.await.unwrap();
        if status == 200 {
            ok_count += 1;
        }
    }

    // All `limit` concurrent requests should succeed
    assert_eq!(
        ok_count, limit,
        "All {} concurrent requests should succeed",
        limit
    );

    // Next request after concurrent batch should be 429
    let resp = client
        .get(format!("{}/test", base_url))
        .header("x-forwarded-for", ip)
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status().as_u16(),
        429,
        "Request after concurrent batch should be rate limited"
    );
}

// =============================================================================
// Test 7: Different endpoints with endpoint-specific limits
// =============================================================================

#[tokio::test]
async fn test_rate_limit_different_endpoints() {
    let mut endpoint_limits = HashMap::new();
    endpoint_limits.insert("/test".to_string(), 2u64);
    // /other has no endpoint limit (uses tenant limit)

    let config = RateLimitConfig {
        global_max_requests: 1000,
        tenant_max_requests: 100,
        window_seconds: 60,
        key_prefix: "e2e:endpoints".to_string(),
        endpoint_limits,
    };

    let app = build_ip_based_app(config);
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();
    let ip = "10.0.6.1";

    // Send 2 requests to /test (endpoint limit = 2)
    for i in 0..2 {
        let resp = client
            .get(format!("{}/test", base_url))
            .header("x-forwarded-for", ip)
            .send()
            .await
            .unwrap();
        assert_eq!(
            resp.status().as_u16(),
            200,
            "/test request {} should succeed",
            i + 1
        );
    }

    // 3rd request to /test should be 429
    let resp = client
        .get(format!("{}/test", base_url))
        .header("x-forwarded-for", ip)
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status().as_u16(),
        429,
        "/test should be rate limited after 2 requests"
    );

    // /other should still work
    let resp_other = client
        .get(format!("{}/other", base_url))
        .header("x-forwarded-for", ip)
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp_other.status().as_u16(),
        200,
        "/other should still work when /test is exhausted"
    );
}
