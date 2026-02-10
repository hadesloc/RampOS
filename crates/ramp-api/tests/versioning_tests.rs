//! Integration tests for the API versioning system
//!
//! Tests the full middleware stack: version negotiation, request transformation,
//! and response transformation via the versioning pipeline.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::from_fn,
    routing::get,
    Router,
};
use serde_json::{json, Value};
use tower::ServiceExt;

use ramp_api::versioning::{ApiVersion, TransformerRegistry};
use ramp_api::middleware::versioning::{
    version_negotiation_middleware, VERSION_HEADER, VERSION_RESPONSE_HEADER,
};

/// Helper: build a simple test app with version negotiation middleware
fn versioned_app() -> Router {
    Router::new()
        .route("/test", get(|| async { "ok" }))
        .layer(from_fn(version_negotiation_middleware))
}

// ===== Version Header Integration Tests =====

#[tokio::test]
async fn test_request_with_version_header_accepted() {
    let app = versioned_app();

    let req = Request::builder()
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
async fn test_request_with_latest_version_header() {
    let app = versioned_app();

    let req = Request::builder()
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
async fn test_request_without_version_uses_default() {
    let app = versioned_app();

    let req = Request::builder()
        .uri("/test")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    // Should fall back to default version 2026-02-01
    assert_eq!(
        resp.headers().get(VERSION_RESPONSE_HEADER).unwrap(),
        "2026-02-01"
    );
}

#[tokio::test]
async fn test_invalid_version_header_returns_error() {
    let app = versioned_app();

    let req = Request::builder()
        .uri("/test")
        .header(VERSION_HEADER, "not-a-date")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body = axum::body::to_bytes(resp.into_body(), 4096)
        .await
        .unwrap();
    let error: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(error["error"]["code"], "INVALID_VERSION_FORMAT");
    assert!(error["error"]["minimum_version"].is_string());
    assert!(error["error"]["latest_version"].is_string());
}

#[tokio::test]
async fn test_too_old_version_header_returns_error() {
    let app = versioned_app();

    let req = Request::builder()
        .uri("/test")
        .header(VERSION_HEADER, "2020-01-01")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body = axum::body::to_bytes(resp.into_body(), 4096)
        .await
        .unwrap();
    let error: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(error["error"]["code"], "VERSION_TOO_OLD");
}

#[tokio::test]
async fn test_future_version_header_returns_error() {
    let app = versioned_app();

    let req = Request::builder()
        .uri("/test")
        .header(VERSION_HEADER, "2030-12-31")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body = axum::body::to_bytes(resp.into_body(), 4096)
        .await
        .unwrap();
    let error: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(error["error"]["code"], "VERSION_UNKNOWN");
}

#[tokio::test]
async fn test_empty_version_header_returns_error() {
    let app = versioned_app();

    let req = Request::builder()
        .uri("/test")
        .header(VERSION_HEADER, "")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ===== Version Parsing Tests =====

#[test]
fn test_version_parsing_valid_dates() {
    assert!(ApiVersion::parse("2026-02-01").is_ok());
    assert!(ApiVersion::parse("2026-03-01").is_ok());
    assert!(ApiVersion::parse("2026-12-31").is_ok());
}

#[test]
fn test_version_parsing_invalid_formats() {
    assert!(ApiVersion::parse("").is_err());
    assert!(ApiVersion::parse("2026").is_err());
    assert!(ApiVersion::parse("2026/02/01").is_err());
    assert!(ApiVersion::parse("not-a-date").is_err());
    assert!(ApiVersion::parse("2026-13-01").is_err()); // invalid month
    assert!(ApiVersion::parse("2026-02-30").is_err()); // invalid day for Feb
}

#[test]
fn test_version_comparison_ordering() {
    let v1 = ApiVersion::parse("2026-02-01").unwrap();
    let v2 = ApiVersion::parse("2026-03-01").unwrap();

    assert!(v1 < v2);
    assert!(v2 > v1);
    assert!(v1.is_older_than(&v2));
    assert!(!v2.is_older_than(&v1));
    assert!(v2.is_at_least(&v1));
    assert!(!v1.is_at_least(&v2));
}

#[test]
fn test_version_equality() {
    let a = ApiVersion::parse("2026-02-01").unwrap();
    let b = ApiVersion::parse("2026-02-01").unwrap();
    assert_eq!(a, b);
    assert!(!a.is_older_than(&b));
    assert!(a.is_at_least(&b));
}

// ===== Transformer Pipeline Integration Tests =====

#[test]
fn test_transformer_upgrade_full_pipeline() {
    let registry = TransformerRegistry::new();
    let v1 = ApiVersion::parse("2026-02-01").unwrap();
    let v2 = ApiVersion::parse("2026-03-01").unwrap();

    let input = json!({
        "amount": 50000,
        "description": "Test payment"
    });

    let upgraded = registry.upgrade_request(&v1, &v2, input).unwrap();

    // amount -> amount_minor
    assert_eq!(upgraded["amount_minor"], 50000);
    assert!(upgraded.get("amount").is_none());
    // currency defaulted
    assert_eq!(upgraded["currency"], "VND");
    // other fields preserved
    assert_eq!(upgraded["description"], "Test payment");
}

#[test]
fn test_transformer_downgrade_full_pipeline() {
    let registry = TransformerRegistry::new();
    let v1 = ApiVersion::parse("2026-02-01").unwrap();
    let v2 = ApiVersion::parse("2026-03-01").unwrap();

    let response = json!({
        "amount_minor": 50000,
        "currency": "VND",
        "status": "awaiting_confirmation",
        "api_version": "2026-03-01",
        "id": "intent_abc"
    });

    let downgraded = registry.downgrade_response(&v1, &v2, response).unwrap();

    // amount_minor -> amount
    assert_eq!(downgraded["amount"], 50000);
    assert!(downgraded.get("amount_minor").is_none());
    // status mapped back
    assert_eq!(downgraded["status"], "pending");
    // new fields removed
    assert!(downgraded.get("api_version").is_none());
    assert!(downgraded.get("currency").is_none());
    // id preserved
    assert_eq!(downgraded["id"], "intent_abc");
}

#[test]
fn test_transformer_same_version_noop() {
    let registry = TransformerRegistry::new();
    let v = ApiVersion::parse("2026-03-01").unwrap();

    let input = json!({ "amount_minor": 1000, "currency": "VND" });

    let upgraded = registry.upgrade_request(&v, &v, input.clone()).unwrap();
    assert_eq!(upgraded, input);

    let downgraded = registry.downgrade_response(&v, &v, input.clone()).unwrap();
    assert_eq!(downgraded, input);
}

#[test]
fn test_transformer_roundtrip() {
    let registry = TransformerRegistry::new();
    let v1 = ApiVersion::parse("2026-02-01").unwrap();
    let v2 = ApiVersion::parse("2026-03-01").unwrap();

    // Start with a v1 request
    let original = json!({
        "amount": 25000,
        "description": "roundtrip test"
    });

    // Upgrade to v2
    let upgraded = registry.upgrade_request(&v1, &v2, original.clone()).unwrap();
    assert_eq!(upgraded["amount_minor"], 25000);

    // Simulate a v2 response based on the upgraded request
    let v2_response = json!({
        "amount_minor": 25000,
        "currency": "VND",
        "status": "awaiting_confirmation",
        "description": "roundtrip test"
    });

    // Downgrade response back to v1
    let downgraded = registry.downgrade_response(&v1, &v2, v2_response).unwrap();
    assert_eq!(downgraded["amount"], 25000);
    assert_eq!(downgraded["status"], "pending");
    assert_eq!(downgraded["description"], "roundtrip test");
}

#[test]
fn test_all_known_versions_are_compatible() {
    for v in ApiVersion::all_known() {
        assert!(
            v.is_compatible(),
            "Known version {} should be compatible",
            v
        );
        assert!(
            v.is_known(),
            "Known version {} should be recognized as known",
            v
        );
    }
}

#[test]
fn test_known_versions_are_sorted() {
    let versions = ApiVersion::all_known();
    for window in versions.windows(2) {
        assert!(
            window[0] < window[1],
            "Known versions should be in chronological order: {} >= {}",
            window[0],
            window[1]
        );
    }
}
