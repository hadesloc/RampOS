//! Error sanitizer middleware
//!
//! Maps internal errors (DB errors, provider errors) to generic client-safe messages.
//! Logs full error details server-side with request_id.
//! Never leaks stack traces, SQL errors, or internal paths to clients.

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use tracing::error;

const REQUEST_ID_HEADER: &str = "X-Request-Id";

#[derive(Serialize)]
struct SanitizedErrorResponse {
    error: SanitizedErrorBody,
}

#[derive(Serialize)]
struct SanitizedErrorBody {
    code: String,
    message: String,
    request_id: String,
}

/// Middleware that intercepts 5xx responses and replaces their bodies with
/// sanitized, client-safe error messages while logging full details server-side.
pub async fn error_sanitizer_middleware(req: Request, next: Next) -> Response {
    let request_id = req
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let method = req.method().clone();
    let uri = req.uri().path().to_string();

    let response = next.run(req).await;
    let status = response.status();

    // Only sanitize 5xx server errors
    if status.is_server_error() {
        error!(
            request_id = %request_id,
            status = %status.as_u16(),
            method = %method,
            path = %uri,
            "Internal server error occurred - details sanitized from client response"
        );

        let sanitized = SanitizedErrorResponse {
            error: SanitizedErrorBody {
                code: "INTERNAL_ERROR".to_string(),
                message: "An internal error occurred. Please try again later.".to_string(),
                request_id: request_id.clone(),
            },
        };

        return (StatusCode::INTERNAL_SERVER_ERROR, Json(sanitized)).into_response();
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, middleware, routing::get, Router};
    use tower::ServiceExt;

    async fn handler_ok() -> &'static str {
        "ok"
    }

    async fn handler_500() -> impl IntoResponse {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "SQL error: connection refused at pg://localhost:5432/rampos",
        )
    }

    async fn handler_400() -> impl IntoResponse {
        (StatusCode::BAD_REQUEST, "bad request from user")
    }

    fn test_app() -> Router {
        Router::new()
            .route("/ok", get(handler_ok))
            .route("/err", get(handler_500))
            .route("/bad", get(handler_400))
            .layer(middleware::from_fn(error_sanitizer_middleware))
    }

    #[tokio::test]
    async fn test_200_passes_through() {
        let app = test_app();
        let req = Request::builder()
            .uri("/ok")
            .header(REQUEST_ID_HEADER, "test-req-1")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_500_is_sanitized() {
        let app = test_app();
        let req = Request::builder()
            .uri("/err")
            .header(REQUEST_ID_HEADER, "test-req-2")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        // Must NOT contain the original SQL error
        assert!(!body_str.contains("SQL error"));
        assert!(!body_str.contains("pg://"));
        assert!(!body_str.contains("connection refused"));

        // Must contain sanitized response
        assert!(body_str.contains("INTERNAL_ERROR"));
        assert!(body_str.contains("test-req-2"));
    }

    #[tokio::test]
    async fn test_400_passes_through() {
        let app = test_app();
        let req = Request::builder()
            .uri("/bad")
            .header(REQUEST_ID_HEADER, "test-req-3")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
