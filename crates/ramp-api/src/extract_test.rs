#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        // async_trait, // Unused
        body::Body,
        // extract::{FromRequest, Request}, // Unused
        http::{StatusCode, Request as HttpRequest},
        Json,
        routing::post,
        Router,
    };
    use serde::{Deserialize, Serialize};
    use tower::ServiceExt;
    use validator::Validate;

    use crate::extract::ValidatedJson;

    #[derive(Debug, Deserialize, Validate, Serialize)]
    struct TestPayload {
        #[validate(length(min = 1))]
        name: String,
        #[validate(range(min = 18))]
        age: u32,
    }

    async fn test_handler(ValidatedJson(payload): ValidatedJson<TestPayload>) -> Json<TestPayload> {
        Json(payload)
    }

    #[tokio::test]
    async fn test_valid_payload() {
        let app = Router::new().route("/test", post(test_handler));

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
                    .body(Body::from(serde_json::to_string(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_invalid_payload() {
        let app = Router::new().route("/test", post(test_handler));

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
                    .body(Body::from(serde_json::to_string(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_invalid_json() {
        let app = Router::new().route("/test", post(test_handler));

        let response = app
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/test")
                    .header("content-type", "application/json")
                    .body(Body::from("invalid json"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
