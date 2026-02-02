use axum::{extract::Request, middleware::Next, response::Response};
use ramp_common::types::TenantId;
use tracing::warn;

/// Tenant context extracted from authentication
#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant_id: TenantId,
    pub name: String,
}

/// Extract tenant context from request extensions
pub fn extract_tenant_context(req: &Request) -> Option<&TenantContext> {
    req.extensions().get::<TenantContext>()
}

/// Middleware to ensure tenant context is present
pub async fn require_tenant(req: Request, next: Next) -> Result<Response, axum::http::StatusCode> {
    if extract_tenant_context(&req).is_none() {
        warn!("Request rejected: missing tenant context");
        return Err(axum::http::StatusCode::UNAUTHORIZED);
    }

    // Additional check: Ensure tenant_id in header matches context if provided (defense in depth)
    if let Some(header_tenant) = req.headers().get("X-Tenant-ID") {
        if let Ok(header_val) = header_tenant.to_str() {
            if let Some(ctx) = extract_tenant_context(&req) {
                if ctx.tenant_id.0 != header_val {
                    warn!(
                        "Request rejected: tenant ID mismatch (header: {}, ctx: {})",
                        header_val, ctx.tenant_id.0
                    );
                    return Err(axum::http::StatusCode::FORBIDDEN);
                }
            }
        }
    }

    Ok(next.run(req).await)
}
