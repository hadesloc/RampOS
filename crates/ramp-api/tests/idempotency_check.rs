use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::{self, from_fn_with_state},
    routing::post,
    Router,
};
use ramp_api::middleware::{
    IdempotencyConfig, IdempotencyHandler, idempotency_middleware,
};
use std::sync::Arc;
use tower::ServiceExt; // for oneshot

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

    let body1 = axum::body::to_bytes(response1.into_body(), usize::MAX).await.unwrap();
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
