use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};
use uuid::Uuid;

const REQUEST_ID_HEADER: &str = "X-Request-Id";

/// Middleware to add request ID to all requests
pub async fn request_id_middleware(mut req: Request, next: Next) -> Response {
    // Check if request already has an ID
    let request_id = req
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::now_v7().to_string());

    // Add to request headers
    req.headers_mut().insert(
        REQUEST_ID_HEADER,
        HeaderValue::from_str(&request_id).unwrap_or_else(|_| HeaderValue::from_static("unknown")),
    );

    // Run request
    let mut response = next.run(req).await;

    // Add to response headers
    response.headers_mut().insert(
        REQUEST_ID_HEADER,
        HeaderValue::from_str(&request_id).unwrap_or_else(|_| HeaderValue::from_static("unknown")),
    );

    response
}
