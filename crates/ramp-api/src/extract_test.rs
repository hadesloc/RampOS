use super::*;
use axum::{
    body::Body,
    http::{Request as HttpRequest, StatusCode},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower::ServiceExt;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, Serialize)]
struct TestPayload {
    #[validate(length(min = 1, message = "Name cannot be empty"))]
    name: String,
    #[validate(range(min = 18, message = "Age must be at least 18"))]
    age: u32,
}

async fn test_handler(ValidatedJson(payload): ValidatedJson<TestPayload>) -> Json<TestPayload> {
    Json(payload)
}

#[tokio::test]
async fn test_valid_payload() {
    let app: Router = Router::new().route("/test", post(test_handler));

    let payload = TestPayload {
        name: "Alice".to_string(),
        age: 25,
    };

    let response = app
        .oneshot(
            HttpRequest::builder()
                .method("POST")
                .uri("/test")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&payload).expect("serialization failed"),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_invalid_payload_empty_name() {
    let app: Router = Router::new().route("/test", post(test_handler));

    let payload = TestPayload {
        name: "".to_string(), // Invalid: too short
        age: 25,
    };

    let response = app
        .oneshot(
            HttpRequest::builder()
                .method("POST")
                .uri("/test")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&payload).expect("serialization failed"),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Check response body contains validation error
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("VALIDATION_ERROR"));
    assert!(body_str.contains("name"));
}

#[tokio::test]
async fn test_invalid_payload_age_too_low() {
    let app: Router = Router::new().route("/test", post(test_handler));

    let payload = TestPayload {
        name: "Bob".to_string(),
        age: 15, // Invalid: under 18
    };

    let response = app
        .oneshot(
            HttpRequest::builder()
                .method("POST")
                .uri("/test")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&payload).expect("serialization failed"),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("VALIDATION_ERROR"));
    assert!(body_str.contains("age"));
}

#[tokio::test]
async fn test_invalid_json_syntax() {
    let app: Router = Router::new().route("/test", post(test_handler));

    let response = app
        .oneshot(
            HttpRequest::builder()
                .method("POST")
                .uri("/test")
                .header("content-type", "application/json")
                .body(Body::from("invalid json {"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("INVALID_JSON"));
}

#[tokio::test]
async fn test_missing_required_field() {
    let app: Router = Router::new().route("/test", post(test_handler));

    // Missing 'age' field
    let json_body = r#"{"name": "Alice"}"#;

    let response = app
        .oneshot(
            HttpRequest::builder()
                .method("POST")
                .uri("/test")
                .header("content-type", "application/json")
                .body(Body::from(json_body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("INVALID_JSON") || body_str.contains("missing field"));
}

#[tokio::test]
async fn test_wrong_content_type() {
    let app: Router = Router::new().route("/test", post(test_handler));

    let response = app
        .oneshot(
            HttpRequest::builder()
                .method("POST")
                .uri("/test")
                .header("content-type", "text/plain")
                .body(Body::from(r#"{"name": "Alice", "age": 25}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_multiple_validation_errors() {
    let app: Router = Router::new().route("/test", post(test_handler));

    let payload = TestPayload {
        name: "".to_string(), // Invalid: too short
        age: 10,              // Invalid: under 18
    };

    let response = app
        .oneshot(
            HttpRequest::builder()
                .method("POST")
                .uri("/test")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&payload).expect("serialization failed"),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("VALIDATION_ERROR"));
    // Should report multiple fields
    assert!(body_str.contains("name"));
    assert!(body_str.contains("age"));
}
