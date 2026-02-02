use crate::middleware::tenant::TenantContext;
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, TimeZone, Utc};
use ramp_common::types::TenantId;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use ramp_core::repository::tenant::TenantRepository;

const MAX_PAST_DRIFT_SECONDS: i64 = 300; // 5 minutes
const MAX_FUTURE_DRIFT_SECONDS: i64 = 60; // 1 minute

#[derive(Debug, PartialEq)]
enum TimestampValidationError {
    Missing,
    InvalidFormat,
    Expired,
    Future,
}

/// Authentication middleware
pub async fn auth_middleware(
    State(tenant_repo): State<Arc<dyn TenantRepository>>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Check timestamp first to fail fast
    if let Err(e) = validate_timestamp(req.headers()) {
        let response = match e {
            TimestampValidationError::Missing => (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "missing_timestamp",
                    "message": "X-Timestamp header is missing"
                })),
            )
                .into_response(),
            TimestampValidationError::InvalidFormat => (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "invalid_format",
                    "message": "Invalid timestamp format"
                })),
            )
                .into_response(),
            TimestampValidationError::Expired | TimestampValidationError::Future => (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "timestamp_expired",
                    "message": "Request timestamp is outside acceptable range",
                    "server_time": Utc::now().to_rfc3339()
                })),
            )
                .into_response(),
        };
        return Ok(response);
    }

    // Extract API key from Authorization header
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok());

    let api_key = match auth_header {
        Some(header) if header.starts_with("Bearer ") => &header[7..],
        _ => return Err(StatusCode::UNAUTHORIZED),
    };

    // Hash the API key for lookup
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let api_key_hash = hex::encode(hasher.finalize());

    // Look up tenant
    let tenant = tenant_repo
        .get_by_api_key_hash(&api_key_hash)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if tenant.status != "ACTIVE" {
        return Err(StatusCode::FORBIDDEN);
    }

    // Add tenant context to request extensions
    let context = TenantContext {
        tenant_id: TenantId::new(&tenant.id),
        name: tenant.name,
    };
    req.extensions_mut().insert(context);

    Ok(next.run(req).await)
}

fn validate_timestamp(headers: &HeaderMap) -> Result<(), TimestampValidationError> {
    let timestamp_str = headers
        .get("X-Timestamp")
        .ok_or(TimestampValidationError::Missing)?
        .to_str()
        .map_err(|_| TimestampValidationError::InvalidFormat)?;

    // Try parsing as ISO8601 first
    let timestamp = if let Ok(dt) = DateTime::parse_from_rfc3339(timestamp_str) {
        dt.with_timezone(&Utc)
    } else {
        // Try parsing as Unix timestamp (seconds or milliseconds)
        let ts_val = timestamp_str
            .parse::<i64>()
            .map_err(|_| TimestampValidationError::InvalidFormat)?;

        // Simple heuristic: if > 10^11, likely milliseconds (valid until year 5138)
        if ts_val > 100_000_000_000 {
            Utc.timestamp_millis_opt(ts_val)
                .single()
                .ok_or(TimestampValidationError::InvalidFormat)?
        } else {
            Utc.timestamp_opt(ts_val, 0)
                .single()
                .ok_or(TimestampValidationError::InvalidFormat)?
        }
    };

    let now = Utc::now();
    let drift = now.signed_duration_since(timestamp).num_seconds();

    // Positive drift means timestamp is in the past
    // Negative drift means timestamp is in the future

    if drift > MAX_PAST_DRIFT_SECONDS {
        return Err(TimestampValidationError::Expired);
    }

    if drift < -MAX_FUTURE_DRIFT_SECONDS {
        return Err(TimestampValidationError::Future);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    // use tower::ServiceExt; // Unused

    #[test]
    fn test_validate_timestamp_iso8601() {
        let mut headers = HeaderMap::new();
        let now = Utc::now();
        headers.insert("X-Timestamp", now.to_rfc3339().parse().unwrap());
        assert_eq!(validate_timestamp(&headers), Ok(()));

        // Too old
        let past = now - chrono::Duration::seconds(301);
        headers.insert("X-Timestamp", past.to_rfc3339().parse().unwrap());
        assert_eq!(
            validate_timestamp(&headers),
            Err(TimestampValidationError::Expired)
        );

        // Future > 1 min (use 65 seconds to account for test execution time)
        let future = now + chrono::Duration::seconds(65);
        headers.insert("X-Timestamp", future.to_rfc3339().parse().unwrap());
        assert_eq!(
            validate_timestamp(&headers),
            Err(TimestampValidationError::Future)
        );
    }

    #[test]
    fn test_validate_timestamp_unix() {
        let mut headers = HeaderMap::new();
        let now = Utc::now();

        // Seconds
        headers.insert("X-Timestamp", now.timestamp().to_string().parse().unwrap());
        assert_eq!(validate_timestamp(&headers), Ok(()));

        // Milliseconds
        headers.insert(
            "X-Timestamp",
            now.timestamp_millis().to_string().parse().unwrap(),
        );
        assert_eq!(validate_timestamp(&headers), Ok(()));
    }

    #[test]
    fn test_validate_timestamp_errors() {
        let mut headers = HeaderMap::new();

        // Missing
        assert_eq!(
            validate_timestamp(&headers),
            Err(TimestampValidationError::Missing)
        );

        // Invalid format
        headers.insert("X-Timestamp", "invalid".parse().unwrap());
        assert_eq!(
            validate_timestamp(&headers),
            Err(TimestampValidationError::InvalidFormat)
        );
    }
}
