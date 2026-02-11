use axum::{
    extract::{Extension, State},
    Json,
};
use ramp_common::types::*;
use ramp_core::service::ledger::LedgerService;
use serde_json::json;
use std::sync::Arc;

use crate::dto::{BalanceDto, UserBalancesResponse};
use crate::error::ApiError;
use crate::middleware::TenantContext;

pub type LedgerServiceState = Arc<LedgerService>;

/// Get user balances
///
/// Retrieves current balances for a user across all currencies.
#[utoipa::path(
    get,
    path = "/v1/balance/{user_id}",
    tag = "users",
    params(
        ("user_id" = String, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User balances", body = UserBalancesResponse,
         example = json!({
             "balances": [
                 {"accountType": "SPOT", "currency": "VND", "balance": "15750000.00"},
                 {"accountType": "SPOT", "currency": "BTC", "balance": "0.05230000"},
                 {"accountType": "SPOT", "currency": "ETH", "balance": "1.25000000"}
             ]
         })),
        (status = 404, description = "User not found", body = ErrorResponse,
         example = json!({"error": {"code": "NOT_FOUND", "message": "User user_vn_12345 not found"}})),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_user_balances(
    State(service): State<LedgerServiceState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> Result<Json<UserBalancesResponse>, ApiError> {
    let balances = service
        .get_user_balances(&tenant_ctx.tenant_id, &UserId::new(&user_id))
        .await?;

    let balance_dtos: Vec<BalanceDto> = balances
        .into_iter()
        .map(|b| BalanceDto {
            account_type: b.account_type,
            currency: b.currency,
            balance: b.balance.to_string(),
        })
        .collect();

    Ok(Json(UserBalancesResponse {
        balances: balance_dtos,
    }))
}

/// GET /v1/users/{tenant_id}/{user_id}/balances - Alias for balance endpoint
#[utoipa::path(
    get,
    path = "/v1/users/{tenant_id}/{user_id}/balances",
    tag = "users",
    params(
        ("tenant_id" = String, Path, description = "Tenant ID"),
        ("user_id" = String, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User balances", body = UserBalancesResponse,
         example = json!({
             "balances": [
                 {"accountType": "SPOT", "currency": "VND", "balance": "15750000.00"},
                 {"accountType": "SPOT", "currency": "USDT", "balance": "500.00"}
             ]
         })),
        (status = 403, description = "Tenant mismatch", body = ErrorResponse,
         example = json!({"error": {"code": "FORBIDDEN", "message": "Tenant mismatch"}})),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_user_balances_for_tenant(
    State(service): State<LedgerServiceState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    axum::extract::Path((tenant_id, user_id)): axum::extract::Path<(String, String)>,
) -> Result<Json<UserBalancesResponse>, ApiError> {
    if tenant_id != tenant_ctx.tenant_id.0 {
        return Err(ApiError::Forbidden("Tenant mismatch".to_string()));
    }

    let balances = service
        .get_user_balances(&tenant_ctx.tenant_id, &UserId::new(&user_id))
        .await?;

    let balance_dtos: Vec<BalanceDto> = balances
        .into_iter()
        .map(|b| BalanceDto {
            account_type: b.account_type,
            currency: b.currency,
            balance: b.balance.to_string(),
        })
        .collect();

    Ok(Json(UserBalancesResponse {
        balances: balance_dtos,
    }))
}
