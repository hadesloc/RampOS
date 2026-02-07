//! Usage Billing Middleware
//!
//! Automatically tracks API usage for authenticated requests.

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::IntoResponse,
};
use ramp_core::billing::{MeterType, MetricValue};
use tokio::time::Instant;

use crate::AppState;

/// Middleware to track API usage
pub async fn usage_metering_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> impl IntoResponse {
    // 1. Extract tenant ID from extensions (set by auth_middleware)
    let tenant_id = request
        .extensions()
        .get::<ramp_common::types::TenantId>()
        .cloned();

    // 2. Process request
    let start = Instant::now();
    let response = next.run(request).await;
    let _duration = start.elapsed();

    // 3. Record usage asynchronously (fire and forget)
    // Only if tenant_id is present (authenticated request)
    // and request was successful (optional, but usually we bill for success)
    if let Some(tenant_id) = tenant_id {
        // We only bill for 2xx responses?
        // Or all responses? Usually API calls are billed regardless of outcome if they hit the server.
        // Let's bill everything for now.

        let billing_service = state.billing_service.clone();

        tokio::spawn(async move {
            // Record API Call
            let _ = billing_service
                .record_usage(&tenant_id, MeterType::ApiCalls, MetricValue::Count(1))
                .await;

            // TODO: Record data transfer volume (request + response size) if MeterType::DataTransfer is used
        });
    }

    response
}
