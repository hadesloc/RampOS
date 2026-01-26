
use crate::middleware::RateLimitConfig;
use crate::middleware::RateLimiter;
// use crate::middleware::RateLimitStore; // Unused
// use std::sync::Arc; // Unused
use tokio::time::Duration;

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
