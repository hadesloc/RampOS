use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::{self, from_fn_with_state},
    routing::post,
    Router,
};
use ramp_api::middleware::{
    idempotency_middleware, IdempotencyConfig, IdempotencyHandler, IdempotencyStore,
    StoredResponse,
};
use std::sync::Arc;
use tower::ServiceExt; // for oneshot

struct FailingStore;

#[async_trait::async_trait]
impl IdempotencyStore for FailingStore {
    async fn get(&self, _tenant: &str, _key: &str, _prefix: &str) -> Option<StoredResponse> {
        None
    }

    async fn store(
        &self,
        _tenant: &str,
        _key: &str,
        _resp: &StoredResponse,
        _ttl: u64,
        _prefix: &str,
    ) -> Result<(), String> {
        Err("store error".to_string())
    }

    async fn try_lock(&self, _tenant: &str, _key: &str, _prefix: &str) -> Result<bool, String> {
        Err("lock error".to_string())
    }

    async fn unlock(&self, _tenant: &str, _key: &str, _prefix: &str) -> Result<(), String> {
        Ok(())
    }
}

struct StoreFailingStore;

#[async_trait::async_trait]
impl IdempotencyStore for StoreFailingStore {
    async fn get(&self, _tenant: &str, _key: &str, _prefix: &str) -> Option<StoredResponse> {
        None
    }

    async fn store(
        &self,
        _tenant: &str,
        _key: &str,
        _resp: &StoredResponse,
        _ttl: u64,
        _prefix: &str,
    ) -> Result<(), String> {
        Err("store error".to_string())
    }

    async fn try_lock(&self, _tenant: &str, _key: &str, _prefix: &str) -> Result<bool, String> {
        Ok(true)
    }

    async fn unlock(&self, _tenant: &str, _key: &str, _prefix: &str) -> Result<(), String> {
        Ok(())
    }
}

#[tokio::test]
async fn test_idempotency_lock_error_returns_503() {
    let config = IdempotencyConfig::default();
    let handler = Arc::new(IdempotencyHandler::new(Arc::new(FailingStore), config));

    let app = Router::new()
        .route("/", post(|| async { "Success" }))
        .layer(from_fn_with_state(handler.clone(), idempotency_middleware));

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("X-Idempotency-Key", "key-fail")
        .body(Body::from("Request Body A"))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["error"], "idempotency_store_unavailable");
    assert_eq!(
        payload["message"],
        "Idempotency store error; request may have been processed"
    );
}

#[tokio::test]
async fn test_idempotency_store_failure_after_lock_returns_503() {
    let config = IdempotencyConfig::default();
    let handler = Arc::new(IdempotencyHandler::new(Arc::new(StoreFailingStore), config));

    let app = Router::new()
        .route("/", post(|| async { "Success" }))
        .layer(from_fn_with_state(handler.clone(), idempotency_middleware));

    let req = Request::builder()
        .method("POST")
        .uri("/")
        .header("X-Idempotency-Key", "key-store-fail")
        .body(Body::from("Request Body A"))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn test_idempotency_different_request_hash_conflict() {
    // Setup
    let config = IdempotencyConfig::default();
    let handler = Arc::new(IdempotencyHandler::with_memory(config));

    let app = Router::new()
        .route("/", post(|| async { "Success" }))
        .layer(from_fn_with_state(handler.clone(), idempotency_middleware));

    // Request 1
    let req1 = Request::builder()
        .method("POST")
        .uri("/")
        .header("X-Idempotency-Key", "key-1")
        .body(Body::from("Request Body A"))
        .unwrap();

    let response1 = app.clone().oneshot(req1).await.unwrap();
    assert_eq!(response1.status(), StatusCode::OK);

    let body1 = axum::body::to_bytes(response1.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(body1, "Success");

    // Request 2 - Same key, different body
    let req2 = Request::builder()
        .method("POST")
        .uri("/")
        .header("X-Idempotency-Key", "key-1")
        .body(Body::from("Request Body B")) // Different body
        .unwrap();

    let response2 = app.clone().oneshot(req2).await.unwrap();

    // Expect 409 Conflict because body is different
    // Currently this will likely fail (return 200 OK)
    assert_eq!(response2.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_idempotency_same_request_hash_success() {
    // Setup
    let config = IdempotencyConfig::default();
    let handler = Arc::new(IdempotencyHandler::with_memory(config));

    let app = Router::new()
        .route("/", post(|| async { "Success" }))
        .layer(from_fn_with_state(handler.clone(), idempotency_middleware));

    // Request 1
    let req1 = Request::builder()
        .method("POST")
        .uri("/")
        .header("X-Idempotency-Key", "key-2")
        .body(Body::from("Request Body A"))
        .unwrap();

    let response1 = app.clone().oneshot(req1).await.unwrap();
    assert_eq!(response1.status(), StatusCode::OK);

    // Request 2 - Same key, same body
    let req2 = Request::builder()
        .method("POST")
        .uri("/")
        .header("X-Idempotency-Key", "key-2")
        .body(Body::from("Request Body A")) // Same body
        .unwrap();

    let response2 = app.clone().oneshot(req2).await.unwrap();

    // Expect 200 OK (cached)
    assert_eq!(response2.status(), StatusCode::OK);

    // Check header to verify it was cached
    assert!(response2.headers().contains_key("Idempotent-Replayed"));
}
