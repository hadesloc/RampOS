//! E2E integration tests for API versioning with a real HTTP server.
//!
//! These tests spin up an actual Axum server on a random port and send
//! real HTTP requests via reqwest. This validates the full version negotiation
//! middleware pipeline including header parsing, version resolution, error
//! responses, transformer chains, and response header echoing.

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use ramp_api::middleware::versioning::{
    version_negotiation_middleware, TenantApiVersion, VersionContext, VersionSource,
};
use ramp_api::versioning::{ApiVersion, TransformerRegistry};
use serde_json::{json, Value};
use tokio::net::TcpListener;

// =============================================================================
// Test helpers
// =============================================================================

/// Start a real HTTP server on a random port, returning the base URL.
async fn start_server(app: Router) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give the server a moment to start accepting connections
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    base_url
}

/// A handler that echoes back version context information as JSON.
async fn echo_version_handler(req: Request) -> Response {
    let version_ctx = req.extensions().get::<VersionContext>().cloned();

    match version_ctx {
        Some(ctx) => {
            let source_str = match ctx.source {
                VersionSource::Header => "header",
                VersionSource::TenantPinned => "tenant_pinned",
                VersionSource::Default => "default",
            };
            Json(json!({
                "version": ctx.version.to_string(),
                "source": source_str,
            }))
            .into_response()
        }
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "no version context"})),
        )
            .into_response(),
    }
}

/// A handler that returns a response in the latest (v2026-03-01) internal format.
/// The response uses the new field names: amount_minor, currency, status=awaiting_confirmation.
async fn latest_format_handler() -> Json<Value> {
    Json(json!({
        "id": "intent_abc123",
        "amount_minor": 50000,
        "currency": "VND",
        "status": "awaiting_confirmation",
        "api_version": "2026-03-01",
        "description": "Test payment"
    }))
}

/// A POST handler that echoes back the received JSON body.
/// This is used to test request transformation (upgrade).
async fn echo_body_handler(Json(body): Json<Value>) -> Json<Value> {
    Json(body)
}

/// Build a basic app with the versioning middleware and the echo handler.
fn build_versioning_app() -> Router {
    Router::new()
        .route("/version", get(echo_version_handler))
        .route("/test", get(|| async { "ok" }))
        .layer(middleware::from_fn(version_negotiation_middleware))
}

/// Build an app that also injects a TenantApiVersion before versioning middleware.
fn build_tenant_pinned_app(pinned_version: &str) -> Router {
    let pinned = ApiVersion::parse(pinned_version).expect("valid pinned version");

    Router::new()
        .route("/version", get(echo_version_handler))
        .route("/test", get(|| async { "ok" }))
        .layer(middleware::from_fn(version_negotiation_middleware))
        .layer(middleware::from_fn(
            move |mut req: Request, next: Next| {
                let version = pinned.clone();
                async move {
                    req.extensions_mut().insert(TenantApiVersion { version });
                    Ok::<_, StatusCode>(next.run(req).await)
                }
            },
        ))
}

/// Build an app that applies response downgrade transformations.
/// The handler returns data in the latest format, and a post-middleware
/// downgrades it based on the negotiated version.
fn build_transformer_app() -> Router {
    let registry = TransformerRegistry::new();

    Router::new()
        .route("/payment", get(latest_format_handler))
        .route("/payment", post(echo_body_handler))
        .layer(middleware::from_fn(move |req: Request, next: Next| {
            let reg = registry.clone();
            async move {
                // Extract the version context (set by the versioning middleware)
                let version_ctx = req.extensions().get::<VersionContext>().cloned();

                let response = next.run(req).await;

                // If we have a version context, downgrade the response
                if let Some(ctx) = version_ctx {
                    let latest = ApiVersion::latest();
                    if ctx.version < latest {
                        // Read the response body
                        let (parts, body) = response.into_parts();
                        let bytes = axum::body::to_bytes(body, 65536).await.unwrap();

                        if let Ok(payload) = serde_json::from_slice::<Value>(&bytes) {
                            let downgraded = reg
                                .downgrade_response(&ctx.version, &latest, payload)
                                .unwrap();
                            let mut resp = Json(downgraded).into_response();
                            *resp.status_mut() = parts.status;
                            // Copy version header
                            for (k, v) in parts.headers.iter() {
                                resp.headers_mut().insert(k.clone(), v.clone());
                            }
                            return resp;
                        }

                        // If body is not JSON, reassemble the response unchanged
                        let resp = Response::from_parts(parts, axum::body::Body::from(bytes));
                        return resp;
                    }
                }

                response
            }
        }))
        .layer(middleware::from_fn(version_negotiation_middleware))
}

/// Build an app that applies request upgrade transformations.
/// Incoming requests from older clients get their payloads upgraded to the latest format.
fn build_request_upgrade_app() -> Router {
    let registry = TransformerRegistry::new();

    Router::new()
        .route("/payment", post(echo_body_handler))
        .layer(middleware::from_fn(move |req: Request, next: Next| {
            let reg = registry.clone();
            async move {
                let version_ctx = req.extensions().get::<VersionContext>().cloned();
                let latest = ApiVersion::latest();

                if let Some(ctx) = &version_ctx {
                    if ctx.version < latest {
                        // Read the request body
                        let (mut parts, body) = req.into_parts();
                        let bytes = axum::body::to_bytes(body, 65536).await.unwrap();

                        if let Ok(payload) = serde_json::from_slice::<Value>(&bytes) {
                            let upgraded = reg
                                .upgrade_request(&ctx.version, &latest, payload)
                                .unwrap();
                            let new_body = serde_json::to_vec(&upgraded).unwrap();
                            // Update content-length
                            parts.headers.insert(
                                axum::http::header::CONTENT_LENGTH,
                                new_body.len().to_string().parse().unwrap(),
                            );
                            let req = Request::from_parts(parts, axum::body::Body::from(new_body));
                            return next.run(req).await;
                        }

                        // Reassemble if not JSON
                        let req = Request::from_parts(parts, axum::body::Body::from(bytes));
                        return next.run(req).await;
                    }
                }

                next.run(req).await
            }
        }))
        .layer(middleware::from_fn(version_negotiation_middleware))
}

// =============================================================================
// Test 1: Client sends RampOS-Version header -> server negotiates correct version
// =============================================================================

#[tokio::test]
async fn test_version_header_negotiation() {
    let app = build_versioning_app();
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    // Test with v2026-02-01
    let resp = client
        .get(format!("{}/version", base_url))
        .header("RampOS-Version", "2026-02-01")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["version"], "2026-02-01");
    assert_eq!(body["source"], "header");

    // Test with v2026-03-01
    let resp = client
        .get(format!("{}/version", base_url))
        .header("RampOS-Version", "2026-03-01")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["version"], "2026-03-01");
    assert_eq!(body["source"], "header");
}

// =============================================================================
// Test 2: Missing version header -> server uses default version
// =============================================================================

#[tokio::test]
async fn test_missing_version_header_uses_default() {
    let app = build_versioning_app();
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
        "Should default to 2026-02-01 when no header is present"
    );
    assert_eq!(
        body["source"], "default",
        "Source should be 'default' when no header is provided"
    );
}

// =============================================================================
// Test 3: Invalid version format -> server returns 400
// =============================================================================

#[tokio::test]
async fn test_invalid_version_format_returns_400() {
    let app = build_versioning_app();
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    // Test various invalid formats
    let invalid_versions = vec![
        "invalid",
        "not-a-date",
        "2026/02/01",
        "",
        "v1",
        "2026-13-01",
        "abc-def-ghi",
        "2026-02-32",
    ];

    for invalid in invalid_versions {
        let resp = client
            .get(format!("{}/test", base_url))
            .header("RampOS-Version", invalid)
            .send()
            .await
            .unwrap();

        assert_eq!(
            resp.status().as_u16(),
            400,
            "Version '{}' should return 400 Bad Request",
            invalid
        );

        let body: Value = resp.json().await.unwrap();
        assert_eq!(
            body["error"]["code"], "INVALID_VERSION_FORMAT",
            "Error code for '{}' should be INVALID_VERSION_FORMAT",
            invalid
        );
        assert!(
            body["error"]["message"].as_str().unwrap().contains(invalid),
            "Error message should contain the invalid version string"
        );
    }
}

// =============================================================================
// Test 4: Version too old (before minimum) -> server returns 400 with VERSION_TOO_OLD
// =============================================================================

#[tokio::test]
async fn test_version_too_old_returns_400() {
    let app = build_versioning_app();
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    let old_versions = vec!["2020-01-01", "2025-01-01", "2026-01-31"];

    for old_ver in old_versions {
        let resp = client
            .get(format!("{}/test", base_url))
            .header("RampOS-Version", old_ver)
            .send()
            .await
            .unwrap();

        assert_eq!(
            resp.status().as_u16(),
            400,
            "Version '{}' should return 400 (too old)",
            old_ver
        );

        let body: Value = resp.json().await.unwrap();
        assert_eq!(
            body["error"]["code"], "VERSION_TOO_OLD",
            "Version '{}' should get VERSION_TOO_OLD error code",
            old_ver
        );
        assert_eq!(
            body["error"]["minimum_version"], "2026-02-01",
            "Error should include minimum_version"
        );
        assert_eq!(
            body["error"]["latest_version"], "2026-03-01",
            "Error should include latest_version"
        );
    }
}

// =============================================================================
// Test 5: Future/unknown version returns 400 with VERSION_UNKNOWN
// =============================================================================

#[tokio::test]
async fn test_future_version_returns_400_version_unknown() {
    let app = build_versioning_app();
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    let future_versions = vec!["2030-01-01", "2028-06-15", "2099-12-31"];

    for future_ver in future_versions {
        let resp = client
            .get(format!("{}/test", base_url))
            .header("RampOS-Version", future_ver)
            .send()
            .await
            .unwrap();

        assert_eq!(
            resp.status().as_u16(),
            400,
            "Version '{}' should return 400 (unknown/future)",
            future_ver
        );

        let body: Value = resp.json().await.unwrap();
        assert_eq!(
            body["error"]["code"], "VERSION_UNKNOWN",
            "Version '{}' should get VERSION_UNKNOWN error code",
            future_ver
        );
    }
}

// =============================================================================
// Test 6: Response includes RampOS-Version header
// =============================================================================

#[tokio::test]
async fn test_response_includes_version_header() {
    let app = build_versioning_app();
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    // With explicit header
    let resp = client
        .get(format!("{}/test", base_url))
        .header("RampOS-Version", "2026-03-01")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let version_header = resp
        .headers()
        .get("RampOS-Version")
        .expect("Response should include RampOS-Version header")
        .to_str()
        .unwrap();
    assert_eq!(
        version_header, "2026-03-01",
        "Response version header should echo back the requested version"
    );

    // Without header (should echo default version)
    let resp = client
        .get(format!("{}/test", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let version_header = resp
        .headers()
        .get("RampOS-Version")
        .expect("Response should include RampOS-Version header even for default")
        .to_str()
        .unwrap();
    assert_eq!(
        version_header, "2026-02-01",
        "Response version header should be the default version when no header was sent"
    );

    // With oldest supported version
    let resp = client
        .get(format!("{}/test", base_url))
        .header("RampOS-Version", "2026-02-01")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let version_header = resp
        .headers()
        .get("RampOS-Version")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(version_header, "2026-02-01");
}

// =============================================================================
// Test 7: Version context is correctly passed to handlers
// =============================================================================

#[tokio::test]
async fn test_version_context_passed_to_handlers() {
    let app = build_versioning_app();
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    // Test header source
    let resp = client
        .get(format!("{}/version", base_url))
        .header("RampOS-Version", "2026-02-01")
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["version"], "2026-02-01");
    assert_eq!(body["source"], "header");

    // Test default source
    let resp = client
        .get(format!("{}/version", base_url))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["version"], "2026-02-01");
    assert_eq!(body["source"], "default");
}

// =============================================================================
// Test 8: Tenant pinned version context
// =============================================================================

#[tokio::test]
async fn test_tenant_pinned_version_used_as_fallback() {
    let app = build_tenant_pinned_app("2026-03-01");
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    // Without explicit header, should use tenant's pinned version
    let resp = client
        .get(format!("{}/version", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(
        body["version"], "2026-03-01",
        "Should use tenant pinned version when no header is set"
    );
    assert_eq!(body["source"], "tenant_pinned");

    // Version response header should also reflect tenant pinned version
    let resp = client
        .get(format!("{}/test", base_url))
        .send()
        .await
        .unwrap();
    let version_header = resp
        .headers()
        .get("RampOS-Version")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(version_header, "2026-03-01");
}

// =============================================================================
// Test 9: Header takes precedence over tenant pinned version
// =============================================================================

#[tokio::test]
async fn test_header_takes_precedence_over_tenant_pinned() {
    let app = build_tenant_pinned_app("2026-03-01");
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    // Explicitly request an older version via header
    let resp = client
        .get(format!("{}/version", base_url))
        .header("RampOS-Version", "2026-02-01")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(
        body["version"], "2026-02-01",
        "Header should override tenant pinned version"
    );
    assert_eq!(
        body["source"], "header",
        "Source should be 'header' when explicitly set"
    );
}

// =============================================================================
// Test 10: Response downgrade through transformer chain
// =============================================================================

#[tokio::test]
async fn test_response_downgrade_transformer_chain() {
    let app = build_transformer_app();
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    // Request with the older version (v2026-02-01) - response should be downgraded
    let resp = client
        .get(format!("{}/payment", base_url))
        .header("RampOS-Version", "2026-02-01")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 200);
    let body: Value = resp.json().await.unwrap();

    // Field renaming: amount_minor -> amount
    assert_eq!(
        body["amount"], 50000,
        "amount_minor should be renamed back to amount for v1 clients"
    );
    assert!(
        body.get("amount_minor").is_none(),
        "amount_minor should not be present in v1 response"
    );

    // Status mapping: awaiting_confirmation -> pending
    assert_eq!(
        body["status"], "pending",
        "Status 'awaiting_confirmation' should be mapped back to 'pending' for v1 clients"
    );

    // api_version field removed for v1
    assert!(
        body.get("api_version").is_none(),
        "api_version field should be removed for v1 clients"
    );

    // currency field removed for v1
    assert!(
        body.get("currency").is_none(),
        "currency field should be removed for v1 clients"
    );

    // Unrelated fields preserved
    assert_eq!(body["id"], "intent_abc123", "Unrelated fields should be preserved");
    assert_eq!(body["description"], "Test payment");
}

// =============================================================================
// Test 11: No downgrade when requesting with latest version
// =============================================================================

#[tokio::test]
async fn test_no_downgrade_for_latest_version() {
    let app = build_transformer_app();
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    // Request with the latest version - no transformation should occur
    let resp = client
        .get(format!("{}/payment", base_url))
        .header("RampOS-Version", "2026-03-01")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 200);
    let body: Value = resp.json().await.unwrap();

    // All latest-format fields should be present unchanged
    assert_eq!(body["amount_minor"], 50000);
    assert_eq!(body["currency"], "VND");
    assert_eq!(body["status"], "awaiting_confirmation");
    assert_eq!(body["api_version"], "2026-03-01");
    assert_eq!(body["id"], "intent_abc123");
}

// =============================================================================
// Test 12: Request upgrade through transformer chain
// =============================================================================

#[tokio::test]
async fn test_request_upgrade_transformer_chain() {
    let app = build_request_upgrade_app();
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    // Send a request in v1 format (old field names)
    let v1_request = json!({
        "amount": 75000,
        "description": "Old-style payment"
    });

    let resp = client
        .post(format!("{}/payment", base_url))
        .header("RampOS-Version", "2026-02-01")
        .header("Content-Type", "application/json")
        .json(&v1_request)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 200);
    let body: Value = resp.json().await.unwrap();

    // After upgrade: amount -> amount_minor
    assert_eq!(
        body["amount_minor"], 75000,
        "After upgrade, 'amount' should become 'amount_minor'"
    );
    assert!(
        body.get("amount").is_none(),
        "'amount' should be removed after upgrade"
    );

    // After upgrade: currency defaults to VND
    assert_eq!(
        body["currency"], "VND",
        "Default currency 'VND' should be added during upgrade"
    );

    // Unrelated fields preserved
    assert_eq!(body["description"], "Old-style payment");
}

// =============================================================================
// Test 13: Request with latest version passes through unchanged
// =============================================================================

#[tokio::test]
async fn test_request_no_upgrade_for_latest_version() {
    let app = build_request_upgrade_app();
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    // Send a request already in latest format
    let latest_request = json!({
        "amount_minor": 99000,
        "currency": "USD",
        "description": "Latest-format payment"
    });

    let resp = client
        .post(format!("{}/payment", base_url))
        .header("RampOS-Version", "2026-03-01")
        .header("Content-Type", "application/json")
        .json(&latest_request)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 200);
    let body: Value = resp.json().await.unwrap();

    // No transformation should happen
    assert_eq!(body["amount_minor"], 99000);
    assert_eq!(body["currency"], "USD");
    assert_eq!(body["description"], "Latest-format payment");
    assert!(body.get("amount").is_none(), "No 'amount' field should appear");
}

// =============================================================================
// Test 14: Request upgrade preserves explicit currency
// =============================================================================

#[tokio::test]
async fn test_request_upgrade_preserves_explicit_currency() {
    let app = build_request_upgrade_app();
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    // Old client explicitly sends currency
    let v1_request = json!({
        "amount": 10000,
        "currency": "USD"
    });

    let resp = client
        .post(format!("{}/payment", base_url))
        .header("RampOS-Version", "2026-02-01")
        .header("Content-Type", "application/json")
        .json(&v1_request)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 200);
    let body: Value = resp.json().await.unwrap();

    // Currency should be preserved (not overridden with VND default)
    assert_eq!(
        body["currency"], "USD",
        "Explicit currency from old client should be preserved during upgrade"
    );
    assert_eq!(body["amount_minor"], 10000);
}

// =============================================================================
// Test 15: Error response body structure validation
// =============================================================================

#[tokio::test]
async fn test_error_response_body_structure() {
    let app = build_versioning_app();
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    // Invalid format error
    let resp = client
        .get(format!("{}/test", base_url))
        .header("RampOS-Version", "not-valid")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 400);
    let body: Value = resp.json().await.unwrap();

    // Validate full error structure
    assert!(body["error"].is_object(), "Error response should have an 'error' object");
    assert!(
        body["error"]["code"].is_string(),
        "Error should have a 'code' string"
    );
    assert!(
        body["error"]["message"].is_string(),
        "Error should have a 'message' string"
    );
    assert!(
        body["error"]["minimum_version"].is_string(),
        "Error should include 'minimum_version'"
    );
    assert!(
        body["error"]["latest_version"].is_string(),
        "Error should include 'latest_version'"
    );
    assert_eq!(body["error"]["minimum_version"], "2026-02-01");
    assert_eq!(body["error"]["latest_version"], "2026-03-01");
}

// =============================================================================
// Test 16: Multiple sequential requests maintain correct versioning
// =============================================================================

#[tokio::test]
async fn test_multiple_sequential_requests_different_versions() {
    let app = build_versioning_app();
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    // Send alternating version requests to ensure no state leakage
    let versions = vec![
        "2026-02-01",
        "2026-03-01",
        "2026-02-01",
        "2026-03-01",
        "2026-02-01",
    ];

    for version in &versions {
        let resp = client
            .get(format!("{}/version", base_url))
            .header("RampOS-Version", *version)
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status().as_u16(), 200);

        // Extract header before consuming response with .json()
        let resp_header = resp
            .headers()
            .get("RampOS-Version")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert_eq!(resp_header, *version);

        let body: Value = resp.json().await.unwrap();
        assert_eq!(
            body["version"].as_str().unwrap(),
            *version,
            "Each request should independently resolve its version"
        );
    }
}

// =============================================================================
// Test 17: Concurrent requests with different versions
// =============================================================================

#[tokio::test]
async fn test_concurrent_requests_different_versions() {
    let app = build_versioning_app();
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    let mut handles = Vec::new();

    // Spawn 10 concurrent requests alternating between versions
    for i in 0..10 {
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

            let resp_ver = resp
                .headers()
                .get("RampOS-Version")
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();

            let body: Value = resp.json().await.unwrap();
            let body_ver = body["version"].as_str().unwrap().to_string();

            (ver, resp_ver, body_ver)
        }));
    }

    for h in handles {
        let (requested, resp_header, body_version) = h.await.unwrap();
        assert_eq!(
            resp_header, requested,
            "Response header should match requested version"
        );
        assert_eq!(
            body_version, requested,
            "Body version should match requested version"
        );
    }
}

// =============================================================================
// Test 18: Invalid version does not set response header
// =============================================================================

#[tokio::test]
async fn test_invalid_version_no_response_header() {
    let app = build_versioning_app();
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/test", base_url))
        .header("RampOS-Version", "bad-version")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 400);

    // On error, the middleware returns early without setting the response header
    // The error response should NOT have a RampOS-Version header
    let has_version_header = resp.headers().get("RampOS-Version").is_some();
    assert!(
        !has_version_header,
        "Error responses should not include a RampOS-Version response header"
    );
}

// =============================================================================
// Test 19: Response downgrade preserves non-transformed fields
// =============================================================================

#[tokio::test]
async fn test_response_downgrade_preserves_non_transformed_fields() {
    let app = build_transformer_app();
    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/payment", base_url))
        .header("RampOS-Version", "2026-02-01")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 200);
    let body: Value = resp.json().await.unwrap();

    // These fields should be preserved through the transformation
    assert_eq!(body["id"], "intent_abc123");
    assert_eq!(body["description"], "Test payment");

    // Transformed fields
    assert_eq!(body["amount"], 50000);
    assert_eq!(body["status"], "pending");
}

// =============================================================================
// Test 20: Full round-trip: upgrade request + downgrade response
// =============================================================================

#[tokio::test]
async fn test_full_roundtrip_upgrade_and_downgrade() {
    // Build an app that does both request upgrade AND response downgrade
    let registry_req = TransformerRegistry::new();
    let registry_resp = TransformerRegistry::new();

    let app = Router::new()
        .route(
            "/payment",
            post(|Json(body): Json<Value>| async move {
                // Handler receives upgraded (latest-format) request
                // and returns a response in latest format
                let amount = body["amount_minor"].as_i64().unwrap_or(0);
                Json(json!({
                    "id": "intent_roundtrip",
                    "amount_minor": amount,
                    "currency": body["currency"].as_str().unwrap_or("VND"),
                    "status": "awaiting_confirmation",
                    "api_version": "2026-03-01"
                }))
            }),
        )
        // Response downgrade layer (outermost, runs last on request / first on response)
        .layer(middleware::from_fn(move |req: Request, next: Next| {
            let reg = registry_resp.clone();
            async move {
                let version_ctx = req.extensions().get::<VersionContext>().cloned();
                let response = next.run(req).await;

                if let Some(ctx) = version_ctx {
                    let latest = ApiVersion::latest();
                    if ctx.version < latest {
                        let (parts, body) = response.into_parts();
                        let bytes = axum::body::to_bytes(body, 65536).await.unwrap();
                        if let Ok(payload) = serde_json::from_slice::<Value>(&bytes) {
                            let downgraded =
                                reg.downgrade_response(&ctx.version, &latest, payload).unwrap();
                            let mut resp = Json(downgraded).into_response();
                            *resp.status_mut() = parts.status;
                            for (k, v) in parts.headers.iter() {
                                resp.headers_mut().insert(k.clone(), v.clone());
                            }
                            return resp;
                        }
                        return Response::from_parts(parts, axum::body::Body::from(bytes));
                    }
                }
                response
            }
        }))
        // Request upgrade layer
        .layer(middleware::from_fn(move |req: Request, next: Next| {
            let reg = registry_req.clone();
            async move {
                let version_ctx = req.extensions().get::<VersionContext>().cloned();
                let latest = ApiVersion::latest();

                if let Some(ctx) = &version_ctx {
                    if ctx.version < latest {
                        let (mut parts, body) = req.into_parts();
                        let bytes = axum::body::to_bytes(body, 65536).await.unwrap();
                        if let Ok(payload) = serde_json::from_slice::<Value>(&bytes) {
                            let upgraded =
                                reg.upgrade_request(&ctx.version, &latest, payload).unwrap();
                            let new_body = serde_json::to_vec(&upgraded).unwrap();
                            parts.headers.insert(
                                axum::http::header::CONTENT_LENGTH,
                                new_body.len().to_string().parse().unwrap(),
                            );
                            let req =
                                Request::from_parts(parts, axum::body::Body::from(new_body));
                            return next.run(req).await;
                        }
                        let req = Request::from_parts(parts, axum::body::Body::from(bytes));
                        return next.run(req).await;
                    }
                }
                next.run(req).await
            }
        }))
        // Version negotiation (innermost layer)
        .layer(middleware::from_fn(version_negotiation_middleware));

    let base_url = start_server(app).await;
    let client = reqwest::Client::new();

    // Client sends v1-format request
    let v1_request = json!({
        "amount": 42000,
        "description": "Round-trip test"
    });

    let resp = client
        .post(format!("{}/payment", base_url))
        .header("RampOS-Version", "2026-02-01")
        .header("Content-Type", "application/json")
        .json(&v1_request)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 200);
    let body: Value = resp.json().await.unwrap();

    // Client should receive v1-format response
    assert_eq!(
        body["amount"], 42000,
        "Round-trip: response should use v1 field name 'amount'"
    );
    assert!(
        body.get("amount_minor").is_none(),
        "Round-trip: 'amount_minor' should not be in v1 response"
    );
    assert_eq!(
        body["status"], "pending",
        "Round-trip: status should be 'pending' in v1 format"
    );
    assert!(
        body.get("api_version").is_none(),
        "Round-trip: 'api_version' should not be in v1 response"
    );
    assert!(
        body.get("currency").is_none(),
        "Round-trip: 'currency' should not be in v1 response"
    );
    assert_eq!(body["id"], "intent_roundtrip");
}
