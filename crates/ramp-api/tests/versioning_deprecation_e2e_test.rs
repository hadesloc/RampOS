//! E2E tests for API version deprecation, lifecycle, and negotiation edge cases.
//!
//! These tests focus on:
//! - Version deprecation flag detection
//! - Sunset header generation for deprecated versions
//! - Version pinning per tenant/client
//! - Version fallback chain (requested -> tenant pinned -> default)
//! - Deprecation warning in response headers
//! - Version comparison and ordering
//! - Breaking change detection between versions
//! - Version lifecycle (active -> deprecated -> sunset)
//! - Default version when no header specified
//! - Invalid version format handling
//! - Version-specific transformer selection
//! - Concurrent requests with different versions

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use ramp_api::middleware::versioning::{
    version_negotiation_middleware, TenantApiVersion, VersionContext, VersionSource,
};
use ramp_api::versioning::{ApiVersion, TransformError, TransformerRegistry, VersionTransformer};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::net::TcpListener;

// =============================================================================
// Test helpers
// =============================================================================

async fn start_server(app: Router) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    base_url
}

/// Handler that echoes version context plus deprecation info in headers.
async fn deprecation_aware_handler(req: Request) -> Response {
    let version_ctx = req.extensions().get::<VersionContext>().cloned();

    match version_ctx {
        Some(ctx) => {
            let source_str = match ctx.source {
                VersionSource::Header => "header",
                VersionSource::TenantPinned => "tenant_pinned",
                VersionSource::Default => "default",
            };

            let version = ctx.version.clone();
            let is_deprecated = version < ApiVersion::latest() && version.is_known();
            let is_sunset = !version.is_compatible();

            let mut resp = Json(json!({
                "version": version.to_string(),
                "source": source_str,
                "is_deprecated": is_deprecated,
                "is_sunset": is_sunset,
            }))
            .into_response();

            // Add deprecation headers for older versions
            if is_deprecated {
                resp.headers_mut()
                    .insert("Deprecation", "true".parse().unwrap());
                // Sunset header: indicate when this version will be removed
                resp.headers_mut()
                    .insert("Sunset", "2027-02-01T00:00:00Z".parse().unwrap());
                resp.headers_mut().insert(
                    "X-RampOS-Deprecation-Warning",
                    format!(
                        "API version {} is deprecated. Please upgrade to {}.",
                        version,
                        ApiVersion::latest()
                    )
                    .parse()
                    .unwrap(),
                );
            }

            resp
        }
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "no version context"})),
        )
            .into_response(),
    }
}

/// Handler that returns payment data in latest format.
async fn latest_payment_handler() -> Json<Value> {
    Json(json!({
        "id": "intent_dep_test",
        "amount_minor": 100000,
        "currency": "VND",
        "status": "awaiting_confirmation",
        "api_version": "2026-03-01",
        "description": "Deprecation test"
    }))
}

/// Build app with deprecation-aware handler.
fn build_deprecation_app() -> Router {
    Router::new()
        .route("/version", get(deprecation_aware_handler))
        .layer(middleware::from_fn(version_negotiation_middleware))
}

/// Build app with tenant pinning and deprecation headers.
fn build_tenant_app(pinned_version: &str) -> Router {
    let pinned = ApiVersion::parse(pinned_version).expect("valid pinned version");

    Router::new()
        .route("/version", get(deprecation_aware_handler))
        .layer(middleware::from_fn(version_negotiation_middleware))
        .layer(middleware::from_fn(move |mut req: Request, next: Next| {
            let version = pinned.clone();
            async move {
                req.extensions_mut().insert(TenantApiVersion { version });
                Ok::<_, StatusCode>(next.run(req).await)
            }
        }))
}

/// Build an app with both response downgrade and deprecation headers.
fn build_full_deprecation_app() -> Router {
    let registry = TransformerRegistry::new();

    Router::new()
        .route("/payment", get(latest_payment_handler))
        .layer(middleware::from_fn(move |req: Request, next: Next| {
            let reg = registry.clone();
            async move {
                let version_ctx = req.extensions().get::<VersionContext>().cloned();
                let response = next.run(req).await;

                if let Some(ctx) = version_ctx {
                    let latest = ApiVersion::latest();
                    if ctx.version < latest {
                        let (parts, body) = response.into_parts();
                        let bytes = axum::body::to_bytes(body, 65536).await.unwrap();

                        if let Ok(payload) = serde_json::from_slice::<Value>(&bytes) {
                            let downgraded = reg
                                .downgrade_response(&ctx.version, &latest, payload)
                                .unwrap();
                            let mut resp = Json(downgraded).into_response();
                            *resp.status_mut() = parts.status;
                            for (k, v) in parts.headers.iter() {
                                resp.headers_mut().insert(k.clone(), v.clone());
                            }
                            // Add deprecation headers
                            resp.headers_mut()
                                .insert("Deprecation", "true".parse().unwrap());
                            resp.headers_mut()
                                .insert("Sunset", "2027-02-01T00:00:00Z".parse().unwrap());
                            return resp;
                        }

                        return Response::from_parts(parts, axum::body::Body::from(bytes));
                    }
                }

                response
            }
        }))
        .layer(middleware::from_fn(version_negotiation_middleware))
}

// =============================================================================
// Test 1: Deprecated version gets Deprecation + Sunset headers
// =============================================================================

#[tokio::test]
async fn test_deprecated_version_gets_deprecation_headers() {
    let app = build_deprecation_app();
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    // v2026-02-01 is older than latest (v2026-03-01), so it's "deprecated"
    let resp = client
        .get(format!("{}/version", base_url))
        .header("RampOS-Version", "2026-02-01")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 200);

    // Check deprecation headers
    let deprecation = resp
        .headers()
        .get("Deprecation")
        .expect("Should have Deprecation header for old version")
        .to_str()
        .unwrap();
    assert_eq!(deprecation, "true");

    let sunset = resp
        .headers()
        .get("Sunset")
        .expect("Should have Sunset header for deprecated version")
        .to_str()
        .unwrap();
    assert!(
        sunset.contains("2027"),
        "Sunset date should be in the future"
    );

    let warning = resp
        .headers()
        .get("X-RampOS-Deprecation-Warning")
        .expect("Should have deprecation warning header")
        .to_str()
        .unwrap();
    assert!(
        warning.contains("deprecated"),
        "Warning should mention deprecation"
    );
    assert!(
        warning.contains("2026-03-01"),
        "Warning should mention latest version"
    );

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["is_deprecated"], true);
}

// =============================================================================
// Test 2: Latest version does NOT get deprecation headers
// =============================================================================

#[tokio::test]
async fn test_latest_version_no_deprecation_headers() {
    let app = build_deprecation_app();
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/version", base_url))
        .header("RampOS-Version", "2026-03-01")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 200);

    // No deprecation headers for the latest version
    assert!(
        resp.headers().get("Deprecation").is_none(),
        "Latest version should NOT have Deprecation header"
    );
    assert!(
        resp.headers().get("Sunset").is_none(),
        "Latest version should NOT have Sunset header"
    );
    assert!(
        resp.headers().get("X-RampOS-Deprecation-Warning").is_none(),
        "Latest version should NOT have deprecation warning"
    );

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["is_deprecated"], false);
}

// =============================================================================
// Test 3: Version pinning per tenant - tenant uses older deprecated version
// =============================================================================

#[tokio::test]
async fn test_tenant_pinned_to_deprecated_version() {
    // Tenant pinned to the older version
    let app = build_tenant_app("2026-02-01");
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    // No header - should use tenant pinned version
    let resp = client
        .get(format!("{}/version", base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 200);

    // Read headers before consuming response body
    let has_deprecation = resp
        .headers()
        .get("Deprecation")
        .map(|v| v.to_str().unwrap().to_string());

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["version"], "2026-02-01");
    assert_eq!(body["source"], "tenant_pinned");
    assert_eq!(body["is_deprecated"], true);

    // Should still get deprecation headers
    let deprecation =
        has_deprecation.expect("Tenant on deprecated version should see Deprecation header");
    assert_eq!(deprecation, "true");
}

// =============================================================================
// Test 4: Version fallback chain - header > tenant > default
// =============================================================================

#[tokio::test]
async fn test_version_fallback_chain_priority() {
    // Tenant pinned to v2026-03-01
    let app = build_tenant_app("2026-03-01");
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    // Case 1: Header provided -> uses header version (overrides tenant)
    let resp = client
        .get(format!("{}/version", base_url))
        .header("RampOS-Version", "2026-02-01")
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["version"], "2026-02-01", "Header should take priority");
    assert_eq!(body["source"], "header");

    // Case 2: No header -> falls to tenant pinned
    let resp = client
        .get(format!("{}/version", base_url))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    assert_eq!(
        body["version"], "2026-03-01",
        "Tenant pinned should be used when no header"
    );
    assert_eq!(body["source"], "tenant_pinned");
}

// =============================================================================
// Test 5: Version fallback to default when no header and no tenant
// =============================================================================

#[tokio::test]
async fn test_version_fallback_to_default() {
    let app = build_deprecation_app(); // No tenant injection
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/version", base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(
        body["version"], "2026-02-01",
        "Should default to 2026-02-01"
    );
    assert_eq!(body["source"], "default");
}

// =============================================================================
// Test 6: Version comparison and ordering correctness
// =============================================================================

#[tokio::test]
async fn test_version_comparison_and_ordering() {
    let v1 = ApiVersion::parse("2026-02-01").unwrap();
    let v2 = ApiVersion::parse("2026-03-01").unwrap();
    let v_future = ApiVersion::parse("2026-06-15").unwrap();
    let v_past = ApiVersion::parse("2025-01-01").unwrap();

    // Basic ordering
    assert!(v1 < v2, "v1 should be less than v2");
    assert!(v2 > v1, "v2 should be greater than v1");
    assert!(v_past < v1, "past should be less than v1");
    assert!(v_future > v2, "future should be greater than v2");

    // Sorted ordering
    let mut versions = vec![v_future.clone(), v1.clone(), v_past.clone(), v2.clone()];
    versions.sort();
    assert_eq!(versions[0], v_past);
    assert_eq!(versions[1], v1);
    assert_eq!(versions[2], v2);
    assert_eq!(versions[3], v_future);

    // All known versions sorted
    let known = ApiVersion::all_known();
    for i in 1..known.len() {
        assert!(
            known[i - 1] < known[i],
            "Known versions should be in chronological order"
        );
    }
}

// =============================================================================
// Test 7: Breaking change detection - transformer transforms fields
// =============================================================================

#[tokio::test]
async fn test_breaking_change_detection_via_transformer() {
    let registry = TransformerRegistry::new();
    let v1 = ApiVersion::parse("2026-02-01").unwrap();
    let v2 = ApiVersion::parse("2026-03-01").unwrap();

    // v1 -> v2 introduces breaking changes:
    // 1. "amount" renamed to "amount_minor"
    // 2. "currency" field added
    let v1_payload = json!({
        "amount": 50000,
        "description": "Test"
    });

    let upgraded = registry
        .upgrade_request(&v1, &v2, v1_payload.clone())
        .unwrap();

    // Verify breaking changes applied
    assert!(
        upgraded.get("amount").is_none(),
        "Breaking: 'amount' should be renamed"
    );
    assert_eq!(
        upgraded["amount_minor"], 50000,
        "Breaking: 'amount_minor' should exist"
    );
    assert_eq!(
        upgraded["currency"], "VND",
        "Breaking: 'currency' should be added"
    );

    // Verify round-trip: downgrade should reverse changes
    let downgraded = registry
        .downgrade_response(&v1, &v2, upgraded.clone())
        .unwrap();
    assert_eq!(
        downgraded["amount"], 50000,
        "Downgrade should restore 'amount'"
    );
    assert!(
        downgraded.get("amount_minor").is_none(),
        "Downgrade should remove 'amount_minor'"
    );
    assert!(
        downgraded.get("currency").is_none(),
        "Downgrade should remove 'currency'"
    );
}

// =============================================================================
// Test 8: Version lifecycle states (active / deprecated / sunset)
// =============================================================================

#[tokio::test]
async fn test_version_lifecycle_states() {
    // Active + Latest: is_compatible, is_known, == latest
    let latest = ApiVersion::latest();
    assert!(latest.is_compatible(), "Latest should be compatible");
    assert!(latest.is_known(), "Latest should be known");

    // Active but Deprecated: is_compatible, is_known, < latest
    let deprecated = ApiVersion::parse("2026-02-01").unwrap();
    assert!(
        deprecated.is_compatible(),
        "Deprecated should be compatible"
    );
    assert!(deprecated.is_known(), "Deprecated should be known");
    assert!(
        deprecated < latest,
        "Deprecated should be older than latest"
    );

    // Sunset (too old): not compatible
    let sunset = ApiVersion::parse("2025-01-01").unwrap();
    assert!(
        !sunset.is_compatible(),
        "Sunset version should not be compatible"
    );
    assert!(!sunset.is_known(), "Sunset version should not be known");

    // Future (not yet released): not compatible
    let future = ApiVersion::parse("2030-01-01").unwrap();
    assert!(
        !future.is_compatible(),
        "Future version should not be compatible"
    );
    assert!(!future.is_known(), "Future version should not be known");
}

// =============================================================================
// Test 9: Invalid version format handling - various edge cases
// =============================================================================

#[tokio::test]
async fn test_invalid_version_format_edge_cases() {
    let app = build_deprecation_app();
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    let edge_cases = vec![
        ("v1", "semantic-style"),
        ("1.0.0", "semver format"),
        ("2026.02.01", "dot-separated"),
        ("20260201", "no separators"),
        ("latest", "string keyword"),
        ("2026-00-01", "zero month"),
        ("2026-02-00", "zero day"),
    ];

    for (version, desc) in edge_cases {
        let resp = client
            .get(format!("{}/version", base_url))
            .header("RampOS-Version", version)
            .send()
            .await
            .unwrap();

        assert_eq!(
            resp.status().as_u16(),
            400,
            "Version '{}' ({}) should return 400",
            version,
            desc
        );

        let body: Value = resp.json().await.unwrap();
        assert!(
            body["error"]["code"].is_string(),
            "Error for '{}' should have a code",
            version
        );
    }
}

// =============================================================================
// Test 10: Version-specific transformer selection
// =============================================================================

#[tokio::test]
async fn test_version_specific_transformer_selection() {
    let registry = TransformerRegistry::new();
    let v1 = ApiVersion::parse("2026-02-01").unwrap();
    let v2 = ApiVersion::parse("2026-03-01").unwrap();

    // Upgrade from v1 to v2 should apply the v1->v2 transformer
    let input = json!({"amount": 1000, "extra_field": "keep"});
    let upgraded = registry.upgrade_request(&v1, &v2, input).unwrap();
    assert_eq!(upgraded["amount_minor"], 1000);
    assert_eq!(upgraded["currency"], "VND");
    assert_eq!(upgraded["extra_field"], "keep");

    // "Upgrade" from v2 to v2 should be a no-op
    let v2_input = json!({"amount_minor": 2000, "currency": "USD"});
    let noop = registry
        .upgrade_request(&v2, &v2, v2_input.clone())
        .unwrap();
    assert_eq!(noop, v2_input, "Same version should be a no-op");

    // "Upgrade" from v2 to v1 (client is newer) should be a no-op
    let reverse = registry
        .upgrade_request(&v2, &v1, v2_input.clone())
        .unwrap();
    assert_eq!(
        reverse, v2_input,
        "Newer client -> older target should be no-op"
    );
}

// =============================================================================
// Test 11: Concurrent requests - deprecated vs latest
// =============================================================================

#[tokio::test]
async fn test_concurrent_deprecated_and_latest_requests() {
    let app = build_deprecation_app();
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    let mut handles = Vec::new();

    // 20 concurrent requests: half deprecated, half latest
    for i in 0..20 {
        let c = client.clone();
        let url = format!("{}/version", base_url);
        let version = if i % 2 == 0 {
            "2026-02-01"
        } else {
            "2026-03-01"
        };
        let ver = version.to_string();

        handles.push(tokio::spawn(async move {
            let resp = c
                .get(&url)
                .header("RampOS-Version", &ver)
                .send()
                .await
                .unwrap();

            assert_eq!(resp.status().as_u16(), 200);

            let has_deprecation = resp.headers().get("Deprecation").is_some();
            let body: Value = resp.json().await.unwrap();
            let body_ver = body["version"].as_str().unwrap().to_string();
            let is_deprecated = body["is_deprecated"].as_bool().unwrap();

            (ver, body_ver, has_deprecation, is_deprecated)
        }));
    }

    for h in handles {
        let (requested, body_version, has_deprecation, is_deprecated) = h.await.unwrap();
        assert_eq!(body_version, requested);

        if requested == "2026-02-01" {
            assert!(
                has_deprecation,
                "Deprecated version should have Deprecation header"
            );
            assert!(is_deprecated, "Deprecated version body flag should be true");
        } else {
            assert!(
                !has_deprecation,
                "Latest version should NOT have Deprecation header"
            );
            assert!(!is_deprecated, "Latest version body flag should be false");
        }
    }
}

// =============================================================================
// Test 12: Response downgrade + deprecation headers combined
// =============================================================================

#[tokio::test]
async fn test_response_downgrade_with_deprecation_headers() {
    let app = build_full_deprecation_app();
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    // Request with deprecated version - should get downgraded response + deprecation headers
    let resp = client
        .get(format!("{}/payment", base_url))
        .header("RampOS-Version", "2026-02-01")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 200);

    // Deprecation headers present
    let deprecation = resp
        .headers()
        .get("Deprecation")
        .expect("Deprecated version should have Deprecation header")
        .to_str()
        .unwrap();
    assert_eq!(deprecation, "true");

    let sunset = resp
        .headers()
        .get("Sunset")
        .expect("Should have Sunset header")
        .to_str()
        .unwrap();
    assert!(sunset.contains("2027"));

    // Response should be downgraded
    let body: Value = resp.json().await.unwrap();
    assert_eq!(
        body["amount"], 100000,
        "Should be downgraded: amount_minor -> amount"
    );
    assert!(body.get("amount_minor").is_none());
    assert_eq!(
        body["status"], "pending",
        "Should be downgraded: awaiting_confirmation -> pending"
    );
    assert!(body.get("api_version").is_none());
    assert!(body.get("currency").is_none());
    assert_eq!(body["id"], "intent_dep_test");
}

// =============================================================================
// Test 13: Latest version request - no deprecation, no downgrade
// =============================================================================

#[tokio::test]
async fn test_latest_version_no_downgrade_no_deprecation() {
    let app = build_full_deprecation_app();
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/payment", base_url))
        .header("RampOS-Version", "2026-03-01")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 200);

    // No deprecation headers
    assert!(resp.headers().get("Deprecation").is_none());
    assert!(resp.headers().get("Sunset").is_none());

    // Response NOT downgraded - latest format preserved
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["amount_minor"], 100000);
    assert_eq!(body["currency"], "VND");
    assert_eq!(body["status"], "awaiting_confirmation");
    assert_eq!(body["api_version"], "2026-03-01");
}

// =============================================================================
// Test 14: Sunset version (too old) rejected by middleware
// =============================================================================

#[tokio::test]
async fn test_sunset_version_rejected() {
    let app = build_deprecation_app();
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    // Version that's past sunset (before minimum)
    let resp = client
        .get(format!("{}/version", base_url))
        .header("RampOS-Version", "2025-01-01")
        .send()
        .await
        .unwrap();

    assert_eq!(
        resp.status().as_u16(),
        400,
        "Sunset version should be rejected"
    );

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["code"], "VERSION_TOO_OLD");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("no longer supported"),
        "Error message should indicate version is no longer supported"
    );
    assert_eq!(body["error"]["minimum_version"], "2026-02-01");
}

// =============================================================================
// Test 15: Tenant with incompatible pinned version falls to default
// =============================================================================

#[tokio::test]
async fn test_tenant_incompatible_pinned_version_falls_to_default() {
    // Tenant pinned to a version that's too old (incompatible)
    let past = ApiVersion::parse("2020-01-01").unwrap();

    let app = Router::new()
        .route("/version", get(deprecation_aware_handler))
        .layer(middleware::from_fn(version_negotiation_middleware))
        .layer(middleware::from_fn(move |mut req: Request, next: Next| {
            let version = past.clone();
            async move {
                req.extensions_mut().insert(TenantApiVersion { version });
                Ok::<_, StatusCode>(next.run(req).await)
            }
        }));

    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/version", base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 200);

    let body: Value = resp.json().await.unwrap();
    assert_eq!(
        body["version"], "2026-02-01",
        "Should fall back to default when tenant pinned version is incompatible"
    );
    assert_eq!(body["source"], "default");
}

// =============================================================================
// Test 16: Custom transformer registration and selection
// =============================================================================

#[tokio::test]
async fn test_custom_transformer_registration() {
    /// A custom transformer that renames "name" to "full_name"
    struct CustomTransformer;

    impl VersionTransformer for CustomTransformer {
        fn from_version(&self) -> ApiVersion {
            ApiVersion::parse("2026-03-01").unwrap()
        }
        fn to_version(&self) -> ApiVersion {
            ApiVersion::parse("2026-04-01").unwrap()
        }
        fn transform_request(&self, mut payload: Value) -> Result<Value, TransformError> {
            if let Some(obj) = payload.as_object_mut() {
                if let Some(name) = obj.remove("name") {
                    obj.insert("full_name".to_string(), name);
                }
            }
            Ok(payload)
        }
        fn transform_response(&self, mut payload: Value) -> Result<Value, TransformError> {
            if let Some(obj) = payload.as_object_mut() {
                if let Some(full_name) = obj.remove("full_name") {
                    obj.insert("name".to_string(), full_name);
                }
            }
            Ok(payload)
        }
    }

    let mut registry = TransformerRegistry::new();
    registry.register(Arc::new(CustomTransformer));

    let v1 = ApiVersion::parse("2026-02-01").unwrap();
    let v3 = ApiVersion::parse("2026-04-01").unwrap();

    // Multi-hop upgrade: v1 -> v2 -> v3
    let input = json!({"amount": 5000, "name": "Test User"});
    let upgraded = registry.upgrade_request(&v1, &v3, input).unwrap();

    // v1->v2 transformer: amount -> amount_minor, add currency
    assert_eq!(upgraded["amount_minor"], 5000);
    assert_eq!(upgraded["currency"], "VND");
    // v2->v3 transformer: name -> full_name
    assert_eq!(upgraded["full_name"], "Test User");
    assert!(upgraded.get("name").is_none());
    assert!(upgraded.get("amount").is_none());

    // Multi-hop downgrade: v3 -> v2 -> v1
    let response = json!({
        "amount_minor": 5000,
        "currency": "VND",
        "full_name": "Test User",
        "id": "test_123"
    });
    let downgraded = registry.downgrade_response(&v1, &v3, response).unwrap();

    assert_eq!(downgraded["amount"], 5000);
    assert_eq!(downgraded["name"], "Test User");
    assert!(downgraded.get("amount_minor").is_none());
    assert!(downgraded.get("full_name").is_none());
    assert!(downgraded.get("currency").is_none());
    assert_eq!(downgraded["id"], "test_123");
}

// =============================================================================
// Test 17: Version error response includes upgrade guidance
// =============================================================================

#[tokio::test]
async fn test_error_response_includes_upgrade_guidance() {
    let app = build_deprecation_app();
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    // Too old version
    let resp = client
        .get(format!("{}/version", base_url))
        .header("RampOS-Version", "2025-06-01")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 400);
    let body: Value = resp.json().await.unwrap();

    // Error should guide user to upgrade
    assert_eq!(body["error"]["minimum_version"], "2026-02-01");
    assert_eq!(body["error"]["latest_version"], "2026-03-01");
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("2025-06-01"));

    // Future unknown version
    let resp = client
        .get(format!("{}/version", base_url))
        .header("RampOS-Version", "2099-01-01")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 400);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["code"], "VERSION_UNKNOWN");
    assert_eq!(body["error"]["latest_version"], "2026-03-01");
}

// =============================================================================
// Test 18: Version negotiation idempotency across multiple requests
// =============================================================================

#[tokio::test]
async fn test_version_negotiation_idempotency() {
    let app = build_full_deprecation_app();
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    // Send the same request 5 times - results should be identical
    for _ in 0..5 {
        let resp = client
            .get(format!("{}/payment", base_url))
            .header("RampOS-Version", "2026-02-01")
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status().as_u16(), 200);

        let has_deprecation = resp.headers().get("Deprecation").is_some();
        assert!(
            has_deprecation,
            "Each request should consistently have deprecation header"
        );

        let body: Value = resp.json().await.unwrap();
        assert_eq!(body["amount"], 100000);
        assert_eq!(body["status"], "pending");
        assert!(body.get("amount_minor").is_none());
    }
}
