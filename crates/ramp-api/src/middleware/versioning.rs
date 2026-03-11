//! Version negotiation middleware
//!
//! Reads the `RampOS-Version` header from incoming requests, validates it,
//! and injects the resolved `ApiVersion` into request extensions.
//!
//! Fallback priority:
//! 1. `RampOS-Version` header on the request
//! 2. Tenant's pinned API version (from `TenantContext`)
//! 3. Default version (`2026-02-01`)

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use tracing::{debug, warn};

use crate::versioning::version::{ApiVersion, LATEST_VERSION, MINIMUM_VERSION};

/// The HTTP header name for specifying the API version.
pub const VERSION_HEADER: &str = "RampOS-Version";

/// The response header echoing back the resolved API version.
pub const VERSION_RESPONSE_HEADER: &str = "RampOS-Version";

/// Context injected into request extensions after version negotiation.
#[derive(Debug, Clone)]
pub struct VersionContext {
    /// The resolved API version for this request.
    pub version: ApiVersion,
    /// Where the version came from.
    pub source: VersionSource,
}

/// Indicates how the API version was determined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionSource {
    /// Explicitly set via the `RampOS-Version` header.
    Header,
    /// From the tenant's pinned version (stored in `TenantContext`).
    TenantPinned,
    /// Fell back to the system default.
    Default,
}

/// Context that can be set on a `TenantContext` to indicate a pinned API version.
/// This is read by the versioning middleware as a fallback when no header is present.
#[derive(Debug, Clone)]
pub struct TenantApiVersion {
    pub version: ApiVersion,
}

#[derive(Serialize)]
struct VersionErrorResponse {
    error: VersionErrorBody,
}

#[derive(Serialize)]
struct VersionErrorBody {
    code: String,
    message: String,
    minimum_version: String,
    latest_version: String,
}

/// Middleware that negotiates the API version for each request.
///
/// Resolution order:
/// 1. `RampOS-Version` header
/// 2. `TenantApiVersion` from request extensions (set by auth middleware)
/// 3. System default version
///
/// If the resolved version is invalid or unsupported, returns 400 Bad Request.
/// The resolved version is injected as `VersionContext` into request extensions
/// and echoed back in the `RampOS-Version` response header.
pub async fn version_negotiation_middleware(
    req: Request,
    next: Next,
) -> Result<Response, Response> {
    let (version, source) = resolve_version(&req)?;

    debug!(
        version = %version,
        source = ?source,
        "API version resolved"
    );

    // Inject version context into request extensions
    let mut req = req;
    req.extensions_mut().insert(VersionContext {
        version: version.clone(),
        source,
    });

    // Run the next handler
    let mut response = next.run(req).await;

    // Echo the resolved version in the response header
    if let Ok(header_value) = version.to_string().parse() {
        response
            .headers_mut()
            .insert(VERSION_RESPONSE_HEADER, header_value);
    }

    Ok(response)
}

/// Resolve the API version from the request, returning the version and its source.
fn resolve_version(req: &Request) -> Result<(ApiVersion, VersionSource), Response> {
    // 1. Check for explicit RampOS-Version header
    if let Some(header_value) = req.headers().get(VERSION_HEADER) {
        let version_str = header_value.to_str().unwrap_or("");

        let version = ApiVersion::parse(version_str).map_err(|_| {
            build_version_error(
                StatusCode::BAD_REQUEST,
                "INVALID_VERSION_FORMAT",
                &format!(
                    "Invalid API version format: '{}'. Expected YYYY-MM-DD.",
                    version_str
                ),
            )
        })?;

        // Check that version is within supported range
        if !version.is_compatible() {
            if version < ApiVersion::minimum() {
                return Err(build_version_error(
                    StatusCode::BAD_REQUEST,
                    "VERSION_TOO_OLD",
                    &format!(
                        "API version '{}' is no longer supported. Minimum supported version: {}",
                        version, MINIMUM_VERSION
                    ),
                ));
            } else {
                return Err(build_version_error(
                    StatusCode::BAD_REQUEST,
                    "VERSION_UNKNOWN",
                    &format!(
                        "Unknown API version '{}'. Latest version: {}",
                        version, LATEST_VERSION
                    ),
                ));
            }
        }

        return Ok((version, VersionSource::Header));
    }

    // 2. Check for tenant's pinned version in extensions
    if let Some(tenant_version) = req.extensions().get::<TenantApiVersion>() {
        let version = tenant_version.version.clone();
        if version.is_compatible() {
            return Ok((version, VersionSource::TenantPinned));
        }
        // If tenant's pinned version is incompatible, log and fall through to default
        warn!(
            tenant_version = %version,
            "Tenant's pinned API version is incompatible, falling back to default"
        );
    }

    // 3. Fall back to default
    Ok((ApiVersion::default_version(), VersionSource::Default))
}

/// Build a JSON error response for version negotiation failures.
fn build_version_error(status: StatusCode, code: &str, message: &str) -> Response {
    let body = VersionErrorResponse {
        error: VersionErrorBody {
            code: code.to_string(),
            message: message.to_string(),
            minimum_version: MINIMUM_VERSION.to_string(),
            latest_version: LATEST_VERSION.to_string(),
        },
    };
    (status, Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body, http::Request as HttpRequest, middleware::from_fn, routing::get, Router,
    };
    use tower::ServiceExt;

    fn test_app() -> Router {
        Router::new()
            .route("/test", get(|| async { "ok" }))
            .layer(from_fn(version_negotiation_middleware))
    }

    #[tokio::test]
    async fn test_header_version_extraction() {
        let app = test_app();
        let req = HttpRequest::builder()
            .uri("/test")
            .header(VERSION_HEADER, "2026-02-01")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(VERSION_RESPONSE_HEADER).unwrap(),
            "2026-02-01"
        );
    }

    #[tokio::test]
    async fn test_default_version_when_no_header() {
        let app = test_app();
        let req = HttpRequest::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(VERSION_RESPONSE_HEADER).unwrap(),
            "2026-02-01"
        );
    }

    #[tokio::test]
    async fn test_invalid_version_format_returns_400() {
        let app = test_app();
        let req = HttpRequest::builder()
            .uri("/test")
            .header(VERSION_HEADER, "invalid")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["code"], "INVALID_VERSION_FORMAT");
    }

    #[tokio::test]
    async fn test_too_old_version_returns_400() {
        let app = test_app();
        let req = HttpRequest::builder()
            .uri("/test")
            .header(VERSION_HEADER, "2020-01-01")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["code"], "VERSION_TOO_OLD");
    }

    #[tokio::test]
    async fn test_future_version_returns_400() {
        let app = test_app();
        let req = HttpRequest::builder()
            .uri("/test")
            .header(VERSION_HEADER, "2030-01-01")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["code"], "VERSION_UNKNOWN");
    }

    #[tokio::test]
    async fn test_tenant_pinned_version_fallback() {
        let app = Router::new()
            .route("/test", get(|| async { "ok" }))
            .layer(from_fn(version_negotiation_middleware));

        let mut req = HttpRequest::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        // Inject tenant pinned version
        req.extensions_mut().insert(TenantApiVersion {
            version: ApiVersion::parse("2026-03-01").unwrap(),
        });

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(VERSION_RESPONSE_HEADER).unwrap(),
            "2026-03-01"
        );
    }

    #[tokio::test]
    async fn test_header_takes_precedence_over_tenant_pinned() {
        let app = Router::new()
            .route("/test", get(|| async { "ok" }))
            .layer(from_fn(version_negotiation_middleware));

        let mut req = HttpRequest::builder()
            .uri("/test")
            .header(VERSION_HEADER, "2026-02-01")
            .body(Body::empty())
            .unwrap();

        // Inject tenant pinned version (should be overridden by header)
        req.extensions_mut().insert(TenantApiVersion {
            version: ApiVersion::parse("2026-03-01").unwrap(),
        });

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(VERSION_RESPONSE_HEADER).unwrap(),
            "2026-02-01"
        );
    }

    #[tokio::test]
    async fn test_latest_version_accepted() {
        let app = test_app();
        let req = HttpRequest::builder()
            .uri("/test")
            .header(VERSION_HEADER, "2026-03-01")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(VERSION_RESPONSE_HEADER).unwrap(),
            "2026-03-01"
        );
    }

    #[tokio::test]
    async fn test_error_response_includes_version_info() {
        let app = test_app();
        let req = HttpRequest::builder()
            .uri("/test")
            .header(VERSION_HEADER, "bad-version")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        // Error response should include version guidance
        assert!(json["error"]["minimum_version"].is_string());
        assert!(json["error"]["latest_version"].is_string());
    }

    #[tokio::test]
    async fn test_incompatible_tenant_version_falls_to_default() {
        let app = Router::new()
            .route("/test", get(|| async { "ok" }))
            .layer(from_fn(version_negotiation_middleware));

        let mut req = HttpRequest::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        // Inject an incompatible tenant version
        req.extensions_mut().insert(TenantApiVersion {
            version: ApiVersion::parse("2020-01-01").unwrap(),
        });

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        // Should fall back to default
        assert_eq!(
            resp.headers().get(VERSION_RESPONSE_HEADER).unwrap(),
            "2026-02-01"
        );
    }
}
