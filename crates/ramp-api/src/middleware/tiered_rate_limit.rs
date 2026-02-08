use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use tracing::warn;

use crate::middleware::rate_limit::RateLimiter;
use crate::middleware::tenant::{TenantContext, TenantTier};

/// Configuration for tiered rate limiting
#[derive(Clone)]
pub struct TieredRateLimitState {
    pub limiter: Arc<RateLimiter>,
}

impl TieredRateLimitState {
    pub fn new(limiter: Arc<RateLimiter>) -> Self {
        Self { limiter }
    }
}

/// struct to hold rate limit config for a tenant
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TenantRateLimitConfig {
    max_requests: u64,
    window_seconds: u64,
}

/// Mock function to load tenant config (in production this would come from DB/Cache)
async fn get_tenant_config(tenant_id: &str, tier: &TenantTier) -> TenantRateLimitConfig {
    // In a real implementation, we would look up specific overrides for this tenant_id
    // For now, we return defaults based on tier, but with higher limits for Enterprise
    match tier {
        TenantTier::Enterprise => {
            // Check for specific VIP tenants (mock)
            if tenant_id == "tenant_vip_1" {
                TenantRateLimitConfig {
                    max_requests: 5000,
                    window_seconds: 60,
                }
            } else {
                TenantRateLimitConfig {
                    max_requests: 1000,
                    window_seconds: 60,
                }
            }
        }
        TenantTier::Standard => TenantRateLimitConfig {
            max_requests: 100,
            window_seconds: 60,
        },
    }
}

pub async fn tiered_rate_limit_middleware(
    State(state): State<TieredRateLimitState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 0. Check global rate limit first
    let global_result = state
        .limiter
        .check("global", state.limiter.config.global_max_requests)
        .await;

    match global_result {
        Ok(res) if !res.allowed => {
            warn!("Global rate limit exceeded in tiered middleware");
            let mut response = Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .body(axum::body::Body::empty())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            response.headers_mut().insert(
                "Retry-After",
                res.reset_after_seconds
                    .to_string()
                    .parse()
                    .unwrap_or(axum::http::HeaderValue::from_static("60")),
            );
            return Ok(response);
        }
        Err(e) => {
            warn!(error = ?e, "Rate limiter error (global) in tiered middleware");
            return Ok(Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .body(axum::body::Body::empty())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?);
        }
        _ => {}
    }

    // 1. Extract tenant context
    let (tenant_id, tier) = if let Some(ctx) = req.extensions().get::<TenantContext>() {
        (ctx.tenant_id.0.clone(), ctx.tier)
    } else {
        return Ok(next.run(req).await);
    };

    // 2. Load config for this tenant
    let config = get_tenant_config(&tenant_id, &tier).await;

    // 3. Check rate limit
    // We access the inner store directly to pass custom window/limit
    // Note: RateLimiter in rate_limit.rs uses a fixed window in its `check` method wrapper,
    // but the store `check` method takes window as argument.
    // However, RateLimiter struct fields are private, so we can't access `store` directly if it's not public.
    // Let's check if `store` field on RateLimiter is public.
    // It is NOT public in the file I read: `store: Arc<dyn RateLimitStore>,`

    // BUT `RateLimiter` has a `check` method:
    // pub async fn check(&self, key: &str, limit: u64) -> Result<RateLimitResult, RateLimitError>
    // This uses `self.config.window_seconds`.

    // If we want custom windows per tenant, we might need to extend RateLimiter or expose the store.
    // Since I cannot modify `rate_limit.rs`, I should probably instantiate my own Store wrapper or
    // if I can't access the store from the `RateLimiter` instance passed in state...

    // The `TieredRateLimitState` receives `Arc<RateLimiter>`.
    // If I can't change window size, I have to stick to the configured window in RateLimiter (default 60s).
    // The requirements say "Load config... (mock or db)".
    // If the window is fixed to 60s, I can still vary `max_requests`.

    let result = state.limiter.check(&tenant_id, config.max_requests).await;

    match result {
        Ok(res) => {
            if !res.allowed {
                warn!(
                    tenant = %tenant_id,
                    tier = ?tier,
                    limit = %config.max_requests,
                    "Tiered rate limit exceeded"
                );

                let mut response = Response::builder()
                    .status(StatusCode::TOO_MANY_REQUESTS)
                    .body(axum::body::Body::empty())
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                response.headers_mut().insert(
                    "Retry-After",
                    res.reset_after_seconds.to_string().parse().unwrap_or(
                        axum::http::HeaderValue::from_static("60")
                    ),
                );

                // Add rate limit headers
                response.headers_mut().insert(
                    "X-RateLimit-Limit",
                    config.max_requests.to_string().parse().unwrap_or(
                         axum::http::HeaderValue::from_static("0")
                    )
                );
                 response.headers_mut().insert(
                    "X-RateLimit-Remaining",
                    "0".parse().unwrap_or(
                         axum::http::HeaderValue::from_static("0")
                    )
                );
                response.headers_mut().insert(
                    "X-RateLimit-Reset",
                    res.reset_after_seconds.to_string().parse().unwrap_or(
                         axum::http::HeaderValue::from_static("0")
                    )
                );

                return Ok(response);
            }

            // Allow request
            let mut response = next.run(req).await;

            // Add headers to successful response too
            response.headers_mut().insert(
                "X-RateLimit-Limit",
                config.max_requests.to_string().parse().unwrap_or(
                    axum::http::HeaderValue::from_static("0")
                )
            );
            response.headers_mut().insert(
                "X-RateLimit-Remaining",
                res.remaining.to_string().parse().unwrap_or(
                    axum::http::HeaderValue::from_static("0")
                )
            );
            response.headers_mut().insert(
                "X-RateLimit-Reset",
                res.reset_after_seconds.to_string().parse().unwrap_or(
                    axum::http::HeaderValue::from_static("0")
                )
            );

            Ok(response)
        }
        Err(e) => {
            warn!(error = ?e, "Rate limiter error in tiered middleware");
            // Fail open? or Service Unavailable?
            // Usually safer to fail open for rate limits unless under attack, but strict compliance might say otherwise.
            // Let's return ServiceUnavailable to be safe.
            Ok(Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .body(axum::body::Body::empty())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
        }
    }
}
