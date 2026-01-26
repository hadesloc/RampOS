use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use ramp_common::types::TenantId;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use ramp_core::service::onboarding::{ApiKeyPair, OnboardingService};

// ============================================================================
// DTOs
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTenantRequest {
    pub name: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TenantResponse {
    pub id: String,
    pub name: String,
    pub status: String,
    pub webhook_url: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTenantRequest {
    pub daily_payin_limit_vnd: Option<Decimal>,
    pub daily_payout_limit_vnd: Option<Decimal>,
    pub webhook_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuspendTenantRequest {
    pub reason: String,
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /v1/admin/tenants - Create a new tenant
pub async fn create_tenant(
    State(onboarding_service): State<Arc<OnboardingService>>,
    Json(request): Json<CreateTenantRequest>,
) -> Result<Json<TenantResponse>, ApiError> {
    info!(name = %request.name, "Creating new tenant");

    let tenant = onboarding_service
        .create_tenant(&request.name, request.config)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(TenantResponse {
        id: tenant.id,
        name: tenant.name,
        status: tenant.status,
        webhook_url: tenant.webhook_url,
        created_at: tenant.created_at.to_rfc3339(),
    }))
}

/// POST /v1/admin/tenants/:id/api-keys - Generate new API keys
pub async fn generate_api_keys(
    State(onboarding_service): State<Arc<OnboardingService>>,
    Path(tenant_id): Path<String>,
) -> Result<Json<ApiKeyPair>, ApiError> {
    info!(tenant_id = %tenant_id, "Generating API keys");

    let keys = onboarding_service
        .generate_api_keys(&TenantId::new(tenant_id))
        .await
        .map_err(ApiError::from)?;

    Ok(Json(keys))
}

/// POST /v1/admin/tenants/:id/activate - Activate tenant
pub async fn activate_tenant(
    State(onboarding_service): State<Arc<OnboardingService>>,
    Path(tenant_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    info!(tenant_id = %tenant_id, "Activating tenant");

    onboarding_service
        .activate_tenant(&TenantId::new(tenant_id))
        .await
        .map_err(ApiError::from)?;

    Ok(StatusCode::OK)
}

/// POST /v1/admin/tenants/:id/suspend - Suspend tenant
pub async fn suspend_tenant(
    State(onboarding_service): State<Arc<OnboardingService>>,
    Path(tenant_id): Path<String>,
    Json(request): Json<SuspendTenantRequest>,
) -> Result<StatusCode, ApiError> {
    info!(tenant_id = %tenant_id, reason = %request.reason, "Suspending tenant");

    onboarding_service
        .suspend_tenant(&TenantId::new(tenant_id), &request.reason)
        .await
        .map_err(ApiError::from)?;

    Ok(StatusCode::OK)
}

/// PATCH /v1/admin/tenants/:id - Update tenant config
pub async fn update_tenant(
    State(onboarding_service): State<Arc<OnboardingService>>,
    Path(tenant_id): Path<String>,
    Json(request): Json<UpdateTenantRequest>,
) -> Result<StatusCode, ApiError> {
    info!(tenant_id = %tenant_id, "Updating tenant");

    let id = TenantId::new(tenant_id);

    if let Some(url) = request.webhook_url {
        onboarding_service
            .configure_webhooks(&id, &url)
            .await
            .map_err(ApiError::from)?;
    }

    if request.daily_payin_limit_vnd.is_some() || request.daily_payout_limit_vnd.is_some() {
        onboarding_service
            .set_limits(
                &id,
                request.daily_payin_limit_vnd,
                request.daily_payout_limit_vnd,
            )
            .await
            .map_err(ApiError::from)?;
    }

    Ok(StatusCode::OK)
}
