mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::setup_portal_app;
use sqlx::PgPool;
use tower::ServiceExt;

#[tokio::test]
async fn excluded_portal_auth_methods_are_not_routable() {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => return,
    };
    let pool = PgPool::connect(&database_url)
        .await
        .expect("database connection should succeed");
    let app = setup_portal_app(pool, "portal-auth-scope-test-secret");

    for path in [
        "/v1/auth/webauthn/register/challenge",
        "/v1/auth/webauthn/register/complete",
        "/v1/auth/webauthn/login/challenge",
        "/v1/auth/webauthn/login/complete",
        "/v1/auth/magic-link",
        "/v1/auth/magic-link/verify",
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .method("POST")
                    .header("Content-Type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "{path} must stay outside the core auth surface"
        );
    }
}
