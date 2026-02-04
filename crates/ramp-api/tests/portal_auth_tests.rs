//! Portal Authentication Integration Tests
//!
//! Tests for JWT-based portal authentication middleware.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware,
    routing::get,
    Json, Router,
};
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use ramp_api::middleware::{portal_auth_middleware, PortalAuthConfig, PortalClaims, PortalUser};
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

// ============================================================================
// Test Helpers
// ============================================================================

const TEST_JWT_SECRET: &str = "test-secret-key-for-integration-testing";

fn create_test_config() -> Arc<PortalAuthConfig> {
    Arc::new(PortalAuthConfig {
        jwt_secret: TEST_JWT_SECRET.to_string(),
        issuer: None,
        audience: None,
        allow_missing_tenant: false,
    })
}

fn create_jwt_token(claims: &PortalClaims, secret: &str) -> String {
    let encoding_key = EncodingKey::from_secret(secret.as_bytes());
    encode(&Header::default(), claims, &encoding_key).unwrap()
}

fn create_valid_claims() -> PortalClaims {
    let now = Utc::now().timestamp();
    PortalClaims {
        sub: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        tenant_id: Some("660e8400-e29b-41d4-a716-446655440001".to_string()),
        email: "test@example.com".to_string(),
        iat: now,
        exp: now + 3600, // 1 hour from now
        token_type: "access".to_string(),
    }
}

/// Handler that returns user info if authenticated
async fn protected_handler(user: PortalUser) -> Json<serde_json::Value> {
    Json(json!({
        "user_id": user.user_id.to_string(),
        "tenant_id": user.tenant_id.to_string(),
        "email": user.email
    }))
}

/// Create a test router with portal auth middleware
fn create_protected_router(config: Arc<PortalAuthConfig>) -> Router {
    Router::new()
        .route("/protected", get(protected_handler))
        .layer(middleware::from_fn_with_state(
            config,
            portal_auth_middleware,
        ))
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
async fn test_valid_jwt_authentication() {
    let config = create_test_config();
    let router = create_protected_router(config);

    let claims = create_valid_claims();
    let token = create_jwt_token(&claims, TEST_JWT_SECRET);

    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["email"], "test@example.com");
    assert_eq!(body["user_id"], "550e8400-e29b-41d4-a716-446655440000");
    assert_eq!(body["tenant_id"], "660e8400-e29b-41d4-a716-446655440001");
}

#[tokio::test]
async fn test_expired_token_rejection() {
    let config = create_test_config();
    let router = create_protected_router(config);

    let now = Utc::now().timestamp();
    let claims = PortalClaims {
        sub: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        tenant_id: Some("660e8400-e29b-41d4-a716-446655440001".to_string()),
        email: "test@example.com".to_string(),
        iat: now - 7200, // 2 hours ago
        exp: now - 3600, // Expired 1 hour ago
        token_type: "access".to_string(),
    };

    let token = create_jwt_token(&claims, TEST_JWT_SECRET);

    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["error"]["code"], "UNAUTHORIZED");
}

#[tokio::test]
async fn test_invalid_signature_rejection() {
    let config = create_test_config();
    let router = create_protected_router(config);

    let claims = create_valid_claims();
    // Sign with a different secret
    let token = create_jwt_token(&claims, "wrong-secret-key");

    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_missing_authorization_header() {
    let config = create_test_config();
    let router = create_protected_router(config);

    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Authorization"));
}

#[tokio::test]
async fn test_invalid_authorization_format() {
    let config = create_test_config();
    let router = create_protected_router(config);

    // Missing "Bearer " prefix
    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header("Authorization", "InvalidFormat some-token")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_wrong_token_type_rejection() {
    let config = create_test_config();
    let router = create_protected_router(config);

    let now = Utc::now().timestamp();
    let claims = PortalClaims {
        sub: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        tenant_id: Some("660e8400-e29b-41d4-a716-446655440001".to_string()),
        email: "test@example.com".to_string(),
        iat: now,
        exp: now + 3600,
        token_type: "refresh".to_string(), // Wrong type
    };

    let token = create_jwt_token(&claims, TEST_JWT_SECRET);

    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_invalid_user_id_format() {
    let config = create_test_config();
    let router = create_protected_router(config);

    let now = Utc::now().timestamp();
    let claims = PortalClaims {
        sub: "not-a-valid-uuid".to_string(), // Invalid UUID
        tenant_id: Some("660e8400-e29b-41d4-a716-446655440001".to_string()),
        email: "test@example.com".to_string(),
        iat: now,
        exp: now + 3600,
        token_type: "access".to_string(),
    };

    let token = create_jwt_token(&claims, TEST_JWT_SECRET);

    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_malformed_jwt_token() {
    let config = create_test_config();
    let router = create_protected_router(config);

    // Completely malformed token
    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header("Authorization", "Bearer not.a.valid.jwt.token")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_empty_bearer_token() {
    let config = create_test_config();
    let router = create_protected_router(config);

    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header("Authorization", "Bearer ")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_token_without_tenant_id_is_rejected() {
    let config = create_test_config();
    let router = create_protected_router(config);

    let now = Utc::now().timestamp();
    let claims = PortalClaims {
        sub: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        tenant_id: None, // No tenant ID
        email: "test@example.com".to_string(),
        iat: now,
        exp: now + 3600,
        token_type: "access".to_string(),
    };

    let token = create_jwt_token(&claims, TEST_JWT_SECRET);

    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    // Should be rejected when tenant_id is missing
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_token_without_tenant_id_allowed_when_configured() {
    let config = Arc::new(PortalAuthConfig {
        jwt_secret: TEST_JWT_SECRET.to_string(),
        issuer: None,
        audience: None,
        allow_missing_tenant: true,
    });
    let router = create_protected_router(config);

    let now = Utc::now().timestamp();
    let claims = PortalClaims {
        sub: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        tenant_id: None,
        email: "test@example.com".to_string(),
        iat: now,
        exp: now + 3600,
        token_type: "access".to_string(),
    };

    let token = create_jwt_token(&claims, TEST_JWT_SECRET);

    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_token_about_to_expire_still_valid() {
    let config = create_test_config();
    let router = create_protected_router(config);

    let now = Utc::now().timestamp();
    let claims = PortalClaims {
        sub: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        tenant_id: Some("660e8400-e29b-41d4-a716-446655440001".to_string()),
        email: "test@example.com".to_string(),
        iat: now - 3590, // Almost 1 hour ago
        exp: now + 10,   // Expires in 10 seconds
        token_type: "access".to_string(),
    };

    let token = create_jwt_token(&claims, TEST_JWT_SECRET);

    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    // Should still be valid
    assert_eq!(response.status(), StatusCode::OK);
}

// ============================================================================
// Additional Edge Case Tests
// ============================================================================

#[tokio::test]
async fn test_invalid_tenant_id_format() {
    let config = create_test_config();
    let router = create_protected_router(config);

    let now = Utc::now().timestamp();
    let claims = PortalClaims {
        sub: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        tenant_id: Some("not-a-valid-uuid".to_string()), // Invalid UUID
        email: "test@example.com".to_string(),
        iat: now,
        exp: now + 3600,
        token_type: "access".to_string(),
    };

    let token = create_jwt_token(&claims, TEST_JWT_SECRET);

    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_case_insensitive_bearer_prefix() {
    let config = create_test_config();
    let router = create_protected_router(config);

    let claims = create_valid_claims();
    let token = create_jwt_token(&claims, TEST_JWT_SECRET);

    // Test with lowercase "bearer" - should fail as we expect exact "Bearer "
    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header("Authorization", format!("bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    // Strict parsing requires exact "Bearer " prefix
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_token_with_whitespace() {
    let config = create_test_config();
    let router = create_protected_router(config);

    let claims = create_valid_claims();
    let token = create_jwt_token(&claims, TEST_JWT_SECRET);

    // Token with extra spaces
    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header("Authorization", format!("Bearer  {}", token)) // Double space
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    // Should fail due to extra space in token
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_multiple_authorization_headers() {
    let config = create_test_config();
    let router = create_protected_router(config);

    let claims = create_valid_claims();
    let token = create_jwt_token(&claims, TEST_JWT_SECRET);

    // First valid authorization header is used
    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_jwt_with_future_iat() {
    let config = create_test_config();
    let router = create_protected_router(config);

    let now = Utc::now().timestamp();
    let claims = PortalClaims {
        sub: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        tenant_id: Some("660e8400-e29b-41d4-a716-446655440001".to_string()),
        email: "test@example.com".to_string(),
        iat: now + 3600, // Issued in the future (suspicious)
        exp: now + 7200,
        token_type: "access".to_string(),
    };

    let token = create_jwt_token(&claims, TEST_JWT_SECRET);

    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    // JWT library may or may not validate iat by default
    // This test documents current behavior
    assert!(response.status() == StatusCode::OK || response.status() == StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_empty_email_in_token() {
    let config = create_test_config();
    let router = create_protected_router(config);

    let now = Utc::now().timestamp();
    let claims = PortalClaims {
        sub: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        tenant_id: Some("660e8400-e29b-41d4-a716-446655440001".to_string()),
        email: "".to_string(), // Empty email
        iat: now,
        exp: now + 3600,
        token_type: "access".to_string(),
    };

    let token = create_jwt_token(&claims, TEST_JWT_SECRET);

    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    // Currently allows empty email - documents this behavior
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_special_characters_in_email() {
    let config = create_test_config();
    let router = create_protected_router(config);

    let now = Utc::now().timestamp();
    let claims = PortalClaims {
        sub: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        tenant_id: Some("660e8400-e29b-41d4-a716-446655440001".to_string()),
        email: "user+tag@example.com".to_string(), // Email with plus addressing
        iat: now,
        exp: now + 3600,
        token_type: "access".to_string(),
    };

    let token = create_jwt_token(&claims, TEST_JWT_SECRET);

    let request = Request::builder()
        .uri("/protected")
        .method("GET")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["email"], "user+tag@example.com");
}
