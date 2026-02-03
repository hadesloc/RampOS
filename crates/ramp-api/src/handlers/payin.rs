use axum::{
    extract::{Extension, State},
    http::HeaderMap,
    Json,
};
use ramp_common::types::*;
use ramp_core::service::payin::{
    ConfirmPayinRequest as ServiceConfirmRequest, CreatePayinRequest as ServiceRequest,
    PayinService,
};
use std::sync::Arc;
use tracing::{info, instrument};

use crate::dto::{
    ConfirmPayinRequest, ConfirmPayinResponse, CreatePayinRequest, CreatePayinResponse,
    VirtualAccountDto,
};
use crate::error::ApiError;
use crate::extract::ValidatedJson;
use crate::middleware::TenantContext;

pub type PayinServiceState = Arc<PayinService>;

/// Create a new pay-in intent
///
/// Creates an intent for a user to deposit fiat currency.
#[utoipa::path(
    post,
    path = "/v1/intents/payin",
    tag = "intents",
    request_body = CreatePayinRequest,
    responses(
        (status = 200, description = "Pay-in intent created", body = CreatePayinResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = []),
        ("idempotency_key" = [])
    )
)]
#[instrument(skip_all, fields(tenant_id, user_id, intent_id, amount))]
pub async fn create_payin(
    State(service): State<PayinServiceState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    headers: HeaderMap,
    ValidatedJson(req): ValidatedJson<CreatePayinRequest>,
) -> Result<(HeaderMap, Json<CreatePayinResponse>), ApiError> {
    if tenant_ctx.tenant_id.0 != req.tenant_id {
        return Err(ApiError::Forbidden("Tenant mismatch".to_string()));
    }

    tracing::Span::current()
        .record("tenant_id", &req.tenant_id)
        .record("user_id", &req.user_id)
        .record("amount", req.amount_vnd);

    // Get idempotency key from header
    let idempotency_key = headers
        .get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .map(IdempotencyKey::new);

    // Build service request
    let service_req = ServiceRequest {
        tenant_id: TenantId::new(&req.tenant_id),
        user_id: UserId::new(&req.user_id),
        amount_vnd: VndAmount::from_i64(req.amount_vnd),
        rails_provider: RailsProvider::new(&req.rails_provider),
        idempotency_key,
        metadata: req.metadata.unwrap_or(serde_json::Value::Null),
    };

    // Call service
    let result = service.create_payin(service_req).await?;

    info!(
        intent_id = %result.intent_id,
        "Pay-in intent created via API"
    );

    // Build response
    let response = CreatePayinResponse {
        intent_id: result.intent_id.0,
        reference_code: result.reference_code.0,
        virtual_account: result.virtual_account.map(|va| VirtualAccountDto {
            bank: va.bank,
            account_number: va.account_number,
            account_name: va.account_name,
        }),
        expires_at: result.expires_at.0,
        status: result.status.to_string(),
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        "X-User-Daily-Limit",
        result
            .daily_limit
            .to_string()
            .parse()
            .unwrap_or(axum::http::HeaderValue::from_static("0")),
    );
    headers.insert(
        "X-User-Daily-Remaining",
        result
            .daily_remaining
            .to_string()
            .parse()
            .unwrap_or(axum::http::HeaderValue::from_static("0")),
    );

    Ok((headers, Json(response)))
}

/// Confirm a pay-in (callback)
///
/// Confirms that funds have been received. Usually called by rails provider via webhook.
#[utoipa::path(
    post,
    path = "/v1/intents/payin/confirm",
    tag = "intents",
    request_body = ConfirmPayinRequest,
    responses(
        (status = 200, description = "Pay-in confirmed", body = ConfirmPayinResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Intent not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("hmac_signature" = [])
    )
)]
#[instrument(skip_all, fields(tenant_id, reference_code, intent_id))]
pub async fn confirm_payin(
    State(service): State<PayinServiceState>,
    headers: HeaderMap,
    ValidatedJson(req): ValidatedJson<ConfirmPayinRequest>,
) -> Result<Json<ConfirmPayinResponse>, ApiError> {
    // Verify internal secret to prevent unauthorized access
    let internal_secret = std::env::var("INTERNAL_SERVICE_SECRET")
        .map_err(|_| ApiError::Internal("Internal secret not configured".to_string()))?;

    let provided_secret = headers
        .get("X-Internal-Secret")
        .and_then(|v| v.to_str().ok());

    if provided_secret != Some(&internal_secret) {
        return Err(ApiError::Forbidden(
            "Missing or invalid internal secret".to_string(),
        ));
    }

    tracing::Span::current()
        .record("tenant_id", &req.tenant_id)
        .record("reference_code", &req.reference_code);

    // Build service request
    let service_req = ServiceConfirmRequest {
        tenant_id: TenantId::new(&req.tenant_id),
        reference_code: ReferenceCode(req.reference_code),
        bank_tx_id: req.bank_tx_id,
        amount_vnd: VndAmount::from_i64(req.amount_vnd),
        settled_at: Timestamp::from_datetime(req.settled_at),
        raw_payload_hash: req.raw_payload_hash,
    };

    // Call service
    let intent_id = service.confirm_payin(service_req).await?;

    info!(
        intent_id = %intent_id,
        "Pay-in confirmed via API"
    );

    Ok(Json(ConfirmPayinResponse {
        intent_id: intent_id.0,
        status: "COMPLETED".to_string(),
    }))
}
