//! Authentication middleware with HMAC signature verification
//!
//! This module provides authentication for SDK requests using:
//! 1. API key authentication (Bearer token)
//! 2. HMAC-SHA256 signature verification
//!
//! The signature format matches the SDK implementation:
//! `{method}\n{path}\n{timestamp}\n{body}`

use crate::middleware::tenant::TenantContext;
use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, TimeZone, Utc};
use hmac::{Hmac, Mac};
use http_body_util::BodyExt;
use ramp_common::types::TenantId;
use ramp_core::repository::tenant::TenantRepository;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use subtle::ConstantTimeEq;
use tracing::{debug, warn};

const MAX_PAST_DRIFT_SECONDS: i64 = 300; // 5 minutes
const MAX_FUTURE_DRIFT_SECONDS: i64 = 60; // 1 minute

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, PartialEq)]
enum TimestampValidationError {
    Missing,
    InvalidFormat,
    Expired,
    Future,
}

#[derive(Debug)]
enum SignatureValidationError {
    MissingSignature,
    MissingTimestamp,
    InvalidTimestamp,
    NoApiSecret,
    InvalidSignature,
}

/// Authentication middleware with HMAC signature verification
///
/// This middleware:
/// 1. Validates the X-Timestamp header (must be within 5 minutes)
/// 2. Extracts API key from Authorization: Bearer header
/// 3. Looks up tenant by hashed API key
/// 4. Verifies HMAC-SHA256 signature using tenant's api_secret
/// 5. Sets TenantContext in request extensions
pub async fn auth_middleware(
    State(tenant_repo): State<Arc<dyn TenantRepository>>,
    req: Request,
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
        _ => {
            return Ok((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "missing_authorization",
                    "message": "Authorization header is missing or invalid"
                })),
            )
                .into_response());
        }
    };

    // Hash the API key for lookup
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let api_key_hash = hex::encode(hasher.finalize());

    // Look up tenant
    let tenant = match tenant_repo.get_by_api_key_hash(&api_key_hash).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            warn!(api_key_prefix = %&api_key[..api_key.len().min(8)], "Invalid API key");
            return Ok((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "invalid_api_key",
                    "message": "Invalid API key"
                })),
            )
                .into_response());
        }
        Err(e) => {
            warn!(error = %e, "Database error during auth");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    if tenant.status != "ACTIVE" {
        warn!(tenant_id = %tenant.id, status = %tenant.status, "Tenant not active");
        return Ok((
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "tenant_inactive",
                "message": "Tenant account is not active"
            })),
        )
            .into_response());
    }

    // Extract signature and timestamp for HMAC verification
    let signature = req
        .headers()
        .get("X-Signature")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let timestamp_str = req
        .headers()
        .get("X-Timestamp")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Store method and path before consuming the request
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    // Verify HMAC signature if present
    // Note: If signature is not present, we allow the request (backward compatibility)
    // In production, you may want to require signatures for all requests
    if let Some(ref sig) = signature {
        // Need to read the body for signature verification
        let (parts, body) = req.into_parts();

        // Read the entire body
        let body_bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(e) => {
                warn!(error = %e, "Failed to read request body");
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };

        // Verify the signature
        if let Err(e) = verify_hmac_signature(
            &tenant,
            &method,
            &path,
            timestamp_str.as_deref(),
            &body_bytes,
            sig,
        ) {
            let (status, error, message) = match e {
                SignatureValidationError::MissingSignature => (
                    StatusCode::BAD_REQUEST,
                    "missing_signature",
                    "X-Signature header is required",
                ),
                SignatureValidationError::MissingTimestamp => (
                    StatusCode::BAD_REQUEST,
                    "missing_timestamp",
                    "X-Timestamp header is required for signature verification",
                ),
                SignatureValidationError::InvalidTimestamp => (
                    StatusCode::BAD_REQUEST,
                    "invalid_timestamp",
                    "Invalid timestamp format",
                ),
                SignatureValidationError::NoApiSecret => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "configuration_error",
                    "API secret not configured for tenant",
                ),
                SignatureValidationError::InvalidSignature => (
                    StatusCode::UNAUTHORIZED,
                    "invalid_signature",
                    "HMAC signature verification failed",
                ),
            };

            warn!(
                tenant_id = %tenant.id,
                error = ?e,
                "Signature verification failed"
            );

            return Ok((status, Json(json!({ "error": error, "message": message }))).into_response());
        }

        debug!(tenant_id = %tenant.id, "HMAC signature verified successfully");

        // Reconstruct the request with the body
        let mut new_req = Request::from_parts(parts, Body::from(body_bytes.to_vec()));

        // Add tenant context to request extensions
        let context = TenantContext {
            tenant_id: TenantId::new(&tenant.id),
            name: tenant.name,
        };
        new_req.extensions_mut().insert(context);

        return Ok(next.run(new_req).await);
    }

    // No signature provided - add tenant context and continue
    // Note: Consider making signature required in production
    debug!(
        tenant_id = %tenant.id,
        "Request without HMAC signature (backward compatibility mode)"
    );

    let (parts, body) = req.into_parts();
    let mut new_req = Request::from_parts(parts, body);

    let context = TenantContext {
        tenant_id: TenantId::new(&tenant.id),
        name: tenant.name,
    };
    new_req.extensions_mut().insert(context);

    Ok(next.run(new_req).await)
}

/// Verify HMAC-SHA256 signature
///
/// The signature is computed over: `{method}\n{path}\n{timestamp}\n{body}`
/// This matches the SDK implementation in:
/// - TypeScript: sdk/src/utils/crypto.ts
/// - Go: sdk-go/client.go signRequest function
fn verify_hmac_signature(
    tenant: &ramp_core::repository::tenant::TenantRow,
    method: &Method,
    path: &str,
    timestamp_str: Option<&str>,
    body: &[u8],
    provided_signature: &str,
) -> Result<(), SignatureValidationError> {
    // Get the timestamp
    let timestamp = timestamp_str.ok_or(SignatureValidationError::MissingTimestamp)?;

    // Parse timestamp to ensure it's valid
    let _ts_val: i64 = timestamp
        .parse()
        .map_err(|_| SignatureValidationError::InvalidTimestamp)?;

    // Get the API secret (decrypted)
    // In production, this should use proper decryption
    let api_secret = tenant
        .api_secret_encrypted
        .as_ref()
        .ok_or(SignatureValidationError::NoApiSecret)?;

    // For now, api_secret is stored as plain bytes (TODO: implement proper encryption)
    let api_secret_str =
        String::from_utf8(api_secret.clone()).map_err(|_| SignatureValidationError::NoApiSecret)?;

    // Reconstruct the message: method\npath\ntimestamp\nbody
    // This matches the SDK signing format
    let body_str = String::from_utf8_lossy(body);
    let message = format!("{}\n{}\n{}\n{}", method.as_str(), path, timestamp, body_str);

    // Compute HMAC-SHA256
    let mut mac =
        HmacSha256::new_from_slice(api_secret_str.as_bytes()).expect("HMAC can take any size key");
    mac.update(message.as_bytes());
    let expected_signature = hex::encode(mac.finalize().into_bytes());

    // Constant-time comparison to prevent timing attacks
    let provided_bytes = provided_signature.as_bytes();
    let expected_bytes = expected_signature.as_bytes();

    if provided_bytes.len() != expected_bytes.len() {
        return Err(SignatureValidationError::InvalidSignature);
    }

    if bool::from(provided_bytes.ct_eq(expected_bytes)) {
        Ok(())
    } else {
        Err(SignatureValidationError::InvalidSignature)
    }
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

    #[test]
    fn test_hmac_signature_computation() {
        // Test that our signature computation matches the SDK format
        use ramp_core::repository::tenant::TenantRow;

        let tenant = TenantRow {
            id: "test-tenant".to_string(),
            name: "Test".to_string(),
            status: "ACTIVE".to_string(),
            api_key_hash: "hash".to_string(),
            api_secret_encrypted: Some(b"test-secret".to_vec()),
            webhook_secret_hash: "whash".to_string(),
            webhook_secret_encrypted: None,
            webhook_url: None,
            config: serde_json::json!({}),
            daily_payin_limit_vnd: None,
            daily_payout_limit_vnd: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Compute expected signature using the same method as the SDK
        let method = "POST";
        let path = "/v1/intents/payin";
        let timestamp = "1704067200"; // 2024-01-01 00:00:00 UTC
        let body = r#"{"user_id":"user123","amount":1000000}"#;

        let message = format!("{}\n{}\n{}\n{}", method, path, timestamp, body);
        let mut mac = HmacSha256::new_from_slice(b"test-secret").unwrap();
        mac.update(message.as_bytes());
        let expected = hex::encode(mac.finalize().into_bytes());

        // Verify our function produces the same result
        let result = verify_hmac_signature(
            &tenant,
            &Method::POST,
            path,
            Some(timestamp),
            body.as_bytes(),
            &expected,
        );

        assert!(result.is_ok(), "Signature verification should succeed");

        // Test with wrong signature
        let wrong_result = verify_hmac_signature(
            &tenant,
            &Method::POST,
            path,
            Some(timestamp),
            body.as_bytes(),
            "wrong-signature",
        );

        assert!(
            matches!(
                wrong_result,
                Err(SignatureValidationError::InvalidSignature)
            ),
            "Wrong signature should fail"
        );
    }
}
