use axum::extract::Extension;
use axum::{extract::State, http::HeaderMap, Json};
use ramp_common::types::*;
use ramp_core::service::trade::{TradeExecutedRequest as ServiceRequest, TradeService};
use std::sync::Arc;
use tracing::{info, instrument};
use validator::Validate;

use crate::dto::{TradeExecutedRequest, TradeExecutedResponse};
use crate::error::ApiError;
use crate::extract::ValidatedJson;
use crate::middleware::TenantContext;

pub type TradeServiceState = Arc<TradeService>;

/// Record a trade execution
///
/// Records a trade executed on an external exchange.
#[utoipa::path(
    post,
    path = "/v1/events/trade-executed",
    tag = "events",
    request_body = TradeExecutedRequest,
    responses(
        (status = 200, description = "Trade recorded", body = TradeExecutedResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = []),
        ("idempotency_key" = [])
    )
)]
#[instrument(skip_all, fields(tenant_id, user_id, trade_id, symbol))]
pub async fn record_trade(
    State(service): State<TradeServiceState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    headers: HeaderMap,
    ValidatedJson(req): ValidatedJson<TradeExecutedRequest>,
) -> Result<Json<TradeExecutedResponse>, ApiError> {
    if tenant_ctx.tenant_id.0 != req.tenant_id {
        return Err(ApiError::Forbidden("Tenant mismatch".to_string()));
    }

    tracing::Span::current()
        .record("tenant_id", &req.tenant_id)
        .record("user_id", &req.user_id)
        .record("trade_id", &req.trade_id)
        .record("symbol", &req.symbol);

    // Get idempotency key from header
    let idempotency_key = headers
        .get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .map(|s| IdempotencyKey::new(s));

    // Build service request
    let service_req = ServiceRequest {
        tenant_id: TenantId::new(&req.tenant_id),
        user_id: UserId::new(&req.user_id),
        trade_id: req.trade_id,
        symbol: req.symbol,
        price: req.price,
        vnd_delta: VndAmount::from_i64(req.vnd_delta),
        crypto_delta: req.crypto_delta,
        timestamp: Timestamp::from_datetime(req.timestamp),
        idempotency_key,
        metadata: serde_json::Value::Null,
    };

    // Call service
    let result = service.record_trade(service_req).await?;

    info!(
        intent_id = %result.intent_id,
        "Trade recorded via API"
    );

    Ok(Json(TradeExecutedResponse {
        intent_id: result.intent_id.0,
        status: format!("{:?}", result.status),
    }))
}
