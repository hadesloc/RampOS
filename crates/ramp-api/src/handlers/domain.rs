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
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;

// ============================================================================
// Request/Response DTOs
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDomainRequest {
    /// Domain name to register (e.g., "app.example.com")
    pub domain: String,
    /// Whether this should be the primary domain for the tenant
    #[serde(default)]
    pub is_primary: bool,
    /// Custom health check path (defaults to "/health")
    pub health_check_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SslCertificateInfoResponse {
    pub certificate_id: String,
    pub issuer: String,
    pub valid_from: String,
    pub valid_until: String,
    pub days_until_expiry: i64,
    pub auto_renew: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainListResponse {
    pub domains: Vec<DomainResponse>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsVerificationResponse {
    pub domain_id: String,
    pub status: String,
    pub message: String,
    pub verified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SslProvisioningResponse {
    pub domain_id: String,
    pub status: String,
    pub message: String,
    pub certificate_id: Option<String>,
    pub valid_until: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteDomainResponse {
    pub id: String,
    pub deleted: bool,
    pub message: String,
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /v1/admin/domains - List all domains for the authenticated tenant
pub async fn list_domains(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(_app_state): State<crate::router::AppState>,
) -> Result<Json<DomainListResponse>, ApiError> {
    crate::handlers::admin::tier::check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        "Listing domains for tenant"
    );

    // Return placeholder response - real DomainService integration comes later
    let now = Utc::now();
    let domains = vec![DomainResponse {
        id: "dom_placeholder".to_string(),
        tenant_id: tenant_ctx.tenant_id.0.clone(),
        domain: "app.example.com".to_string(),
        status: "active".to_string(),
        dns_verification_token: None,
        dns_verification_record: None,
        ssl_certificate: Some(SslCertificateInfoResponse {
            certificate_id: "cert_placeholder".to_string(),
            issuer: "Let's Encrypt".to_string(),
            valid_from: now.to_rfc3339(),
            valid_until: (now + chrono::Duration::days(90)).to_rfc3339(),
            days_until_expiry: 90,
            auto_renew: true,
        }),
        health_check_path: "/health".to_string(),
        is_primary: true,
        custom_headers: HashMap::new(),
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
    }];

    let total = domains.len();
    Ok(Json(DomainListResponse { domains, total }))
}

/// POST /v1/admin/domains - Register a new custom domain
pub async fn create_domain(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(_app_state): State<crate::router::AppState>,
    Json(request): Json<CreateDomainRequest>,
) -> Result<Json<DomainResponse>, ApiError> {
    crate::handlers::admin::tier::check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        domain = %request.domain,
        "Registering new custom domain"
    );

    // Basic domain validation
    if request.domain.is_empty() {
        return Err(ApiError::Validation("Domain name cannot be empty".to_string()));
    }

    if !request.domain.contains('.') {
        return Err(ApiError::Validation(
            "Domain must include TLD (e.g., example.com)".to_string(),
        ));
    }

    let now = Utc::now();
    let verification_token = format!("ramp-verify-placeholder-{}", &tenant_ctx.tenant_id.0);
    let verification_record = format!("_ramp-verify.{}", request.domain);

    // Return placeholder response
    Ok(Json(DomainResponse {
        id: format!("dom_{}", uuid::Uuid::new_v4()),
        tenant_id: tenant_ctx.tenant_id.0.clone(),
        domain: request.domain,
        status: "pending_dns_verification".to_string(),
        dns_verification_token: Some(verification_token),
        dns_verification_record: Some(verification_record),
        ssl_certificate: None,
        health_check_path: request.health_check_path.unwrap_or_else(|| "/health".to_string()),
        is_primary: request.is_primary,
        custom_headers: HashMap::new(),
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
    }))
}

/// GET /v1/admin/domains/:domain_id - Get domain details
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

    let now = Utc::now();

    // Return placeholder response
    Ok(Json(DomainResponse {
        id: domain_id,
        tenant_id: tenant_ctx.tenant_id.0.clone(),
        domain: "app.example.com".to_string(),
        status: "active".to_string(),
        dns_verification_token: None,
        dns_verification_record: None,
        ssl_certificate: Some(SslCertificateInfoResponse {
            certificate_id: "cert_placeholder".to_string(),
            issuer: "Let's Encrypt".to_string(),
            valid_from: now.to_rfc3339(),
            valid_until: (now + chrono::Duration::days(90)).to_rfc3339(),
            days_until_expiry: 90,
            auto_renew: true,
        }),
        health_check_path: "/health".to_string(),
        is_primary: true,
        custom_headers: HashMap::new(),
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
    }))
}

/// DELETE /v1/admin/domains/:domain_id - Delete a domain
pub async fn delete_domain(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(_app_state): State<crate::router::AppState>,
    Path(domain_id): Path<String>,
) -> Result<Json<DeleteDomainResponse>, ApiError> {
    crate::handlers::admin::tier::check_admin_key(&headers)?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        domain_id = %domain_id,
        "Deleting domain"
    );

    // Return placeholder response
    Ok(Json(DeleteDomainResponse {
        id: domain_id,
        deleted: true,
        message: "Domain deleted successfully".to_string(),
    }))
}

/// POST /v1/admin/domains/:domain_id/verify-dns - Trigger DNS verification
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

    // Return placeholder response
    Ok(Json(DnsVerificationResponse {
        domain_id,
        status: "pending".to_string(),
        message: "DNS verification initiated. Please ensure the TXT record is configured.".to_string(),
        verified_at: None,
    }))
}

/// POST /v1/admin/domains/:domain_id/provision-ssl - Trigger SSL provisioning
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

    // Return placeholder response
    Ok(Json(SslProvisioningResponse {
        domain_id,
        status: "provisioning".to_string(),
        message: "SSL certificate provisioning initiated via Let's Encrypt.".to_string(),
        certificate_id: None,
        valid_until: None,
    }))
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
