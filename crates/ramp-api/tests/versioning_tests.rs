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

// ===== Transformer Chain: Custom Multi-Version Pipeline =====

/// A second transformer: v2026-03-01 -> v2026-04-01 (hypothetical)
/// Used to test multi-hop chaining through the registry.
struct V20260301ToV20260401;

impl ramp_api::versioning::VersionTransformer for V20260301ToV20260401 {
    fn from_version(&self) -> ApiVersion {
        ApiVersion::parse("2026-03-01").expect("valid")
    }
    fn to_version(&self) -> ApiVersion {
        ApiVersion::parse("2026-04-01").expect("valid")
    }
    fn transform_request(&self, mut payload: Value) -> Result<Value, ramp_api::versioning::TransformError> {
        if let Some(obj) = payload.as_object_mut() {
            // v3 adds a "metadata" wrapper around description
            if let Some(desc) = obj.remove("description") {
                let meta = serde_json::json!({ "note": desc });
                obj.insert("metadata".to_string(), meta);
            }
        }
        Ok(payload)
    }
    fn transform_response(&self, mut payload: Value) -> Result<Value, ramp_api::versioning::TransformError> {
        if let Some(obj) = payload.as_object_mut() {
            // Unwrap metadata.note back to description
            if let Some(meta) = obj.remove("metadata") {
                if let Some(note) = meta.get("note") {
                    obj.insert("description".to_string(), note.clone());
                }
            }
        }
        Ok(payload)
    }
}

#[test]
fn test_multi_hop_upgrade_chain() {
    let mut registry = TransformerRegistry::new();
    registry.register(std::sync::Arc::new(V20260301ToV20260401));

    let v1 = ApiVersion::parse("2026-02-01").unwrap();
    let v3 = ApiVersion::parse("2026-04-01").unwrap();

    let input = json!({
        "amount": 75000,
        "description": "multi-hop test"
    });

    let upgraded = registry.upgrade_request(&v1, &v3, input).unwrap();

    // v1->v2: amount -> amount_minor, currency added
    assert_eq!(upgraded["amount_minor"], 75000);
    assert!(upgraded.get("amount").is_none());
    assert_eq!(upgraded["currency"], "VND");

    // v2->v3: description -> metadata.note
    assert_eq!(upgraded["metadata"]["note"], "multi-hop test");
    assert!(upgraded.get("description").is_none());
}

#[test]
fn test_multi_hop_downgrade_chain() {
    let mut registry = TransformerRegistry::new();
    registry.register(std::sync::Arc::new(V20260301ToV20260401));

    let v1 = ApiVersion::parse("2026-02-01").unwrap();
    let v3 = ApiVersion::parse("2026-04-01").unwrap();

    let response = json!({
        "amount_minor": 75000,
        "currency": "VND",
        "status": "awaiting_confirmation",
        "api_version": "2026-04-01",
        "metadata": { "note": "multi-hop test" },
        "id": "intent_mh1"
    });

    let downgraded = registry.downgrade_response(&v1, &v3, response).unwrap();

    // v3->v2: metadata.note -> description
    assert_eq!(downgraded["description"], "multi-hop test");
    assert!(downgraded.get("metadata").is_none());

    // v2->v1: amount_minor -> amount, status mapped back
    assert_eq!(downgraded["amount"], 75000);
    assert!(downgraded.get("amount_minor").is_none());
    assert_eq!(downgraded["status"], "pending");
    assert!(downgraded.get("api_version").is_none());
    assert!(downgraded.get("currency").is_none());
    assert_eq!(downgraded["id"], "intent_mh1");
}

#[test]
fn test_multi_hop_roundtrip() {
    let mut registry = TransformerRegistry::new();
    registry.register(std::sync::Arc::new(V20260301ToV20260401));

    let v1 = ApiVersion::parse("2026-02-01").unwrap();
    let v3 = ApiVersion::parse("2026-04-01").unwrap();

    let original_request = json!({
        "amount": 12345,
        "description": "roundtrip chain"
    });

    let _upgraded = registry.upgrade_request(&v1, &v3, original_request).unwrap();

    // Simulate v3 response
    let v3_response = json!({
        "amount_minor": 12345,
        "currency": "VND",
        "status": "awaiting_confirmation",
        "metadata": { "note": "roundtrip chain" },
        "id": "rt_001"
    });

    let downgraded = registry.downgrade_response(&v1, &v3, v3_response).unwrap();
    assert_eq!(downgraded["amount"], 12345);
    assert_eq!(downgraded["description"], "roundtrip chain");
    assert_eq!(downgraded["status"], "pending");
    assert_eq!(downgraded["id"], "rt_001");
}

// ===== Version Pinning Behavior Tests =====

#[tokio::test]
async fn test_tenant_pinned_version_used_when_no_header() {
    use ramp_api::middleware::versioning::{TenantApiVersion, VERSION_RESPONSE_HEADER};

    let app = Router::new()
        .route("/test", get(|| async { "ok" }))
        .layer(from_fn(version_negotiation_middleware));

    let mut req = Request::builder()
        .uri("/test")
        .body(Body::empty())
        .unwrap();

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
async fn test_header_overrides_tenant_pinned_version() {
    use ramp_api::middleware::versioning::{TenantApiVersion, VERSION_RESPONSE_HEADER};

    let app = Router::new()
        .route("/test", get(|| async { "ok" }))
        .layer(from_fn(version_negotiation_middleware));

    let mut req = Request::builder()
        .uri("/test")
        .header(VERSION_HEADER, "2026-02-01")
        .body(Body::empty())
        .unwrap();

    // Tenant pinned to latest, but header specifies older
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
async fn test_incompatible_tenant_pinned_falls_to_default() {
    use ramp_api::middleware::versioning::{TenantApiVersion, VERSION_RESPONSE_HEADER};

    let app = Router::new()
        .route("/test", get(|| async { "ok" }))
        .layer(from_fn(version_negotiation_middleware));

    let mut req = Request::builder()
        .uri("/test")
        .body(Body::empty())
        .unwrap();

    // Tenant pinned to an incompatible old version
    req.extensions_mut().insert(TenantApiVersion {
        version: ApiVersion::parse("2020-01-01").unwrap(),
    });

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    // Should fall back to default 2026-02-01
    assert_eq!(
        resp.headers().get(VERSION_RESPONSE_HEADER).unwrap(),
        "2026-02-01"
    );
}

// ===== Backward Compatibility for Deprecated Fields =====

#[test]
fn test_deprecated_amount_field_transformed_to_amount_minor() {
    let registry = TransformerRegistry::new();
    let v1 = ApiVersion::parse("2026-02-01").unwrap();
    let v2 = ApiVersion::parse("2026-03-01").unwrap();

    // Old client sends "amount" (deprecated in v2)
    let input = json!({ "amount": 100000 });
    let upgraded = registry.upgrade_request(&v1, &v2, input).unwrap();
    assert_eq!(upgraded["amount_minor"], 100000);
    assert!(upgraded.get("amount").is_none());
}

#[test]
fn test_deprecated_status_value_mapped_back_on_downgrade() {
    let registry = TransformerRegistry::new();
    let v1 = ApiVersion::parse("2026-02-01").unwrap();
    let v2 = ApiVersion::parse("2026-03-01").unwrap();

    let response = json!({ "status": "awaiting_confirmation" });
    let downgraded = registry.downgrade_response(&v1, &v2, response).unwrap();
    assert_eq!(downgraded["status"], "pending");
}

#[test]
fn test_non_deprecated_status_preserved_on_downgrade() {
    let registry = TransformerRegistry::new();
    let v1 = ApiVersion::parse("2026-02-01").unwrap();
    let v2 = ApiVersion::parse("2026-03-01").unwrap();

    for status in &["completed", "failed", "processing", "cancelled"] {
        let response = json!({ "status": status });
        let downgraded = registry.downgrade_response(&v1, &v2, response).unwrap();
        assert_eq!(downgraded["status"], *status, "Status '{}' should be preserved", status);
    }
}

#[test]
fn test_currency_field_added_by_default_on_upgrade() {
    let registry = TransformerRegistry::new();
    let v1 = ApiVersion::parse("2026-02-01").unwrap();
    let v2 = ApiVersion::parse("2026-03-01").unwrap();

    let input = json!({ "amount": 5000 });
    let upgraded = registry.upgrade_request(&v1, &v2, input).unwrap();
    assert_eq!(upgraded["currency"], "VND");
}

#[test]
fn test_explicit_currency_not_overwritten_on_upgrade() {
    let registry = TransformerRegistry::new();
    let v1 = ApiVersion::parse("2026-02-01").unwrap();
    let v2 = ApiVersion::parse("2026-03-01").unwrap();

    let input = json!({ "amount": 100, "currency": "USD" });
    let upgraded = registry.upgrade_request(&v1, &v2, input).unwrap();
    assert_eq!(upgraded["currency"], "USD");
}

#[test]
fn test_api_version_field_stripped_from_response_on_downgrade() {
    let registry = TransformerRegistry::new();
    let v1 = ApiVersion::parse("2026-02-01").unwrap();
    let v2 = ApiVersion::parse("2026-03-01").unwrap();

    let response = json!({
        "id": "intent_x",
        "amount_minor": 1000,
        "api_version": "2026-03-01"
    });
    let downgraded = registry.downgrade_response(&v1, &v2, response).unwrap();
    assert!(downgraded.get("api_version").is_none());
    assert_eq!(downgraded["id"], "intent_x");
}

// ===== Invalid / Unknown Version Handling =====

#[tokio::test]
async fn test_garbage_version_header_rejected() {
    let app = versioned_app();

    for bad_version in &["abc", "2026", "02-01-2026", "v1.0", "2026.02.01", " "] {
        let req = Request::builder()
            .uri("/test")
            .header(VERSION_HEADER, *bad_version)
            .body(Body::empty())
            .unwrap();

        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "Version '{}' should be rejected",
            bad_version
        );
    }
}

#[tokio::test]
async fn test_version_between_known_versions_accepted_if_compatible() {
    // A date between two known versions that falls within compatible range
    let app = versioned_app();

    let req = Request::builder()
        .uri("/test")
        .header(VERSION_HEADER, "2026-02-15")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    // This version is within min..=latest range so it should be accepted
    assert_eq!(resp.status(), StatusCode::OK);
}

#[test]
fn test_unknown_version_not_in_known_list() {
    let v = ApiVersion::parse("2026-02-15").unwrap();
    assert!(!v.is_known());
    // But it IS compatible since it's within range
    assert!(v.is_compatible());
}

// ===== Default Version Behavior =====

#[test]
fn test_default_version_is_minimum_and_first_known() {
    let default = ApiVersion::default_version();
    let minimum = ApiVersion::minimum();
    let first_known = ApiVersion::all_known().into_iter().next().unwrap();

    assert_eq!(default, minimum);
    assert_eq!(default, first_known);
}

#[test]
fn test_default_version_is_older_than_latest() {
    let default = ApiVersion::default_version();
    let latest = ApiVersion::latest();
    assert!(default < latest);
    assert!(default.is_older_than(&latest));
}

#[tokio::test]
async fn test_no_header_no_tenant_uses_default() {
    let app = versioned_app();

    let req = Request::builder()
        .uri("/test")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers()
            .get(ramp_api::middleware::versioning::VERSION_RESPONSE_HEADER)
            .unwrap(),
        "2026-02-01"
    );
}

// ===== Registry Edge Cases =====

#[test]
fn test_registry_upgrade_with_no_matching_transformer() {
    let registry = TransformerRegistry::new();

    // Try upgrading from a version that has no registered transformer
    let unknown = ApiVersion::parse("2025-06-01").unwrap();
    let latest = ApiVersion::latest();

    let input = json!({ "foo": "bar" });
    let output = registry.upgrade_request(&unknown, &latest, input.clone()).unwrap();
    // No transformer found, so payload passes through unchanged
    assert_eq!(output, input);
}

#[test]
fn test_registry_downgrade_with_no_matching_transformer() {
    let registry = TransformerRegistry::new();

    let unknown = ApiVersion::parse("2025-06-01").unwrap();
    let latest = ApiVersion::latest();

    let input = json!({ "foo": "bar" });
    let output = registry.downgrade_response(&unknown, &latest, input.clone()).unwrap();
    // No transformer chain starts at unknown, payload unchanged
    assert_eq!(output, input);
}

#[test]
fn test_upgrade_with_null_values_in_payload() {
    let registry = TransformerRegistry::new();
    let v1 = ApiVersion::parse("2026-02-01").unwrap();
    let v2 = ApiVersion::parse("2026-03-01").unwrap();

    let input = json!({
        "amount": null,
        "description": null
    });

    let upgraded = registry.upgrade_request(&v1, &v2, input).unwrap();
    // null amount should be renamed to amount_minor as null
    assert!(upgraded["amount_minor"].is_null());
    assert!(upgraded.get("amount").is_none());
}

#[test]
fn test_upgrade_with_nested_objects_preserved() {
    let registry = TransformerRegistry::new();
    let v1 = ApiVersion::parse("2026-02-01").unwrap();
    let v2 = ApiVersion::parse("2026-03-01").unwrap();

    let input = json!({
        "amount": 5000,
        "recipient": {
            "name": "Nguyen Van A",
            "bank": {
                "code": "VCB",
                "account": "1234567890"
            }
        }
    });

    let upgraded = registry.upgrade_request(&v1, &v2, input).unwrap();
    assert_eq!(upgraded["amount_minor"], 5000);
    assert_eq!(upgraded["recipient"]["name"], "Nguyen Van A");
    assert_eq!(upgraded["recipient"]["bank"]["code"], "VCB");
    assert_eq!(upgraded["recipient"]["bank"]["account"], "1234567890");
}

#[test]
fn test_upgrade_with_array_payload_passes_through() {
    let registry = TransformerRegistry::new();
    let v1 = ApiVersion::parse("2026-02-01").unwrap();
    let v2 = ApiVersion::parse("2026-03-01").unwrap();

    let input = json!([1, 2, 3]);
    let output = registry.upgrade_request(&v1, &v2, input.clone()).unwrap();
    // Arrays are not objects, so transformer doesn't modify them
    assert_eq!(output, input);
}

#[test]
fn test_downgrade_with_extra_unknown_fields_preserved() {
    let registry = TransformerRegistry::new();
    let v1 = ApiVersion::parse("2026-02-01").unwrap();
    let v2 = ApiVersion::parse("2026-03-01").unwrap();

    let response = json!({
        "amount_minor": 5000,
        "currency": "VND",
        "status": "completed",
        "custom_field": "should_survive",
        "nested": { "data": true }
    });

    let downgraded = registry.downgrade_response(&v1, &v2, response).unwrap();
    assert_eq!(downgraded["amount"], 5000);
    assert_eq!(downgraded["custom_field"], "should_survive");
    assert_eq!(downgraded["nested"]["data"], true);
}

#[test]
fn test_downgrade_response_without_amount_minor() {
    let registry = TransformerRegistry::new();
    let v1 = ApiVersion::parse("2026-02-01").unwrap();
    let v2 = ApiVersion::parse("2026-03-01").unwrap();

    // Response missing amount_minor entirely
    let response = json!({ "status": "completed", "id": "x" });
    let downgraded = registry.downgrade_response(&v1, &v2, response).unwrap();
    assert_eq!(downgraded["status"], "completed");
    assert_eq!(downgraded["id"], "x");
    assert!(downgraded.get("amount").is_none());
}

// ===== Version Error Response Format =====

#[tokio::test]
async fn test_version_error_response_structure() {
    let app = versioned_app();

    let req = Request::builder()
        .uri("/test")
        .header(VERSION_HEADER, "xyz")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body = axum::body::to_bytes(resp.into_body(), 4096)
        .await
        .unwrap();
    let error: Value = serde_json::from_slice(&body).unwrap();

    // Verify full error structure
    assert!(error["error"].is_object());
    assert_eq!(error["error"]["code"], "INVALID_VERSION_FORMAT");
    assert!(error["error"]["message"].is_string());
    assert_eq!(error["error"]["minimum_version"], "2026-02-01");
    assert_eq!(error["error"]["latest_version"], "2026-03-01");
}

#[tokio::test]
async fn test_too_old_version_error_includes_guidance() {
    let app = versioned_app();

    let req = Request::builder()
        .uri("/test")
        .header(VERSION_HEADER, "2020-01-01")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    let body = axum::body::to_bytes(resp.into_body(), 4096)
        .await
        .unwrap();
    let error: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(error["error"]["code"], "VERSION_TOO_OLD");
    assert_eq!(error["error"]["minimum_version"], "2026-02-01");
    assert_eq!(error["error"]["latest_version"], "2026-03-01");
}

// ===== TransformerRegistry::default() =====

#[test]
fn test_registry_default_equals_new() {
    let r1 = TransformerRegistry::new();
    let r2 = TransformerRegistry::default();

    let v1 = ApiVersion::parse("2026-02-01").unwrap();
    let v2 = ApiVersion::parse("2026-03-01").unwrap();
    let input = json!({ "amount": 42 });

    let out1 = r1.upgrade_request(&v1, &v2, input.clone()).unwrap();
    let out2 = r2.upgrade_request(&v1, &v2, input).unwrap();
    assert_eq!(out1, out2);
}

// ===== Large Payload Stress Test =====

#[test]
fn test_upgrade_large_payload_with_many_fields() {
    let registry = TransformerRegistry::new();
    let v1 = ApiVersion::parse("2026-02-01").unwrap();
    let v2 = ApiVersion::parse("2026-03-01").unwrap();

    let mut obj = serde_json::Map::new();
    obj.insert("amount".to_string(), json!(999999));
    for i in 0..50 {
        obj.insert(format!("field_{}", i), json!(format!("value_{}", i)));
    }

    let input = Value::Object(obj);
    let upgraded = registry.upgrade_request(&v1, &v2, input).unwrap();

    assert_eq!(upgraded["amount_minor"], 999999);
    assert!(upgraded.get("amount").is_none());
    assert_eq!(upgraded["currency"], "VND");
    // All other fields preserved
    for i in 0..50 {
        assert_eq!(upgraded[format!("field_{}", i)], format!("value_{}", i));
    }
}
