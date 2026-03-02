//! Domain Management API Handlers
//!
//! Endpoints for custom domain management including:
//! - List domains for a tenant
//! - Register new custom domains
//! - Get domain details
//! - Delete domains
//! - Trigger DNS verification
//! - Trigger SSL provisioning

use axum::{
    extract::{Extension, Path, State},
    http::HeaderMap,
    Json,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use ramp_core::domain::dns::DnsVerificationStatus;
use ramp_core::domain::{
    CustomDomain, DomainService, DomainServiceConfig, DomainStatus, InMemoryDomainStore,
    LetsEncryptProvider, RegisterDomainRequest, SslCertificate, SslCertificateInfo,
};

type DefaultDomainService =
    DomainService<InMemoryDomainStore, ramp_core::domain::dns::SystemDnsProvider, LetsEncryptProvider>;

static DOMAIN_SERVICE: Lazy<Arc<DefaultDomainService>> = Lazy::new(|| {
    let config = DomainServiceConfig {
        letsencrypt_email: std::env::var("LETSENCRYPT_EMAIL").unwrap_or_default(),
        letsencrypt_staging: std::env::var("LETSENCRYPT_STAGING")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(true),
        ..DomainServiceConfig::default()
    };

    Arc::new(DomainService::new(
        config,
        Arc::new(InMemoryDomainStore::new()),
        Arc::new(ramp_core::domain::dns::SystemDnsProvider::new()),
        Arc::new(LetsEncryptProvider::new(
            std::env::var("LETSENCRYPT_STAGING")
                .map(|v| v != "false" && v != "0")
                .unwrap_or(true),
        )),
    ))
});

fn domain_service() -> &'static Arc<DefaultDomainService> {
    &DOMAIN_SERVICE
}

fn domain_status_to_string(status: DomainStatus) -> String {
    serde_json::to_value(status)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "unknown".to_string())
}

fn map_ssl_certificate_info(info: &SslCertificateInfo) -> SslCertificateInfoResponse {
    SslCertificateInfoResponse {
        certificate_id: info.certificate_id.clone(),
        issuer: info.issuer.clone(),
        valid_from: info.valid_from.to_rfc3339(),
        valid_until: info.valid_until.to_rfc3339(),
        days_until_expiry: info.days_until_expiry,
        auto_renew: info.auto_renew,
    }
}

fn map_custom_domain(domain: CustomDomain) -> DomainResponse {
    DomainResponse {
        id: domain.id,
        tenant_id: domain.tenant_id,
        domain: domain.domain,
        status: domain_status_to_string(domain.status),
        dns_verification_token: domain.dns_verification_token,
        dns_verification_record: domain.dns_verification_record,
        ssl_certificate: domain.ssl_certificate.as_ref().map(map_ssl_certificate_info),
        health_check_path: domain.health_check_path,
        is_primary: domain.is_primary,
        custom_headers: domain.custom_headers,
        created_at: domain.created_at.to_rfc3339(),
        updated_at: domain.updated_at.to_rfc3339(),
    }
}

fn map_ssl_provisioning_response(domain_id: String, cert: SslCertificate) -> SslProvisioningResponse {
    SslProvisioningResponse {
        domain_id,
        status: "active".to_string(),
        message: "SSL certificate provisioned successfully".to_string(),
        certificate_id: Some(cert.certificate_id),
        valid_until: Some(cert.valid_until.to_rfc3339()),
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateDomainRequest {
    pub domain: String,
    #[serde(default)]
    pub is_primary: bool,
    pub health_check_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DomainResponse {
    pub id: String,
    pub tenant_id: String,
    pub domain: String,
    pub status: String,
    pub dns_verification_token: Option<String>,
    pub dns_verification_record: Option<String>,
    pub ssl_certificate: Option<SslCertificateInfoResponse>,
    pub health_check_path: String,
    pub is_primary: bool,
    pub custom_headers: HashMap<String, String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SslCertificateInfoResponse {
    pub certificate_id: String,
    pub issuer: String,
    pub valid_from: String,
    pub valid_until: String,
    pub days_until_expiry: i64,
    pub auto_renew: bool,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DomainListResponse {
    pub domains: Vec<DomainResponse>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DnsVerificationResponse {
    pub domain_id: String,
    pub status: String,
    pub message: String,
    pub verified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SslProvisioningResponse {
    pub domain_id: String,
    pub status: String,
    pub message: String,
    pub certificate_id: Option<String>,
    pub valid_until: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteDomainResponse {
    pub id: String,
    pub deleted: bool,
    pub message: String,
}

#[utoipa::path(
    get,
    path = "/v1/admin/domains",
    tag = "domains",
    responses(
        (status = 200, description = "List of domains", body = DomainListResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_domains(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(_app_state): State<crate::router::AppState>,
) -> Result<Json<DomainListResponse>, ApiError> {
    crate::handlers::admin::tier::check_admin_key(&headers)?;

    info!(tenant = %tenant_ctx.tenant_id.0, "Listing domains for tenant");

    let domains = domain_service()
        .list_by_tenant(&tenant_ctx.tenant_id.0)
        .await
        .map_err(ApiError::from)?;

    let domains: Vec<DomainResponse> = domains.into_iter().map(map_custom_domain).collect();
    let total = domains.len();
    Ok(Json(DomainListResponse { domains, total }))
}

#[utoipa::path(
    post,
    path = "/v1/admin/domains",
    tag = "domains",
    request_body = CreateDomainRequest,
    responses(
        (status = 200, description = "Domain registered", body = DomainResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_domain(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(_app_state): State<crate::router::AppState>,
    Json(request): Json<CreateDomainRequest>,
) -> Result<Json<DomainResponse>, ApiError> {
    crate::handlers::admin::tier::check_admin_key_operator(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        domain = %request.domain,
        "Registering new custom domain"
    );

    let payload = RegisterDomainRequest {
        tenant_id: tenant_ctx.tenant_id.0.clone(),
        domain: request.domain,
        is_primary: request.is_primary,
        health_check_path: request.health_check_path,
    };

    let created = domain_service()
        .register(payload)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(map_custom_domain(created)))
}

#[utoipa::path(
    get,
    path = "/v1/admin/domains/{domain_id}",
    tag = "domains",
    params(
        ("domain_id" = String, Path, description = "Domain ID")
    ),
    responses(
        (status = 200, description = "Domain details", body = DomainResponse),
        (status = 404, description = "Domain not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_domain(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(_app_state): State<crate::router::AppState>,
    Path(domain_id): Path<String>,
) -> Result<Json<DomainResponse>, ApiError> {
    crate::handlers::admin::tier::check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        domain_id = %domain_id,
        "Fetching domain details"
    );

    let domain = domain_service()
        .get(&domain_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("Domain {} not found", domain_id)))?;

    if domain.tenant_id != tenant_ctx.tenant_id.0 {
        return Err(ApiError::NotFound(format!("Domain {} not found", domain_id)));
    }

    Ok(Json(map_custom_domain(domain)))
}

#[utoipa::path(
    delete,
    path = "/v1/admin/domains/{domain_id}",
    tag = "domains",
    params(
        ("domain_id" = String, Path, description = "Domain ID")
    ),
    responses(
        (status = 200, description = "Domain deleted", body = DeleteDomainResponse),
        (status = 404, description = "Domain not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_domain(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(_app_state): State<crate::router::AppState>,
    Path(domain_id): Path<String>,
) -> Result<Json<DeleteDomainResponse>, ApiError> {
    crate::handlers::admin::tier::check_admin_key_operator(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        domain_id = %domain_id,
        "Deleting domain"
    );

    let existing = domain_service()
        .get(&domain_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("Domain {} not found", domain_id)))?;

    if existing.tenant_id != tenant_ctx.tenant_id.0 {
        return Err(ApiError::NotFound(format!("Domain {} not found", domain_id)));
    }

    domain_service()
        .delete(&domain_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(DeleteDomainResponse {
        id: domain_id,
        deleted: true,
        message: "Domain deleted successfully".to_string(),
    }))
}

#[utoipa::path(
    post,
    path = "/v1/admin/domains/{domain_id}/verify-dns",
    tag = "domains",
    params(
        ("domain_id" = String, Path, description = "Domain ID")
    ),
    responses(
        (status = 200, description = "DNS verification initiated", body = DnsVerificationResponse),
        (status = 404, description = "Domain not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn verify_dns(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(_app_state): State<crate::router::AppState>,
    Path(domain_id): Path<String>,
) -> Result<Json<DnsVerificationResponse>, ApiError> {
    crate::handlers::admin::tier::check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        domain_id = %domain_id,
        "Triggering DNS verification"
    );

    let existing = domain_service()
        .get(&domain_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("Domain {} not found", domain_id.clone())))?;

    if existing.tenant_id != tenant_ctx.tenant_id.0 {
        return Err(ApiError::NotFound(format!("Domain {} not found", domain_id)));
    }

    let verification = domain_service()
        .verify_dns(&domain_id)
        .await
        .map_err(ApiError::from)?;

    let (status, message, verified_at) = match verification.status {
        DnsVerificationStatus::Verified => (
            "verified".to_string(),
            "DNS verification successful".to_string(),
            Some(verification.verified_at.to_rfc3339()),
        ),
        DnsVerificationStatus::Pending => (
            "pending".to_string(),
            verification
                .error
                .unwrap_or_else(|| "DNS verification pending".to_string()),
            None,
        ),
        DnsVerificationStatus::Failed => (
            "failed".to_string(),
            verification
                .error
                .unwrap_or_else(|| "DNS verification failed".to_string()),
            None,
        ),
    };

    Ok(Json(DnsVerificationResponse {
        domain_id,
        status,
        message,
        verified_at,
    }))
}

#[utoipa::path(
    post,
    path = "/v1/admin/domains/{domain_id}/provision-ssl",
    tag = "domains",
    params(
        ("domain_id" = String, Path, description = "Domain ID")
    ),
    responses(
        (status = 200, description = "SSL provisioning initiated", body = SslProvisioningResponse),
        (status = 404, description = "Domain not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn provision_ssl(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(_app_state): State<crate::router::AppState>,
    Path(domain_id): Path<String>,
) -> Result<Json<SslProvisioningResponse>, ApiError> {
    crate::handlers::admin::tier::check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        domain_id = %domain_id,
        "Triggering SSL provisioning"
    );

    let existing = domain_service()
        .get(&domain_id)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("Domain {} not found", domain_id.clone())))?;

    if existing.tenant_id != tenant_ctx.tenant_id.0 {
        return Err(ApiError::NotFound(format!("Domain {} not found", domain_id)));
    }

    let cert = domain_service()
        .provision_ssl(&domain_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(map_ssl_provisioning_response(domain_id, cert)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_domain_request_deserialize() {
        let json = r#"{"domain": "app.example.com", "isPrimary": true}"#;
        let request: CreateDomainRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.domain, "app.example.com");
        assert!(request.is_primary);
        assert!(request.health_check_path.is_none());
    }

    #[test]
    fn test_create_domain_request_defaults() {
        let json = r#"{"domain": "app.example.com"}"#;
        let request: CreateDomainRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.domain, "app.example.com");
        assert!(!request.is_primary);
    }
}
