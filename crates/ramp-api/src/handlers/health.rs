use axum::Json;
use chrono::Utc;

use crate::dto::HealthResponse;

/// Health check
///
/// Returns the service health status.
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: Utc::now(),
    })
}

/// Readiness check
///
/// Returns whether the service is ready to accept traffic.
#[utoipa::path(
    get,
    path = "/ready",
    tag = "health",
    responses(
        (status = 200, description = "Service is ready", body = HealthResponse)
    )
)]
pub async fn readiness_check() -> Json<HealthResponse> {
    // In production, would check database, NATS, etc.
    Json(HealthResponse {
        status: "ready".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: Utc::now(),
    })
}
