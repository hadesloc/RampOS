use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use ramp_common::types::TenantId;
use serde::Serialize;
use std::sync::Arc;
use tracing::info;

use crate::dto::{CreateTenantRequest, SuspendTenantRequest, UpdateTenantRequest};
use crate::error::ApiError;
use crate::extract::ValidatedJson;
use ramp_core::service::onboarding::{ApiCredentials, OnboardingService};

// ============================================================================
// Response DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TenantResponse {
    pub id: String,
    pub name: String,
    pub status: String,
    pub webhook_url: Option<String>,
    pub created_at: String,
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /v1/admin/tenants - Create a new tenant
pub async fn create_tenant(
    headers: HeaderMap,
    State(onboarding_service): State<Arc<OnboardingService>>,
    ValidatedJson(request): ValidatedJson<CreateTenantRequest>,
) -> Result<Json<TenantResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
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

/// POST /v1/admin/tenants/:id/api-keys - Generate new API credentials
pub async fn generate_api_keys(
    headers: HeaderMap,
    State(onboarding_service): State<Arc<OnboardingService>>,
    Path(tenant_id): Path<String>,
) -> Result<Json<ApiCredentials>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(tenant_id = %tenant_id, "Generating API credentials");

    let credentials = onboarding_service
        .generate_api_credentials(&TenantId::new(tenant_id))
        .await
        .map_err(ApiError::from)?;

    Ok(Json(credentials))
}

/// POST /v1/admin/tenants/:id/activate - Activate tenant
pub async fn activate_tenant(
    headers: HeaderMap,
    State(onboarding_service): State<Arc<OnboardingService>>,
    Path(tenant_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(tenant_id = %tenant_id, "Activating tenant");

    onboarding_service
        .activate_tenant(&TenantId::new(tenant_id))
        .await
        .map_err(ApiError::from)?;

    Ok(StatusCode::OK)
}

/// POST /v1/admin/tenants/:id/suspend - Suspend tenant
pub async fn suspend_tenant(
    headers: HeaderMap,
    State(onboarding_service): State<Arc<OnboardingService>>,
    Path(tenant_id): Path<String>,
    ValidatedJson(request): ValidatedJson<SuspendTenantRequest>,
) -> Result<StatusCode, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(tenant_id = %tenant_id, reason = %request.reason, "Suspending tenant");

    onboarding_service
        .suspend_tenant(&TenantId::new(tenant_id), &request.reason)
        .await
        .map_err(ApiError::from)?;

    Ok(StatusCode::OK)
}

/// PATCH /v1/admin/tenants/:id - Update tenant config
pub async fn update_tenant(
    headers: HeaderMap,
    State(onboarding_service): State<Arc<OnboardingService>>,
    Path(tenant_id): Path<String>,
    ValidatedJson(request): ValidatedJson<UpdateTenantRequest>,
) -> Result<StatusCode, ApiError> {
    super::tier::check_admin_key(&headers)?;
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
