use axum::{
    extract::{Extension, Path, State},
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
use crate::middleware::tenant::TenantContext;
use ramp_core::service::onboarding::{ApiCredentials, OnboardingService, TenantBootstrapRequest};

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

/// Safe tenant representation for admin list — secrets omitted.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminTenantListItem {
    pub id: String,
    pub name: String,
    /// First 8 chars of the api_key_hash, safe to expose as a visual prefix.
    pub api_key_prefix: String,
    pub status: String,
    pub config: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
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
    super::tier::check_admin_key_operator(&headers)?;
    info!(name = %request.name, "Creating new tenant");

    let tenant = onboarding_service
        .bootstrap_tenant(TenantBootstrapRequest {
            name: request.name,
            config: request.config,
        })
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
    super::tier::check_admin_key_operator(&headers)?;
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
    super::tier::check_admin_key_operator(&headers)?;
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
    super::tier::check_admin_key_operator(&headers)?;
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
    super::tier::check_admin_key_operator(&headers)?;
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

/// GET /v1/admin/tenants - List all tenants (safe fields only, no secrets)
pub async fn list_tenants(
    headers: HeaderMap,
    Extension(_tenant_ctx): Extension<TenantContext>,
    State(app_state): State<crate::router::AppState>,
) -> Result<Json<Vec<AdminTenantListItem>>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!("Listing tenants");

    let ids = app_state
        .tenant_repo
        .list_ids()
        .await
        .map_err(ApiError::from)?;

    let mut items = Vec::with_capacity(ids.len());
    for id in &ids {
        if let Some(row) = app_state
            .tenant_repo
            .get_by_id(id)
            .await
            .map_err(ApiError::from)?
        {
            // Expose only the first 8 chars of the hash as a visual prefix; never the hash itself
            let api_key_prefix = row.api_key_hash.chars().take(8).collect::<String>();
            items.push(AdminTenantListItem {
                id: row.id,
                name: row.name,
                api_key_prefix,
                status: row.status,
                config: row.config,
                created_at: row.created_at.to_rfc3339(),
                updated_at: row.updated_at.to_rfc3339(),
            });
        }
    }

    Ok(Json(items))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use ramp_core::{
        repository::{tenant::TenantRow, TenantRepository},
        service::ledger::LedgerService,
        test_utils::{MockLedgerRepository, MockTenantRepository},
    };
    use rust_decimal_macros::dec;
    use std::sync::Mutex;

    fn env_lock() -> &'static Mutex<()> {
        // Single process-wide lock shared by all admin env-mutating tests.
        crate::handlers::admin::admin_env_lock()
    }

    #[tokio::test]
    async fn test_update_tenant_rejects_viewer_role() {
        let _guard = env_lock().lock().unwrap();

        let tenant_repo = Arc::new(MockTenantRepository::new());
        tenant_repo.add_tenant(TenantRow {
            id: "tenant-1".to_string(),
            name: "Tenant 1".to_string(),
            status: "ACTIVE".to_string(),
            api_key_hash: "hash".to_string(),
            api_secret_encrypted: None,
            webhook_secret_hash: "webhook-hash".to_string(),
            webhook_secret_encrypted: None,
            webhook_url: None,
            config: serde_json::json!({}),
            daily_payin_limit_vnd: Some(dec!(1000)),
            daily_payout_limit_vnd: Some(dec!(500)),
            api_version: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });

        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let onboarding_service = Arc::new(OnboardingService::new(
            tenant_repo.clone(),
            Arc::new(LedgerService::new(ledger_repo)),
        ));

        let mut headers = HeaderMap::new();
        headers.insert("X-Admin-Key", "onboarding-viewer-key".parse().unwrap());

        let request = UpdateTenantRequest {
            webhook_url: Some("https://example.com/webhook".to_string()),
            daily_payin_limit_vnd: None,
            daily_payout_limit_vnd: None,
        };

        std::env::set_var("RAMPOS_ADMIN_KEY", "onboarding-viewer-key");
        std::env::set_var("RAMPOS_ADMIN_ROLE", "viewer");
        let err = update_tenant(
            headers,
            State(onboarding_service),
            Path("tenant-1".to_string()),
            ValidatedJson(request),
        )
        .await
        .unwrap_err();

        match err {
            ApiError::Forbidden(_) => {}
            other => panic!("expected forbidden error, got {other:?}"),
        }

        let stored = tenant_repo
            .get_by_id(&TenantId::new("tenant-1"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.webhook_url, None);

        std::env::remove_var("RAMPOS_ADMIN_KEY");
        std::env::remove_var("RAMPOS_ADMIN_ROLE");
    }
}
