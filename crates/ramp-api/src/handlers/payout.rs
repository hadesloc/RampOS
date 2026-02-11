use axum::{
    extract::{Extension, State},
    http::HeaderMap,
    Json,
};
use ramp_common::types::*;
use ramp_core::service::payout::{CreatePayoutRequest as ServiceRequest, PayoutService};
use serde_json::json;
use std::sync::Arc;
use tracing::{info, instrument};

use crate::dto::{CreatePayoutRequest, CreatePayoutResponse};
use crate::error::ApiError;
use crate::extract::ValidatedJson;
use crate::middleware::TenantContext;

pub type PayoutServiceState = Arc<PayoutService>;

/// Create a new pay-out intent
///
/// Creates an intent for a user to withdraw fiat currency to a bank account.
#[utoipa::path(
    post,
    path = "/v1/intents/payout",
    tag = "intents",
    request_body = CreatePayoutRequest,
    responses(
        (status = 200, description = "Pay-out intent created", body = CreatePayoutResponse,
         example = json!({
             "intentId": "intent_01HX9Z1Y2X3W4V5U6T7S8R9Q0",
             "status": "PENDING_PAYOUT"
         })),
        (status = 400, description = "Invalid request", body = ErrorResponse,
         example = json!({"error": {"code": "BAD_REQUEST", "message": "Amount must be between 10,000 and 500,000,000 VND"}})),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = []),
        ("idempotency_key" = [])
    )
)]
#[instrument(skip_all, fields(tenant_id, user_id, intent_id, amount))]
pub async fn create_payout(
    State(service): State<PayoutServiceState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    headers: HeaderMap,
    ValidatedJson(req): ValidatedJson<CreatePayoutRequest>,
) -> Result<(HeaderMap, Json<CreatePayoutResponse>), ApiError> {
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
        bank_account: BankAccount {
            bank_code: req.bank_account.bank_code,
            account_number: req.bank_account.account_number,
            account_name: req.bank_account.account_name,
        },
        idempotency_key,
        metadata: req.metadata.unwrap_or(serde_json::Value::Null),
    };

    // Call service
    let result = service.create_payout(service_req).await?;

    info!(
        intent_id = %result.intent_id,
        "Pay-out intent created via API"
    );

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

    Ok((
        headers,
        Json(CreatePayoutResponse {
            intent_id: result.intent_id.0,
            status: result.status.to_string(),
        }),
    ))
}
