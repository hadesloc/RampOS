//! Custom Domain API Handlers

use axum::{
    extract::{Path, State},
    Json,
};
use ramp_common::Result;
use ramp_core::domain::{
    CustomDomain, RegisterDomainRequest, UpdateDomainRequest, DnsVerification, SslCertificate
};
use std::sync::Arc;

use crate::AppState;

/// Register a new custom domain
pub async fn register_domain(
    State(state): State<AppState>,
    Json(payload): Json<RegisterDomainRequest>,
) -> Result<Json<CustomDomain>> {
    let domain = state.domain_service.register(payload).await?;
    Ok(Json(domain))
}

/// List domains for a tenant
pub async fn list_domains(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
) -> Result<Json<Vec<CustomDomain>>> {
    let domains = state.domain_service.list_by_tenant(&tenant_id).await?;
    Ok(Json(domains))
}

/// Get domain details
pub async fn get_domain(
    State(state): State<AppState>,
    Path(domain_id): Path<String>,
) -> Result<Json<CustomDomain>> {
    let domain = state.domain_service.get(&domain_id).await?;
    if let Some(d) = domain {
        Ok(Json(d))
    } else {
        Err(ramp_common::Error::NotFound(format!("Domain {} not found", domain_id)))
    }
}

/// Verify DNS configuration
pub async fn verify_dns(
    State(state): State<AppState>,
    Path(domain_id): Path<String>,
) -> Result<Json<DnsVerification>> {
    let result = state.domain_service.verify_dns(&domain_id).await?;
    Ok(Json(result))
}

/// Provision SSL certificate
pub async fn provision_ssl(
    State(state): State<AppState>,
    Path(domain_id): Path<String>,
) -> Result<Json<SslCertificate>> {
    let cert = state.domain_service.provision_ssl(&domain_id).await?;
    Ok(Json(cert))
}

/// Update domain configuration
pub async fn update_domain(
    State(state): State<AppState>,
    Path(domain_id): Path<String>,
    Json(payload): Json<UpdateDomainRequest>,
) -> Result<Json<CustomDomain>> {
    let domain = state.domain_service.update(&domain_id, payload).await?;
    Ok(Json(domain))
}

/// Delete domain
pub async fn delete_domain(
    State(state): State<AppState>,
    Path(domain_id): Path<String>,
) -> Result<Json<()>> {
    state.domain_service.delete(&domain_id).await?;
    Ok(Json(()))
}
